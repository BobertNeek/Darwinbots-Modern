using Avalonia.Controls;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Services;

public sealed record DesktopTextFile(string Name, string Content, string? LocalPath = null);

public interface IDesktopStorageService
{
    Task<IReadOnlyList<DesktopTextFile>> OpenDnaFilesAsync(TopLevel owner);
    Task<bool> SaveLivingSnapshotAsync(
        TopLevel owner,
        string suggestedFileName,
        Func<Stream, Task> writeAsync) => Task.FromResult(false);
    Task<bool> AppendDeadSnapshotAsync(Func<Stream, Task> writeAsync) => Task.FromResult(false);
    Task<byte[]?> OpenOrganismAsync(TopLevel owner) => Task.FromResult<byte[]?>(null);
    Task<DesktopSettings?> OpenSettingsAsync(TopLevel owner);
    Task<string?> PickSimulationPathAsync(TopLevel owner);
    Task<byte[]?> OpenSimulationAsync(TopLevel owner);
    Task<bool> SaveSettingsAsync(TopLevel owner, DesktopSettings settings);
    Task<bool> SaveSimulationAsync(TopLevel owner, ReadOnlyMemory<byte> save);
    Task<bool> SaveLegacySimulationAsync(TopLevel owner, ReadOnlyMemory<byte> save) => Task.FromResult(false);
    Task<bool> SaveDnaAsync(TopLevel owner, string suggestedFileName, string dna);
    Task<bool> SaveOrganismAsync(TopLevel owner, string suggestedFileName, Func<Stream, Task> writeAsync);
    string? FindLatestAutosave();
    Task SaveAutosaveAsync(ReadOnlyMemory<byte> save);
    bool IsFirstRun => false;
    Task MarkFirstRunCompleteAsync() => Task.CompletedTask;
    string? FindLastStart() => null;
    Task SaveLastStartAsync(ReadOnlyMemory<byte> settings) => Task.CompletedTask;
}
