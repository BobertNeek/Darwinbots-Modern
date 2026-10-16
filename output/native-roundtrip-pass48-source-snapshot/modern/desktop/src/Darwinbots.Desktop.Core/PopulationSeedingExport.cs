using System.Globalization;

namespace Darwinbots.Desktop.Core;

public sealed record PopulationSeedDna(PopulationSeedSnapshot Score, string SourceName, string Dna);

public static class PopulationSeedingExport
{
    public static async Task<IReadOnlyList<string>> WriteAsync(string directory, IReadOnlyList<PopulationSeedDna> seeds)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(directory);
        ArgumentNullException.ThrowIfNull(seeds);
        var encoded = new List<byte[]>(seeds.Count);
        foreach (var seed in seeds)
        {
            if (seed.Score.Population < 0 || seed.Score.NativeCycle != 2000)
                throw new ArgumentException("Population seeds must be recorded at original cycle 2000.", nameof(seeds));
            ArgumentException.ThrowIfNullOrWhiteSpace(seed.Dna);
            if (string.IsNullOrWhiteSpace(seed.SourceName) || seed.SourceName.IndexOfAny(['\r', '\n']) >= 0)
                throw new ArgumentException("Population seed source names must fit on one line.", nameof(seeds));
            if (!string.IsNullOrEmpty(seed.Score.FileName))
            {
                var validName = seed.Score.Population.ToString(CultureInfo.InvariantCulture);
                var name = seed.Score.FileName;
                if (!name.StartsWith(validName, StringComparison.Ordinal) || !name.EndsWith(".txt", StringComparison.Ordinal)
                    || name.Length < validName.Length + 4
                    || !IsNativeSuffix(name[validName.Length..^4]))
                    throw new ArgumentException("Population seed filename must contain its score and original collision suffix.", nameof(seeds));
            }
            encoded.Add(LegacyDnaText.Encode(seed.Dna + Environment.NewLine + "'#tag:"
                + seed.SourceName.Replace('\\', '/').Split('/')[^1] + Environment.NewLine));
        }
        if (seeds.Count == 0) return [];
        var target = Path.Combine(Path.GetFullPath(directory), "seeded");
        var persisted = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var seed in seeds.Where(value => !string.IsNullOrEmpty(value.Score.FileName)))
        {
            if (!persisted.Add(seed.Score.FileName))
                throw new ArgumentException("Persisted population seed filenames must be unique.", nameof(seeds));
            var path = Path.Combine(target, seed.Score.FileName);
            if (File.Exists(path) || Directory.Exists(path))
                throw new IOException("A persisted population seed filename already exists. Choose another directory.");
        }
        Directory.CreateDirectory(target);
        var created = new List<(string Path, FileStream Stream, byte[] Bytes)>(seeds.Count);
        try
        {
            // Reserve every name before writing DNA. Persisted names are native
            // schedule identities; only old payloads without names choose suffixes.
            for (var seedIndex = 0; seedIndex < seeds.Count; seedIndex++)
            {
                var seed = seeds[seedIndex];
                var exact = !string.IsNullOrEmpty(seed.Score.FileName);
                for (var suffix = 0; ; suffix++)
                {
                    var name = exact ? seed.Score.FileName
                        : seed.Score.Population.ToString(CultureInfo.InvariantCulture) + CollisionSuffix(suffix) + ".txt";
                    if (!exact && persisted.Contains(name)) continue;
                    var path = Path.Combine(target, name);
                    try
                    {
                        var stream = new FileStream(path, FileMode.CreateNew, FileAccess.Write, FileShare.None,
                            4096, FileOptions.Asynchronous);
                        created.Add((path, stream, encoded[seedIndex]));
                        break;
                    }
                    catch (IOException) when (!exact && File.Exists(path)) { }
                }
            }
            foreach (var output in created)
            {
                await output.Stream.WriteAsync(output.Bytes);
            }
            foreach (var output in created) await output.Stream.DisposeAsync();
            return created.Select(value => value.Path).ToArray();
        }
        catch
        {
            foreach (var output in created)
            {
                await output.Stream.DisposeAsync();
                File.Delete(output.Path);
            }
            throw;
        }
    }

    private static string CollisionSuffix(int index)
    {
        // HDRoutines.NamefileRecursive is a two-character formula, not Excel
        // columns: zz is followed by {a..{z, then the invalid Windows name |a.
        if (index is < 0 or >= 729)
            throw new IOException("Original population seed collision suffix exceeds the valid Windows filename range.");
        if (index == 0) return string.Empty;
        if (index <= 26) return ((char)('a' + index - 1)).ToString();
        return new string(new[] { (char)('a' - 1 + (index - 1) / 26), (char)('a' + (index - 1) % 26) });
    }

    private static bool IsNativeSuffix(string suffix) => suffix.Length switch
    {
        0 => true,
        1 => suffix[0] is >= 'a' and <= 'z',
        2 => suffix[0] is >= 'a' and <= '{' && suffix[1] is >= 'a' and <= 'z',
        _ => false,
    };
}
