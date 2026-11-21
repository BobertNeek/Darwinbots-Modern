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
        var candidates = new[]
        {
            savedPath, Path.Combine(directory, savedPath),
            portablePath, Path.Combine(directory, portablePath),
            Path.Combine(directory, "..", "bots", name), Path.Combine(directory, "bots", name),
            Path.Combine(AppContext.BaseDirectory, "Bots", name), Path.Combine(AppContext.BaseDirectory, "Bots", "Historical", name),
        };
        var source = candidates.FirstOrDefault(File.Exists)
            ?? throw new FileNotFoundException($"DNA file for legacy species '{species.Name}' was not found.", species.DnaPath);
        return species with { Dna = LegacyDnaText.Decode(File.ReadAllBytes(source)), DnaPath = Path.GetFullPath(source) };
    }

    private static bool IsModernJson(string path) =>
        Path.GetExtension(path).Equals(".json", StringComparison.OrdinalIgnoreCase);
}
