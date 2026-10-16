using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class NativeLeagueSeriesTests
{
    [Fact]
    public void ClassicPipelineParsesPhaseHistoryAndOrderedPoolsWithoutReorderingZeroTies()
    {
        const string json = """
            { "organisms":[], "species":[], "local_mode": {
              "status":"running", "config":{"kind":"league","workflow":"population_seeding","finish_with_ladder":true},
              "series":{"workflow":"population_seeding","population_seeding_phase":"tournament","tournament_round":2,
                "entered_ladder":false,"active_match":[3,1],"pending":[4,2],"round_winners":[3],"seeded":[0],
                "seed_file_order":[1,0,3], "population_seeds":[
                  {"species":0,"population":0,"native_cycle":2000,"file_name":"0a.txt"},
                  {"species":1,"population":0,"native_cycle":2000,"file_name":"0.txt"},
                  {"species":3,"population":9,"native_cycle":2000,"file_name":"9.txt"}] }
            } }
            """;
        var mode = NativeSnapshotParser.Parse(json, "CPU").LocalMode;
        Assert.True(mode.Configuration!.FinishWithLadder);
        Assert.Equal(ClassicTournamentPhase.Tournament, mode.Series!.PopulationSeedingPhase);
        Assert.Equal(2u, mode.Series.TournamentRound);
        Assert.True(mode.Series.IsClassicCompetitive);
        Assert.False(mode.Series.IsPopulationCensus);
        Assert.False(mode.Series.EnteredLadder);
        Assert.Equal(new uint[] { 4, 2 }, mode.Series.Pending);
        Assert.Equal(new uint[] { 3 }, mode.Series.RoundWinners);
        Assert.Equal(new uint[] { 0 }, mode.Series.Seeded);
        Assert.Equal(new uint[] { 1, 0, 3 }, mode.Series.SeedFileOrder);
        Assert.Equal(new[] { "0a.txt", "0.txt", "9.txt" }, mode.Series.PopulationSeeds.Select(seed => seed.FileName));
        Assert.Equal(new[] { 0, 0, 9 }, mode.Series.PopulationSeeds.Select(seed => seed.Population));
    }

    [Fact]
    public void CompletedClassicKeepsActualLadderHistoryAndLegacyPhaseRemainsAbsent()
    {
        const string json = """
            {"local_mode":{"status":"completed","config":{"workflow":"population_seeding"},
              "series":{"workflow":"population_seeding","population_seeding_phase":"complete","entered_ladder":true,"ladder":[2,1]}}}
            """;
        var mode = NativeSnapshotParser.Parse(json, "CPU").LocalMode;
        Assert.True(mode.Series!.EnteredLadder);
        Assert.Equal(ClassicTournamentPhase.Complete, mode.Series.PopulationSeedingPhase);
        Assert.False(mode.Configuration!.FinishWithLadder);
        const string legacy = """{"local_mode":{"config":{"workflow":"population_seeding"},"series":{"workflow":"population_seeding","active_seed":1}}}""";
        var old = NativeSnapshotParser.Parse(legacy, "CPU").LocalMode.Series!;
        Assert.Null(old.PopulationSeedingPhase);
        Assert.True(old.IsPopulationCensus);
        Assert.False(old.IsClassicCompetitive);
        Assert.Empty(old.SeedFileOrder);
    }

    [Fact]
    public async Task PopulationArchiveRetainsPersistedFileNamesAndRejectsUnsafeNamesBeforeWriting()
    {
        var directory = Path.Combine(Path.GetTempPath(), "db-persisted-seed-" + Guid.NewGuid().ToString("N"));
        try
        {
            var paths = await PopulationSeedingExport.WriteAsync(directory, [
                new(new(1, 0, 2000) { FileName = "0b.txt" }, "One.txt", "' one\nstart stop"),
                new(new(2, 0, 2000) { FileName = "0.txt" }, "Two.txt", "start stop")]);
            Assert.Equal(new[] { "0b.txt", "0.txt" }, paths.Select(Path.GetFileName));
            await Assert.ThrowsAsync<ArgumentException>(() => PopulationSeedingExport.WriteAsync(directory,
                [new(new(1, 0, 2000) { FileName = "../0.txt" }, "One.txt", "start stop")]));
            Assert.Equal(2, Directory.GetFiles(Path.Combine(directory, "seeded")).Length);
        }
        finally { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task PersistedPopulationSeedCollisionFailsBeforeWritingAnyEarlierSeed()
    {
        var directory = Path.Combine(Path.GetTempPath(), "db-exact-seed-collision-" + Guid.NewGuid().ToString("N"));
        try
        {
            var target = Path.Combine(directory, "seeded");
            Directory.CreateDirectory(target);
            var existing = Path.Combine(target, "0a.txt");
            await File.WriteAllTextAsync(existing, "existing native schedule identity");
            await Assert.ThrowsAsync<IOException>(() => PopulationSeedingExport.WriteAsync(directory, [
                new(new(1, 5, 2000) { FileName = "5.txt" }, "First.txt", "start stop"),
                new(new(2, 0, 2000) { FileName = "0a.txt" }, "Second.txt", "start stop")]));
            Assert.Equal("existing native schedule identity", await File.ReadAllTextAsync(existing));
            Assert.Equal(existing, Assert.Single(Directory.GetFiles(target)));
            Assert.False(File.Exists(Path.Combine(target, "5.txt")));
            Assert.False(File.Exists(Path.Combine(target, "0aa.txt")));
        }
        finally { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task OriginalPopulationDnaUsesWindows1252AndRejectsAnyUnrepresentableSeedBeforeWrites()
    {
        var directory = Path.Combine(Path.GetTempPath(), "db-ansi-population-" + Guid.NewGuid().ToString("N"));
        try
        {
            const string raw = "' é €\r\nstart stop\r\n";
            var paths = await PopulationSeedingExport.WriteAsync(directory,
                [new(new(1, 5, 2000) { FileName = "5.txt" }, "Café.txt", raw)]);
            var bytes = await File.ReadAllBytesAsync(Assert.Single(paths));
            Assert.Equal((byte)0xe9, bytes[2]);
            Assert.Equal((byte)0x80, bytes[4]);
            Assert.Equal(raw + Environment.NewLine + "'#tag:Café.txt" + Environment.NewLine, LegacyDnaText.Decode(bytes));
            var rejected = Path.Combine(directory, "rejected");
            await Assert.ThrowsAsync<InvalidDataException>(() => PopulationSeedingExport.WriteAsync(rejected, [
                new(new(1, 0, 2000), "Safe.txt", "start stop"),
                new(new(2, 5, 2000), "Other.txt", "' 🙂\nstart stop")]));
            Assert.False(Directory.Exists(rejected));
        }
        finally { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task LegacySeedNamesCanChooseSuffixesWithoutTakingPersistedIdentities()
    {
        var directory = Path.Combine(Path.GetTempPath(), "db-mixed-seed-names-" + Guid.NewGuid().ToString("N"));
        try
        {
            var paths = await PopulationSeedingExport.WriteAsync(directory, [
                new(new(1, 0, 2000), "Legacy.txt", "start stop"),
                new(new(2, 0, 2000) { FileName = "0.txt" }, "Exact.txt", "start stop")]);
            Assert.Equal(new[] { "0a.txt", "0.txt" }, paths.Select(Path.GetFileName));
            Assert.EndsWith("'#tag:Exact.txt" + Environment.NewLine, await File.ReadAllTextAsync(paths[1]));
        }
        finally { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task Fresh729SeedArchiveKeepsExactNamesBytesAndRollsBackOnlyItsOwnFilesOnFailure()
    {
        var directory = Path.Combine(Path.GetTempPath(), "db-fresh-729-seeds-" + Guid.NewGuid().ToString("N"));
        try
        {
            // Explicit source filename fixture, independent of CollisionSuffix.
            var names = new List<string> { "0.txt" };
            for (var letter = 'a'; letter <= 'z'; letter++) names.Add($"0{letter}.txt");
            for (var first = 'a'; first <= '{'; first++)
                for (var second = 'a'; second <= 'z'; second++) names.Add($"0{first}{second}.txt");
            Assert.Equal(729, names.Count);
            const string raw = "' é €\r\n'#tag:Earlier.txt\r\nstart stop\r\n";
            var seeds = names.Select((name, index) => new PopulationSeedDna(
                new((uint)index, 0, 2000) { FileName = name }, $"Original {index}.txt", raw)).ToArray();

            var paths = await PopulationSeedingExport.WriteAsync(directory, seeds);

            Assert.Equal(names, paths.Select(Path.GetFileName));
            Assert.Equal("0z.txt", Path.GetFileName(paths[26]));
            Assert.Equal("0aa.txt", Path.GetFileName(paths[27]));
            Assert.Equal("0zz.txt", Path.GetFileName(paths[702]));
            Assert.Equal("0{a.txt", Path.GetFileName(paths[703]));
            Assert.Equal("0{z.txt", Path.GetFileName(paths[728]));
            var target = Path.Combine(directory, "seeded");
            Assert.Equal(729, Directory.GetFiles(target).Length);
            Assert.False(File.Exists(Path.Combine(target, "0aaa.txt")));
            var before = new byte[paths.Count][];
            for (var index = 0; index < paths.Count; index++) before[index] = await File.ReadAllBytesAsync(paths[index]);
            foreach (var index in new[] { 0, 26, 27, 702, 703, 728 })
            {
                Assert.Equal((byte)0xe9, before[index][2]);
                Assert.Equal((byte)0x80, before[index][4]);
                Assert.Equal(raw + Environment.NewLine + $"'#tag:Original {index}.txt" + Environment.NewLine,
                    LegacyDnaText.Decode(before[index]));
            }

            await Assert.ThrowsAsync<IOException>(() => PopulationSeedingExport.WriteAsync(directory, seeds));
            // This legacy request writes one new file, then reaches the original
            // invalid suffix. Rollback must remove that file, not the old archive.
            var failure = await Assert.ThrowsAsync<IOException>(() => PopulationSeedingExport.WriteAsync(directory, [
                new(new(900, 5, 2000), "Temporary.txt", "start stop"),
                new(new(901, 0, 2000), "Overflow.txt", "start stop")]));
            Assert.Contains("collision suffix", failure.Message, StringComparison.Ordinal);
            Assert.False(File.Exists(Path.Combine(target, "5.txt")));
            Assert.Equal(729, Directory.GetFiles(target).Length);
            for (var index = 0; index < paths.Count; index++)
                Assert.Equal(before[index], await File.ReadAllBytesAsync(paths[index]));
        }
        finally { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task PopulationSeedSuffixFollowsOriginalBraceBoundaryAndRejectsTheInvalidPipe()
    {
        var directory = Path.Combine(Path.GetTempPath(), "db-source-seed-suffix-" + Guid.NewGuid().ToString("N"));
        try
        {
            // Reserve the preceding source names directly so this boundary
            // exercise does not require hundreds of quadratic legacy searches.
            var preceding = new List<string> { "0.txt" };
            for (var letter = 'a'; letter <= 'z'; letter++) preceding.Add($"0{letter}.txt");
            for (var first = 'a'; first <= 'z'; first++)
                for (var second = 'a'; second <= 'z'; second++)
                    if (first != 'z' || second != 'z') preceding.Add($"0{first}{second}.txt");
            var target = Path.Combine(directory, "seeded");
            Directory.CreateDirectory(target);
            foreach (var name in preceding)
                await File.WriteAllBytesAsync(Path.Combine(target, name), new byte[] { 0x27, 0x20, 0x6f, 0x6c, 0x64 });
            var boundary = await PopulationSeedingExport.WriteAsync(directory, [
                new(new(702, 0, 2000), "ZZ.txt", "start stop"),
                new(new(703, 0, 2000), "BraceA.txt", "start stop")]);
            Assert.Equal(new[] { "0zz.txt", "0{a.txt" }, boundary.Select(Path.GetFileName));
            await PopulationSeedingExport.WriteAsync(directory, Enumerable.Range('b', 'y' - 'b' + 1).Select((letter, index) =>
                new PopulationSeedDna(new((uint)(704 + index), 0, 2000) { FileName = $"0{{{(char)letter}.txt" }, "Original.txt", "start stop")).ToArray());
            var last = await PopulationSeedingExport.WriteAsync(directory, [new(new(728, 0, 2000), "BraceZ.txt", "start stop")]);
            Assert.Equal("0{z.txt", Path.GetFileName(Assert.Single(last)));
            await Assert.ThrowsAsync<IOException>(() => PopulationSeedingExport.WriteAsync(directory,
                [new(new(729, 0, 2000), "Overflow.txt", "start stop")]));
            Assert.Equal(729, Directory.GetFiles(Path.Combine(directory, "seeded")).Length);
            Assert.False(File.Exists(Path.Combine(directory, "seeded", "0aaa.txt")));
            Assert.Equal(new byte[] { 0x27, 0x20, 0x6f, 0x6c, 0x64 },
                await File.ReadAllBytesAsync(Path.Combine(target, "0zy.txt")));
            Assert.EndsWith("'#tag:BraceZ.txt" + Environment.NewLine, await File.ReadAllTextAsync(last[0]));

            var exact = Path.Combine(directory, "exact");
            var exactPaths = await PopulationSeedingExport.WriteAsync(exact, [
                new(new(1, 5, 2000) { FileName = "5{a.txt" }, "A.txt", "start stop"),
                new(new(2, 5, 2000) { FileName = "5{z.txt" }, "Z.txt", "start stop")]);
            Assert.Equal(new[] { "5{a.txt", "5{z.txt" }, exactPaths.Select(Path.GetFileName));
            await Assert.ThrowsAsync<ArgumentException>(() => PopulationSeedingExport.WriteAsync(exact,
                [new(new(3, 5, 2000) { FileName = "5|a.txt" }, "Invalid.txt", "start stop")]));
            Assert.Equal(2, Directory.GetFiles(Path.Combine(exact, "seeded")).Length);
        }
        finally { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
    }

    [Fact]
    public void ClassicCompetitiveResultsUseTheCurrentPairAndNeverCountPopulationScoresAsWins()
    {
        const string json = """
            {"organisms":[],"species":[{"name":"old"},{"name":"current"},{"name":"opponent"}],
              "local_mode":{"status":"running","current_round":3,
                "config":{"kind":"league","workflow":"population_seeding"},
                "series":{"workflow":"population_seeding","population_seeding_phase":"tournament",
                  "active_match":[1,2],"match_start_result":1,
                  "population_seeds":[{"species":1,"population":99,"native_cycle":2000,"file_name":"99.txt"}]},
                "results":[{"round":1,"winner":0,"reason":"old"},{"round":2,"winner":1,"reason":"current"}]}}
            """;
        var snapshot = NativeSnapshotParser.Parse(json, "CPU");
        var current = Assert.Single(ContestResultSnapshot.FromSnapshot(snapshot).Competitors);
        Assert.Equal("current", current.Name);
        Assert.Equal(1, current.Wins);
        Assert.Equal(0, current.Population);
        Assert.True(current.IsActive);
        var census = snapshot with { LocalMode = snapshot.LocalMode with {
            Series = snapshot.LocalMode.Series! with { PopulationSeedingPhase = ClassicTournamentPhase.Seeding } } };
        Assert.Empty(ContestResultSnapshot.FromSnapshot(census).Competitors);
    }

    [Fact]
    public void PopulationSeedingParsesZeroScoresAndTheActiveCandidateWithoutContestResults()
    {
        const string json = """
            { "native_cycle":1999, "species":[{"name":"Gone.txt"},{"name":"Next.txt"}],
              "local_mode": { "status":"running", "cycle_in_round":2000,
                "config": { "kind":"league", "workflow":"population_seeding", "entrant_order":[0,1] },
                "series": { "workflow":"population_seeding", "active_match":null, "active_seed":1,
                  "population_seeds":[{"species":0,"population":0,"native_cycle":2000}] }
              } }
            """;
        var snapshot = NativeSnapshotParser.Parse(json, "CPU");
        var mode = snapshot.LocalMode;
        Assert.Equal(LeagueWorkflow.PopulationSeeding, mode.Configuration!.Workflow);
        Assert.Equal(4, (int)mode.Configuration.Workflow);
        Assert.Equal(3, (int)LeagueWorkflow.Seeding);
        Assert.Equal(1u, mode.Series!.ActiveSeed);
        Assert.Equal(new PopulationSeedSnapshot(0, 0, 2000), Assert.Single(mode.Series.PopulationSeeds));
        Assert.Empty(mode.Series.ActiveMatch);
        Assert.Empty(mode.Series.Matches);
        Assert.Empty(mode.Series.Seeded);
        Assert.Empty(mode.Results);
        Assert.Equal(ContestResultSnapshot.Empty, ContestResultSnapshot.FromSnapshot(snapshot));
    }

    [Fact]
    public void OldSeedFilterSnapshotKeepsItsTokenAndDefaultsTheNewPopulationFields()
    {
        const string json = """
            {"local_mode":{"config":{"workflow":"seeding"},"series":{"workflow":"seeding","seeded":[2]}}}
            """;
        var mode = NativeSnapshotParser.Parse(json, "CPU").LocalMode;
        Assert.Equal(LeagueWorkflow.Seeding, mode.Configuration!.Workflow);
        Assert.Equal(LeagueWorkflow.Seeding, mode.Series!.Workflow);
        Assert.Equal(new uint[] { 2 }, mode.Series.Seeded);
        Assert.Null(mode.Series.ActiveSeed);
        Assert.Empty(mode.Series.PopulationSeeds);
    }

    [Fact]
    public async Task PopulationSeedExportKeepsZeroAndDuplicateScoresSourceTagsAndExistingFiles()
    {
        var directory = Path.Combine(Path.GetTempPath(), "db-population-export-" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(Path.Combine(directory, "seeded"));
            var existing = Path.Combine(directory, "seeded", "0.txt");
            await File.WriteAllTextAsync(existing, "irreplaceable seed");
            const string original = "start\n1 .up store\nstop";
            var paths = await PopulationSeedingExport.WriteAsync(directory,
            [
                new(new(1, 0, 2000), "Gone.txt", original),
                new(new(2, 0, 2000), "C:\\robots\\Other.txt", "start stop"),
                new(new(3, 27, 2000), "Living.txt", "start stop"),
            ]);
            Assert.Equal(new[] { "0a.txt", "0b.txt", "27.txt" }, paths.Select(Path.GetFileName));
            Assert.Equal("irreplaceable seed", await File.ReadAllTextAsync(existing));
            Assert.Equal(original + Environment.NewLine + "'#tag:Gone.txt" + Environment.NewLine,
                await File.ReadAllTextAsync(paths[0]));
            Assert.EndsWith("'#tag:Other.txt" + Environment.NewLine, await File.ReadAllTextAsync(paths[1]));
            Assert.Equal(4, Directory.GetFiles(Path.Combine(directory, "seeded")).Length);
        }
        finally { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task PopulationSeedExportUsesOriginalAlphabeticCollisionSequenceAndValidatesBeforeWriting()
    {
        var directory = Path.Combine(Path.GetTempPath(), "db-population-collision-" + Guid.NewGuid().ToString("N"));
        try
        {
            var seeds = Enumerable.Range(0, 28).Select(index =>
                new PopulationSeedDna(new((uint)index, 0, 2000), $"{index}.txt", "start stop")).ToArray();
            var paths = await PopulationSeedingExport.WriteAsync(directory, seeds);
            Assert.Equal("0.txt", Path.GetFileName(paths[0]));
            Assert.Equal("0z.txt", Path.GetFileName(paths[26]));
            Assert.Equal("0aa.txt", Path.GetFileName(paths[27]));
            await Assert.ThrowsAsync<ArgumentException>(() => PopulationSeedingExport.WriteAsync(directory,
                [new(new(1, 12, 1999), "TooEarly.txt", "start stop")]));
            Assert.Equal(28, Directory.GetFiles(Path.Combine(directory, "seeded")).Length);
        }
        finally { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
    }

    [Fact]
    public void NativeScheduleKeepsPairRankingsAndConfiguration()
    {
        const string json = """
            { "tick": 40, "organisms": [], "species": [], "local_mode": {
              "status": "running", "current_round": 4, "cycle_in_round": 7,
              "config": { "kind": "league", "workflow": "ladder", "preset": "f1",
                "entrant_order": [4,2,1], "initial_ladder": [4], "seeding_opponent": 3,
                "minimum_rounds": 5, "maximum_rounds": 1, "maximum_cycles": 15000 },
              "series": { "workflow": "ladder", "active_match": [4,1], "match_number": 3,
                "ladder": [4,2], "seeded": [], "matches": [
                  { "number": 1, "entrants": [4,2], "winner": 4, "first_result": 0, "result_count": 2 } ] }
            } }
            """;
        var mode = NativeSnapshotParser.Parse(json, "CPU").LocalMode;
        Assert.Equal(LeagueWorkflow.Ladder, mode.Configuration!.Workflow);
        Assert.Equal(new uint[] { 4, 2, 1 }, mode.Configuration.EntrantOrder);
        Assert.Equal(new uint[] { 4 }, mode.Configuration.InitialLadder);
        Assert.Equal(3U, mode.Configuration.SeedingOpponent);
        Assert.Equal(new uint[] { 4, 1 }, mode.Series!.ActiveMatch);
        Assert.Equal(new uint[] { 4, 2 }, mode.Series.Ladder);
        Assert.Equal(2, Assert.Single(mode.Series.Matches).ResultCount);
    }

    [Fact]
    public void NativeZerobotModeDoesNotFallBackToLeague()
    {
        const string json = """{ "local_mode": { "config": { "kind": "zerobot" } } }""";
        Assert.Equal(LocalModeKind.Zerobot, NativeSnapshotParser.Parse(json, "CPU").LocalMode.Kind);
    }

    [Fact]
    public void ContestResultsUseCurrentPairAndExcludePreviousMatchWins()
    {
        const string json = """
            { "organisms": [], "species": [{"name":"a"},{"name":"b"},{"name":"c"}],
              "local_mode": { "status":"running", "current_round":3,
                "config": { "kind":"league", "workflow":"tournament" },
                "series": { "workflow":"tournament", "active_match":[1,2], "match_start_result":1 },
                "results": [ {"round":1,"winner":0,"reason":"old"}, {"round":2,"winner":1,"reason":"current"} ]
              } }
            """;
        var results = ContestResultSnapshot.FromSnapshot(NativeSnapshotParser.Parse(json, "CPU"));
        var competitor = Assert.Single(results.Competitors);
        Assert.Equal("b", competitor.Name);
        Assert.Equal(1, competitor.Wins);
        Assert.True(competitor.IsActive);
    }
}
