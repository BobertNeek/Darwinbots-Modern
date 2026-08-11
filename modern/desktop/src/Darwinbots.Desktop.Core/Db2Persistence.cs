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
        var lines = new List<string> { "-1", "0" };
        var speciesStart = lines.Count;
        for (var index = 0; index < 40; index++) lines.Add("0");
        lines[speciesStart + 5] = "\"Robots\"";
        lines[speciesStart + 6] = "0";
        lines[speciesStart + 7] = "\"Modern\"";
        lines[speciesStart + 8] = "#FALSE#";
        lines[speciesStart + 9] = "#FALSE#";
        lines[speciesStart + 10] = "0";
        lines[speciesStart + 12] = "1000";

        var globalStart = lines.Count;
        lines.Add("\"\"");
        lines.Add("1");
        lines.Add("0");
        lines.Add(Float(settings.WorldWidth));
        lines.Add(Float(settings.WorldHeight));
        lines.Add(settings.OrganismCapacity.ToString(CultureInfo.InvariantCulture));
        lines.Add("#FALSE#");
        lines.Add("#FALSE#");
        lines.Add("0");
        lines.Add("#TRUE#");
        lines.Add("500000");
        lines.Add("0");
        while (lines.Count < globalStart + 37) lines.Add("0");
        lines.Add("0");

        var modern = Convert.ToBase64String(Encoding.UTF8.GetBytes(settings.ToJson()));
        lines.Add(ModernExtensionMarker);
        lines.Add(modern);
        return Encoding.UTF8.GetBytes(string.Join("\r\n", lines) + "\r\n");
    }

    public static DesktopSettings Decode(ReadOnlySpan<byte> bytes)
    {
        if (bytes.IsEmpty) throw new InvalidDataException("DB2 settings are empty.");
        var text = Encoding.UTF8.GetString(bytes);
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
            || speciesMinusOne < 0)
            throw new InvalidDataException("DB2 settings species count is invalid.");
        var globalStart = 2 + (speciesMinusOne + 1) * 40;
        if (lines.Length <= globalStart + 9)
            throw new InvalidDataException("DB2 settings global record is truncated.");
        return new DesktopSettings
        {
            WorldWidth = ParseFloat(lines[globalStart + 3], "world width"),
            WorldHeight = ParseFloat(lines[globalStart + 4], "world height"),
            OrganismCapacity = ParseInt(lines[globalStart + 5], "maximum population"),
        };
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
        var builder = new StringBuilder().AppendLine(Header);
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
        if (includeHeader) builder.AppendLine(Header);
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

    private static GenomeRecord Record(
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
        dnaLength, offspring, kills, energy, chloroplasts, dna);

    private static string SpeciesName(EngineSnapshot snapshot, uint species) =>
        SpeciesName(snapshot.Species, species);

    private static string SpeciesName(IReadOnlyList<SpeciesSnapshot> species, uint index) =>
        index < species.Count ? species[(int)index].Name : $"Species-{index}";

    private static void Append(StringBuilder builder, GenomeRecord record)
    {
        builder.AppendLine(string.Join(",", new[]
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
            "0",
            record.Energy.ToString(CultureInfo.InvariantCulture),
            record.Chloroplasts.ToString(CultureInfo.InvariantCulture),
        }));
        builder.Append(record.Dna.Replace("\r\n", "\n", StringComparison.Ordinal));
        if (!record.Dna.EndsWith('\n')) builder.Append("\r\n");
    }

    private static string Csv(string value) =>
        value.IndexOfAny([',', '"', '\r', '\n']) < 0
            ? value
            : $"\"{value.Replace("\"", "\"\"", StringComparison.Ordinal)}\"";

    private sealed record GenomeRecord(
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
        int Energy,
        int Chloroplasts,
        string Dna);
}

public sealed record Db2OrganismFile(
    [property: JsonPropertyName("version")] ushort Version,
    [property: JsonPropertyName("tick")] ulong Tick,
    [property: JsonPropertyName("members")] IReadOnlyList<Db2OrganismMember> Members,
    [property: JsonPropertyName("ties")] IReadOnlyList<Db2OrganismTie> Ties)
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
        var payload = JsonSerializer.SerializeToUtf8Bytes(this, JsonOptions);
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
        return file with { Version = version };
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
    [property: JsonPropertyName("body")] int Body);

public sealed record Db2OrganismTie(
    [property: JsonPropertyName("first")] Db2OrganismKey First,
    [property: JsonPropertyName("second")] Db2OrganismKey Second,
    [property: JsonPropertyName("rest_length")] float RestLength);

public sealed record Db2OrganismKey(
    [property: JsonPropertyName("slot")] uint Slot,
    [property: JsonPropertyName("generation")] uint Generation);
