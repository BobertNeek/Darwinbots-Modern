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
}
