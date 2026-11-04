using System.Globalization;
using System.Text;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class Db2MutationRateCodecTests
{
    // Independent literal HDRoutines.Save_mrates record order: preferences, then slots 0..10.
    private const string Original = "13\r\n27\r\n"
        + "101.25\r\n1.5\r\n0.25\r\n"
        + "202.5\r\n-2.5\r\n0.5\r\n"
        + "303.75\r\n3.5\r\n0.75\r\n"
        + "404\r\n4.5\r\n1\r\n"
        + "505.25\r\n5.5\r\n1.25\r\n"
        + "606.5\r\n6.5\r\n1.5\r\n"
        + "707.75\r\n7.5\r\n1.75\r\n"
        + "808\r\n8.5\r\n2\r\n"
        + "909.25\r\n9.5\r\n2.25\r\n"
        + "1010.5\r\n10.5\r\n2.5\r\n"
        + "1111.75\r\n11.5\r\n2.75\r\n";

    [Fact]
    public void OriginalLiteralHasTheExactThirtyFiveFieldMapAndNativeBytes()
    {
        var bytes = Encoding.ASCII.GetBytes(Original);
        var expected = AsymmetricProfile();

        Assert.Equal(expected, Db2MutationRateCodec.Decode(bytes));
        Assert.Equal(bytes, Db2MutationRateCodec.Encode(expected));
    }

    [Fact]
    public void NumericFormattingIsIndependentOfTheCurrentCulture()
    {
        var savedCulture = CultureInfo.CurrentCulture;
        try
        {
            CultureInfo.CurrentCulture = CultureInfo.GetCultureInfo("fr-FR");
            Assert.Equal(Encoding.ASCII.GetBytes(Original), Db2MutationRateCodec.Encode(AsymmetricProfile()));
            Assert.Equal(AsymmetricProfile(), Db2MutationRateCodec.Decode(Encoding.ASCII.GetBytes(Original)));
        }
        finally { CultureInfo.CurrentCulture = savedCulture; }
    }

    [Fact]
    public void DecodeRetainsUnencodedCategoryDistributionsAndDoesNotChangeItsBasis()
    {
        var point = new MutationCategoryDistributionUpdate(100, 0, 0, 0, 0, 0);
        var copy = new MutationCategoryDistributionUpdate(0, 0, 100, 0, 0, 0);
        var basis = MutationProfileUpdate.Default with
        {
            PointCategoryDistribution = point,
            CopyErrorCategoryDistribution = copy,
        };
        var originalPoint = basis.Point;
        var decoded = Db2MutationRateCodec.Decode(Encoding.ASCII.GetBytes(Original), basis);

        Assert.Equal(AsymmetricProfile() with
        {
            PointCategoryDistribution = point,
            CopyErrorCategoryDistribution = copy,
        }, decoded);
        Assert.NotSame(basis, decoded);
        Assert.Same(point, decoded.PointCategoryDistribution);
        Assert.Same(copy, decoded.CopyErrorCategoryDistribution);
        Assert.Same(originalPoint, basis.Point);
        Assert.Equal(MutationProfileUpdate.Default.Point, basis.Point);
        Assert.Equal((byte)80, basis.PointValueChangePercent);
    }

    [Fact]
    public void EncodeUsesSinglePrecisionWithoutChangingTheSuppliedProfile()
    {
        var profile = AsymmetricProfile() with { Point = new(0.1, -0.1, 0.1) };
        var decoded = Db2MutationRateCodec.Decode(Db2MutationRateCodec.Encode(profile));

        Assert.Equal((double)(float)0.1, decoded.Point.OneIn);
        Assert.Equal((double)(float)-0.1, decoded.Point.Mean);
        Assert.Equal((double)(float)0.1, decoded.Point.StdDev);
        Assert.Equal(0.1, profile.Point.OneIn);
    }

    [Theory]
    [InlineData("\r\n")]
    [InlineData("\n")]
    [InlineData("\r")]
    public void ImportAcceptsConventionalTextTerminators(string newline)
    {
        var text = Original.Replace("\r\n", newline, StringComparison.Ordinal);
        Assert.Equal(AsymmetricProfile(), Db2MutationRateCodec.Decode(Encoding.ASCII.GetBytes(text)));
    }

    [Fact]
    public void AFinalLineTerminatorIsOptionalAndZeroRatesRemainDisabled()
    {
        var profile = MutationProfileUpdate.Default with
        {
            Point = new(0, 0, 0), PointValueChangePercent = 0, CopyErrorValueChangePercent = 100,
        };
        var bytes = Db2MutationRateCodec.Encode(profile);
        Assert.Equal(profile, Db2MutationRateCodec.Decode(bytes.AsSpan(0, bytes.Length - 2)));
    }

    [Theory]
    [InlineData(0)]
    [InlineData(34)]
    [InlineData(36)]
    public void ImportRejectsMissingOrExtraNumericRecords(int count)
    {
        var text = string.Join("\r\n", Enumerable.Repeat("0", count)) + "\r\n";
        Assert.Throws<InvalidDataException>(() => Db2MutationRateCodec.Decode(Encoding.ASCII.GetBytes(text)));
    }

    [Theory]
    [InlineData(0, "13.5")]
    [InlineData(0, "101")]
    [InlineData(1, "-1")]
    [InlineData(1, "#TRUE#")]
    [InlineData(2, "NaN")]
    [InlineData(2, "Infinity")]
    [InlineData(2, "1e39")]
    [InlineData(2, "-1")]
    [InlineData(3, "NaN")]
    [InlineData(4, "-0.25")]
    [InlineData(4, "")]
    [InlineData(34, "not-a-number")]
    public void ImportRejectsInvalidNativeNumericValues(int field, string value)
    {
        var fields = Original[..^2].Split("\r\n", StringSplitOptions.None);
        fields[field] = value;
        var bytes = Encoding.ASCII.GetBytes(string.Join("\r\n", fields) + "\r\n");
        Assert.Throws<InvalidDataException>(() => Db2MutationRateCodec.Decode(bytes));
    }

    [Theory]
    [InlineData(double.NaN)]
    [InlineData(double.PositiveInfinity)]
    [InlineData(double.NegativeInfinity)]
    [InlineData(double.MaxValue)]
    [InlineData(-1)]
    public void ExportRejectsUnrepresentableOrNegativeRates(double value)
    {
        var profile = AsymmetricProfile() with { Point = new(value, 1, 0) };
        Assert.Throws<InvalidDataException>(() => Db2MutationRateCodec.Encode(profile));
    }

    [Fact]
    public void ExportValidatesEveryTripleAndBothPreferencesBeforeReturningBytes()
    {
        Assert.Throws<InvalidDataException>(() => Db2MutationRateCodec.Encode(
            AsymmetricProfile() with { CopyError2 = new(1, 0, -1) }));
        Assert.Throws<InvalidDataException>(() => Db2MutationRateCodec.Encode(
            AsymmetricProfile() with { Delta = new(1, double.NaN, 0) }));
        Assert.Throws<InvalidDataException>(() => Db2MutationRateCodec.Encode(
            AsymmetricProfile() with { PointValueChangePercent = 101 }));
        Assert.Throws<InvalidDataException>(() => Db2MutationRateCodec.Encode(
            AsymmetricProfile() with { CopyErrorValueChangePercent = 101 }));
    }

    private static MutationProfileUpdate AsymmetricProfile() => MutationProfileUpdate.Default with
    {
        PointValueChangePercent = 13, CopyErrorValueChangePercent = 27,
        Point = new(101.25, 1.5, 0.25),
        MinorDeletion = new(202.5, -2.5, 0.5),
        Reversal = new(303.75, 3.5, 0.75),
        Insertion = new(404, 4.5, 1),
        Amplification = new(505.25, 5.5, 1.25),
        MajorDeletion = new(606.5, 6.5, 1.5),
        CopyError = new(707.75, 7.5, 1.75),
        Delta = new(808, 8.5, 2),
        Translocation = new(909.25, 9.5, 2.25),
        Point2 = new(1010.5, 10.5, 2.5),
        CopyError2 = new(1111.75, 11.5, 2.75),
    };
}
