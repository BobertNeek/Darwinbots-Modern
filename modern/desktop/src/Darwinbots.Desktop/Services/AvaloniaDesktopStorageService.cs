using Avalonia.Controls;
using Avalonia.Platform.Storage;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Services;

public sealed class AvaloniaDesktopStorageService : IDesktopStorageService
{
    public static AvaloniaDesktopStorageService Instance { get; } = new();

    private const string AutosaveDirectoryVariable = "DARWINBOTS_AUTOSAVE_DIRECTORY";

    public bool IsFirstRun => !File.Exists(FirstRunMarkerPath());

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
            using var memory = new MemoryStream();
            await stream.CopyToAsync(memory);
            var path = LocalPath(file);
            var profile = path is null ? null : await Db2RobotFileStorage.ReadMutationProfileAsync(path);
            result.Add(new DesktopTextFile(file.Name, LegacyDnaText.Decode(memory.ToArray()), path, profile));
        }
        return result;
    }

    public async Task<byte[]?> OpenOrganismAsync(TopLevel owner)
    {
        var files = await owner.StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Import Darwinbots organism",
            AllowMultiple = false,
            FileTypeFilter = [OrganismFileType()],
        });
        if (files.FirstOrDefault() is not { } file) return null;
        await using var stream = await file.OpenReadAsync();
        using var memory = new MemoryStream();
        await stream.CopyToAsync(memory);
        return memory.ToArray();
    }

    public async Task<DesktopSettings?> OpenSettingsAsync(TopLevel owner)
    {
        var files = await owner.StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Load Darwinbots settings",
            AllowMultiple = false,
            FileTypeFilter = [SettingsFileType()],
        });
        if (files.FirstOrDefault() is not { } file) return null;
        await using var stream = await file.OpenReadAsync();
        using var memory = new MemoryStream();
        await stream.CopyToAsync(memory);
        return SetupStorageService.DecodeSettings(LocalPath(file) ?? file.Name, memory.ToArray());
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

    public async Task<bool> SaveLegacySimulationAsync(TopLevel owner, ReadOnlyMemory<byte> save)
    {
        var file = await owner.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Save legacy Darwinbots simulation",
            SuggestedFileName = "simulation.sim",
            DefaultExtension = "sim",
            FileTypeChoices = [LegacySimulationFileType()],
        });
        if (file is null) return false;
        await using var stream = await file.OpenWriteAsync();
        stream.SetLength(0);
        await stream.WriteAsync(save);
        return true;
    }

    public async Task<bool> SaveSettingsAsync(TopLevel owner, DesktopSettings settings)
    {
        ArgumentNullException.ThrowIfNull(settings);
        var file = await owner.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Save Darwinbots settings",
            SuggestedFileName = "darwinbots-settings.set",
            DefaultExtension = "set",
            FileTypeChoices = [SettingsFileType()],
        });
        if (file is null) return false;
        var bytes = SetupStorageService.EncodeSettings(LocalPath(file) ?? file.Name, settings);
        await using var stream = await file.OpenWriteAsync();
        stream.SetLength(0);
        await stream.WriteAsync(bytes);
        return true;
    }

    public async Task<bool> SaveDnaAsync(TopLevel owner, string suggestedFileName, string dna, MutationProfileUpdate? mutationProfile)
    {
        var file = await owner.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Export Darwinbots robot DNA",
            SuggestedFileName = suggestedFileName,
            FileTypeChoices = [RobotFileType()],
        });
        if (file is null) return false;
        await WriteDnaFileAsync(file, dna, mutationProfile);
        return true;
    }

    public static async Task WriteDnaFileAsync(
        IStorageFile file, string dna, MutationProfileUpdate? mutationProfile, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(file);
        cancellationToken.ThrowIfCancellationRequested();
        var bytes = LegacyDnaText.Encode(dna);
        if (mutationProfile is not null) _ = Db2MutationRateCodec.Encode(mutationProfile);
        if (LocalPath(file) is { } path)
        {
            await Db2RobotFileStorage.WriteAsync(path, dna, mutationProfile, cancellationToken);
            return;
        }
        if (mutationProfile is not null)
            throw new NotSupportedException("Saving robot DNA with mutation settings requires a local file.");
        await using var stream = await file.OpenWriteAsync();
        stream.SetLength(0);
        await stream.WriteAsync(bytes, cancellationToken);
    }

    public async Task<string?> PickPopulationSeedDirectoryAsync(TopLevel owner)
    {
        var folders = await owner.StorageProvider.OpenFolderPickerAsync(new FolderPickerOpenOptions
        {
            Title = "Choose a directory for DNA export",
            AllowMultiple = false,
        });
        if (folders.FirstOrDefault() is not { } folder) return null;
        return LocalPath(folder)
            ?? throw new NotSupportedException("Population seed export requires a local directory.");
    }

    public async Task<bool> SaveOrganismAsync(
        TopLevel owner,
        string suggestedFileName,
        Func<Stream, Task> writeAsync)
    {
        var file = await owner.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Save entire organism",
            SuggestedFileName = suggestedFileName,
            DefaultExtension = "dbo",
            FileTypeChoices = [OrganismSaveFileType()],
        });
        if (file is null) return false;
        await using var stream = await file.OpenWriteAsync();
        stream.SetLength(0);
        await writeAsync(stream);
        return true;
    }

    public async Task<bool> SaveLivingSnapshotAsync(
        TopLevel owner,
        string suggestedFileName,
        Func<Stream, Task> writeAsync)
    {
        var file = await owner.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Save snapshot of the living",
            SuggestedFileName = suggestedFileName,
            DefaultExtension = "snp",
            FileTypeChoices = [SnapshotFileType()],
        });
        if (file is null) return false;
        await using var stream = await file.OpenWriteAsync();
        stream.SetLength(0);
        await writeAsync(stream);
        return true;
    }

    public async Task<bool> AppendDeadSnapshotAsync(Func<Stream, Task> writeAsync)
    {
        var path = DeadSnapshotPath();
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        await using var stream = new FileStream(
            path,
            FileMode.OpenOrCreate,
            FileAccess.ReadWrite,
            FileShare.Read,
            bufferSize: 4096,
            options: FileOptions.Asynchronous);
        stream.Seek(0, SeekOrigin.End);
        await writeAsync(stream);
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

    public string? FindLastStart()
    {
        var path = LastStartPath();
        return File.Exists(path) ? path : null;
    }

    public async Task SaveLastStartAsync(ReadOnlyMemory<byte> settings)
    {
        var directory = AutosaveDirectory();
        Directory.CreateDirectory(directory);
        var path = LastStartPath();
        var temporary = path + ".tmp";
        await File.WriteAllBytesAsync(temporary, settings.ToArray());
        File.Move(temporary, path, true);
    }

    public async Task MarkFirstRunCompleteAsync()
    {
        var directory = AutosaveDirectory();
        Directory.CreateDirectory(directory);
        await File.WriteAllTextAsync(FirstRunMarkerPath(), "completed");
    }

    private static FilePickerOpenOptions SimulationOpenOptions(string title) => new()
    {
        Title = title,
        AllowMultiple = false,
        FileTypeFilter = [SimulationFileType(), LegacySimulationFileType()],
    };

    private static FilePickerFileType RobotFileType() =>
        new("Darwinbots robot") { Patterns = ["*.txt"] };

    private static FilePickerFileType SimulationFileType() =>
        new("Darwinbots Modern save") { Patterns = ["*.db3s"] };

    private static FilePickerFileType LegacySimulationFileType() =>
        new("Darwinbots legacy simulation") { Patterns = ["*.sim"] };

    private static FilePickerFileType OrganismFileType() =>
        new("Darwinbots organism") { Patterns = ["*.json", "*.dbo"] };

    private static FilePickerFileType OrganismSaveFileType() =>
        new("Darwinbots DB2 organism") { Patterns = ["*.dbo"] };

    private static FilePickerFileType SnapshotFileType() =>
        new("Darwinbots DB2 genome records") { Patterns = ["*.snp"] };

    private static FilePickerFileType SettingsFileType() =>
        new("Darwinbots settings") { Patterns = ["*.set", "*.json"] };

    private static string? LocalPath(IStorageItem item) => item.Path.IsFile ? item.Path.LocalPath : null;

    private static string AutosaveDirectory() =>
        Environment.GetEnvironmentVariable(AutosaveDirectoryVariable) is { Length: > 0 } configured
            ? Path.GetFullPath(configured)
            : Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "Darwinbots Modern",
                "Autosaves");

    private static string DeadSnapshotPath() => Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "Darwinbots Modern",
        "records",
        "DeadRobots.snp");

    private static string LastStartPath() => Path.Combine(AutosaveDirectory(), "last-start.json");

    private static string FirstRunMarkerPath() => Path.Combine(AutosaveDirectory(), "first-run.complete");
}
