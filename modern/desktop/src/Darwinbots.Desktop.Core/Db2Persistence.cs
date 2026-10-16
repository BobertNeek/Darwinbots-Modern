using System.Buffers.Binary;
using System.Globalization;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Darwinbots.Desktop.Core;

public static class Db2SettingsCodec
{
    private const string ModernExtensionMarker = "#DARWINBOTS_MODERN_SETTINGS_V1#";

    public static byte[] Encode(DesktopSettings settings)
    {
        ArgumentNullException.ThrowIfNull(settings);
        var lines = LegacySettingsAdapter.Encode(settings);

        var modern = Convert.ToBase64String(Encoding.UTF8.GetBytes(settings.ToJson()));
        lines.Add(ModernExtensionMarker);
        lines.Add(modern);
        return LegacyDnaText.Encode(string.Join("\r\n", lines) + "\r\n");
    }

    public static DesktopSettings Decode(ReadOnlySpan<byte> bytes)
    {
        if (bytes.IsEmpty) throw new InvalidDataException("DB2 settings are empty.");
        var text = LegacyDnaText.Decode(bytes);
        var lines = text.Replace("\r\n", "\n", StringComparison.Ordinal).Split('\n');
        var marker = Array.IndexOf(lines, ModernExtensionMarker);
        if (marker >= 0 && marker + 1 < lines.Length)
        {
            try
            {
                return DesktopSettings.FromJson(Encoding.UTF8.GetString(Convert.FromBase64String(lines[marker + 1])));
            }
            catch (FormatException error)
            {
                throw new InvalidDataException("DB2 settings modern extension is invalid.", error);
            }
        }

        if (lines.Length < 12 || !int.TryParse(lines[0].Trim('"'), NumberStyles.Integer, CultureInfo.InvariantCulture, out _))
            throw new InvalidDataException("DB2 settings header is invalid.");
        if (!int.TryParse(lines[1].Trim('"'), NumberStyles.Integer, CultureInfo.InvariantCulture, out var speciesMinusOne)
            || speciesMinusOne < -1 || speciesMinusOne > 999)
            throw new InvalidDataException("DB2 settings species count is invalid.");
        var globalStart = 2 + (speciesMinusOne + 1) * 40;
        if (lines.Length <= globalStart + 9)
            throw new InvalidDataException("DB2 settings global record is truncated.");
        return LegacySettingsAdapter.Decode(lines, speciesMinusOne + 1, globalStart)
            with { CorpseDecay = DecodeLegacyCorpseDecay(lines, globalStart) };
    }

    private static string Float(float value) => value.ToString("R", CultureInfo.InvariantCulture);

    private static float ParseFloat(string value, string label) =>
        float.TryParse(value.Trim().Trim('"'), NumberStyles.Float, CultureInfo.InvariantCulture, out var parsed)
            ? parsed
            : throw new InvalidDataException($"DB2 settings {label} is invalid.");

    private static int ParseInt(string value, string label) =>
        int.TryParse(value.Trim().Trim('"'), NumberStyles.Integer, CultureInfo.InvariantCulture, out var parsed)
            ? parsed
            : throw new InvalidDataException($"DB2 settings {label} is invalid.");

    private static float OptionalFloat(string[] lines, int index, float fallback, string label) =>
        HasValue(lines, index) ? ParseFloat(lines[index], label) : fallback;

    private static Db2CorpseDecayOptions DecodeLegacyCorpseDecay(string[] lines, int globalStart)
    {
        var enabled = OptionalBool(lines, globalStart + 37, false, "corpse decay enabled");
        var packetSize = OptionalInt(lines, globalStart + 38, 0, "corpse decay packet size");
        var modeValue = OptionalInt(lines, globalStart + 42, 1, "corpse decay mode");
        var period = OptionalUInt(lines, globalStart + 43, 0, "corpse decay period");
        var mode = modeValue switch
        {
            0 or 1 => Db2CorpseDecayMode.None,
            2 => Db2CorpseDecayMode.WasteShot,
            3 => Db2CorpseDecayMode.EnergyShot,
            _ => throw new InvalidDataException($"DB2 settings corpse decay mode '{modeValue}' is invalid."),
        };

        // ManageDeath uses a fixed 15-energy threshold when corpse mode is enabled.
        return new Db2CorpseDecayOptions(enabled, enabled ? 15 : 0, mode, packetSize, period);
    }

    private static bool OptionalBool(string[] lines, int index, bool fallback, string label) =>
        HasValue(lines, index) ? ParseBool(lines[index], label) : fallback;

    private static int OptionalInt(string[] lines, int index, int fallback, string label) =>
        HasValue(lines, index) ? ParseInt(lines[index], label) : fallback;

    private static ulong OptionalUInt(string[] lines, int index, ulong fallback, string label) =>
        HasValue(lines, index) ? ParseUInt(lines[index], label) : fallback;

