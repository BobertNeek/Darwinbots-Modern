using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class DesktopSettingsTests
{
    [Fact]
    public void VersionOneSettingsRoundTrip()
    {
        var expected = new DesktopSettings { Backend = "Gpu", TicksPerUpdate = 4, SnapshotEveryTicks = 2 };
        var actual = DesktopSettings.FromJson(expected.ToJson());
        Assert.Equal(expected.ToJson(), actual.ToJson());
    }

    [Fact]
    public void VersionZeroSettingsMigrateToCurrentSchema()
    {
        var settings = DesktopSettings.FromJson("""{"schemaVersion":0,"backend":"Cpu","snapshotEvery":5,"capacity":2000}""");
        Assert.Equal(DesktopSettings.CurrentSchemaVersion, settings.SchemaVersion);
        Assert.Equal("Cpu", settings.Backend);
        Assert.Equal(5U, settings.SnapshotEveryTicks);
        Assert.Equal(2000, settings.OrganismCapacity);
    }

    [Fact]
    public void FutureSettingsSchemasAreRejected()
    {
        var error = Assert.Throws<NotSupportedException>(() => DesktopSettings.FromJson("""{"schemaVersion":99}"""));
        Assert.Contains("version 99", error.Message);
    }

    [Fact]
    public void InvalidSettingsValuesAreRejected()
    {
        Assert.Throws<InvalidDataException>(() => DesktopSettings.FromJson("""{"schemaVersion":1,"backend":"Quantum","ticksPerUpdate":1,"snapshotEveryTicks":1,"organismCapacity":1,"worldWidth":1,"worldHeight":1}"""));
    }

    [Fact]
    public void LastExitUsesStartupDefinitionsInsteadOfTheLiveSpeciesFallback()
    {
        var liveSpecies = new SpeciesImport("Descendant", "start stop", false, 0xff123456, 1, 1000, 0, false, 0);
        var startupSpecies = liveSpecies with { Name = "Starter", SpeciesId = 5, InitialCount = 12 };
        var configuration = new WorldConfiguration(EnvironmentUpdate.Default, 1, 2000, 150, [liveSpecies, startupSpecies])
        {
            StartupSpecies = [startupSpecies],
        };

        var settings = DesktopSettings.FromLiveWorld(new WorldSetupOptions(), configuration, EngineSnapshot.Empty);

        var saved = Assert.Single(settings.Species);
        Assert.Equal("Starter", saved.Name);
        Assert.Equal(12, saved.InitialCount);
        Assert.Null(saved.SpeciesId);
        var empty = DesktopSettings.FromLiveWorld(new WorldSetupOptions(), configuration with { StartupSpecies = [] }, EngineSnapshot.Empty);
        Assert.Empty(empty.Species);
    }
}
