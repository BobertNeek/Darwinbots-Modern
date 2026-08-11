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
    public int OrganismCapacity { get; init; } = 25_000;
    public float WorldWidth { get; init; } = 16_000f;
    public float WorldHeight { get; init; } = 12_000f;
    public StartingMode StartingMode { get; init; } = StartingMode.StarterBotsAndVegetables;
    public ZerobotSustenance ZerobotSustenance { get; init; } = ZerobotSustenance.DisabledMetabolism;
    public bool AutomaticZerobotProgression { get; init; } = true;
    public int VegetablePopulationCap { get; init; } = 500;
    public int MetabolismCost { get; init; }
    public int VegetableEnergyPerTick { get; init; }
    public int SunlightEnergy { get; init; } = 100;
    public float[] Gravity { get; init; } = [0f, 0f];
    public float Drag { get; init; }
    public float BrownianMotion { get; init; } = 0.5f;
    public Db2PhysicsOptions Physics { get; init; } = Db2PhysicsOptions.Default;
    public Db2ShotOptions Shots { get; init; } = Db2ShotOptions.Default;
    public Db2CorpseDecayOptions CorpseDecay { get; init; } = new();
    public Db2WasteOptions Waste { get; init; } = new();
    public Db2VegetationOptions Vegetation { get; init; } = Db2VegetationOptions.Default;
    public Db2RecordingOptions Recording { get; init; } = new();
    public bool AutoSpeciation { get; init; }
    public float SpeciationGeneticDistancePercent { get; init; } = 20f;
    public bool ToroidalWorld { get; init; } = true;
    public bool WrapX { get; init; } = true;
    public bool WrapY { get; init; } = true;
    public IReadOnlyList<SpeciesImport> Species { get; init; } = [];
    public GlobalSettings GlobalSettings { get; init; } = new();

    public static DesktopSettings FromWorldSetup(WorldSetupOptions setup) => new()
    {
        Backend = setup.Backend,
        TicksPerUpdate = setup.TicksPerUpdate,
        SnapshotEveryTicks = 1,
        OrganismCapacity = setup.PopulationCapacity,
        WorldWidth = setup.WorldWidth,
        WorldHeight = setup.WorldHeight,
        StartingMode = setup.StartingMode,
        ZerobotSustenance = setup.ZerobotSustenance,
        AutomaticZerobotProgression = setup.AutomaticZerobotProgression,
        VegetablePopulationCap = setup.VegetablePopulationCap,
        MetabolismCost = setup.MetabolismCost,
        VegetableEnergyPerTick = setup.VegetableEnergyPerTick,
        SunlightEnergy = setup.SunlightEnergy,
        Gravity = setup.Gravity.ToArray(),
        Drag = setup.Drag,
        BrownianMotion = setup.BrownianMotion,
        Physics = setup.Physics,
        Shots = setup.Shots,
        CorpseDecay = setup.CorpseDecay,
        Waste = setup.Waste,
        Vegetation = setup.Vegetation,
        Recording = setup.Recording,
        AutoSpeciation = setup.AutoSpeciation,
        SpeciationGeneticDistancePercent = setup.SpeciationGeneticDistancePercent,
        ToroidalWorld = setup.ToroidalWorld,
        WrapX = setup.WrapX,
        WrapY = setup.WrapY,
        Species = setup.Species.ToArray(),
        GlobalSettings = setup.GlobalSettings,
    };

    public WorldSetupOptions ToWorldSetup(WorldSetupOptions? fallback = null)
    {
        fallback ??= new WorldSetupOptions();
        return new WorldSetupOptions
        {
            StartingMode = StartingMode,
            ZerobotSustenance = ZerobotSustenance,
            AutomaticZerobotProgression = AutomaticZerobotProgression,
            Backend = Backend,
            Seed = fallback.Seed,
            PopulationCapacity = OrganismCapacity,
            VegetablePopulationCap = VegetablePopulationCap,
            WorldWidth = WorldWidth,
            WorldHeight = WorldHeight,
            MetabolismCost = MetabolismCost,
            VegetableEnergyPerTick = VegetableEnergyPerTick,
            SunlightEnergy = SunlightEnergy,
            Gravity = Gravity.ToArray(),
            Drag = Drag,
            BrownianMotion = BrownianMotion,
            Physics = Physics,
            Shots = Shots,
            CorpseDecay = CorpseDecay,
            Waste = Waste,
            Vegetation = Vegetation,
            Recording = Recording,
            AutoSpeciation = AutoSpeciation,
            SpeciationGeneticDistancePercent = SpeciationGeneticDistancePercent,
            ToroidalWorld = ToroidalWorld,
            WrapX = WrapX,
            WrapY = WrapY,
            TicksPerUpdate = TicksPerUpdate,
            Species = Species.Count == 0 ? fallback.Species.ToArray() : Species.ToArray(),
            GlobalSettings = GlobalSettings,
            LoadSavePath = fallback.LoadSavePath,
        };
    }

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
        OrganismCapacity = ReadInt(root, "capacity", 25_000),
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
