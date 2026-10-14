using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class NativeDelta2Tests
{
    [Fact]
    public void NativeParametersAndDisabledLifetimeIntervalSurviveSettingsRoundtrip()
    {
        var native = new NativeDelta2Settings { RateExponential = 2, RateLinear = 3,
            DistributionExponential = 5, DistributionLinear = 7, RateChancePercent = 80,
            DistributionChancePercent = 40, PreferenceStep = 9, MaximumRatePerBase = 111 };
        var settings = new GlobalSettings { AdvancedMutation = new GlobalAdvancedMutationSettings {
            Delta2Enabled = true, Delta2Interval = 0, NativeDelta2 = native,
        }};
        var restored = GlobalSettings.FromJson(settings.ToJson());
        Assert.Equal(native, restored.AdvancedMutation.NativeDelta2);
        Assert.Equal(0UL, restored.AdvancedMutation.Delta2Interval);
    }

    [Fact]
    public void NativeDefaultsMatchOriginalGlobalSettingsLoader()
    {
        var settings = new GlobalAdvancedMutationSettings();
        Assert.Equal(3000UL, settings.Delta2Interval);
        Assert.Equal(1071, settings.NormalizationDnaLength);
        Assert.Equal(1, settings.NativeDelta2.RateExponential);
        Assert.Equal(7, settings.NativeDelta2.DistributionExponential);
        Assert.Equal(100, settings.NativeDelta2.RateChancePercent);
        Assert.Equal(30, settings.NativeDelta2.DistributionChancePercent);
        Assert.Equal(15, settings.NativeDelta2.PreferenceStep);
    }
}
