using Avalonia.Controls;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Services;

public sealed record DesktopTextFile(string Name, string Content, string? LocalPath = null,
    MutationProfileUpdate? MutationProfile = null);

public enum SimulationSaveFormat
{
    Modern,
    Legacy,
}

public sealed record DesktopSimulationFile(byte[] Data, string? LocalPath = null);

public interface IDesktopStorageService
{
    Task<IReadOnlyList<DesktopTextFile>> OpenDnaFilesAsync(TopLevel owner, string? mainDirectory = null, bool allowMultiple = true);
    Task<DesktopTextFile?> OpenNativeRepopulationSourceAsync(TopLevel owner, NativeRestartSource source) =>
        Task.FromResult<DesktopTextFile?>(null);
    Task<bool> SaveLivingSnapshotAsync(
        TopLevel owner,
        string suggestedFileName,
        byte[] snapshot,
        Func<Task<byte[]?>> chooseMutationHistoryAsync) => Task.FromResult(false);
    Task<bool> AppendDeadSnapshotAsync(
        Func<Stream, Task> writeAsync,
        Func<Stream, Task>? writeMutationHistoryAsync) => Task.FromResult(false);
    Task<byte[]?> OpenOrganismAsync(TopLevel owner, string? mainDirectory = null) => Task.FromResult<byte[]?>(null);
    Task<byte[]?> OpenRgbMonitorPresetAsync(TopLevel owner) => Task.FromResult<byte[]?>(null);
    Task<bool> SaveRgbMonitorPresetAsync(TopLevel owner, ReadOnlyMemory<byte> preset) => Task.FromResult(false);
    Task<DesktopSettings?> OpenSettingsAsync(TopLevel owner, string? mainDirectory = null);
    Task<string?> PickSimulationPathAsync(TopLevel owner);
    Task<byte[]?> OpenSimulationAsync(TopLevel owner);
    async Task<DesktopSimulationFile?> OpenSimulationWithSourceAsync(TopLevel owner) =>
        await OpenSimulationAsync(owner) is { } data ? new(data) : null;
    Task<bool> SaveSettingsAsync(TopLevel owner, DesktopSettings settings, string? mainDirectory = null);
    Task<bool> SaveSimulationAsync(TopLevel owner, ReadOnlyMemory<byte> save);
    Task<bool> SaveLegacySimulationAsync(TopLevel owner, ReadOnlyMemory<byte> save) => Task.FromResult(false);
    async Task<SimulationSaveFormat?> SaveSimulationWithFormatAsync(
        TopLevel owner, Func<SimulationSaveFormat, Task<byte[]>> createSave)
    {
        var format = SimulationSaveFormat.Modern;
        return await SaveSimulationAsync(owner, await createSave(format)) ? format : null;
    }
    Task<SimulationSaveFormat?> SaveSimulationWithMutationOptionsAsync(
        TopLevel owner, Func<SimulationSaveFormat, Task<byte[]>> createSave) =>
        SaveSimulationWithFormatAsync(owner, createSave);
    Task<bool> SaveDnaAsync(TopLevel owner, string suggestedFileName, string dna, MutationProfileUpdate? mutationProfile,
        string? mainDirectory = null);
    Task<string?> PickPopulationSeedDirectoryAsync(TopLevel owner) => Task.FromResult<string?>(null);
    Task<bool> SaveOrganismAsync(TopLevel owner, string suggestedFileName, Func<Stream, Task> writeAsync,
        string? mainDirectory = null);
    string? FindLatestAutosave();
    Task SaveAutosaveAsync(ReadOnlyMemory<byte> save);
    Task<string?> ArchiveLatestAutosaveAsync(DateTime localBoundary, CancellationToken cancellationToken) => Task.FromResult<string?>(null);
    bool SupportsLastExitSimulation => false;
    string? FindLastExitSimulation() => null;
    Task SaveLastExitSimulationAsync(ReadOnlyMemory<byte> save) => Task.CompletedTask;
    bool IsFirstRun => false;
    Task MarkFirstRunCompleteAsync() => Task.CompletedTask;
    string? FindLastStart() => null;
    Task SaveLastStartAsync(ReadOnlyMemory<byte> settings) => Task.CompletedTask;
    GlobalSettings? LoadGlobalSettings() => null;
    Task SaveGlobalSettingsAsync(GlobalSettings settings) => Task.CompletedTask;
}
