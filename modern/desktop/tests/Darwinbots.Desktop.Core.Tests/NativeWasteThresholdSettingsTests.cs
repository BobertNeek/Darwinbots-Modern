using System.Globalization;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class NativeWasteThresholdSettingsTests
{
    [Theory]
    [InlineData(0, -1)]
    [InlineData(-1, -1)]
    [InlineData(-32768, -32768)]
    [InlineData(1, 1)]
    [InlineData(1234, 1234)]
    [InlineData(32767, 32767)]
    public void OriginalSettingsRepresentThresholdsWithoutChangingModernDisabledZero(int modern, int native)
    {
        var settings = new DesktopSettings { Waste = new Db2WasteOptions(modern) };
        Assert.Equal(modern, DesktopSettings.FromJson(settings.ToJson()).Waste.AlzheimerThreshold);
        var bytes = Db2SettingsCodec.Encode(settings);
        Assert.Equal(modern, Db2SettingsCodec.Decode(bytes).Waste.AlzheimerThreshold);
        var text = LegacyDnaText.Decode(bytes);
        var nativeText = text[..text.IndexOf("#DARWINBOTS_MODERN_SETTINGS_V1#", StringComparison.Ordinal)];
        // SaveSett's zero-species tail starts at171; BadWastelevel is its next Integer.
        Assert.Equal(native.ToString(CultureInfo.InvariantCulture), nativeText.Split("\r\n")[172]);
        var imported = Db2SettingsCodec.Decode(LegacyDnaText.Encode(nativeText));
        Assert.Equal(native, imported.Waste.AlzheimerThreshold);
        Assert.Equal(32_000, imported.Waste.DefecationThreshold);
    }

    [Theory]
    [InlineData(-32769)]
    [InlineData(32768)]
    public void OriginalSettingsRejectAnAlzheimerThresholdOutsideTheVbIntegerRange(int threshold)
    {
        var settings = new DesktopSettings { Waste = new Db2WasteOptions(threshold) };
        var error = Assert.Throws<InvalidDataException>(() => Db2SettingsCodec.Encode(settings));
        Assert.Contains("signed 16-bit Integer", error.Message, StringComparison.Ordinal);
        Assert.Equal(threshold, DesktopSettings.FromJson(settings.ToJson()).Waste.AlzheimerThreshold);
    }

    [Fact]
    public void OriginalSettingsRejectCustomDefecationThresholdsWithoutChangingModernJson()
    {
        foreach (int threshold in new[] { 0, 200, 32_001, int.MaxValue })
        {
            var settings = new DesktopSettings { Waste = new Db2WasteOptions(-1, threshold) };
            var before = settings.ToJson();
            var error = Assert.Throws<InvalidDataException>(() => Db2SettingsCodec.Encode(settings));
            Assert.Contains("defecation threshold", error.Message, StringComparison.Ordinal);
            Assert.Contains("32000", error.Message, StringComparison.Ordinal);
            Assert.Equal(before, settings.ToJson());
            Assert.Equal(threshold, DesktopSettings.FromJson(before).Waste.DefecationThreshold);
        }
    }
}
