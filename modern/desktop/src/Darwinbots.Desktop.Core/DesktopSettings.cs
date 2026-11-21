using System.Text.Json;
using System.Text.Json.Serialization;

namespace Darwinbots.Desktop.Core;

public sealed record DesktopSettings
{
    public const int CurrentSchemaVersion = 1;

    public int SchemaVersion { get; init; } = CurrentSchemaVersion;
    public string Backend { get; init; } = "Auto";
    public ulong? Seed { get; init; }
    public IReadOnlyList<string>? LegacySettingsFields { get; init; }
    public MutationSettingsUpdate MutationSettings { get; init; } = MutationSettingsUpdate.Default;
    public uint TicksPerUpdate { get; init; } = 1;
    public uint SnapshotEveryTicks { get; init; } = 1;
    public int OrganismCapacity { get; init; } = 25_000;
    public float WorldWidth { get; init; } = 16_000f;
    public float WorldHeight { get; init; } = 12_000f;
    public int? NativeFieldSize { get; init; }
    public StartingMode StartingMode { get; init; } = StartingMode.StarterBotsAndVegetables;
    public ZerobotSustenance ZerobotSustenance { get; init; } = ZerobotSustenance.DisabledMetabolism;
    public bool AutomaticZerobotProgression { get; init; } = true;
    public bool OrdinaryRestartOnExtinction { get; init; }
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
    public bool ShapesVisible { get; init; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public Db2ShapeCreationOptions? ShapeCreation { get; init; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public Db2ShapeDriftOptions? ShapeDrift { get; init; }
    public bool FixedBotRadii { get; init; }
    public bool TiesEnabled { get; init; } = true;
    public bool RobotFixingAllowed { get; init; } = true;
    public bool AsexualReproductionAllowed { get; init; } = true;
    public IReadOnlyList<ObstacleSnapshot> Obstacles { get; init; } = [];
    public bool WrapX { get; init; } = true;
    public bool WrapY { get; init; } = true;
    public IReadOnlyList<SpeciesImport> Species { get; init; } = [];
    public GlobalSettings GlobalSettings { get; init; } = new();

    public static DesktopSettings FromWorldSetup(WorldSetupOptions setup) => new()
    {
        Backend = setup.Backend,
        Seed = setup.Seed,
        LegacySettingsFields = setup.LegacySettingsFields,
        MutationSettings = setup.MutationSettings,
        TicksPerUpdate = setup.TicksPerUpdate,
        SnapshotEveryTicks = 1,
        OrganismCapacity = setup.PopulationCapacity,
        WorldWidth = setup.WorldWidth,
        WorldHeight = setup.WorldHeight,
        NativeFieldSize = setup.NativeFieldSize,
        StartingMode = setup.StartingMode,
        ZerobotSustenance = setup.ZerobotSustenance,
        AutomaticZerobotProgression = setup.AutomaticZerobotProgression,
        OrdinaryRestartOnExtinction = setup.OrdinaryRestartOnExtinction,
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
        ShapesVisible = setup.ShapesVisible, FixedBotRadii = setup.FixedBotRadii, TiesEnabled = setup.TiesEnabled,
        ShapeCreation = setup.ShapeCreation,
        ShapeDrift = setup.ShapeDrift,
        RobotFixingAllowed = setup.RobotFixingAllowed, AsexualReproductionAllowed = setup.AsexualReproductionAllowed,
        Obstacles = setup.Obstacles.ToArray(),
        WrapX = setup.WrapX,
        WrapY = setup.WrapY,
        Species = setup.Species.ToArray(),
        GlobalSettings = setup.GlobalSettings,
    };

    public static DesktopSettings FromLiveWorld(
        WorldSetupOptions startup, WorldConfiguration configuration, EngineSnapshot snapshot)
    {
        var environment = configuration.Environment;
        var settings = startup.GlobalSettings;
        return FromWorldSetup(startup with
        {
            Backend = snapshot.Backend is "CPU" or "GPU" ? snapshot.Backend : startup.Backend,
            Seed = configuration.Seed ?? startup.Seed,
            NativeFieldSize = configuration.NativeFieldSize,
            PopulationCapacity = configuration.PopulationCapacity,
            VegetablePopulationCap = configuration.VegetablePopulationCap,
            WorldWidth = snapshot.WorldSize[0],
            WorldHeight = snapshot.WorldSize[1],
            MetabolismCost = environment.MetabolismCost,
            VegetableEnergyPerTick = environment.VegetableEnergyPerTick,
            SunlightEnergy = environment.SunlightEnergy,
            Gravity = environment.Gravity,
            Drag = environment.Drag,
            BrownianMotion = environment.BrownianMotion,
            Physics = environment.Physics,
            Shots = environment.Shots,
            CorpseDecay = environment.CorpseDecay ?? new(),
            Waste = environment.Waste ?? new(),
            Vegetation = environment.Vegetation,
            Recording = environment.Recording ?? new(),
            AutoSpeciation = environment.AutoSpeciation,
            SpeciationGeneticDistancePercent = environment.SpeciationGeneticDistancePercent,
            ToroidalWorld = environment.ToroidalWorld,
            WrapX = environment.WrapX,
            WrapY = environment.WrapY,
            ShapesVisible = environment.ShapesVisible ?? startup.ShapesVisible,
            FixedBotRadii = configuration.FixedBotRadii ?? startup.FixedBotRadii,
            TiesEnabled = snapshot.TiesEnabled,
            RobotFixingAllowed = snapshot.RobotFixingAllowed,
            AsexualReproductionAllowed = snapshot.AsexualReproductionAllowed,
            ShapeDrift = configuration.ShapeDrift ?? startup.ShapeDrift,
            Obstacles = snapshot.Obstacles,
            Species = (configuration.StartupSpecies ?? configuration.Species)
                .Select(species => species with { SpeciesId = null })
                .ToArray(),
            MutationSettings = new(snapshot.MutationSettings.Enabled, snapshot.MutationSettings.GlobalMultiplier, [])
            {
                Profile = snapshot.MutationSettings.Profile,
            },
            GlobalSettings = settings with
            {
                Main = settings.Main with
                {
                    BodyCutoff = configuration.BodyCutoff,
                    GreedyEnforcement = configuration.GreedyReproduction,
                    SafeMode = environment.Recording?.SafeModeBackup ?? false,
                    // Configuration already contains the main epigenetic factors.
                    EpigeneticReset = false,
                    EpigeneticAmplification = 1.0,
                    TemporaryGeneData = false,
                },
                Costs = configuration.Costs,
                AdvancedMutation = configuration.AdvancedMutation,
                League = settings.League with
                {
                    DisqualificationRule = configuration.DisqualificationRule,
                    RecognitionFudging = configuration.RecognitionFudging,
                },
            },
        });
    }

    public WorldSetupOptions ToWorldSetup(WorldSetupOptions? fallback = null)
    {
        fallback ??= new WorldSetupOptions();
        return new WorldSetupOptions
        {
            StartingMode = StartingMode,
            ZerobotSustenance = ZerobotSustenance,
            AutomaticZerobotProgression = AutomaticZerobotProgression,
            OrdinaryRestartOnExtinction = OrdinaryRestartOnExtinction,
            Backend = Backend,
            Seed = Seed ?? fallback.Seed,
            LegacySettingsFields = LegacySettingsFields,
            MutationSettings = MutationSettings,
            PopulationCapacity = OrganismCapacity,
            VegetablePopulationCap = VegetablePopulationCap,
            WorldWidth = WorldWidth,
            WorldHeight = WorldHeight,
            NativeFieldSize = NativeFieldSize,
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
            ShapesVisible = ShapesVisible, FixedBotRadii = FixedBotRadii, TiesEnabled = TiesEnabled,
            ShapeCreation = ShapeCreation,
            ShapeDrift = ShapeDrift,
            RobotFixingAllowed = RobotFixingAllowed, AsexualReproductionAllowed = AsexualReproductionAllowed,
            Obstacles = Obstacles.ToArray(),
            WrapX = WrapX,
            WrapY = WrapY,
            TicksPerUpdate = TicksPerUpdate,
            Species = Species.Count == 0 && LegacySettingsFields is null ? fallback.Species.ToArray() : Species.ToArray(),
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
        if (settings.ShapeDrift is null && !root.TryGetProperty("shapeDrift", out _))
            settings = settings with { ShapeDrift = LegacySettingsAdapter.ReadRetainedShapeDrift(settings.LegacySettingsFields) };
        if (!root.TryGetProperty("nativeFieldSize", out _))
            settings = settings with { NativeFieldSize = LegacySettingsAdapter.ReadRetainedFieldSize(settings.LegacySettingsFields) };
        if (!root.TryGetProperty("ordinaryRestartOnExtinction", out _)
            && LegacySettingsAdapter.ReadRetainedFlag(settings.LegacySettingsFields, LegacySettingsAdapter.OrdinaryRestartOffset))
            settings = settings with { OrdinaryRestartOnExtinction = true };
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
        if (NativeFieldSize is { } size && !Db2FieldSize.IsValid(size))
            throw new InvalidDataException("Original field size must be between 1 and 25.");
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
