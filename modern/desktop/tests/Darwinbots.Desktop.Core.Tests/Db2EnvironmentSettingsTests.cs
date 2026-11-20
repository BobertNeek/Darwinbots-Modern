using System.Text.Json;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class Db2EnvironmentSettingsTests
{
    [Fact]
    public void ShapeVisibilitySurvivesSettingsAndEnvironmentCommandsIndependentlyOfOpacity()
    {
        var setup = new WorldSetupOptions { ShapesVisible = true };
        using var engine = JsonDocument.Parse(NativeCommandSerializer.SerializeEngineConfig(setup));
        Assert.True(engine.RootElement.GetProperty("shapes_visible").GetBoolean());
        var settings = DesktopSettings.FromWorldSetup(setup);
        Assert.True(settings.ShapesVisible);
        Assert.True(settings.ToWorldSetup(setup).ShapesVisible);
        using var environment = JsonDocument.Parse(NativeCommandSerializer.SerializeEnvironment(
            setup.ToEnvironmentUpdate() with { ShapesVisible = false }));
        Assert.False(environment.RootElement.GetProperty("commands")[0].GetProperty("shapes_visible").GetBoolean());
    }
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

        Assert.Contains("\"max_velocity\":60", json);
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
    public void EnergyWeatherThresholdAndAxisTopologySettingsSerializeIntoEnvironmentCommand()
    {
        var vegetation = EnvironmentUpdate.Default.Vegetation with
        {
            Tides = 64,
            TidesOffset = 7,
            WeatherEnabled = true,
            SunUpEnabled = true,
            SunUpThreshold = 500,
            SunDownEnabled = true,
            SunDownThreshold = 1_000,
            SunThresholdMode = 2,
        };
        var json = NativeCommandSerializer.SerializeEnvironment(EnvironmentUpdate.Default with
        {
            Vegetation = vegetation,
            WrapX = true,
            WrapY = false,
        });

        Assert.Contains("\"tides\":64", json);
        Assert.Contains("\"tides_offset\":7", json);
        Assert.Contains("\"weather_enabled\":true", json);
        Assert.Contains("\"sun_up_threshold\":500", json);
        Assert.Contains("\"sun_down_threshold\":1000", json);
        Assert.Contains("\"sun_threshold_mode\":2", json);
        Assert.Contains("\"axis_topology\":[true,false]", json);
    }

    [Fact]
    public void NativeRemoveManyBatchSerializesExactStableKeys()
    {
        OrganismKey[] organisms = [new(3, 2), new(7, 5)];

        var json = JsonSerializer.Serialize(NativeCommandSerializer.CreateRemoveManyBatch(organisms));

        using var document = JsonDocument.Parse(json);
        var root = document.RootElement;
        Assert.Equal(1, root.GetProperty("version").GetInt32());
        var command = Assert.Single(root.GetProperty("commands").EnumerateArray());
        Assert.Equal("remove_many", command.GetProperty("type").GetString());
        Assert.False(command.TryGetProperty("suppress_death_poffs", out _));
        Assert.Equal(json, JsonSerializer.Serialize(NativeCommandSerializer.CreateRemoveManyBatch(organisms, false)));
        var serializedOrganisms = command.GetProperty("organisms").EnumerateArray().ToArray();
        Assert.Equal(2, serializedOrganisms.Length);
        Assert.Equal(3U, serializedOrganisms[0].GetProperty("slot").GetUInt32());
        Assert.Equal(2U, serializedOrganisms[0].GetProperty("generation").GetUInt32());
        Assert.Equal(7U, serializedOrganisms[1].GetProperty("slot").GetUInt32());
        Assert.Equal(5U, serializedOrganisms[1].GetProperty("generation").GetUInt32());
    }

    [Fact]
    public void DeathParticleCommandsUsePositivePolicyAndOnlyExplicitBulkSuppression()
    {
        foreach (var enabled in new[] { false, true })
        {
            using var document = JsonDocument.Parse(JsonSerializer.Serialize(NativeCommandSerializer.CreateDeathPoffsBatch(enabled)));
            Assert.Equal(1, document.RootElement.GetProperty("version").GetInt32());
            var command = Assert.Single(document.RootElement.GetProperty("commands").EnumerateArray());
            Assert.Equal("set_death_poffs_enabled", command.GetProperty("type").GetString());
            Assert.Equal(enabled, command.GetProperty("enabled").GetBoolean());
            Assert.Equal(2, command.EnumerateObject().Count());
        }
        OrganismKey[] organisms = [new(3, 2), new(7, 5)];
        using var suppressed = JsonDocument.Parse(JsonSerializer.Serialize(NativeCommandSerializer.CreateRemoveManyBatch(organisms, true)));
        var removal = Assert.Single(suppressed.RootElement.GetProperty("commands").EnumerateArray());
        Assert.Equal("remove_many", removal.GetProperty("type").GetString());
        Assert.True(removal.GetProperty("suppress_death_poffs").GetBoolean());
        Assert.Equal(new[] { 3U, 7U }, removal.GetProperty("organisms").EnumerateArray().Select(value => value.GetProperty("slot").GetUInt32()));
        Assert.Equal(new[] { 2U, 5U }, removal.GetProperty("organisms").EnumerateArray().Select(value => value.GetProperty("generation").GetUInt32()));
    }

    [Fact]
    public void DesktopDefaultsMatchDb2NormalSimulationDefaults()
    {
        var update = EnvironmentUpdate.Default;

        Assert.Equal(0.5f, update.BrownianMotion);
        Assert.Equal(new Db2PhysicsOptions(60f, 0.66f, 0f, 0f, 0f, 0d, 0d, 0f), update.Physics);
        Assert.Equal(new Db2ShotOptions(40f, 1f, 40f, false, false), update.Shots);
        Assert.Equal(new Db2VegetationOptions(16_000, 100, 50, 10, 10, 0.75f, true, false, 10_000, Gradient: 1.02f), update.Vegetation);
        Assert.False(update.AutoSpeciation);
        Assert.Equal(20f, update.SpeciationGeneticDistancePercent);

        var setup = new WorldSetupOptions();
        Assert.Equal(16_000f, setup.WorldWidth);
        Assert.Equal(12_000f, setup.WorldHeight);
        Assert.Equal(0.5f, setup.BrownianMotion);
        Assert.Equal(update.Physics, setup.Physics);
        Assert.Equal(update.Shots, setup.Shots);
        Assert.Equal(update.Vegetation, setup.Vegetation);
        Assert.False(setup.AutoSpeciation);
        Assert.Equal(20f, setup.SpeciationGeneticDistancePercent);
    }
}
