using System.Text;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class Db2PersistenceTests
{
    [Fact]
    public void OriginalSettingsUseWindows1252ForNonAsciiSpeciesNamesAndPaths()
    {
        var expected = new DesktopSettings {
            Species = [new("Café.txt", "start stop", false, 0xff123456, 5, 3000, 0, false, 0) {
                DnaPath = "Robots/Café €/Café.txt", Comment = "é €",
            }],
        };
        var bytes = Db2SettingsCodec.Encode(expected);
        Assert.Contains((byte)0xe9, bytes);
        Assert.Contains((byte)0x80, bytes);
        Assert.Equal(expected.ToJson(), Db2SettingsCodec.Decode(bytes).ToJson());
        var originalText = LegacyDnaText.Decode(bytes);
        originalText = originalText[..originalText.IndexOf("#DARWINBOTS_MODERN_SETTINGS_V1#", StringComparison.Ordinal)];
        var original = Db2SettingsCodec.Decode(LegacyDnaText.Encode(originalText));
        Assert.Equal("Café.txt", Assert.Single(original.Species).Name);
        Assert.Contains("Café €", original.Species[0].DnaPath);
    }

    [Fact]
    public void SettingsCodecWritesDb2SetTextAndRestoresTheModernWorldFields()
    {
        var expected = new DesktopSettings
        {
            Backend = "Gpu",
            TicksPerUpdate = 20,
            OrganismCapacity = 44_000,
            WorldWidth = 91_000,
            WorldHeight = 82_000,
            Species = [new SpeciesImport("Modern.txt", "start stop", false, 0xff123456, 5, 1000, 0, false, 0) { DnaPath = "Robots/Modern.txt" }],
        };

        var bytes = Db2SettingsCodec.Encode(expected);
        var text = Encoding.UTF8.GetString(bytes);
        Assert.StartsWith("-1\r\n0\r\n", text);
        var lines = text.Split("\r\n", StringSplitOptions.None);
        Assert.Equal("\"Robots\"", lines[7]);
        Assert.Equal("91000", lines[45]);
        Assert.Equal("82000", lines[46]);
        Assert.Equal("500", lines[47]); // Native MaxPopulation is the vegetable cap.

        var actual = Db2SettingsCodec.Decode(bytes);

        Assert.Equal(expected.Backend, actual.Backend);
        Assert.Equal(expected.TicksPerUpdate, actual.TicksPerUpdate);
        Assert.Equal(expected.OrganismCapacity, actual.OrganismCapacity);
        Assert.Equal(expected.WorldWidth, actual.WorldWidth);
        Assert.Equal(expected.WorldHeight, actual.WorldHeight);
    }

    [Fact]
    public void SettingsCodecMapsLegacyCorpseDecayFieldsWithoutModernExtension()
    {
        var lines = Enumerable.Repeat("0", 100).ToArray();
        lines[0] = "-1";
        lines[1] = "0";
        lines[45] = "91000";
        lines[46] = "82000";
        lines[47] = "44000";
        lines[79] = "#TRUE#";
        lines[80] = "75";
        lines[84] = "2";
        lines[85] = "3";

        var actual = Db2SettingsCodec.Decode(Encoding.UTF8.GetBytes(string.Join("\r\n", lines)));

        Assert.Equal(new Db2CorpseDecayOptions(true, 15, Db2CorpseDecayMode.WasteShot, 75, 3), actual.CorpseDecay);
    }

    [Fact]
    public void DeadGenomeRecordsKeepTheDb2HeaderAndSkipEmptyDna()
    {
        var corpse = new CorpseSnapshot([2f, 3f], [0f, 0f], 42, 4, 9, 0, false)
        {
            Dna = "stop",
            UniqueId = 99,
            DnaLength = 1,
        };
        var empty = corpse with { Dna = string.Empty, UniqueId = 100 };

        var text = Db2GenomeRecordFile.EncodeDead(
            [corpse, empty],
            [new SpeciesSnapshot("Fixture", false, 0, 0, false)],
            includeHeader: true);

        Assert.StartsWith(Db2GenomeRecordFile.Header + "\r\n", text);
        Assert.Contains("99,0,Fixture,0,0,9,0,0,1,0,0,82,82,0\r\nstop\r\n", text);
        Assert.DoesNotContain("100,", text);
    }

    [Fact]
    public void GenomeRecordCodecWritesTheDb2HeaderAndDetokenizedGenome()
    {
        var organism = new OrganismSnapshot(7, 3, [12.5f, 34.5f], [1f, -2f], 875, 19, Body: 1)
        {
            UniqueId = 7003,
            DnaLength = 2,
            NewMutations = 1,
            TotalMutations = 4,
            Offspring = 5,
            Chloroplasts = 12,
        };
        var snapshot = new EngineSnapshot(91, 1, "CPU", [organism])
        {
            Species = [new SpeciesSnapshot("Fixture", false, 0xff239ac0, 0, false)],
        };

        var text = Db2GenomeRecordFile.EncodeLiving(
            snapshot,
            new Dictionary<OrganismKey, string> { [new OrganismKey(7, 3)] = "start\nstop" });

        Assert.StartsWith(Db2GenomeRecordFile.Header + "\r\n", text);
        Assert.Contains("7003,0,Fixture,3,0,19,4,1,2,5,0,885,885,12\r\nstart\nstop\r\n", text);
    }

    [Fact]
    public void GenomeRecordRoundTripPreservesFitnessMutationSidecarsAndDeadRetentionWithoutDecay()
    {
        var settings = new DesktopSettings
        {
            CorpseDecay = new Db2CorpseDecayOptions(false, 0, Db2CorpseDecayMode.None, 0, 0),
        };
        var restoredSettings = Db2SettingsCodec.Decode(Db2SettingsCodec.Encode(settings));

        var organism = new OrganismSnapshot(7, 3, [12.5f, 34.5f], [1f, -2f], 7, 19, Body: 12)
        {
            UniqueId = 7003,
            DnaLength = 2,
            NewMutations = 3,
            TotalMutations = 12,
        };
        var snapshot = new EngineSnapshot(91, 1, "CPU", [organism])
        {
            Species = [new SpeciesSnapshot("Fixture", false, 0xff239ac0, 0, false)],
        };
        var living = Db2GenomeRecordFile.EncodeLiving(
            snapshot,
            new Dictionary<OrganismKey, string> { [new OrganismKey(7, 3)] = "start\nstop" });

        var retainedDead = new CorpseSnapshot([2f, 3f], [0f, 0f], 0, 12, 4)
        {
            Dna = "stop",
            UniqueId = 99,
            NewMutations = 2,
            TotalMutations = 11,
            DnaLength = 1,
        };
        var dead = Db2GenomeRecordFile.EncodeDead(
            [retainedDead],
            snapshot.Species,
            includeHeader: true);

        Assert.False(restoredSettings.CorpseDecay.Enabled);
        Assert.Contains("7003,0,Fixture,3,0,19,12,3,2,0,0,127,127,0\r\nstart\nstop\r\n", living);
        Assert.Contains("99,0,Fixture,0,0,4,11,2,1,0,0,120,120,0\r\nstop\r\n", dead);
    }

    [Fact]
    public void GenomeRecordCodecRoundTripPreservesEveryLegacyHeaderFieldAndDnaPayload()
    {
        var organism = new OrganismSnapshot(7, 3, [12.5f, 34.5f], [1f, -2f], 875, 19, Body: 1)
        {
            UniqueId = 7003,
            DnaLength = 2,
            NewMutations = 1,
            TotalMutations = 4,
            Offspring = 5,
            Chloroplasts = 12,
            BirthCycle = 812,
            Kills = 6,
            Parents = [new OrganismKey(11, 2), null],
        };
        var snapshot = new EngineSnapshot(91, 1, "CPU", [organism])
        {
            Species = [new SpeciesSnapshot("Fixture", false, 0xff239ac0, 0, false)],
        };
        const string dna = "start\n10 .up store\nstop";

        var encoded = Db2GenomeRecordFile.EncodeLiving(
            snapshot,
            new Dictionary<OrganismKey, string> { [new OrganismKey(7, 3)] = dna });

        Assert.Equal(Db2GenomeRecordFile.Header, encoded[..encoded.IndexOf("\r\n", StringComparison.Ordinal)]);
        var record = Assert.Single(Db2GenomeRecordFile.Decode(encoded));

        Assert.Equal(7003UL, record.Id);
        Assert.Equal(11U, record.ParentId);
        Assert.Equal("Fixture", record.Founder);
        Assert.Equal(3U, record.Generation);
        Assert.Equal(812UL, record.BirthCycle);
        Assert.Equal(19UL, record.Age);
        Assert.Equal(4UL, record.Mutations);
        Assert.Equal(1UL, record.NewMutations);
        Assert.Equal(2U, record.DnaLength);
        Assert.Equal(5UL, record.Offspring);
        Assert.Equal(6UL, record.Kills);
        Assert.Equal(885, record.Fitness);
        Assert.Equal(885, record.Energy);
        Assert.Equal(12, record.Chloroplasts);
        Assert.Equal(dna, record.Dna);
    }

    [Fact]
    public void OrganismCodecWritesTheDb2OFrameConsumedByTheRustImporter()
    {
        var expected = new Db2OrganismFile(
            1,
            812,
            [new Db2OrganismMember(
                7,
                3,
                "stop",
                0,
                false,
                false,
                false,
                [12f, 34f],
                [1f, -2f],
                777,
                91,
                44,
                321)],
            []);

        var bytes = expected.Encode();
        Assert.Equal("DB2O", Encoding.ASCII.GetString(bytes, 0, 4));
        var actual = Db2OrganismFile.Decode(bytes);
        Assert.Equal(expected.Version, actual.Version);
        Assert.Equal(expected.Tick, actual.Tick);
        var member = Assert.Single(actual.Members);
        Assert.Equal(7u, member.Slot);
        Assert.Equal(3u, member.Generation);
        Assert.Equal("stop", member.Dna);
        Assert.Equal([12f, 34f], member.Position);
        Assert.Equal([1f, -2f], member.Velocity);
        Assert.Equal(777, member.Energy);
        Assert.Equal(91UL, member.Age);
        Assert.Equal(44UL, member.BirthCycle);
        Assert.Equal(321, member.Body);
        Assert.Empty(actual.Ties);
    }

    [Fact]
    public void OrganismCodecRoundTripsMultiMemberRelativePlacementTiesAndSelectionOrder()
    {
        var expected = new Db2OrganismFile(
            1,
            812,
            [
                new Db2OrganismMember(
                    7,
                    3,
                    "stop",
                    0,
                    false,
                    false,
                    false,
                    [100f, 200f],
                    [1f, -2f],
                    777,
                    91,
                    44,
                    321,
                    [0f, 0f]),
                new Db2OrganismMember(
                    12,
                    5,
                    "stop",
                    0,
                    false,
                    false,
                    false,
                    [138f, 226f],
                    [0f, 3f],
                    555,
                    12,
                    8,
                    222,
                    [38f, 26f]),
            ],
            [new Db2OrganismTie(
                new Db2OrganismKey(7, 3),
                new Db2OrganismKey(12, 5),
                42.5f)],
            new Db2OrganismKey(7, 3));

        var actual = Db2OrganismFile.Decode(expected.Encode());

        Assert.Equal(new Db2OrganismKey(7, 3), actual.SelectedMember);
        Assert.Equal([(7u, 3u), (12u, 5u)],
            actual.Members.Select(member => (member.Slot, member.Generation)));
        Assert.Equal([0f, 0f], actual.Members[0].RelativePosition!);
        Assert.Equal([38f, 26f], actual.Members[1].RelativePosition!);
        Assert.Equal([100f, 200f], actual.Members[0].Position);
        Assert.Equal([138f, 226f], actual.Members[1].Position);
        var tie = Assert.Single(actual.Ties);
        Assert.Equal((7u, 3u), (tie.First.Slot, tie.First.Generation));
        Assert.Equal((12u, 5u), (tie.Second.Slot, tie.Second.Generation));
        Assert.Equal(42.5f, tie.RestLength);
    }
}