    private static bool HasValue(string[] lines, int index) =>
        index >= 0 && index < lines.Length && !string.IsNullOrWhiteSpace(lines[index]);

    private static bool ParseBool(string value, string label)
    {
        var normalized = value.Trim().Trim('"').Trim('#');
        if (bool.TryParse(normalized, out var parsed)) return parsed;
        if (int.TryParse(normalized, NumberStyles.Integer, CultureInfo.InvariantCulture, out var integer))
            return integer != 0;
        throw new InvalidDataException($"DB2 settings {label} is invalid.");
    }

    private static ulong ParseUInt(string value, string label) =>
        ulong.TryParse(value.Trim().Trim('"'), NumberStyles.Integer, CultureInfo.InvariantCulture, out var parsed)
            ? parsed
            : throw new InvalidDataException($"DB2 settings {label} is invalid.");
}

public static class Db2GenomeRecordFile
{
    public const string Header = "Rob id,Parent id,Founder name,Generation,Birth cycle,Age,Mutations,New mutations,Dna length,Offspring number,kills,Fitness,Energy,Chloroplasts";

    public static string EncodeLiving(
        EngineSnapshot snapshot,
        IReadOnlyDictionary<OrganismKey, string> dnaByOrganism)
    {
        ArgumentNullException.ThrowIfNull(snapshot);
        ArgumentNullException.ThrowIfNull(dnaByOrganism);
        var builder = new StringBuilder().Append(Header).Append("\r\n");
        foreach (var organism in snapshot.Organisms)
        {
            var key = new OrganismKey(organism.Slot, organism.Generation);
            if (!dnaByOrganism.TryGetValue(key, out var dna)) continue;
            Append(builder, Record(
                organism.UniqueId == 0 ? organism.Slot : organism.UniqueId,
                organism.Parents?.FirstOrDefault()?.Slot ?? 0,
                SpeciesName(snapshot, organism.Species),
                organism.Generation,
                organism.BirthCycle,
                organism.Age,
                organism.TotalMutations,
                organism.NewMutations,
                organism.DnaLength,
                organism.Offspring,
                organism.Kills,
                organism.Energy + organism.Body * 10,
                organism.Chloroplasts,
                dna));
        }
        return builder.ToString();
    }

    public static string EncodeDead(
        IEnumerable<CorpseSnapshot> corpses,
        IReadOnlyList<SpeciesSnapshot> species,
        bool includeHeader)
    {
        ArgumentNullException.ThrowIfNull(corpses);
        ArgumentNullException.ThrowIfNull(species);
        var builder = new StringBuilder();
        if (includeHeader) builder.Append(Header).Append("\r\n");
        foreach (var corpse in corpses)
        {
            if (string.IsNullOrWhiteSpace(corpse.Dna)) continue;
            Append(builder, Record(
                corpse.UniqueId,
                corpse.Parent?.Slot ?? 0,
                SpeciesName(species, corpse.Species),
                corpse.Generation,
                corpse.BirthCycle,
                corpse.Age,
                corpse.TotalMutations,
                corpse.NewMutations,
                corpse.DnaLength,
                corpse.Offspring,
                corpse.Kills,
                corpse.Energy + corpse.Body * 10,
                corpse.Chloroplasts,
                corpse.Dna));
        }
        return builder.ToString();
    }

    public static IReadOnlyList<Db2GenomeRecord> Decode(string text)
    {
        ArgumentNullException.ThrowIfNull(text);
        var normalized = text.Replace("\r\n", "\n", StringComparison.Ordinal)
            .Replace('\r', '\n');
        if (normalized.EndsWith('\n')) normalized = normalized[..^1];
        var lines = normalized.Split('\n');
        if (lines.Length == 0 || !string.Equals(lines[0], Header, StringComparison.Ordinal))
            throw new InvalidDataException("DB2 genome record header is invalid.");

        var records = new List<Db2GenomeRecord>();
        for (var lineIndex = 1; lineIndex < lines.Length;)
        {
            if (string.IsNullOrWhiteSpace(lines[lineIndex]))
            {
                lineIndex++;
                continue;
            }

            var fields = CsvFields(lines[lineIndex]);
            if (fields.Count != 14)
                throw new InvalidDataException("DB2 genome record metadata is invalid.");
            var record = ParseRecord(fields);
            lineIndex++;

            var payload = new List<string>();
            while (lineIndex < lines.Length && !LooksLikeRecord(lines[lineIndex]))
            {
                payload.Add(lines[lineIndex]);
                lineIndex++;
            }
            records.Add(record with { Dna = string.Join("\n", payload) });
        }

        return records;
    }

