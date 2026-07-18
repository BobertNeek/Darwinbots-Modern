using Avalonia.Controls;
using Avalonia.Platform.Storage;

namespace Darwinbots.Desktop.Services;

public sealed class AvaloniaDesktopStorageService : IDesktopStorageService
{
    public static AvaloniaDesktopStorageService Instance { get; } = new();

    private const string AutosaveDirectoryVariable = "DARWINBOTS_AUTOSAVE_DIRECTORY";

    private AvaloniaDesktopStorageService()
    {
    }

    public async Task<IReadOnlyList<DesktopTextFile>> OpenDnaFilesAsync(TopLevel owner)
    {
        var files = await owner.StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Import Darwinbots robot DNA",
            AllowMultiple = true,
            FileTypeFilter = [RobotFileType()],
        });
        var result = new List<DesktopTextFile>(files.Count);
        foreach (var file in files)
        {
            await using var stream = await file.OpenReadAsync();
            using var reader = new StreamReader(stream);
            result.Add(new DesktopTextFile(file.Name, await reader.ReadToEndAsync(), LocalPath(file)));
        }
        return result;
    }

    public async Task<string?> PickSimulationPathAsync(TopLevel owner)
    {
        var files = await owner.StorageProvider.OpenFilePickerAsync(SimulationOpenOptions("Open Darwinbots Modern save"));
        return files.FirstOrDefault() is { } file ? LocalPath(file) : null;
    }

    public async Task<byte[]?> OpenSimulationAsync(TopLevel owner)
    {
        var files = await owner.StorageProvider.OpenFilePickerAsync(SimulationOpenOptions("Load Darwinbots simulation"));
        if (files.FirstOrDefault() is not { } file) return null;
        await using var stream = await file.OpenReadAsync();
        using var memory = new MemoryStream();
        await stream.CopyToAsync(memory);
        return memory.ToArray();
    }

    public async Task<bool> SaveSimulationAsync(TopLevel owner, ReadOnlyMemory<byte> save)
    {
        var file = await owner.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Save Darwinbots simulation",
            SuggestedFileName = "simulation.db3s",
            FileTypeChoices = [SimulationFileType()],
        });
        if (file is null) return false;
        await using var stream = await file.OpenWriteAsync();
        stream.SetLength(0);
        await stream.WriteAsync(save);
        return true;
    }

    public async Task<bool> SaveDnaAsync(TopLevel owner, string suggestedFileName, string dna)
    {
        var file = await owner.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Export Darwinbots robot DNA",
            SuggestedFileName = suggestedFileName,
            FileTypeChoices = [RobotFileType()],
        });
        if (file is null) return false;
        await using var stream = await file.OpenWriteAsync();
        stream.SetLength(0);
        await using var writer = new StreamWriter(stream);
        await writer.WriteAsync(dna);
        return true;
    }

    public string? FindLatestAutosave()
    {
        var directory = AutosaveDirectory();
        return Directory.Exists(directory)
            ? Directory.GetFiles(directory, "*.db3s").OrderByDescending(File.GetLastWriteTimeUtc).FirstOrDefault()
            : null;
    }

    public async Task SaveAutosaveAsync(ReadOnlyMemory<byte> save)
    {
        var directory = AutosaveDirectory();
        Directory.CreateDirectory(directory);
        var path = Path.Combine(directory, $"autosave-{DateTime.UtcNow:yyyyMMdd-HHmmss-fffffff}.db3s");
        var temporary = path + ".tmp";
        await File.WriteAllBytesAsync(temporary, save.ToArray());
        File.Move(temporary, path, true);
        foreach (var stale in Directory.GetFiles(directory, "*.db3s").OrderByDescending(File.GetLastWriteTimeUtc).Skip(5))
            File.Delete(stale);
    }

    private static FilePickerOpenOptions SimulationOpenOptions(string title) => new()
    {
        Title = title,
        AllowMultiple = false,
        FileTypeFilter = [SimulationFileType()],
    };

    private static FilePickerFileType RobotFileType() =>
        new("Darwinbots robot") { Patterns = ["*.txt"] };

    private static FilePickerFileType SimulationFileType() =>
        new("Darwinbots Modern save") { Patterns = ["*.db3s"] };

    private static string? LocalPath(IStorageItem item) => item.Path.IsFile ? item.Path.LocalPath : null;

    private static string AutosaveDirectory() =>
        Environment.GetEnvironmentVariable(AutosaveDirectoryVariable) is { Length: > 0 } configured
            ? Path.GetFullPath(configured)
            : Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "Darwinbots Modern",
                "Autosaves");
}
