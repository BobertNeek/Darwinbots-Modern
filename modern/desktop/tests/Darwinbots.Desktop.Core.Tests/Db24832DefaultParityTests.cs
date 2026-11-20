using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class Db24832DefaultParityTests
{
    [Fact]
    public void CleanStartDomainDefaultsMatchDb24832()
    {
        var update = EnvironmentUpdate.Default;

        Assert.Equal(60f, update.Physics.MaxVelocity);
        Assert.Equal(0.66f, update.Physics.MovementEfficiency);
        Assert.Equal(0f, update.Physics.SurfaceGravity);
        Assert.Equal(0f, update.Physics.StaticFriction);
        Assert.Equal(0f, update.Physics.KineticFriction);
        Assert.Equal(0.5f, update.BrownianMotion);
        Assert.False(update.CorpseDecay!.Enabled);
        Assert.Equal(Db2CorpseDecayMode.None, update.CorpseDecay.Mode);
        Assert.Equal(15, update.CorpseDecay.DeathEnergyThreshold);
        Assert.Equal(0, update.CorpseDecay.PacketSize);
        Assert.Equal(0UL, update.CorpseDecay.Period);
        Assert.Equal(200, update.Recording!.SamplingInterval);
        Assert.Equal(16_000, update.Vegetation.StartChloroplasts);
        Assert.Equal(100, update.Vegetation.MaxEnergyPerTick);
        Assert.Equal(50, update.Vegetation.MinimumChloroplastEquivalents);
        Assert.Equal(10UL, update.Vegetation.RepopulationCooldown);
        Assert.Equal(0.75f, update.Vegetation.FeedingToBody);
        Assert.Equal(1.02f, update.Vegetation.Gradient);
    }

    [Fact]
    public void SetupAndPersistedSettingsMirrorDomainDefaultsAndPreserveModernization()
    {
        var setup = new WorldSetupOptions();
        var settings = new DesktopSettings();

        var setupUpdate = setup.ToEnvironmentUpdate();
        Assert.Equal(EnvironmentUpdate.Default.BrownianMotion, setupUpdate.BrownianMotion);
        Assert.Equal(EnvironmentUpdate.Default.Physics, setupUpdate.Physics);
        Assert.Equal(EnvironmentUpdate.Default.Vegetation, setupUpdate.Vegetation);
        Assert.Equal(EnvironmentUpdate.Default.CorpseDecay, setupUpdate.CorpseDecay);
        Assert.Equal(EnvironmentUpdate.Default.Recording, setupUpdate.Recording);
        Assert.Equal(25_000, setup.PopulationCapacity);
        Assert.Equal(25_000, settings.OrganismCapacity);
        Assert.Equal("Auto", setup.Backend);
        Assert.Equal("Auto", settings.Backend);
        Assert.Equal(500, settings.VegetablePopulationCap);
        Assert.True(settings.AutomaticZerobotProgression);

        var roundTrip = DesktopSettings.FromJson(settings.ToJson());
        Assert.Equal(settings.Backend, roundTrip.Backend);
        Assert.Equal(settings.OrganismCapacity, roundTrip.OrganismCapacity);
        Assert.Equal(settings.Physics, roundTrip.Physics);
        Assert.Equal(settings.Vegetation, roundTrip.Vegetation);
        Assert.Equal(settings.Recording, roundTrip.Recording);
    }

    [Fact]
    public void NativeSerializationCarriesDb24832Defaults()
    {
        var json = NativeCommandSerializer.SerializeEnvironment(EnvironmentUpdate.Default);

        Assert.Contains("\"max_velocity\":60", json);
        Assert.Contains("\"brownian_motion\":0.5", json);
        Assert.Contains("\"enabled\":false", json);
        Assert.Contains("\"mode\":\"none\"", json);
        Assert.Contains("\"sampling_interval\":200", json);
        Assert.Contains("\"max_energy_per_tick\":100", json);
        Assert.Contains("\"minimum_chloroplast_equivalents\":50", json);
        Assert.Contains("\"gradient\":1.02", json);
    }
}
