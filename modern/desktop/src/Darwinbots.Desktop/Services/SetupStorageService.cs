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
            Directory.CreateDirectory(directory);
            var species = new List<SpeciesImport>();
            foreach (var value in settings.Species)
            {
                var data = Encoding.UTF8.GetBytes(value.Dna);
                var dnaPath = Path.Combine(directory, Convert.ToHexString(SHA256.HashData(data)) + ".txt");
                if (!File.Exists(dnaPath)) File.WriteAllBytes(dnaPath, data);
                species.Add(value with { DnaPath = dnaPath });
            }
            settings = settings with { Species = species };
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
        var name = Path.GetFileName(species.DnaPath ?? species.Name);
        var candidates = new[]
        {
            species.DnaPath ?? species.Name, Path.Combine(directory, species.DnaPath ?? species.Name),
            Path.Combine(directory, "..", "bots", name), Path.Combine(directory, "bots", name),
            Path.Combine(AppContext.BaseDirectory, "Bots", name), Path.Combine(AppContext.BaseDirectory, "Bots", "Historical", name),
        };
        var source = candidates.FirstOrDefault(File.Exists)
            ?? throw new FileNotFoundException($"DNA file for legacy species '{species.Name}' was not found.", species.DnaPath);
        return species with { Dna = File.ReadAllText(source), DnaPath = Path.GetFullPath(source) };
    }

    private static bool IsModernJson(string path) =>
        Path.GetExtension(path).Equals(".json", StringComparison.OrdinalIgnoreCase);
}