    private static Db2GenomeRecord ParseRecord(IReadOnlyList<string> fields) => new(
        ParseUInt64(fields[0], "robot id"),
        ParseUInt32(fields[1], "parent id"),
        fields[2],
        ParseUInt32(fields[3], "generation"),
        ParseUInt64(fields[4], "birth cycle"),
        ParseUInt64(fields[5], "age"),
        ParseUInt64(fields[6], "mutations"),
        ParseUInt64(fields[7], "new mutations"),
        ParseUInt32(fields[8], "DNA length"),
        ParseUInt64(fields[9], "offspring number"),
        ParseUInt64(fields[10], "kills"),
        ParseInt(fields[11], "fitness"),
        ParseInt(fields[12], "energy"),
        ParseInt(fields[13], "chloroplasts"),
        string.Empty);

    private static Db2GenomeRecord Record(
        ulong id,
        uint parentId,
        string founder,
        uint generation,
        ulong birthCycle,
        ulong age,
        ulong mutations,
        ulong newMutations,
        uint dnaLength,
        ulong offspring,
        ulong kills,
        int energy,
        int chloroplasts,
        string dna) => new(id, parentId, founder, generation, birthCycle, age, mutations, newMutations,
        dnaLength, offspring, kills, Math.Max(0, energy), energy, chloroplasts, dna);

    private static bool LooksLikeRecord(string line)
    {
        var fields = CsvFields(line);
        if (fields.Count != 14) return false;
        try
        {
            _ = ParseRecord(fields);
            return true;
        }
        catch (InvalidDataException)
        {
            return false;
        }
    }

    private static List<string> CsvFields(string line)
    {
        var fields = new List<string>();
        var field = new StringBuilder();
        var quoted = false;
        for (var index = 0; index < line.Length; index++)
        {
            var character = line[index];
            if (character == '"')
            {
                if (quoted && index + 1 < line.Length && line[index + 1] == '"')
                {
                    field.Append('"');
                    index++;
                }
                else
                {
                    quoted = !quoted;
                }
            }
            else if (character == ',' && !quoted)
            {
                fields.Add(field.ToString());
                field.Clear();
            }
            else
            {
                field.Append(character);
            }
        }
        if (quoted) throw new InvalidDataException("DB2 genome record CSV is unterminated.");
        fields.Add(field.ToString());
        return fields;
    }

    private static ulong ParseUInt64(string value, string label) =>
        ulong.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out var parsed)
            ? parsed
            : throw new InvalidDataException($"DB2 genome record {label} is invalid.");

    private static uint ParseUInt32(string value, string label) =>
        uint.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out var parsed)
            ? parsed
            : throw new InvalidDataException($"DB2 genome record {label} is invalid.");

    private static int ParseInt(string value, string label) =>
        int.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out var parsed)
            ? parsed
            : throw new InvalidDataException($"DB2 genome record {label} is invalid.");

    private static string SpeciesName(EngineSnapshot snapshot, uint species) =>
        SpeciesName(snapshot.Species, species);

    private static string SpeciesName(IReadOnlyList<SpeciesSnapshot> species, uint index) =>
        index < species.Count ? species[(int)index].Name : $"Species-{index}";

    private static void Append(StringBuilder builder, Db2GenomeRecord record)
    {
        builder.Append(string.Join(",", new[]
        {
            record.Id.ToString(CultureInfo.InvariantCulture),
            record.ParentId.ToString(CultureInfo.InvariantCulture),
            Csv(record.Founder),
            record.Generation.ToString(CultureInfo.InvariantCulture),
            record.BirthCycle.ToString(CultureInfo.InvariantCulture),
            record.Age.ToString(CultureInfo.InvariantCulture),
            record.Mutations.ToString(CultureInfo.InvariantCulture),
            record.NewMutations.ToString(CultureInfo.InvariantCulture),
            record.DnaLength.ToString(CultureInfo.InvariantCulture),
            record.Offspring.ToString(CultureInfo.InvariantCulture),
            record.Kills.ToString(CultureInfo.InvariantCulture),
            record.Fitness.ToString(CultureInfo.InvariantCulture),
            record.Energy.ToString(CultureInfo.InvariantCulture),
            record.Chloroplasts.ToString(CultureInfo.InvariantCulture),
        })).Append("\r\n");
        builder.Append(record.Dna.Replace("\r\n", "\n", StringComparison.Ordinal));
        if (!record.Dna.EndsWith('\n')) builder.Append("\r\n");
    }

    private static string Csv(string value) =>
        value.IndexOfAny([',', '"', '\r', '\n']) < 0
            ? value
            : $"\"{value.Replace("\"", "\"\"", StringComparison.Ordinal)}\"";

}

public sealed record Db2GenomeRecord(
    ulong Id,
    uint ParentId,
    string Founder,
    uint Generation,
    ulong BirthCycle,
    ulong Age,
    ulong Mutations,
    ulong NewMutations,
    uint DnaLength,
    ulong Offspring,
    ulong Kills,
    int Fitness,
    int Energy,
    int Chloroplasts,
    string Dna);

