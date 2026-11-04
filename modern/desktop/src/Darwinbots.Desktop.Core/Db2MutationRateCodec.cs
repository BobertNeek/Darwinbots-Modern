using System.Globalization;

namespace Darwinbots.Desktop.Core;

/// <summary>Original HDRoutines.Save_mrates/Load_mrates: two Integer preferences and eleven Single triples.</summary>
public static class Db2MutationRateCodec
{
    private const int FieldCount = 35;

    public static byte[] Encode(MutationProfileUpdate profile)
    {
        ArgumentNullException.ThrowIfNull(profile);
        ValidatePreference(profile.PointValueChangePercent, "point");
        ValidatePreference(profile.CopyErrorValueChangePercent, "copy-error");
        var fields = new string[FieldCount];
        fields[0] = profile.PointValueChangePercent.ToString(CultureInfo.InvariantCulture);
        fields[1] = profile.CopyErrorValueChangePercent.ToString(CultureInfo.InvariantCulture);
        var rates = new[]
        {
            profile.Point, profile.MinorDeletion, profile.Reversal, profile.Insertion,
            profile.Amplification, profile.MajorDeletion, profile.CopyError, profile.Delta,
            profile.Translocation, profile.Point2, profile.CopyError2,
        };
        for (var index = 0; index < rates.Length; index++)
        {
            var rate = rates[index]
                ?? throw new InvalidDataException($"Mutation-rate triple {index} is missing.");
            fields[2 + 3 * index] = NativeSingle(rate.OneIn, index, "rate", nonnegative: true)
                .ToString("R", CultureInfo.InvariantCulture);
            fields[3 + 3 * index] = NativeSingle(rate.Mean, index, "mean", nonnegative: false)
                .ToString("R", CultureInfo.InvariantCulture);
            fields[4 + 3 * index] = NativeSingle(rate.StdDev, index, "standard deviation", nonnegative: true)
                .ToString("R", CultureInfo.InvariantCulture);
        }
        return LegacyDnaText.Encode(string.Join("\r\n", fields) + "\r\n");
    }

    public static MutationProfileUpdate Decode(ReadOnlySpan<byte> bytes, MutationProfileUpdate? basis = null)
    {
        // Accept conventional text terminators on import; original Write # always emits CRLF.
        var text = LegacyDnaText.Decode(bytes).Replace("\r\n", "\n", StringComparison.Ordinal).Replace('\r', '\n');
        if (text.EndsWith('\n')) text = text[..^1];
        var fields = text.Split('\n');
        if (fields.Length != FieldCount)
            throw new InvalidDataException("Original mutation-rate files must contain exactly 35 numeric records.");
        var point = ParsePreference(fields[0], "point");
        var copy = ParsePreference(fields[1], "copy-error");
        var rates = new MutationRateUpdate[11];
        for (var index = 0; index < rates.Length; index++)
            rates[index] = new(
                ParseSingle(fields[2 + 3 * index], index, "rate", nonnegative: true),
                ParseSingle(fields[3 + 3 * index], index, "mean", nonnegative: false),
                ParseSingle(fields[4 + 3 * index], index, "standard deviation", nonnegative: true));

        // The sidecar has no enable flag or category distributions. Keep those caller-owned settings.
        return (basis ?? MutationProfileUpdate.Default) with
        {
            PointValueChangePercent = point,
            CopyErrorValueChangePercent = copy,
            Point = rates[0], MinorDeletion = rates[1], Reversal = rates[2], Insertion = rates[3],
            Amplification = rates[4], MajorDeletion = rates[5], CopyError = rates[6], Delta = rates[7],
            Translocation = rates[8], Point2 = rates[9], CopyError2 = rates[10],
        };
    }

    private static byte ParsePreference(string value, string name)
    {
        if (!byte.TryParse(value.Trim(), NumberStyles.Integer, CultureInfo.InvariantCulture, out var result))
            throw new InvalidDataException($"Original {name} mutation preference must be an integer from 0 to 100.");
        ValidatePreference(result, name);
        return result;
    }

    private static void ValidatePreference(byte value, string name)
    {
        if (value > 100)
            throw new InvalidDataException($"Original {name} mutation preference must be an integer from 0 to 100.");
    }

    private static double ParseSingle(string value, int index, string field, bool nonnegative)
    {
        if (!float.TryParse(value.Trim(), NumberStyles.Float, CultureInfo.InvariantCulture, out var result)
            || !float.IsFinite(result) || (nonnegative && result < 0))
            throw new InvalidDataException($"Mutation-rate triple {index} {field} is not a valid original Single.");
        return result;
    }

    private static float NativeSingle(double value, int index, string field, bool nonnegative)
    {
        var result = (float)value;
        if (!double.IsFinite(value) || !float.IsFinite(result) || (nonnegative && value < 0))
            throw new InvalidDataException($"Mutation-rate triple {index} {field} is not a valid original Single.");
        return result;
    }
}
