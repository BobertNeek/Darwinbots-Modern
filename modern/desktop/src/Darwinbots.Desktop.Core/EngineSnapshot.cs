namespace Darwinbots.Desktop.Core;

public sealed record SkinPointSnapshot(float Radius, int Angle);

public sealed record EyeSnapshot(
    int Direction,
    int Width,
    float CenterRadians,
    float HalfWidthRadians,
    float Range,
    int Value);

public sealed record VisionSnapshot(byte FocusEye, EyeSnapshot[] Eyes)
{
    public static VisionSnapshot Default => new(4, VisualSnapshotDefaults.DefaultEyes());
}

public sealed record VisualPhenotypeSnapshot(
    ulong LineageId,
    uint Color,
    SkinPointSnapshot[] Skin,
    uint AccumulatedMutations)
{
    public static VisualPhenotypeSnapshot Default =>
        new(0, 0xff62a844, VisualSnapshotDefaults.DefaultSkin(), 0);
}

public sealed record OrganismSnapshot(
    uint Slot,
    uint Generation,
    float[] Position,
    float[] Velocity,
    int Energy,
    ulong Age,
    uint Species = 0,
    bool Vegetable = false,
    int Body = 100,
    int Waste = 0,
    int Shell = 0,
    int Slime = 0,
    int Venom = 0,
    int Poison = 0,
    int Chloroplasts = 0,
    int Aim = 0,
    int Paralyzed = 0,
    int Poisoned = 0,
    OrganismKey?[]? Parents = null)
{
    public bool IsCorpse { get; init; }
    public bool IsHidden { get; init; }
    public VisualPhenotypeSnapshot Phenotype { get; init; } = VisualPhenotypeSnapshot.Default;
    public VisionSnapshot Vision { get; init; } = VisionSnapshot.Default;
    public uint CurrentGene { get; init; }
    public uint ConditionCount { get; init; }
    public bool[] ActivationBitmap { get; init; } = [];
    public IReadOnlyList<ActivationFrameSnapshot> ActivationHistory { get; init; } = [];
    public string DebugText { get; init; } = string.Empty;
    public IReadOnlyList<MemoryCellSnapshot> Memory { get; init; } = [];
    public float Mass { get; init; }
    public float Radius { get; init; }
    public int Touch { get; init; }
    public int Taste { get; init; }
    public int PermanentWaste { get; init; }
    public ulong Offspring { get; init; }
    public ulong Descendants { get; init; }
    public ulong SimulationId { get; init; }
    public ulong UniqueId { get; init; }
    public int VirusTimer { get; init; }
    public ulong NewMutations { get; init; }
    public ulong TotalMutations { get; init; }
    public ulong? LastMutationCycle { get; init; }
    public string? LastMutationKind { get; init; }
    public IReadOnlyList<string> MutationLog { get; init; } = [];
    public IReadOnlyDictionary<string, ulong> MutationClassCounts { get; init; } =
        new Dictionary<string, ulong>();
    public int Subspecies { get; init; }
    public MutationProfileUpdate MutationProfile { get; init; } = MutationProfileUpdate.Default;
    public uint DnaLength { get; init; }
    public uint GeneCount { get; init; }
    public ulong BirthCycle { get; init; }
    public ulong Kills { get; init; }
}

public sealed record ActivationFrameSnapshot(ulong Tick, bool[] Activations)
{
    public int ActiveGeneCount => Activations.Count(active => active);
}

public sealed record MemoryCellSnapshot(ushort Address, int Value);

public sealed record SpeciesSnapshot(
    string Name,
    bool Vegetable,
    uint Color,
    int MinimumPopulation,
    bool Reseed,
    float MutationRate = 0f);

public sealed record MutationSettingsSnapshot(bool Enabled, float GlobalMultiplier)
{
    public static MutationSettingsSnapshot Default { get; } = new(true, 1f);
    public MutationProfileUpdate Profile { get; init; } = MutationProfileUpdate.Default;
}

