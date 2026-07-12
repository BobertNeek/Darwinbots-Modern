using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class DesktopSettingsTests
{
    [Fact]
    public void VersionOneSettingsRoundTrip()
    {
        var expected = new DesktopSettings { Backend = "Gpu", TicksPerUpdate = 4, SnapshotEveryTicks = 2 };
        Assert.Equal(expected, DesktopSettings.FromJson(expected.ToJson()));
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
}
