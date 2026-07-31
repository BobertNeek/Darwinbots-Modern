using Avalonia.Controls;

namespace Darwinbots.Desktop.Services;

public sealed record DesktopTextFile(string Name, string Content, string? LocalPath = null);

public interface IDesktopStorageService
{
    Task<IReadOnlyList<DesktopTextFile>> OpenDnaFilesAsync(TopLevel owner);
    Task<string?> PickSimulationPathAsync(TopLevel owner);
    Task<byte[]?> OpenSimulationAsync(TopLevel owner);
    Task<bool> SaveSimulationAsync(TopLevel owner, ReadOnlyMemory<byte> save);
    Task<bool> SaveDnaAsync(TopLevel owner, string suggestedFileName, string dna);
    Task<bool> SaveOrganismAsync(TopLevel owner, string suggestedFileName, Func<Stream, Task> writeAsync);
    string? FindLatestAutosave();
    Task SaveAutosaveAsync(ReadOnlyMemory<byte> save);
}
