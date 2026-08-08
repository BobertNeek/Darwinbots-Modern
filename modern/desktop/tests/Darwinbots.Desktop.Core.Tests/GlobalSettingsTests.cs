using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class GlobalSettingsTests
{
    [Fact]
    public void ViewModelExposesFourTypedTabsAndPersistsSettings()
    {
        var settings = new GlobalSettings
        {
            Main = new GlobalMainSettings
            {
                MainDirectory = @"D:\Darwinbots 2",
                NewSeed = true,
                SuppressBuoyancy = true,
            },
            AdvancedMutation = new GlobalAdvancedMutationSettings
            {
                Delta2Enabled = true,
                NormalizationEnabled = true,
                NormalizationDnaLength = 32000,
                EpigeneticOverloadThreshold = 12,
            },
            League = new GlobalLeagueSettings
            {
                OwnedByOfflineModes = true,
            },
            Evolution = new GlobalEvolutionSettings
            {
                Mode = EvolutionMode.Zerobot,
                StageLength = 24,
                CycleLength = 400,
            },
        };

        var viewModel = new GlobalSettingsViewModel(settings);

        Assert.Equal(
            new[]
            {
                GlobalSettingsTab.Main,
                GlobalSettingsTab.AdvancedMutation,
                GlobalSettingsTab.League,
                GlobalSettingsTab.Evolution,
            },
            viewModel.Tabs.Select(tab => tab.Tab));
        Assert.True(viewModel.League.IsTypedSeam);
        Assert.True(viewModel.League.OwnedByOfflineModes);

        var roundTrip = GlobalSettings.FromJson(settings.ToJson());

        Assert.Equal(settings, roundTrip);
        Assert.True(roundTrip.AdvancedMutation.Delta2Enabled);
        Assert.Equal(EvolutionMode.Zerobot, roundTrip.Evolution.Mode);
    }
}