public sealed record SimulationStatsSnapshot(
    ulong Births,
    ulong Deaths,
    ulong ShotsFired,
    ulong EnergyHarvested,
    ulong Mutations,
    ulong TiesCreated,
    long TotalEnergy,
    ulong EnergyDonated = 0,
    ulong Reseeds = 0,
    ulong SelfReproductions = 0,
    ulong FeedingEvents = 0,
    ulong IntentionalMovementEvents = 0,
    ulong ProjectileImpacts = 0,
    ulong ProjectileEffects = 0,
    ulong PlantEnergyGenerated = 0)
{
    public static SimulationStatsSnapshot Empty { get; } = new(0, 0, 0, 0, 0, 0, 0);
}

public sealed record RenderInstanceSnapshot(
    uint Slot,
    float[] Position,
    float Radius,
    uint Color)
{
    public uint Generation { get; init; }
    public int Aim { get; init; }
    public SkinPointSnapshot[] Skin { get; init; } = VisualSnapshotDefaults.DefaultSkin();
    public ulong LineageId { get; init; }
    public bool Vegetable { get; init; }
}

internal static class VisualSnapshotDefaults
{
    public static SkinPointSnapshot[] DefaultSkin() =>
    [
        new(0.45f, 0),
        new(0.55f, 314),
        new(0.45f, 628),
        new(0.55f, 942),
    ];

    public static EyeSnapshot[] DefaultEyes() => Enumerable.Range(0, 9)
        .Select(index => new EyeSnapshot(
            0,
            0,
            NormalizeRadians((4 - index) * MathF.PI / 18f),
            MathF.PI / 36f,
            1_440f,
            0))
        .ToArray();

    private static float NormalizeRadians(float value) =>
        (value % MathF.Tau + MathF.Tau) % MathF.Tau;
}

public sealed record ObstacleSnapshot(uint Id, float[] Minimum, float[] Maximum)
{
    public uint Color { get; init; } = 0xffb9bdb6u;
    public float Opacity { get; init; } = 1f;
    public float[] Drift { get; init; } = [0f, 0f];
    public string VisionPolicy { get; init; } = "opaque";
    public string ShotPolicy { get; init; } = "absorb";
}
public sealed record TeleporterSnapshot(uint Id, float[] Center, float Radius, float[] Destination)
{
    public string Mode { get; init; } = "intrasim";
    public uint Color { get; init; } = 0xffb9bdb6u;
    public float[] Velocity { get; init; } = [0f, 0f];
    public string Path { get; init; } = string.Empty;
    public string InboundPath { get; init; } = string.Empty;
    public string OutboundPath { get; init; } = string.Empty;
    public bool Inbound { get; init; } = true;
    public bool Outbound { get; init; }
    public bool Local { get; init; } = true;
    public bool Internet { get; init; }
    public bool DriftHorizontal { get; init; }
    public bool DriftVertical { get; init; }
    public bool FixedPosition { get; init; } = true;
    public bool Highlight { get; init; }
    public bool TeleportVegetables { get; init; } = true;
    public bool TeleportCorpses { get; init; } = true;
    public bool TeleportHeterotrophs { get; init; } = true;
    public bool RespectShapes { get; init; }
    public ulong Sent { get; init; }
    public ulong Received { get; init; }
    public uint PollInterval { get; init; } = 1;
    public uint PollCountdown { get; init; }
    public int Capacity { get; init; } = 1;
    public uint BackflowLimit { get; init; }
}
public sealed record TeleporterFilterSnapshot(
    bool TeleportVegetables,
    bool TeleportCorpses,
    bool TeleportHeterotrophs);
public sealed record FileTeleporterSnapshot(
    uint Id,
    float[] Center,
    float Radius,
    string InboundPath,
    string OutboundPath,
    uint PollInterval,
    uint PollCountdown,
    int Capacity,
    TeleporterFilterSnapshot Filters,
    ulong Sent,
    ulong Received,
    string? BackflowGuardPath = null);
public sealed record CorpseSnapshot(float[] Position, float[] Velocity, int Energy, int Body, ulong Age, uint Species = 0, bool Repopulating = false)
{
    public string Dna { get; init; } = string.Empty;
    public uint Generation { get; init; }
    public ulong BirthCycle { get; init; }
    public OrganismKey? Parent { get; init; }
    public ulong Offspring { get; init; }
    public ulong Kills { get; init; }
    public ulong NewMutations { get; init; }
    public ulong TotalMutations { get; init; }
    public uint DnaLength { get; init; }
    public ulong UniqueId { get; init; }
    public int Chloroplasts { get; init; }
}
public sealed record ShotSnapshot(
    OrganismKey Owner,
    float[] Start,
    float[] End,
    float[] Velocity,
    int Kind,
    int Value,
    uint Age,
    uint Range,
    float Energy,
    bool ImpactFlash);
