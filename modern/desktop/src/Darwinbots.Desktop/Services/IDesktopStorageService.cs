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

public interface IDesktopStorageService
{
    Task<IReadOnlyList<DesktopTextFile>> OpenDnaFilesAsync(TopLevel owner);
    Task<bool> SaveLivingSnapshotAsync(
        TopLevel owner,
        string suggestedFileName,
        byte[] snapshot,
        Func<Task<byte[]?>> chooseMutationHistoryAsync) => Task.FromResult(false);
    Task<bool> AppendDeadSnapshotAsync(
        Func<Stream, Task> writeAsync,
        Func<Stream, Task>? writeMutationHistoryAsync) => Task.FromResult(false);
    Task<byte[]?> OpenOrganismAsync(TopLevel owner) => Task.FromResult<byte[]?>(null);
    Task<byte[]?> OpenRgbMonitorPresetAsync(TopLevel owner) => Task.FromResult<byte[]?>(null);
    Task<bool> SaveRgbMonitorPresetAsync(TopLevel owner, ReadOnlyMemory<byte> preset) => Task.FromResult(false);
    Task<DesktopSettings?> OpenSettingsAsync(TopLevel owner);
    Task<string?> PickSimulationPathAsync(TopLevel owner);
    Task<byte[]?> OpenSimulationAsync(TopLevel owner);
    Task<bool> SaveSettingsAsync(TopLevel owner, DesktopSettings settings);
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
    Task<bool> SaveDnaAsync(TopLevel owner, string suggestedFileName, string dna, MutationProfileUpdate? mutationProfile);
    Task<string?> PickPopulationSeedDirectoryAsync(TopLevel owner) => Task.FromResult<string?>(null);
    Task<bool> SaveOrganismAsync(TopLevel owner, string suggestedFileName, Func<Stream, Task> writeAsync);
    string? FindLatestAutosave();
    Task SaveAutosaveAsync(ReadOnlyMemory<byte> save);
    bool IsFirstRun => false;
    Task MarkFirstRunCompleteAsync() => Task.CompletedTask;
    string? FindLastStart() => null;
    Task SaveLastStartAsync(ReadOnlyMemory<byte> settings) => Task.CompletedTask;
    GlobalSettings? LoadGlobalSettings() => null;
    Task SaveGlobalSettingsAsync(GlobalSettings settings) => Task.CompletedTask;
}
