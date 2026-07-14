namespace Darwinbots.Desktop.Core;

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
    OrganismKey?[]? Parents = null);

public sealed record SpeciesSnapshot(
    string Name,
    bool Vegetable,
    uint Color,
    int MinimumPopulation,
    bool Reseed);

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
    uint Color);

public sealed record ObstacleSnapshot(uint Id, float[] Minimum, float[] Maximum);
public sealed record TeleporterSnapshot(uint Id, float[] Center, float Radius, float[] Destination);
public sealed record CorpseSnapshot(float[] Position, float[] Velocity, int Energy, int Body, ulong Age);
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
public sealed record HistorySampleSnapshot(
    ulong Tick, int Population, long TotalEnergy, ulong Births, ulong Deaths, ulong Mutations, ulong ShotsFired);
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
    public IReadOnlyList<RenderInstanceSnapshot> RenderInstances { get; init; } = [];
    public IReadOnlyList<SpeciesSnapshot> Species { get; init; } = [];
    public SimulationStatsSnapshot Stats { get; init; } = SimulationStatsSnapshot.Empty;
    public float[] WorldSize { get; init; } = [16_000f, 12_000f];
    public IReadOnlyList<ObstacleSnapshot> Obstacles { get; init; } = [];
    public IReadOnlyList<TeleporterSnapshot> Teleporters { get; init; } = [];
    public IReadOnlyList<CorpseSnapshot> Corpses { get; init; } = [];
    public IReadOnlyList<ShotSnapshot> Shots { get; init; } = [];
    public IReadOnlyList<TieSnapshot> Ties { get; init; } = [];
    public IReadOnlyList<HistorySampleSnapshot> History { get; init; } = [];
    public PhaseTimingsSnapshot PhaseTimings { get; init; } = PhaseTimingsSnapshot.Empty;
    public double TicksPerSecond { get; init; }
    public double SnapshotsPerSecond { get; init; }
    public static EngineSnapshot Empty { get; } = new(0, 0, "UNKNOWN", []);
}
