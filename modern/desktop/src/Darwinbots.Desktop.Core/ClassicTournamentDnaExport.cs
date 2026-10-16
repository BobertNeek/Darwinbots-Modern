using System.Globalization;

namespace Darwinbots.Desktop.Core;

public sealed record ClassicTournamentDna(uint Species, string SourceName, string Dna);

public static class ClassicTournamentDnaExport
{
    public static bool IsEligible(LocalModeSnapshot mode) =>
        mode.Status == LocalModeStatus.Completed
        && mode.Configuration?.Workflow == LeagueWorkflow.PopulationSeeding
        && mode.Series is { Workflow: LeagueWorkflow.PopulationSeeding,
            PopulationSeedingPhase: ClassicTournamentPhase.Complete, Ladder.Count: > 0 } series
        && (series.EnteredLadder || series.Ladder.Count == 1)
        && series.Ladder.Distinct().Count() == series.Ladder.Count;

    public static async Task<IReadOnlyList<string>> WriteAsync(string directory, LocalModeSnapshot mode,
        IReadOnlyList<ClassicTournamentDna> finalists)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(directory);
        ArgumentNullException.ThrowIfNull(mode);
        ArgumentNullException.ThrowIfNull(finalists);
        if (!IsEligible(mode) || !mode.Series!.Ladder.SequenceEqual(finalists.Select(value => value.Species)))
            throw new ArgumentException("Final DNA export requires the completed classic tournament ranking.", nameof(mode));
        var ladder = mode.Series.EnteredLadder;
        var target = Path.Combine(Path.GetFullPath(directory), ladder ? "stepladder" : "Tournament_Results");
        var outputs = finalists.Select((value, index) =>
        {
            ArgumentException.ThrowIfNullOrWhiteSpace(value.Dna);
            var name = OriginalFileName(value.SourceName);
            var outputName = ladder ? (index + 1).ToString(CultureInfo.InvariantCulture) + "-" + name : name;
            if (outputName.Length > 255)
                throw new ArgumentException("Classic tournament output filename is too long.", nameof(finalists));
            return (Path: Path.Combine(target, outputName),
                Bytes: LegacyDnaText.Encode(value.Dna + "\r\n" + "'#tag:" + value.SourceName + "\r\n"));
        }).ToArray();
        if (outputs.Select(value => value.Path).Distinct(StringComparer.OrdinalIgnoreCase).Count() != outputs.Length)
            throw new ArgumentException("Final DNA output names must be unique.", nameof(finalists));
        if (outputs.Any(value => File.Exists(value.Path) || Directory.Exists(value.Path)))
            throw new IOException("A classic tournament DNA result already exists. Choose another directory.");

        // Validate the entire request before writing. Reserve all names with
        // CreateNew so a concurrent writer can never be overwritten.
        Directory.CreateDirectory(target);
        var created = new List<(string Path, FileStream Stream)>();
        try
        {
            foreach (var output in outputs)
                created.Add((output.Path, new FileStream(output.Path, FileMode.CreateNew,
                    FileAccess.Write, FileShare.None, 4096, FileOptions.Asynchronous)));
            for (var index = 0; index < outputs.Length; index++)
            {
                await created[index].Stream.WriteAsync(outputs[index].Bytes);
            }
            foreach (var output in created) await output.Stream.DisposeAsync();
            return outputs.Select(value => value.Path).ToArray();
        }
        catch
        {
            foreach (var output in created)
            {
                await output.Stream.DisposeAsync();
                File.Delete(output.Path);
            }
            throw;
        }
    }

    internal static string OriginalFileName(string sourceName)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(sourceName);
        // Source names are original tag basenames, not arbitrary export paths.
        if (sourceName is "." or ".." || sourceName.EndsWith(' ') || sourceName.EndsWith('.')
            || sourceName.Any(value => value < ' ' || "<>:\"/\\|?*".Contains(value)))
            throw new ArgumentException("Classic tournament source names must be safe local filenames.", nameof(sourceName));
        var stem = sourceName.Split('.')[0].TrimEnd(' ', '.');
        if (new[] { "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
                "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9" }.Contains(stem, StringComparer.OrdinalIgnoreCase))
            throw new ArgumentException("Classic tournament source name is reserved by Windows.", nameof(sourceName));
        return sourceName.EndsWith(".txt", StringComparison.OrdinalIgnoreCase) ? sourceName : sourceName + ".txt";
    }
}
