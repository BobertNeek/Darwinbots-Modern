using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class NativeMutationSettingsTests
{
    [Fact]
    public void OriginalDoseAndSeparateOscillationDurationsArePersistedAndSent()
    {
        var settings = new GlobalSettings { AdvancedMutation = new GlobalAdvancedMutationSettings {
            EpigeneticReset = true, EpigeneticExponent = 1.7, EpigeneticOverloadThreshold = 21,
            OscillationMode = GlobalMutationOscillationMode.Sine, OscillationHighCycles = 123,
            OscillationLowCycles = 456, SaveAsTemporaryGene = true,
        }};
        Assert.Equal(settings, GlobalSettings.FromJson(settings.ToJson()));
        var json = NativeCommandSerializer.SerializeEngineConfig(new WorldSetupOptions { GlobalSettings = settings });
        Assert.Contains("\"native_exponent\":1.7", json);
        Assert.Contains("\"high_cycles\":123", json);
        Assert.Contains("\"low_cycles\":456", json);
        var defaults = new GlobalAdvancedMutationSettings();
        Assert.Equal(1.3, defaults.EpigeneticExponent);
        Assert.Equal(17UL, defaults.EpigeneticOverloadThreshold);
    }

    [Fact]
    public void InvalidDoseExponentIsRejected()
    {
        var invalid = new GlobalSettings { AdvancedMutation = new GlobalAdvancedMutationSettings { EpigeneticExponent = -1 } };
        Assert.Throws<InvalidDataException>(() => GlobalSettings.FromJson(invalid.ToJson()));
    }
}
