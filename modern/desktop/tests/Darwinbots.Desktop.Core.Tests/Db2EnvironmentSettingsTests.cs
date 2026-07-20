using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class Db2EnvironmentSettingsTests
{
    [Fact]
    public void StarterModeUsesNormalMetabolismEvenWhenZerobotSustenanceDefaultsToDisabled()
    {
        var options = new WorldSetupOptions
        {
            StartingMode = StartingMode.StarterBotsAndVegetables,
            ZerobotSustenance = ZerobotSustenance.DisabledMetabolism,
            MetabolismCost = 1,
        };

        Assert.Equal(1, options.EffectiveMetabolismCost);
    }

    [Fact]
    public void ZerobotModeCanDisableMetabolism()
    {
        var options = new WorldSetupOptions
        {
            StartingMode = StartingMode.Zerobots,
            ZerobotSustenance = ZerobotSustenance.DisabledMetabolism,
            MetabolismCost = 1,
        };

        Assert.Equal(0, options.EffectiveMetabolismCost);
    }

    [Fact]
    public void Db2SettingsSerializeIntoVersionedEnvironmentCommand()
    {
        var json = NativeCommandSerializer.SerializeEnvironment(EnvironmentUpdate.Default);

        Assert.Contains("\"max_velocity\":180", json);
        Assert.Contains("\"movement_efficiency\":0.66", json);
        Assert.Contains("\"speed\":40", json);
        Assert.Contains("\"start_chloroplasts\":16000", json);
    }

    [Fact]
    public void AutomaticSpeciationSettingsSerializeIntoEnvironmentCommand()
    {
        var update = EnvironmentUpdate.Default with
        {
            AutoSpeciation = true,
            SpeciationGeneticDistancePercent = 12.5f,
        };

        var json = NativeCommandSerializer.SerializeEnvironment(update);

        Assert.Contains("\"auto_speciation\":true", json);
        Assert.Contains("\"speciation_genetic_distance_percent\":12.5", json);
    }

    [Fact]
    public void DesktopDefaultsMatchDb2NormalSimulationDefaults()
    {
        var update = EnvironmentUpdate.Default;

        Assert.Equal(0f, update.BrownianMotion);
        Assert.Equal(new Db2PhysicsOptions(180f, 0.66f, 2f, 0.6f, 0.4f, 0d, 0d, 0f), update.Physics);
        Assert.Equal(new Db2ShotOptions(40f, 1f, 40f, false, false), update.Shots);
        Assert.Equal(new Db2VegetationOptions(16_000, 40, 10, 10, 25, 0.5f, true, false, 10_000), update.Vegetation);
        Assert.False(update.AutoSpeciation);
        Assert.Equal(20f, update.SpeciationGeneticDistancePercent);

        var setup = new WorldSetupOptions();
        Assert.Equal(16_000f, setup.WorldWidth);
        Assert.Equal(12_000f, setup.WorldHeight);
        Assert.Equal(0f, setup.BrownianMotion);
        Assert.Equal(update.Physics, setup.Physics);
        Assert.Equal(update.Shots, setup.Shots);
        Assert.Equal(update.Vegetation, setup.Vegetation);
        Assert.False(setup.AutoSpeciation);
        Assert.Equal(20f, setup.SpeciationGeneticDistancePercent);
    }
}
