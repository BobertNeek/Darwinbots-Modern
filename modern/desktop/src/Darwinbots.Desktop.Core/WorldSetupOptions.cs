namespace Darwinbots.Desktop.Core;

public enum StartingMode
{
    StarterBotsAndVegetables,
    Zerobots,
    ZerobotsAndVegetables,
}

public enum ZerobotSustenance
{
    FeederReproducerVegetables,
    EnergyOnlyVegetables,
    PhotosyntheticZerobots,
    DisabledMetabolism,
    None,
}

public sealed record WorldSetupOptions
{
    public StartingMode StartingMode { get; init; } = StartingMode.StarterBotsAndVegetables;
    public ZerobotSustenance ZerobotSustenance { get; init; } = ZerobotSustenance.DisabledMetabolism;
    public bool AutomaticZerobotProgression { get; init; } = true;
    public string Backend { get; init; } = "Auto";
    public ulong Seed { get; init; } = 1;
    public int PopulationCapacity { get; init; } = 25_000;
    public int VegetablePopulationCap { get; init; } = 500;
    public float WorldWidth { get; init; } = 16_000f;
    public float WorldHeight { get; init; } = 12_000f;
    public int MetabolismCost { get; init; }
    public int VegetableEnergyPerTick { get; init; }
    public int SunlightEnergy { get; init; } = 100;
    public float[] Gravity { get; init; } = [0f, 0f];
    public float Drag { get; init; }
    public float BrownianMotion { get; init; } = 0.5f;
    public Db2PhysicsOptions Physics { get; init; } = Db2PhysicsOptions.Default;
    public Db2ShotOptions Shots { get; init; } = Db2ShotOptions.Default;
    public Db2VegetationOptions Vegetation { get; init; } = Db2VegetationOptions.Default;
    public bool AutoSpeciation { get; init; }
    public float SpeciationGeneticDistancePercent { get; init; } = 20f;
    public bool ToroidalWorld { get; init; } = true;
    public uint TicksPerUpdate { get; init; } = 1;
    public IReadOnlyList<SpeciesImport> Species { get; init; } = [];
    public string? LoadSavePath { get; init; }

    public int EffectiveMetabolismCost => StartingMode == StartingMode.StarterBotsAndVegetables
        ? MetabolismCost
        : ZerobotSustenance == ZerobotSustenance.DisabledMetabolism ? 0 : MetabolismCost;

    public EnvironmentUpdate ToEnvironmentUpdate() => new(
        EffectiveMetabolismCost,
        VegetableEnergyPerTick,
        SunlightEnergy,
        Gravity.ToArray(),
        Drag,
        BrownianMotion,
        Physics,
        Shots,
        Vegetation,
        AutoSpeciation,
        SpeciationGeneticDistancePercent,
        ToroidalWorld);
}
