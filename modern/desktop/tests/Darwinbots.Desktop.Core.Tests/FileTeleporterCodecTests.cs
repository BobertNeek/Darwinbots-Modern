using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class FileTeleporterCodecTests
{
    [Fact]
    public void LegacyFileTeleporterPathsAndRecordsRoundTripThroughDboFiles()
    {
        var root = Path.Combine(Path.GetTempPath(), $"darwinbots-file-teleporter-{Guid.NewGuid():N}");
        var inbound = Path.Combine(root, "inbound");
        var outbound = Path.Combine(root, "outbound");
        Directory.CreateDirectory(inbound);
        Directory.CreateDirectory(outbound);

        try
        {
            var paths = FileTeleporterPaths.Create($"{inbound}{Path.DirectorySeparatorChar}", $"{outbound}{Path.DirectorySeparatorChar}");
            var record = new LegacySimulationFile(
                2,
                812,
                [16_000f, 12_000f],
                true,
                [new LegacySimulationSpecies("Legacy", false, 0xff62a844u)],
                [new LegacySimulationOrganism(
                    "start\nstop",
                    0,
                    false,
                    false,
                    false,
                    [12.5f, 34.5f],
                    [1.25f, -2.5f],
                    777,
                    91,
                    44,
                    321)]);
            var fileName = FileTeleporterPaths.RecordFileName(record.Tick, 7, 3);

            var writtenPath = FileTeleporterCodec.Write(paths.OutboundDirectory, fileName, record);
            var restored = FileTeleporterCodec.Read(writtenPath);

            Assert.Equal(Path.GetFullPath(outbound), paths.OutboundDirectory);
            Assert.Equal(fileName, Path.GetFileName(writtenPath));
            Assert.Equal(record.Tick, restored.Tick);
            Assert.Equal(record.WorldSize, restored.WorldSize);
            Assert.Equal(record.Species[0], restored.Species[0]);
            Assert.Equal(record.Organisms[0].Dna, restored.Organisms[0].Dna);
            Assert.Equal(record.Organisms[0].Position, restored.Organisms[0].Position);
            Assert.Equal(record.Organisms[0].Velocity, restored.Organisms[0].Velocity);
            Assert.Equal(record.Organisms[0].Energy, restored.Organisms[0].Energy);
            Assert.Equal(record.Organisms[0].Body, restored.Organisms[0].Body);
        }
        finally
        {
            if (Directory.Exists(root)) Directory.Delete(root, recursive: true);
        }
    }

    [Fact]
    public void LegacySimulationExtensionRoundTripsSettingsSpeciesOrganismsTiesTeleportersAndObstaclesThroughLoadBoundary()
    {
        var expected = new LegacySimulationFile(
            Version: 2,
            Tick: 812,
            WorldSize: [16_000f, 12_000f],
            ToroidalWorld: true,
            Species: [
                new LegacySimulationSpecies("Legacy", false, 0xff62a844u),
                new LegacySimulationSpecies("Algae", true, 0xff2b9b61u)],
            Organisms: [
                new LegacySimulationOrganism(
                    "start\nstore\nstop",
                    0,
                    false,
                    false,
                    true,
                    [12.5f, 34.5f],
                    [1.25f, -2.5f],
                    777,
                    91,
                    44,
                    321)],
            Settings: new DesktopSettings
            {
                Backend = "Auto",
                TicksPerUpdate = 20,
                OrganismCapacity = 44_000,
                WorldWidth = 16_000,
                WorldHeight = 12_000,
            },
            Ties: [new Db2OrganismTie(
                new Db2OrganismKey(7, 3),
                new Db2OrganismKey(12, 5),
                42.5f)],
            Teleporters: [new LegacySimulationTeleporter(
                "gate-a",
                [100f, 200f],
                [900f, 800f],
                true)],
            Obstacles: [new LegacySimulationObstacle(
                "rock-a",
                [300f, 400f],
                [80f, 120f],
                true)]);

        var restored = FileTeleporterCodec.Decode(FileTeleporterCodec.Encode(expected));

        Assert.Equal(expected.Settings!.ToJson(), restored.Settings!.ToJson());
        Assert.Equal(expected.Species, restored.Species);
        var organism = Assert.Single(restored.Organisms);
        Assert.Equal(expected.Organisms[0].Dna, organism.Dna);
        Assert.Equal(expected.Organisms[0].Species, organism.Species);
        Assert.Equal(expected.Organisms[0].Repopulating, organism.Repopulating);
        Assert.Equal(expected.Organisms[0].Position, organism.Position);
        Assert.Equal(expected.Organisms[0].Velocity, organism.Velocity);
        Assert.Equal(expected.Organisms[0].Energy, organism.Energy);
        Assert.Equal(expected.Organisms[0].Age, organism.Age);
        Assert.Equal(expected.Organisms[0].BirthCycle, organism.BirthCycle);
        Assert.Equal(expected.Organisms[0].Body, organism.Body);
        Assert.Equal(expected.Ties, restored.Ties);
        var teleporter = Assert.Single(restored.Teleporters!);
        Assert.Equal(expected.Teleporters![0].Name, teleporter.Name);
        Assert.Equal(expected.Teleporters[0].Entry, teleporter.Entry);
        Assert.Equal(expected.Teleporters[0].Exit, teleporter.Exit);
        Assert.Equal(expected.Teleporters[0].Enabled, teleporter.Enabled);
        var obstacle = Assert.Single(restored.Obstacles!);
        Assert.Equal(expected.Obstacles![0].Name, obstacle.Name);
        Assert.Equal(expected.Obstacles[0].Position, obstacle.Position);
        Assert.Equal(expected.Obstacles[0].Size, obstacle.Size);
        Assert.Equal(expected.Obstacles[0].Enabled, obstacle.Enabled);
    }
}
