namespace Darwinbots.Desktop.Core;

public sealed record PlayerBotBinding(int KeyCode, ushort MemoryAddress, int Value, bool Invert = false)
{
    public void Validate()
    {
        if (KeyCode is < 0 or > 65_535)
            throw new ArgumentOutOfRangeException(nameof(KeyCode));
        if (MemoryAddress is < 1 or > 999)
            throw new ArgumentOutOfRangeException(nameof(MemoryAddress));
        if (Value is < -32_000 or > 32_000)
            throw new ArgumentOutOfRangeException(nameof(Value));
    }

    public override string ToString() =>
        $"key {KeyCode} -> mem {MemoryAddress} = {Value}{(Invert ? " (inverted)" : string.Empty)}";
}

public sealed record PlayerBotConfig(
    bool Enabled,
    OrganismKey? Focus,
    IReadOnlyList<PlayerBotBinding> Bindings)
{
    public static PlayerBotConfig Disabled { get; } = new(false, null, []);
}

public sealed record PlayerBotSnapshot(
    bool Enabled,
    OrganismKey? Focus,
    IReadOnlyList<PlayerBotBinding> Bindings,
    IReadOnlyList<int> PressedKeys)
{
    public static PlayerBotSnapshot Default { get; } = new(false, null, [], []);
}

public sealed record RestrictionColumn(
    bool KillMultibot = false,
    bool KillDisqualified = false,
    bool DisableChloroplasts = false,
    bool FixedInPlace = false,
    bool DisableVision = false,
    bool DisableDna = false,
    bool DisableReproduction = false,
    bool VirusImmune = false,
    bool DisableMutations = false,
    bool DisableMovement = false);

public sealed record RestrictionOptions(
    RestrictionColumn NonRepopulating,
    RestrictionColumn Repopulating)
{
    public static RestrictionOptions Default { get; } = new(new(), new());
}

public static class PlayerBotKeyMapFile
{
    public static void Save(string path, IEnumerable<PlayerBotBinding> bindings)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(path);
        ArgumentNullException.ThrowIfNull(bindings);
        var values = bindings.ToArray();
        foreach (var binding in values) binding.Validate();
        var lines = values.SelectMany(binding => new[]
        {
            binding.KeyCode.ToString(System.Globalization.CultureInfo.InvariantCulture),
            binding.MemoryAddress.ToString(System.Globalization.CultureInfo.InvariantCulture),
            binding.Value.ToString(System.Globalization.CultureInfo.InvariantCulture),
            binding.Invert ? "True" : "False",
        });
        File.WriteAllText(path, string.Concat(lines.Select(line => line + "\r\n")));
    }

    public static IReadOnlyList<PlayerBotBinding> Load(string path)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(path);
        var lines = File.ReadAllLines(path)
            .Select(line => line.Trim())
            .Where(line => line.Length > 0)
            .ToArray();
        if (lines.Length % 4 != 0)
            throw new InvalidDataException("PlayerBot key-map files must contain four lines per binding.");

        var result = new List<PlayerBotBinding>(lines.Length / 4);
        for (var index = 0; index < lines.Length; index += 4)
        {
            if (!int.TryParse(lines[index], out var key)
                || !ushort.TryParse(lines[index + 1], out var address)
                || !int.TryParse(lines[index + 2], out var value))
            {
                throw new InvalidDataException($"Invalid PlayerBot binding at line {index + 1}.");
            }
            var binding = new PlayerBotBinding(key, address, value, ParseBoolean(lines[index + 3]));
            binding.Validate();
            result.Add(binding);
        }
        return result;
    }

    private static bool ParseBoolean(string value) => value.ToLowerInvariant() switch
    {
        "true" or "1" or "yes" => true,
        "false" or "0" or "no" => false,
        _ => throw new InvalidDataException($"Invalid PlayerBot inversion value '{value}'."),
    };
}

public static class RestrictionPresetFile
{
    public static void Save(string path, RestrictionOptions options)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(path);
        ArgumentNullException.ThrowIfNull(options);
        if (options.Repopulating.DisableChloroplasts)
            throw new InvalidDataException(
                "Original restriction presets cannot represent DisableChloroplasts for repopulating species.");
        // Original .resp files omit only the repopulating chloroplast switch.
        var text = string.Concat(Values(options)
            .Where((_, index) => index != 12)
            .Select(value => value ? "1\r\n" : "0\r\n"));
        File.WriteAllText(path, text);
    }

    public static RestrictionOptions Load(string path)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(path);
        var lines = File.ReadAllLines(path)
            .Select(line => line.Trim())
            .Where(line => line.Length > 0)
            .ToArray();
        if (lines.Length is not (19 or 20))
            throw new InvalidDataException("Restriction preset files must contain nineteen original or twenty modern lines.");
        var values = lines.Select(ParseBoolean).ToArray();
        if (values.Length == 19)
            values = [.. values[..12], false, .. values[12..]];
        return new RestrictionOptions(Column(values, 0), Column(values, 10));
    }

    private static bool[] Values(RestrictionOptions options) =>
    [
        .. Values(options.NonRepopulating),
        .. Values(options.Repopulating),
    ];

    private static bool[] Values(RestrictionColumn column) =>
    [
        column.KillMultibot,
        column.KillDisqualified,
        column.DisableChloroplasts,
        column.FixedInPlace,
        column.DisableVision,
        column.DisableDna,
        column.DisableReproduction,
        column.VirusImmune,
        column.DisableMutations,
        column.DisableMovement,
    ];

    private static RestrictionColumn Column(bool[] values, int offset) => new(
        values[offset], values[offset + 1], values[offset + 2], values[offset + 3], values[offset + 4],
        values[offset + 5], values[offset + 6], values[offset + 7], values[offset + 8], values[offset + 9]);

    private static bool ParseBoolean(string value) => value.ToLowerInvariant() switch
    {
        "true" or "1" or "yes" => true,
        "false" or "0" or "no" => false,
        _ => throw new InvalidDataException($"Invalid restriction preset value '{value}'."),
    };
}
