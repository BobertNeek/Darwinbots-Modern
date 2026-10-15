using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class GlobalSettingsTests
{
    [Fact]
    public void GlobalDisqualificationRuleRoundTripsAndRejectsUnknownRules()
    {
        var settings = new GlobalSettings { League = new GlobalLeagueSettings { DisqualificationRule = 2 } };
        Assert.Equal(2, GlobalSettings.FromJson(settings.ToJson()).League.DisqualificationRule);
        var invalid = settings with { League = settings.League with { DisqualificationRule = 3 } };
        Assert.Throws<InvalidDataException>(() => GlobalSettings.FromJson(invalid.ToJson()));
    }

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
                SaveAsTemporaryGene = true,
            },
            Costs = new GlobalSimulationCostSettings
            {
                GlobalMultiplier = 1.75,
                DnaUpkeep = 2.5,
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

        var createConfig = NativeCommandSerializer.SerializeEngineConfig(new WorldSetupOptions
        {
            GlobalSettings = settings,
        });

        Assert.Contains("\"dna_costs\"", createConfig);
        Assert.Contains("\"dna_upkeep\":2.5", createConfig);
        Assert.Contains("\"global_multiplier\":1.75", createConfig);
        Assert.Contains("\"advanced_mutation\"", createConfig);
        Assert.Contains("\"delta2\"", createConfig);
        Assert.Contains("\"rate_step\":1", createConfig);
        Assert.Contains("\"save_as_temporary_gene\":true", createConfig);
    }
    [Theory]
    [InlineData(0)]
    [InlineData(50)]
    [InlineData(100)]
    [InlineData(150)]
    [InlineData(200)]
    public void FindBestWeightingPreservesOriginalRange(int weight)
    {
        var settings = new GlobalSettings { Main = new GlobalMainSettings { FindBestWeight = weight } };
        Assert.Equal(weight, GlobalSettings.FromJson(settings.ToJson()).Main.EffectiveFindBestWeight);
    }

    [Fact]
    public void FindBestLegacyBooleanMigratesAndInvalidWeightsAreRejected()
    {
        Assert.Equal(100, new GlobalMainSettings { FindBestEnergyWeighting = true }.EffectiveFindBestWeight);
        Assert.Equal(100, new GlobalMainSettings().EffectiveFindBestWeight);
        Assert.Equal(0, new GlobalMainSettings { FindBestEnergyWeighting = false }.EffectiveFindBestWeight);
        var invalid = new GlobalSettings { Main = new GlobalMainSettings { FindBestWeight = 201 } };
        Assert.Throws<InvalidDataException>(() => GlobalSettings.FromJson(invalid.ToJson()));
    }

}
