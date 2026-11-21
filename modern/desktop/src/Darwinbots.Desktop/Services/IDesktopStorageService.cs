using Avalonia.Controls;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Services;

public sealed record DesktopTextFile(string Name, string Content, string? LocalPath = null,
    MutationProfileUpdate? MutationProfile = null);

public sealed record SavedRobotDna(string? FileName);

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
    Task<bool> SaveDnaAsync(TopLevel owner, string suggestedFileName, string dna, MutationProfileUpdate? mutationProfile);
    async Task<SavedRobotDna?> SaveDnaWithNameAsync(TopLevel owner, string suggestedFileName, string dna,
        MutationProfileUpdate? mutationProfile) =>
        await SaveDnaAsync(owner, suggestedFileName, dna, mutationProfile) ? new(null) : null;
    Task<string?> PickPopulationSeedDirectoryAsync(TopLevel owner) => Task.FromResult<string?>(null);
    Task<bool> SaveOrganismAsync(TopLevel owner, string suggestedFileName, Func<Stream, Task> writeAsync);
    string? FindLatestAutosave();
    Task SaveAutosaveAsync(ReadOnlyMemory<byte> save);
    bool SupportsLastExitSimulation => false;
    string? FindLastExitSimulation() => null;
    Task SaveLastExitSimulationAsync(ReadOnlyMemory<byte> save) => Task.CompletedTask;
    string? FindRestartContinuation() => null;
    Task SaveRestartContinuationAsync(ReadOnlyMemory<byte> save) =>
        throw new NotSupportedException("This storage does not support restart mode continuation.");
    void ClearRestartContinuation() { }
    bool SupportsLastExitSettings => false;
    bool HasLastExitSettings => false;
    DesktopSettings? LoadLastExitSettings() => null;
    Task SaveLastExitSettingsAsync(DesktopSettings settings) => Task.CompletedTask;
    bool IsFirstRun => false;
    Task MarkFirstRunCompleteAsync() => Task.CompletedTask;
    string? FindLastStart() => null;
    Task SaveLastStartAsync(ReadOnlyMemory<byte> settings) => Task.CompletedTask;
    GlobalSettings? LoadGlobalSettings() => null;
    Task SaveGlobalSettingsAsync(GlobalSettings settings) => Task.CompletedTask;
}
