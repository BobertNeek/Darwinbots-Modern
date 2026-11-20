using Darwinbots.Desktop.Core.LegacyMenus;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class LegacyRecordingHelpTests
{
    [Fact]
    public void LegacyRecordingFormatAndLocalHelpRoutesResolveWithoutBrowserAccess()
    {
        var organism = new OrganismSnapshot(7, 3, [12.5f, 34.5f], [1f, -2f], 875, 19, Body: 1)
        {
            UniqueId = 7003,
            DnaLength = 2,
        };
        var snapshot = new EngineSnapshot(91, 1, "CPU", [organism])
        {
            Species = [new SpeciesSnapshot("Fixture", false, 0xff239ac0, 0, false)],
        };
        var dna = new Dictionary<OrganismKey, string>
        {
            [new OrganismKey(7, 3)] = "start\nstop",
        };

        var livingSelection = LegacyRecordingSerializer.Select(LegacyCommandId.RecordingSnapshotLiving);
        Assert.Equal(LegacyRecordingFormat.Db2GenomeRecords, livingSelection.Format);
        Assert.Equal("snp", livingSelection.Extension);
        var living = LegacyRecordingSerializer.SerializeLiving(snapshot, dna, livingSelection.Format);
        Assert.StartsWith(Db2GenomeRecordFile.Header + "\r\n", living);
        Assert.Contains("7003,0,Fixture,3,0,19,0,0,2,0,0,885,885,0\r\nstart\nstop\r\n", living);

        var deadSelection = LegacyRecordingSerializer.Select(LegacyCommandId.RecordingDeadEnable);
        var dead = LegacyRecordingSerializer.SerializeDead(
            [new CorpseSnapshot([2f, 3f], [0f, 0f], 42, 4, 9, 0, false)
            {
                Dna = "stop",
                UniqueId = 99,
                DnaLength = 1,
            }],
            snapshot.Species,
            includeHeader: true,
            format: deadSelection.Format);
        Assert.Equal(LegacyRecordingFormat.Db2GenomeRecords, deadSelection.Format);
        Assert.Contains("99,0,Fixture,0,0,9,0,0,1,0,0,82,82,0\r\nstop\r\n", dead);

        var expectedHelpCommands = new[]
        {
            LegacyCommandId.HelpDna,
            LegacyCommandId.HelpRobotTagInformation,
            LegacyCommandId.HelpSurvivalInfo,
            LegacyCommandId.HelpAbout,
        };
        Assert.Equal(expectedHelpCommands, LegacyHelpResolver.Topics.Select(topic => topic.Command));
        foreach (var command in expectedHelpCommands)
        {
            var topic = LegacyHelpResolver.Resolve(command);
            Assert.NotEmpty(topic.Title);
            Assert.NotEmpty(topic.Section);
            Assert.NotEmpty(topic.Content);
            Assert.DoesNotContain("http://", topic.Content, StringComparison.OrdinalIgnoreCase);
            Assert.DoesNotContain("https://", topic.Content, StringComparison.OrdinalIgnoreCase);
        }
    }
}
