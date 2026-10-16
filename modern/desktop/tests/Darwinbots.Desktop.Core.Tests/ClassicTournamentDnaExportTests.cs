using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class ClassicTournamentDnaExportTests
{
    [Fact]
    public async Task OriginalFinalDnaUsesLiteralWindows1252BytesAndPreflightsEveryEncoding()
    {
        var directory = TemporaryDirectory();
        try
        {
            const string raw = "' é €\r\n'#tag:Old.txt\r\nstart stop\r\n";
            var paths = await ClassicTournamentDnaExport.WriteAsync(directory, Complete(false, [1]), [new(1, "Café.txt", raw)]);
            var bytes = await File.ReadAllBytesAsync(Assert.Single(paths));
            Assert.Equal((byte)0xe9, bytes[2]);
            Assert.Equal((byte)0x80, bytes[4]);
            Assert.Equal(raw + "\r\n" + "'#tag:Café.txt" + "\r\n", LegacyDnaText.Decode(bytes));

            var rejected = Path.Combine(directory, "rejected");
            await Assert.ThrowsAsync<InvalidDataException>(() => ClassicTournamentDnaExport.WriteAsync(rejected,
                Complete(true, [1, 2]), [new(1, "Safe.txt", "start stop"), new(2, "Other.txt", "' 🙂\nstart stop")]));
            Assert.False(Directory.Exists(rejected));
        }
        finally { Delete(directory); }
    }

    [Fact]
    public async Task FinalExportKeepsTheFullOriginalNameBeyondTheActorTagLimit()
    {
        var directory = TemporaryDirectory();
        try
        {
            const string name = "Café - this original filename keeps every character beyond forty five.txt";
            var paths = await ClassicTournamentDnaExport.WriteAsync(directory, Complete(false, [1]),
                [new(1, name, "'#tag:An older tag.txt\r\nstart stop")]);
            Assert.Equal(name, Path.GetFileName(Assert.Single(paths)));
            Assert.EndsWith("'#tag:" + name + "\r\n",
                LegacyDnaText.Decode(await File.ReadAllBytesAsync(paths[0])));
        }
        finally { Delete(directory); }
    }

    [Fact]
    public async Task CompletedChampionUsesOriginalNameAndRawStartupDnaEvenWhenLadderWasRequested()
    {
        var directory = TemporaryDirectory();
        try
        {
            var mode = Complete(false, [4]) with { Configuration = LocalModeConfig.Default with {
                Workflow = LeagueWorkflow.PopulationSeeding, FinishWithLadder = true } };
            const string dna = "' Original comments\r\n'#tag:PreviousName.txt\r\ncond\r\nstart 7 .up store stop\r\n";
            var paths = await ClassicTournamentDnaExport.WriteAsync(directory, mode, [new(4, "Champion.txt", dna)]);
            Assert.Equal(Path.Combine(directory, "Tournament_Results", "Champion.txt"), Assert.Single(paths));
            Assert.Equal(dna + "\r\n" + "'#tag:Champion.txt" + "\r\n",
                await File.ReadAllTextAsync(paths[0]));
            Assert.False(Directory.Exists(Path.Combine(directory, "stepladder")));
            Assert.Equal(new uint[] { 4 }, mode.Series!.Ladder);
        }
        finally { Delete(directory); }
    }

    [Fact]
    public async Task ActualLadderExportsOneBasedEngineOrderAndPreservesExistingFiles()
    {
        var directory = TemporaryDirectory();
        try
        {
            var mode = Complete(true, [9, 2]);
            var finalists = new ClassicTournamentDna[] { new(9, "Winner.txt", "' raw winner\nstart stop"), new(2, "Second", "start 2 .up store stop") };
            var paths = await ClassicTournamentDnaExport.WriteAsync(directory, mode, finalists);
            Assert.Equal(new[] { "1-Winner.txt", "2-Second.txt" }, paths.Select(Path.GetFileName));
            Assert.All(paths, path => Assert.Equal("stepladder", Path.GetFileName(Path.GetDirectoryName(path))));
            Assert.Equal(finalists[1].Dna + "\r\n" + "'#tag:Second" + "\r\n",
                await File.ReadAllTextAsync(paths[1]));
            await Assert.ThrowsAsync<IOException>(() => ClassicTournamentDnaExport.WriteAsync(directory, mode, finalists));
            Assert.Equal(finalists[0].Dna + "\r\n" + "'#tag:Winner.txt" + "\r\n",
                await File.ReadAllTextAsync(paths[0]));
            Assert.Equal(2, Directory.GetFiles(Path.Combine(directory, "stepladder")).Length);
        }
        finally { Delete(directory); }
    }

    [Theory]
    [InlineData("../Escape.txt")]
    [InlineData("folder\\Escape.txt")]
    [InlineData("Bad\nName.txt")]
    [InlineData("CON.txt")]
    [InlineData("Bad?.txt")]
    [InlineData("Trailing. ")]
    public async Task InvalidFinalNamesAreRejectedBeforeAnyFilesAreWritten(string invalid)
    {
        var directory = TemporaryDirectory();
        try
        {
            await Assert.ThrowsAsync<ArgumentException>(() => ClassicTournamentDnaExport.WriteAsync(directory,
                Complete(true, [1, 2]), [new(1, "Valid.txt", "start stop"), new(2, invalid, "start stop")]));
            Assert.False(Directory.Exists(directory));
        }
        finally { Delete(directory); }
    }

    [Fact]
    public async Task IncompleteLegacyAndGenericSchedulesCannotExportOrInventFinalRankings()
    {
        var directory = TemporaryDirectory();
        var complete = Complete(false, [1]);
        var rejected = new[] {
            complete with { Status = LocalModeStatus.Running },
            complete with { Series = complete.Series! with { PopulationSeedingPhase = null } },
            complete with { Series = complete.Series! with { PopulationSeedingPhase = ClassicTournamentPhase.Tournament } },
            complete with { Series = complete.Series! with { Workflow = LeagueWorkflow.Tournament } },
            complete with { Series = complete.Series! with { Ladder = [1, 2] } },
        };
        foreach (var mode in rejected)
        {
            Assert.False(ClassicTournamentDnaExport.IsEligible(mode));
            await Assert.ThrowsAsync<ArgumentException>(() => ClassicTournamentDnaExport.WriteAsync(directory, mode,
                [new(1, "One.txt", "start stop")]));
        }
        await Assert.ThrowsAsync<ArgumentException>(() => ClassicTournamentDnaExport.WriteAsync(directory,
            Complete(true, [1, 2]), [new(2, "Two.txt", "start stop"), new(1, "One.txt", "start stop")]));
        Assert.False(Directory.Exists(directory));
    }

    private static LocalModeSnapshot Complete(bool enteredLadder, IReadOnlyList<uint> ranking) =>
        new(LocalModeStatus.Completed, LocalModeKind.League, LeaguePreset.F1, 1, 1, 0, 15000, 0, [], null, new()) {
            Configuration = LocalModeConfig.Default with { Workflow = LeagueWorkflow.PopulationSeeding },
            Series = new(LeagueWorkflow.PopulationSeeding, [], 1, ranking, [], []) {
                PopulationSeedingPhase = ClassicTournamentPhase.Complete, EnteredLadder = enteredLadder,
            },
        };
    private static string TemporaryDirectory() => Path.Combine(Path.GetTempPath(), "db-classic-dna-" + Guid.NewGuid().ToString("N"));
    private static void Delete(string directory) { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
}
