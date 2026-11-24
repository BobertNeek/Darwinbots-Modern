using System.Globalization;
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
            var directory = Path.ChangeExtension(path, ".robots");
            var sidecars = new List<(SpeciesImport Species, string Path, byte[] Bytes)>(settings.Species.Count);
            foreach (var species in settings.Species)
            {
                var bytes = LegacyDnaText.Encode(species.Dna);
                var hash = Convert.ToHexString(SHA256.HashData(bytes));
                // SaveSett uses the same basename for loading DNA and naming the original roster row.
                var name = NativeSidecarName(species.Name)
                    ?? NativeSidecarName(Path.GetFileName(species.DnaPath?.Replace('\\', '/')))
                    ?? hash + ".txt";
                var dnaPath = Path.Combine(directory, hash, name);
                sidecars.Add((species with { DnaPath = dnaPath }, dnaPath, bytes));
            }
            var encoded = Db2SettingsCodec.Encode(settings with { Species = sidecars.Select(entry => entry.Species).ToArray() });
            foreach (var sidecar in sidecars)
                WriteSidecar(sidecar.Path, sidecar.Bytes);
            return encoded;
        }
        return Db2SettingsCodec.Encode(settings);
    }

    private static string? NativeSidecarName(string? name)
    {
        if (string.IsNullOrWhiteSpace(name) || name.EndsWith('.') || name.EndsWith(' ')
            || name.IndexOfAny(['<', '>', ':', '"', '/', '\\', '|', '?', '*']) >= 0
            || name.Any(char.IsControl) || Encoding.UTF8.GetByteCount(name) > 255)
            return null;

        var device = name.Split('.', 2)[0].TrimEnd().ToUpperInvariant();
        if (device is "CON" or "PRN" or "AUX" or "NUL" or "CONIN$" or "CONOUT$"
            || (device.Length == 4 && device[..3] is ("COM" or "LPT") && "123456789¹²³".Contains(device[3])))
            return null;

        try { _ = LegacyDnaText.Encode(name); }
        catch (InvalidDataException) { return null; }
        return name;
    }

    private static void WriteSidecar(string path, byte[] bytes)
    {
        if (File.Exists(path)) return;
        var directory = Path.GetDirectoryName(path)!;
        Directory.CreateDirectory(directory);
        var temporary = Path.Combine(directory, Path.GetRandomFileName());
        var stream = new FileStream(temporary, FileMode.CreateNew, FileAccess.Write, FileShare.None);
        try
        {
            using (stream) stream.Write(bytes);
            try { File.Move(temporary, path); }
            catch (IOException) when (File.Exists(path)) { }
        }
        finally { File.Delete(temporary); }
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
        var useDefaultDirectory = string.IsNullOrWhiteSpace(savedDirectory);

        if (savedDirectory.StartsWith("&#", StringComparison.Ordinal))
        {
            var relative = savedDirectory[2..].TrimStart('/');
            if (IsLocalPath(relative) && !Path.IsPathRooted(relative))
            {
                if (originalRoot is not null) yield return Path.Combine(originalRoot, relative, name);
                if (adjacentRoot is not null) yield return Path.Combine(adjacentRoot, relative, name);
            }
        }
        else if (!useDefaultDirectory && IsLocalPath(savedDirectory))
        {
            if (Path.IsPathFullyQualified(savedDirectory)) yield return Path.Combine(savedDirectory, name);
            else if (!Path.IsPathRooted(savedDirectory))
            {
                if (saveDirectory is not null) yield return Path.Combine(saveDirectory, savedDirectory, name);
                if (originalRoot is not null) yield return Path.Combine(originalRoot, savedDirectory, name);
            }
        }
        // LoadDNA tries MainDir\Robots after the saved location fails.
        if (originalRoot is not null) yield return Path.Combine(originalRoot, "Robots", name);
        if (useDefaultDirectory && adjacentRoot is not null) yield return Path.Combine(adjacentRoot, "Robots", name);
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

    internal static int? ReadNativeStartingChloroplasts(string? mainDirectory)
    {
        if (string.IsNullOrWhiteSpace(mainDirectory)) return null;
        var directory = PortablePath(mainDirectory);
        if (!IsLocalPath(directory) || !Path.IsPathFullyQualified(directory)) return null;
        try
        {
            // frmGset writes one record per line. HDRoutines reads StartChlr as a VB Integer.
            var entry = File.ReadLines(Path.Combine(directory, "Global.gset")).Skip(29).FirstOrDefault();
            if (double.TryParse(entry, NumberStyles.Float, CultureInfo.InvariantCulture, out var amount) && double.IsFinite(amount))
                return (int)Math.Clamp(Math.Round(amount, MidpointRounding.ToEven), 0, 32_000);
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException) { }
        return null;
    }

    private static bool IsModernJson(string path) =>
        Path.GetExtension(path).Equals(".json", StringComparison.OrdinalIgnoreCase);
}
