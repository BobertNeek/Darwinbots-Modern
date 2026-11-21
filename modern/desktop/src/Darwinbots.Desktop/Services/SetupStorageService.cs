using System.Text;
using System.Security.Cryptography;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Services;

public static class SetupStorageService
{
    public static byte[] EncodeSettings(string path, DesktopSettings settings)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(path);
        ArgumentNullException.ThrowIfNull(settings);

        if (IsModernJson(path)) return Encoding.UTF8.GetBytes(settings.ToJson());
        if (Path.IsPathRooted(path) && settings.Species.Count > 0)
        {
            var originals = settings.Species.Select(value => (Species: value, Bytes: LegacyDnaText.Encode(value.Dna))).ToArray();
            var directory = Path.ChangeExtension(path, ".robots");
            var species = new List<SpeciesImport>();
            foreach (var original in originals)
            {
                var value = original.Species;
                var data = original.Bytes;
                var dnaPath = Path.Combine(directory, Convert.ToHexString(SHA256.HashData(data)) + ".txt");
                species.Add(value with { DnaPath = dnaPath });
            }
            settings = settings with { Species = species };
            var encoded = Db2SettingsCodec.Encode(settings);
            Directory.CreateDirectory(directory);
            foreach (var original in originals)
            {
                var dnaPath = Path.Combine(directory, Convert.ToHexString(SHA256.HashData(original.Bytes)) + ".txt");
                if (!File.Exists(dnaPath)) File.WriteAllBytes(dnaPath, original.Bytes);
            }
            return encoded;
        }
        return Db2SettingsCodec.Encode(settings);
    }

    public static DesktopSettings DecodeSettings(string path, ReadOnlyMemory<byte> bytes)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(path);

        if (IsModernJson(path)) return DesktopSettings.FromJson(Encoding.UTF8.GetString(bytes.Span));
        var settings = Db2SettingsCodec.Decode(bytes.Span);
        var directory = Path.GetDirectoryName(Path.GetFullPath(path))!;
        return settings with { Species = settings.Species.Select(species => ResolveSpecies(species, directory)).ToArray() };
    }

    private static SpeciesImport ResolveSpecies(SpeciesImport species, string directory)
    {
        if (!string.IsNullOrWhiteSpace(species.Dna)) return species;
        var savedPath = species.DnaPath ?? species.Name;
        // Original .set files use Windows separators even when loaded on Unix.
        var portablePath = savedPath.Replace('\\', Path.DirectorySeparatorChar);
        var name = Path.GetFileName(portablePath);
        string[] candidates =
        [
            savedPath, Path.Combine(directory, savedPath),
            portablePath, Path.Combine(directory, portablePath),
            Path.Combine(directory, "..", "bots", name), Path.Combine(directory, "bots", name),
            .. PackagedSpeciesPaths(name),
        ];
        var source = candidates.FirstOrDefault(File.Exists)
            ?? throw new FileNotFoundException($"DNA file for legacy species '{species.Name}' was not found.", species.DnaPath);
        return species with { Dna = LegacyDnaText.Decode(File.ReadAllBytes(source)), DnaPath = Path.GetFullPath(source) };
    }

    public static string? ResolveNativeRestartSource(NativeRestartSource source, string? simulationPath, string mainDirectory)
    {
        var name = PortablePath(source.Filename);
        if (string.IsNullOrWhiteSpace(name) || name is "." or ".." || name.Contains('/') || !IsLocalPath(name))
            return null;

        foreach (var path in NativeRestartSourcePaths(name, PortablePath(source.Directory), simulationPath, mainDirectory))
        {
            try
            {
                if (!File.Exists(path) || new FileInfo(path).Length == 0) continue;
                var dna = LegacyDnaText.Decode(File.ReadAllBytes(path));
                if (!string.IsNullOrWhiteSpace(dna)) return dna;
            }
            catch (Exception error) when (error is IOException or UnauthorizedAccessException or DecoderFallbackException) { }
        }
        return null;
    }

    internal static string? SelectedDnaDirectory(string? path) =>
        path is not null && IsLocalPath(PortablePath(path)) && Path.IsPathFullyQualified(path)
            ? Path.GetDirectoryName(path) : null;

    internal static bool? ReadNativeGlobalBoolean(string mainDirectory, int recordIndex)
    {
        var directory = PortablePath(mainDirectory);
        if (!IsLocalPath(directory) || !Path.IsPathFullyQualified(directory)) return null;
        try
        {
            // frmGset writes one record per line.
            return File.ReadLines(Path.Combine(directory, "Global.gset")).ElementAtOrDefault(recordIndex)
                ?.Trim().ToUpperInvariant() switch
            {
                "#TRUE#" => true,
                "#FALSE#" => false,
                _ => null,
            };
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException) { return null; }
    }

    private static IEnumerable<string> NativeRestartSourcePaths(string name, string savedDirectory, string? simulationPath,
        string mainDirectory)
    {
        var savePath = PortablePath(simulationPath ?? "");
        var saveDirectory = IsLocalPath(savePath) && Path.IsPathFullyQualified(savePath) ? Path.GetDirectoryName(savePath) : null;
        var configuredRoot = PortablePath(mainDirectory);
        var originalRoot = IsLocalPath(configuredRoot) && Path.IsPathFullyQualified(configuredRoot) ? configuredRoot : null;
        var adjacentRoot = string.Equals(Path.GetFileName(saveDirectory), "Saves", StringComparison.OrdinalIgnoreCase)
            ? Path.GetDirectoryName(saveDirectory) : saveDirectory;

        if (savedDirectory.StartsWith("&#", StringComparison.Ordinal))
        {
            var relative = savedDirectory[2..].TrimStart('/');
            if (IsLocalPath(relative) && !Path.IsPathRooted(relative))
            {
                if (originalRoot is not null) yield return Path.Combine(originalRoot, relative, name);
                if (adjacentRoot is not null) yield return Path.Combine(adjacentRoot, relative, name);
            }
        }
        else if (string.IsNullOrWhiteSpace(savedDirectory))
        {
            if (originalRoot is not null) yield return Path.Combine(originalRoot, "Robots", name);
            if (adjacentRoot is not null) yield return Path.Combine(adjacentRoot, "Robots", name);
        }
        else if (IsLocalPath(savedDirectory))
        {
            if (Path.IsPathFullyQualified(savedDirectory)) yield return Path.Combine(savedDirectory, name);
            else if (!Path.IsPathRooted(savedDirectory))
            {
                if (saveDirectory is not null) yield return Path.Combine(saveDirectory, savedDirectory, name);
                if (originalRoot is not null) yield return Path.Combine(originalRoot, savedDirectory, name);
            }
        }
        if (saveDirectory is not null) yield return Path.Combine(saveDirectory, name);
        foreach (var packaged in PackagedSpeciesPaths(name)) yield return packaged;
    }

    private static IEnumerable<string> PackagedSpeciesPaths(string name)
    {
        yield return Path.Combine(AppContext.BaseDirectory, "Bots", name);
        yield return Path.Combine(AppContext.BaseDirectory, "Bots", "Historical", name);
    }

    private static string PortablePath(string path) => path.Replace('\\', '/');

    private static bool IsLocalPath(string path)
    {
        if (path.Contains('\0') || path.StartsWith("//", StringComparison.Ordinal)) return false;
        var colon = path.IndexOf(':');
        return colon < 0 || (OperatingSystem.IsWindows() && colon == 1 && char.IsAsciiLetter(path[0])
            && Path.IsPathFullyQualified(path) && path.LastIndexOf(':') == colon);
    }

    private static bool IsModernJson(string path) =>
        Path.GetExtension(path).Equals(".json", StringComparison.OrdinalIgnoreCase);
}
