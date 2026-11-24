using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class Db2RgbMonitorPresetCodecTests
{
    private const string LegacyHex = "4D001302E703008318FB20006400E804007D";

    [Fact]
    public void PresetsUseOriginalSignedLittleEndianFieldOrder()
    {
        var expected = new RgbMemoryMonitorSettings(new(77, -32_000, 100), new(531, -1_256, 1_256), new(999, 32, 32_000));

        Assert.Equal(expected, Db2RgbMonitorPresetCodec.Decode(Convert.FromHexString(LegacyHex)));
        Assert.Equal(expected, Db2RgbMonitorPresetCodec.Decode(Convert.FromHexString(LegacyHex + "AAFF")));
        Assert.Equal(Convert.FromHexString(LegacyHex), Db2RgbMonitorPresetCodec.Encode(expected));
    }

    [Theory]
    [InlineData(0, 0)] // zero address
    [InlineData(1, 255)] // negative address
    [InlineData(4, 232)] // address 1000
    [InlineData(7, 128)] // floor -32768
    [InlineData(17, 0)] // blue ceiling below its floor
    public void PresetsRejectInvalidChannelValues(int offset, byte value)
    {
        var bytes = Convert.FromHexString(LegacyHex);
        bytes[offset] = value;
        Assert.Throws<InvalidDataException>(() => Db2RgbMonitorPresetCodec.Decode(bytes));
    }

    [Fact]
    public void PresetsRequireAllNineFields()
    {
        var truncated = Convert.FromHexString(LegacyHex)[..^1];
        Assert.Throws<InvalidDataException>(() => Db2RgbMonitorPresetCodec.Decode(truncated));
    }
}
