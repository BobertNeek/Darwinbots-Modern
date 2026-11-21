using System.Globalization;
using Avalonia.Controls;
using Avalonia.Platform.Storage;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Services;

public sealed class AvaloniaDesktopStorageService : IDesktopStorageService
{
    public static AvaloniaDesktopStorageService Instance { get; } = new();

    internal const string ProfileDirectoryVariable = "DARWINBOTS_PROFILE_DIRECTORY";
    internal const string AutosaveDirectoryVariable = "DARWINBOTS_AUTOSAVE_DIRECTORY";
    private readonly string? _profileDirectory;

    public bool IsFirstRun => !File.Exists(FirstRunMarkerPath());

    private AvaloniaDesktopStorageService()
    {
    }

    public AvaloniaDesktopStorageService(string profileDirectory)
    {
        _profileDirectory = Path.GetFullPath(profileDirectory);
    }

    public async Task<IReadOnlyList<DesktopTextFile>> OpenDnaFilesAsync(TopLevel owner, string? mainDirectory = null, bool allowMultiple = true)
    {
        var storage = owner.StorageProvider;
        using var folder = mainDirectory is null ? null : await NativeRobotFolderAsync(storage, mainDirectory);
        var files = await storage.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Import Darwinbots robot DNA",
            SuggestedStartLocation = folder,
            AllowMultiple = allowMultiple,
            FileTypeFilter = [RobotFileType()],
        });
        var result = new List<DesktopTextFile>(files.Count);
        foreach (var file in files)
        {
            var dna = await ReadDnaAsync(file);
            var path = LocalPath(file);
            var profile = path is null ? null : await Db2RobotFileStorage.ReadMutationProfileAsync(path);
            result.Add(new DesktopTextFile(file.Name, dna, path, profile));
        }
        return result;
    }

    public async Task<DesktopTextFile?> OpenNativeRepopulationSourceAsync(TopLevel owner, NativeRestartSource source)
    {
        var files = await owner.StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = $"Find original Darwinbots DNA for {source.Filename}",
            AllowMultiple = false,
            FileTypeFilter = [RobotFileType()],
        });
        if (files.FirstOrDefault() is not { } file) return null;
        return new DesktopTextFile(file.Name, await ReadDnaAsync(file), file.Path.IsUnc ? null : LocalPath(file));
    }

    private static async Task<string> ReadDnaAsync(IStorageFile file)
    {
        await using var stream = await file.OpenReadAsync();
        using var memory = new MemoryStream();
        await stream.CopyToAsync(memory);
        return LegacyDnaText.Decode(memory.ToArray());
    }

    public async Task<byte[]?> OpenOrganismAsync(TopLevel owner, string? mainDirectory = null)
    {
        using var folder = await NativeRobotFolderAsync(owner.StorageProvider, mainDirectory);
        var files = await owner.StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Import Darwinbots organism",
            SuggestedStartLocation = folder,
            AllowMultiple = false,
            FileTypeFilter = [OrganismFileType()],
        });
        if (files.FirstOrDefault() is not { } file) return null;
        await using var stream = await file.OpenReadAsync();
        using var memory = new MemoryStream();
        await stream.CopyToAsync(memory);
        return memory.ToArray();
    }

    public async Task<byte[]?> OpenRgbMonitorPresetAsync(TopLevel owner)
    {
        var files = await owner.StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Load RGB monitor preset",
            AllowMultiple = false,
            FileTypeFilter = [RgbMonitorPresetFileType()],
        });
        if (files.FirstOrDefault() is not { } file) return null;
        await using var stream = await file.OpenReadAsync();
        var bytes = new byte[Db2RgbMonitorPresetCodec.ByteCount];
        await stream.ReadExactlyAsync(bytes);
        return bytes;
    }

    public async Task<bool> SaveRgbMonitorPresetAsync(TopLevel owner, ReadOnlyMemory<byte> preset)
    {
        var file = await owner.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Save RGB monitor preset",
            SuggestedFileName = "monitor.mtrp",
            DefaultExtension = "mtrp",
            FileTypeChoices = [RgbMonitorPresetFileType()],
        });
        if (file is null) return false;
        await using var stream = await file.OpenWriteAsync();
        stream.SetLength(0);
        await stream.WriteAsync(preset);
        return true;
    }

    public async Task<DesktopSettings?> OpenSettingsAsync(TopLevel owner, string? mainDirectory = null)
    {
        var storage = owner.StorageProvider;
        using var folder = await NativeSettingsFolderAsync(storage, mainDirectory);
        var files = await storage.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Load Darwinbots settings",
            SuggestedStartLocation = folder,
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
        var files = await owner.StorageProvider.OpenFilePickerAsync(SimulationOpenOptions(
            "Open Darwinbots Modern save", SimulationFileType(), LegacySimulationFileType()));
        return files.FirstOrDefault() is { } file ? LocalPath(file) : null;
    }

    public async Task<byte[]?> OpenSimulationAsync(TopLevel owner) =>
        (await OpenSimulationWithSourceAsync(owner))?.Data;

    public async Task<DesktopSimulationFile?> OpenSimulationWithSourceAsync(TopLevel owner)
    {
        var files = await owner.StorageProvider.OpenFilePickerAsync(SimulationOpenOptions(
            "Load Darwinbots simulation", LegacySimulationFileType(), SimulationFileType()));
        if (files.FirstOrDefault() is not { } file) return null;
        await using var stream = await file.OpenReadAsync();
        using var memory = new MemoryStream();
        await stream.CopyToAsync(memory);
        return new(memory.ToArray(), LocalPath(file));
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

    public Task<SimulationSaveFormat?> SaveSimulationWithFormatAsync(
        TopLevel owner, Func<SimulationSaveFormat, Task<byte[]>> createSave) =>
        PickAndSaveSimulationAsync(owner, createSave, "Save Darwinbots simulation",
            [LegacySimulationFileType(), SimulationFileType()]);

    public Task<SimulationSaveFormat?> SaveSimulationWithMutationOptionsAsync(
        TopLevel owner, Func<SimulationSaveFormat, Task<byte[]>> createSave) =>
        PickAndSaveSimulationAsync(owner, createSave, "Save Darwinbots simulation without mutations",
            [
                new("Darwinbots 2 simulation without mutation details") { Patterns = ["*.sim"] },
                new("Darwinbots Modern save with mutations disabled") { Patterns = ["*.db3s"] },
            ]);

    private static async Task<SimulationSaveFormat?> PickAndSaveSimulationAsync(
        TopLevel owner,
        Func<SimulationSaveFormat, Task<byte[]>> createSave,
        string title,
        IReadOnlyList<FilePickerFileType> fileTypes)
    {
        var file = await owner.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = title,
            SuggestedFileName = "simulation.sim",
            SuggestedFileType = fileTypes[0],
            DefaultExtension = "sim",
            FileTypeChoices = fileTypes,
        });
        if (file is null) return null;
        var format = Path.GetExtension(file.Name).ToLowerInvariant() switch
        {
            ".sim" => SimulationSaveFormat.Legacy,
            ".db3s" => SimulationSaveFormat.Modern,
            _ => throw new InvalidDataException("Use .sim or .db3s for the simulation file."),
        };
        var save = await createSave(format);
        await using var stream = await file.OpenWriteAsync();
        stream.SetLength(0);
        await stream.WriteAsync(save);
        return format;
    }

    public async Task<bool> SaveSettingsAsync(TopLevel owner, DesktopSettings settings, string? mainDirectory = null)
    {
        ArgumentNullException.ThrowIfNull(settings);
        var storage = owner.StorageProvider;
        using var folder = await NativeSettingsFolderAsync(storage, mainDirectory);
        var file = await storage.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Save Darwinbots settings",
            SuggestedStartLocation = folder,
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

    public async Task<bool> SaveDnaAsync(TopLevel owner, string suggestedFileName, string dna, MutationProfileUpdate? mutationProfile,
        string? mainDirectory = null)
    {
        using var folder = mainDirectory is null ? null : await NativeRobotFolderAsync(owner.StorageProvider, mainDirectory);
        var file = await owner.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Export Darwinbots robot DNA",
            SuggestedStartLocation = folder,
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
        await WriteDnaFileAsync(LocalPath(file), file.OpenWriteAsync, dna, mutationProfile, cancellationToken);
    }

    public static async Task WriteDnaFileAsync(
        string? localPath, Func<Task<Stream>> openWriteAsync, string dna,
        MutationProfileUpdate? mutationProfile, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(openWriteAsync);
        cancellationToken.ThrowIfCancellationRequested();
        var bytes = LegacyDnaText.Encode(dna);
        if (mutationProfile is not null) _ = Db2MutationRateCodec.Encode(mutationProfile);
        if (localPath is not null)
        {
            await Db2RobotFileStorage.WriteAsync(localPath, dna, mutationProfile, cancellationToken);
            return;
        }
        if (mutationProfile is not null)
            throw new NotSupportedException("Saving robot DNA with mutation settings requires a local file.");
        await using var stream = await openWriteAsync();
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
        Func<Stream, Task> writeAsync,
        string? mainDirectory = null)
    {
        using var folder = await NativeRobotFolderAsync(owner.StorageProvider, mainDirectory);
        var file = await owner.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Save entire organism",
            SuggestedStartLocation = folder,
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

    private static async Task<IStorageFolder?> NativeRobotFolderAsync(IStorageProvider storage, string? mainDirectory)
    {
        var root = string.IsNullOrWhiteSpace(mainDirectory) ? AppContext.BaseDirectory : mainDirectory;
        foreach (var name in new[] { "robots", "Robots" })
        {
            var path = Path.Combine(root, name);
            if (Directory.Exists(path))
                return await storage.TryGetFolderFromPathAsync(new Uri(Path.GetFullPath(path)));
        }
        return null;
    }

    public async Task<bool> SaveLivingSnapshotAsync(
        TopLevel owner,
        string suggestedFileName,
        byte[] snapshot,
        Func<Task<byte[]?>> chooseMutationHistoryAsync)
    {
        var file = await owner.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Save snapshot of the living",
            SuggestedFileName = suggestedFileName,
            DefaultExtension = "snp",
            FileTypeChoices = [SnapshotFileType()],
        });
        if (file is null) return false;
        var history = await chooseMutationHistoryAsync();
        if (history is not null)
        {
            var path = LocalPath(file) ??
                throw new NotSupportedException("Saving mutation history requires a local snapshot file.");
            await Db2GenomeRecordFile.WriteWithMutationHistoryAsync(path, snapshot, history);
            return true;
        }

        await using var stream = await file.OpenWriteAsync();
        stream.SetLength(0);
        await stream.WriteAsync(snapshot);
        return true;
    }

    public async Task<bool> AppendDeadSnapshotAsync(Func<Stream, Task> writeAsync, Func<Stream, Task>? writeMutationHistoryAsync)
    {
        var path = DeadSnapshotPath();
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        await using var history = writeMutationHistoryAsync is null ? null : OpenRecording(Db2GenomeRecordFile.MutationHistoryPath(path));
        await using var stream = OpenRecording(path);
        var start = stream.Seek(0, SeekOrigin.End);
        var historyStart = history?.Seek(0, SeekOrigin.End);
        try
        {
            await writeAsync(stream);
            if (history is not null)
            {
                await writeMutationHistoryAsync!(history);
                await history.FlushAsync();
            }
            await stream.FlushAsync();
            return true;
        }
        catch (Exception error)
        {
            try
            {
                stream.SetLength(start);
                if (historyStart is { } length) history!.SetLength(length);
            }
            catch (Exception rollback)
            {
                throw new IOException("Unable to restore dead recording files after a failed write.", new AggregateException(error, rollback));
            }
            throw;
        }
    }

    private static FileStream OpenRecording(string path) =>
        new(
            path,
            FileMode.OpenOrCreate,
            FileAccess.ReadWrite,
            FileShare.Read,
            bufferSize: 4096,
            options: FileOptions.Asynchronous);

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

    public async Task<string?> ArchiveLatestAutosaveAsync(DateTime localBoundary, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var directory = Path.Combine(AutosaveDirectory(), "Backups");
        var stamp = localBoundary.ToString("yyyyMMdd-HHmmss", CultureInfo.InvariantCulture);
        var path = Path.Combine(directory, $"autosave-{stamp}.db3s");
        if (File.Exists(path)) return path;
        if (FindLatestAutosave() is not { } sourcePath) return null;

        await using var source = new FileStream(sourcePath, FileMode.Open, FileAccess.Read, FileShare.Read | FileShare.Delete,
            bufferSize: 81920, useAsync: true);
        Directory.CreateDirectory(directory);
        var temporary = $"{path}.{Guid.NewGuid():N}.tmp";
        try
        {
            await using (var copy = new FileStream(temporary, FileMode.CreateNew, FileAccess.Write, FileShare.None,
                bufferSize: 81920, useAsync: true))
                await source.CopyToAsync(copy, cancellationToken);

            cancellationToken.ThrowIfCancellationRequested();
            try { File.Move(temporary, path); }
            catch (IOException) when (File.Exists(path)) { /* Another window already archived this boundary. */ }
            return path;
        }
        finally { File.Delete(temporary); }
    }

    public bool SupportsLastExitSimulation => true;

    public string? FindLastExitSimulation()
    {
        var file = new FileInfo(LastExitSimulationPath());
        return file.Exists && file.Length > 0 ? file.FullName : null;
    }

    public Task SaveLastExitSimulationAsync(ReadOnlyMemory<byte> save) =>
        WriteLastExitFileAsync(LastExitSimulationPath(), save);

    public string? FindRestartContinuation()
    {
        var file = new FileInfo(RestartContinuationPath());
        return file.Exists ? file.FullName : null;
    }

    public Task SaveRestartContinuationAsync(ReadOnlyMemory<byte> save) =>
        WriteLastExitFileAsync(RestartContinuationPath(), save);

    public void ClearRestartContinuation()
    {
        try { File.Delete(RestartContinuationPath()); }
        catch (DirectoryNotFoundException) { }
    }

    public bool SupportsLastExitSettings => true;

    public bool HasLastExitSettings => File.Exists(LastExitSettingsPath());

    public DesktopSettings? LoadLastExitSettings()
    {
        try { return DesktopSettings.FromJson(File.ReadAllText(LastExitSettingsPath())); }
        catch (FileNotFoundException) { return null; }
        catch (DirectoryNotFoundException) { return null; }
    }

    public Task SaveLastExitSettingsAsync(DesktopSettings settings) =>
        WriteLastExitFileAsync(LastExitSettingsPath(), System.Text.Encoding.UTF8.GetBytes(settings.ToJson()));

    private static async Task WriteLastExitFileAsync(string path, ReadOnlyMemory<byte> bytes)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        var temporary = path + "." + Guid.NewGuid().ToString("N") + ".tmp";
        try
        {
            await File.WriteAllBytesAsync(temporary, bytes);
            File.Move(temporary, path, overwrite: true);
        }
        finally
        {
            File.Delete(temporary);
        }
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

    public GlobalSettings? LoadGlobalSettings()
    {
        try
        {
            var settings = GlobalSettings.FromJson(File.ReadAllText(GlobalSettingsPath()));
            if (settings.Main.StartChloroplasts is null
                && SetupStorageService.ReadNativeStartingChloroplasts(settings.Main.MainDirectory) is { } amount)
                settings = settings with { Main = settings.Main with { StartChloroplasts = amount } };
            return settings;
        }
        catch (FileNotFoundException) { return null; }
        catch (DirectoryNotFoundException) { return null; }
    }

    public async Task SaveGlobalSettingsAsync(GlobalSettings settings)
    {
        var json = settings.ToJson();
        Directory.CreateDirectory(ProfileDirectory());
        var path = GlobalSettingsPath();
        var temporary = path + "." + Guid.NewGuid().ToString("N") + ".tmp";
        try
        {
            await File.WriteAllTextAsync(temporary, json);
            File.Move(temporary, path, overwrite: true);
        }
        finally
        {
            File.Delete(temporary);
        }
    }

    private static async Task<IStorageFolder?> NativeSettingsFolderAsync(IStorageProvider storage, string? mainDirectory)
    {
        var root = string.IsNullOrWhiteSpace(mainDirectory) ? AppContext.BaseDirectory : mainDirectory;
        foreach (var name in new[] { "settings", "Settings" })
        {
            var path = Path.Combine(root, name);
            if (Directory.Exists(path))
                return await storage.TryGetFolderFromPathAsync(new Uri(Path.GetFullPath(path)));
        }
        return null;
    }

    private static FilePickerOpenOptions SimulationOpenOptions(string title, params FilePickerFileType[] filters) => new()
    {
        Title = title,
        AllowMultiple = false,
        FileTypeFilter = filters,
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

    private static FilePickerFileType RgbMonitorPresetFileType() =>
        new("Darwinbots RGB monitor preset") { Patterns = ["*.mtrp"] };

    private static string? LocalPath(IStorageItem item) => item.Path.IsFile ? item.Path.LocalPath : null;

    internal string GraphDirectory(string mainDirectory) =>
        string.IsNullOrWhiteSpace(mainDirectory) ? ProfileDirectory() : mainDirectory;

    private string ProfileDirectory() => _profileDirectory
        ?? (Environment.GetEnvironmentVariable(ProfileDirectoryVariable) is { Length: > 0 } configured
            ? Path.GetFullPath(configured)
            : Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Darwinbots Modern"));

    internal string AutosaveDirectory() =>
        _profileDirectory is null && Environment.GetEnvironmentVariable(AutosaveDirectoryVariable) is { Length: > 0 } configured
            ? Path.GetFullPath(configured)
            : Path.Combine(ProfileDirectory(), "Autosaves");

    private string DeadSnapshotPath() => Path.Combine(ProfileDirectory(), "records", "DeadRobots.snp");

    private string LastStartPath() => Path.Combine(AutosaveDirectory(), "last-start.json");

    private string LastExitSimulationPath() => Path.Combine(AutosaveDirectory(), "Resume", "last-exit.db3s");

    private string RestartContinuationPath() => Path.Combine(AutosaveDirectory(), "Resume", "restart-mode.db3s");

    private string LastExitSettingsPath() => Path.Combine(ProfileDirectory(), "last-exit-settings.json");

    private string FirstRunMarkerPath() => Path.Combine(AutosaveDirectory(), "first-run.complete");

    private string GlobalSettingsPath() => Path.Combine(ProfileDirectory(), "global-settings.json");
}
