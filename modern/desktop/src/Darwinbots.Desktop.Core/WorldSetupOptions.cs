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
    public float WorldWidth { get; init; } = 16_000f;
    public float WorldHeight { get; init; } = 12_000f;
    public int MetabolismCost { get; init; } = 1;
    public int VegetableEnergyPerTick { get; init; } = 4;
    public int SunlightEnergy { get; init; } = 100;
    public float[] Gravity { get; init; } = [0f, 0f];
    public float Drag { get; init; }
    public float BrownianMotion { get; init; }
    public uint TicksPerUpdate { get; init; } = 1;
    public IReadOnlyList<SpeciesImport> Species { get; init; } = [];
    public string? LoadSavePath { get; init; }
}
