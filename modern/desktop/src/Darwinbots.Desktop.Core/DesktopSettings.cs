using System.Text.Json;
using System.Text.Json.Serialization;

namespace Darwinbots.Desktop.Core;

public sealed record DesktopSettings
{
    public const int CurrentSchemaVersion = 1;

    public int SchemaVersion { get; init; } = CurrentSchemaVersion;
    public string Backend { get; init; } = "Auto";
    public uint TicksPerUpdate { get; init; } = 1;
    public uint SnapshotEveryTicks { get; init; } = 1;
    public int OrganismCapacity { get; init; } = 100_000;
    public float WorldWidth { get; init; } = 16_000f;
    public float WorldHeight { get; init; } = 12_000f;

    public string ToJson() => JsonSerializer.Serialize(this, SerializerOptions);

    public static DesktopSettings FromJson(string json)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(json);
        using var document = JsonDocument.Parse(json);
        var root = document.RootElement;
        var version = root.TryGetProperty("schemaVersion", out var versionElement)
            ? versionElement.GetInt32()
            : 0;
        var settings = version switch
        {
            0 => MigrateVersionZero(root),
            CurrentSchemaVersion => JsonSerializer.Deserialize<DesktopSettings>(json, SerializerOptions)
                ?? throw new InvalidDataException("Settings document was empty."),
            _ => throw new NotSupportedException($"Settings schema version {version} is not supported."),
        };
        settings.Validate();
        return settings;
    }

    private static DesktopSettings MigrateVersionZero(JsonElement root) => new()
    {
        Backend = ReadString(root, "backend", "Auto"),
        TicksPerUpdate = ReadUInt(root, "ticksPerUpdate", 1),
        SnapshotEveryTicks = ReadUInt(root, "snapshotEvery", 1),
        OrganismCapacity = ReadInt(root, "capacity", 100_000),
        WorldWidth = ReadFloat(root, "worldWidth", 16_000f),
        WorldHeight = ReadFloat(root, "worldHeight", 12_000f),
    };

    private void Validate()
    {
        if (!new[] { "Auto", "Cpu", "Gpu" }.Contains(Backend, StringComparer.OrdinalIgnoreCase))
            throw new InvalidDataException($"Unknown simulation backend '{Backend}'.");
        if (TicksPerUpdate == 0 || SnapshotEveryTicks == 0)
            throw new InvalidDataException("Tick and snapshot intervals must be positive.");
        if (OrganismCapacity <= 0 || WorldWidth <= 0 || WorldHeight <= 0)
            throw new InvalidDataException("Capacity and world dimensions must be positive.");
    }

    private static string ReadString(JsonElement root, string name, string fallback) =>
        root.TryGetProperty(name, out var value) && value.ValueKind == JsonValueKind.String ? value.GetString() ?? fallback : fallback;
    private static uint ReadUInt(JsonElement root, string name, uint fallback) =>
        root.TryGetProperty(name, out var value) && value.TryGetUInt32(out var parsed) ? parsed : fallback;
    private static int ReadInt(JsonElement root, string name, int fallback) =>
        root.TryGetProperty(name, out var value) && value.TryGetInt32(out var parsed) ? parsed : fallback;
    private static float ReadFloat(JsonElement root, string name, float fallback) =>
        root.TryGetProperty(name, out var value) && value.TryGetSingle(out var parsed) ? parsed : fallback;

    private static readonly JsonSerializerOptions SerializerOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = true,
        Converters = { new JsonStringEnumConverter() },
    };
}