public sealed record Db2OrganismFile(
    [property: JsonPropertyName("version")] ushort Version,
    [property: JsonPropertyName("tick")] ulong Tick,
    [property: JsonPropertyName("members")] IReadOnlyList<Db2OrganismMember> Members,
    [property: JsonPropertyName("ties")] IReadOnlyList<Db2OrganismTie> Ties,
    [property: JsonPropertyName("selected_member")] Db2OrganismKey? SelectedMember = null)
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = null,
        WriteIndented = false,
    };

    public byte[] Encode()
    {
        if (Version == 0 || Version > 1) throw new InvalidDataException($"Unsupported DB2 organism version {Version}.");
        if (Members.Count == 0) throw new InvalidDataException("DB2 organism file has no members.");
        var payload = JsonSerializer.SerializeToUtf8Bytes(Normalize(), JsonOptions);
        var bytes = new byte[6 + payload.Length];
        Encoding.ASCII.GetBytes("DB2O").CopyTo(bytes, 0);
        BinaryPrimitives.WriteUInt16LittleEndian(bytes.AsSpan(4), Version);
        payload.CopyTo(bytes, 6);
        return bytes;
    }

    public static Db2OrganismFile Decode(ReadOnlySpan<byte> bytes)
    {
        if (bytes.Length < 6 || !bytes[..4].SequenceEqual("DB2O"u8))
            throw new InvalidDataException("DB2 organism header is invalid.");
        var version = BinaryPrimitives.ReadUInt16LittleEndian(bytes[4..6]);
        var file = JsonSerializer.Deserialize<Db2OrganismFile>(bytes[6..], JsonOptions)
            ?? throw new InvalidDataException("DB2 organism document is empty.");
        if (version == 0 || version > 1 || file.Members.Count == 0)
            throw new InvalidDataException("DB2 organism document is invalid.");
        return (file with { Version = version }).Normalize();
    }

    private Db2OrganismFile Normalize()
    {
        var selected = SelectedMember ?? new Db2OrganismKey(Members[0].Slot, Members[0].Generation);
        var selectedIndex = -1;
        for (var index = 0; index < Members.Count; index++)
        {
            if (Members[index].Slot == selected.Slot && Members[index].Generation == selected.Generation)
            {
                selectedIndex = index;
                break;
            }
        }

        if (selectedIndex < 0)
            throw new InvalidDataException("DB2 organism selected member is not in the member list.");

        var anchor = Members[selectedIndex].Position;
        var members = Members
            .Select((member, index) => (member, index))
            .OrderBy(value => value.index == selectedIndex ? 0 : 1)
            .ThenBy(value => value.index)
            .Select(value => value.member with
            {
                RelativePosition = RelativePositionFrom(anchor, value.member.Position),
            })
            .ToArray();
        return this with { SelectedMember = selected, Members = members };
    }

    private static float[] RelativePositionFrom(float[] anchor, float[] position)
    {
        if (anchor.Length != 2 || position.Length != 2)
            throw new InvalidDataException("DB2 organism positions must contain two coordinates.");
        var relative = new[] { position[0] - anchor[0], position[1] - anchor[1] };
        if (relative.Any(value => !float.IsFinite(value)))
            throw new InvalidDataException("DB2 organism positions must be finite.");
        return relative;
    }
}

public sealed record Db2OrganismMember(
    [property: JsonPropertyName("slot")] uint Slot,
    [property: JsonPropertyName("generation")] uint Generation,
    [property: JsonPropertyName("dna")] string Dna,
    [property: JsonPropertyName("species")] uint Species,
    [property: JsonPropertyName("vegetable")] bool Vegetable,
    [property: JsonPropertyName("corpse")] bool Corpse,
    [property: JsonPropertyName("repopulating")] bool Repopulating,
    [property: JsonPropertyName("position")] float[] Position,
    [property: JsonPropertyName("velocity")] float[] Velocity,
    [property: JsonPropertyName("energy")] int Energy,
    [property: JsonPropertyName("age")] ulong Age,
    [property: JsonPropertyName("birth_cycle")] ulong BirthCycle,
    [property: JsonPropertyName("body")] int Body,
    [property: JsonPropertyName("relative_position")] float[]? RelativePosition = null);

public sealed record Db2OrganismTie(
    [property: JsonPropertyName("first")] Db2OrganismKey First,
    [property: JsonPropertyName("second")] Db2OrganismKey Second,
    [property: JsonPropertyName("rest_length")] float RestLength);

public sealed record Db2OrganismKey(
    [property: JsonPropertyName("slot")] uint Slot,
    [property: JsonPropertyName("generation")] uint Generation);