public sealed record TieSnapshot(OrganismKey First, OrganismKey Second, float RestLength);
public sealed partial record HistorySampleSnapshot(
    ulong Tick, int Population, long TotalEnergy, ulong Births, ulong Deaths, ulong Mutations, ulong ShotsFired);

public sealed partial record HistorySampleSnapshot
{
    public double AverageMutations { get; init; }
    public double AverageMutationsPerDnaLength { get; init; }
    public double AverageAge { get; init; }
    public double AverageDescendents { get; init; }
    public double AverageEnergy { get; init; }
    public double AverageDnaLength { get; init; }
    public double AverageDnaConditions { get; init; }
    public double DynamicCostStats { get; init; }
    public double SpeciesDiversity { get; init; }
    public double AverageChloroplasts { get; init; }
    public double SelectiveGeneticDistance { get; init; }
    public double GenerationalDistance { get; init; }
    public double SlowGeneticDistance { get; init; }
    public IReadOnlyList<SpeciesGraphSampleSnapshot> SpeciesGraphs { get; init; } = [];
    public IReadOnlyList<double> DynamicCostSeries { get; init; } = [];
}
public sealed record SpeciesGraphSampleSnapshot(string Name, uint Color, IReadOnlyDictionary<string, double> Values);
public sealed record GraphSamplingSettings(bool Selective = false, bool Generational = false, bool Slow = false);

public sealed record PhaseTimingsSnapshot(
    double Dna, double Intent, double Spatial, double Sensing, double Interactions,
    double Physics, double Lifecycle, double Mutation, double Snapshot)
{
    public static PhaseTimingsSnapshot Empty { get; } = new(0, 0, 0, 0, 0, 0, 0, 0, 0);
}

public sealed record EngineSnapshot(
    ulong Tick,
    int Population,
    string Backend,
    IReadOnlyList<OrganismSnapshot> Organisms)
{
    // Original VB6 cycle; Tick remains the unsigned scheduling clock.
    public int? NativeCycle { get; init; }
    public IReadOnlyList<RenderInstanceSnapshot> RenderInstances { get; init; } = [];
    public IReadOnlyList<SpeciesSnapshot> Species { get; init; } = [];
    public MutationSettingsSnapshot MutationSettings { get; init; } = MutationSettingsSnapshot.Default;
    public bool TiesEnabled { get; init; } = true;
    public bool RobotFixingAllowed { get; init; } = true;
    public bool AsexualReproductionAllowed { get; init; } = true;
    public PlayerBotSnapshot PlayerBot { get; init; } = PlayerBotSnapshot.Default;
    public RestrictionOptions RestrictionOptions { get; init; } = RestrictionOptions.Default;
    public SimulationStatsSnapshot Stats { get; init; } = SimulationStatsSnapshot.Empty;
    public float[] WorldSize { get; init; } = [16_000f, 12_000f];
    public IReadOnlyList<ObstacleSnapshot> Obstacles { get; init; } = [];
    public IReadOnlyList<TeleporterSnapshot> Teleporters { get; init; } = [];
    public IReadOnlyList<FileTeleporterSnapshot> FileTeleporters { get; init; } = [];
    public IReadOnlyList<CorpseSnapshot> Corpses { get; init; } = [];
    public IReadOnlyList<ShotSnapshot> Shots { get; init; } = [];
    public IReadOnlyList<TieSnapshot> Ties { get; init; } = [];
    public IReadOnlyList<HistorySampleSnapshot> History { get; init; } = [];
    public PhaseTimingsSnapshot PhaseTimings { get; init; } = PhaseTimingsSnapshot.Empty;
    public LocalModeSnapshot LocalMode { get; init; } = LocalModeSnapshot.Idle;
    public double TicksPerSecond { get; init; }
    public double SnapshotsPerSecond { get; init; }
    public static EngineSnapshot Empty { get; } = new(0, 0, "UNKNOWN", []);
}
