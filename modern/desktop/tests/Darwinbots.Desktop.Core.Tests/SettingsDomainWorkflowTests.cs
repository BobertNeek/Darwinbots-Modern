using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class SettingsDomainWorkflowTests
{
    [Fact]
    public void SixTabWorkingCopyCommitsAndSerializesTheNativeSettingsPayload()
    {
        var initial = new WorldSetupOptions
        {
            Seed = 17,
            Species =
            [
                new SpeciesImport("Seeded", "start\nstop", false, 0xff239ac0, 12, 900, 4, true, 2.5f),
            ],
            Physics = Db2PhysicsOptions.Default with { Density = 1.2, Viscosity = 0.8 },
            Vegetation = Db2VegetationOptions.Default with { PondMode = true, Tides = 64 },
            GlobalSettings = new GlobalSettings
            {
                AdvancedMutation = new GlobalAdvancedMutationSettings
                {
                    OscillationMode = GlobalMutationOscillationMode.Sine,
                    OscillationPeriod = 120,
                    OscillationAmplitude = 0.25,
                },
            },
        };
        var workflow = new SimulationSettingsWorkflow(initial);

        workflow.UpdateWorkingCopy(current => current with
        {
            Seed = 23,
            Species = current.Species.Select(species => species with { InitialCount = 24 }).ToArray(),
        });

        Assert.Equal(17UL, workflow.Live.Seed);
        Assert.Equal(23UL, workflow.WorkingCopy.Seed);
        var committed = workflow.Change();
        Assert.Equal(23UL, committed.Seed);
        Assert.Equal(24, committed.Species[0].InitialCount);

        var engineJson = NativeCommandSerializer.SerializeEngineConfig(committed);
        var mutationJson = NativeCommandSerializer.SerializeMutationSettings(
            MutationSettingsUpdate.Default with
            {
                GlobalMultiplier = 2.5f,
                Profile = MutationProfileUpdate.Default with
                {
                    Delta = new MutationRateUpdate(17, 500, 150),
                    PointCategoryDistribution = new MutationCategoryDistributionUpdate(40, 20, 10, 10, 10, 10),
                },
            });

        var missing = new List<string>();
        foreach (var expected in new[]
                 {
                     "\"species\"", "\"initial_count\"", "\"initial_energy\"", "\"minimum_population\"",
                     "\"reseed\"", "\"color\"", "\"placement\"", "\"skin\"", "\"restrictions\"",
                     "\"corpse_decay\"", "\"waste\"", "\"sampling_interval\"", "\"reynolds\"", "\"oscillation\"",
                 })
        {
            if (!engineJson.Contains(expected, StringComparison.Ordinal))
                missing.Add(expected);
        }

        foreach (var expected in new[]
                 {
                     "\"point\"", "\"point2\"", "\"delta\"", "\"copy_error\"", "\"copy_error2\"",
                     "\"insertion\"", "\"reversal\"", "\"minor_deletion\"", "\"major_deletion\"",
                     "\"amplification\"", "\"translocation\"", "\"point_category_distribution\"",
                     "\"copy_error_category_distribution\"",
                 })
        {
            if (!mutationJson.Contains(expected, StringComparison.Ordinal))
                missing.Add(expected);
        }

        Assert.Empty(missing);
    }
}
