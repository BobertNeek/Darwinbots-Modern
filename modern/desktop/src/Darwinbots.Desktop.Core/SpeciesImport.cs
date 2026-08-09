namespace Darwinbots.Desktop.Core;

public enum SpeciesPlacementMode
{
    Random,
    Center,
    Fixed,
}

public sealed record SpeciesPlacement(
    SpeciesPlacementMode Mode = SpeciesPlacementMode.Random,
    float X = 0f,
    float Y = 0f);

public sealed record SpeciesRestrictionSettings
{
    public bool KillMultibot { get; init; }
    public bool KillDisqualified { get; init; }
    public bool FixedInPlace { get; init; }
    public bool DisableVision { get; init; }
    public bool DisableDna { get; init; }
    public bool DisableMovement { get; init; }
    public bool DisableReproduction { get; init; }
    public bool VirusImmune { get; init; }
    public bool DisableMutations { get; init; }
    public bool DisableChloroplasts { get; init; }
}

public sealed record SpeciesImport(
    string Name,
    string Dna,
    bool Vegetable,
    uint Color,
    int InitialCount,
    int InitialEnergy,
    int MinimumPopulation,
    bool Reseed,
    float MutationRate)
{
    public string Comment { get; init; } = string.Empty;
    public SpeciesPlacement Placement { get; init; } = new();
    public SkinPointSnapshot[] Skin { get; init; } = DefaultSkin();
    public SpeciesRestrictionSettings Restrictions { get; init; } = new();

    public SpeciesImport Duplicate(string? name = null) => this with
    {
        Name = string.IsNullOrWhiteSpace(name) ? $"{Name} Copy" : name,
        Skin = Skin.ToArray(),
        Restrictions = Restrictions with { },
    };

    public void Validate()
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(Name);
        ArgumentException.ThrowIfNullOrWhiteSpace(Dna);
        if (InitialCount <= 0) throw new ArgumentOutOfRangeException(nameof(InitialCount));
        if (InitialEnergy <= 0) throw new ArgumentOutOfRangeException(nameof(InitialEnergy));
        if (MinimumPopulation < 0) throw new ArgumentOutOfRangeException(nameof(MinimumPopulation));
        if (!float.IsFinite(MutationRate) || MutationRate < 0f)
            throw new ArgumentOutOfRangeException(nameof(MutationRate));
        if (Skin is null || Skin.Length != 4)
            throw new ArgumentException("Species skin must contain four points.", nameof(Skin));
        if (Skin.Any(point => !float.IsFinite(point.Radius) || point.Radius is < 0.15f or > 0.82f))
            throw new ArgumentOutOfRangeException(nameof(Skin));
        if (Placement is null || !float.IsFinite(Placement.X) || !float.IsFinite(Placement.Y))
            throw new ArgumentOutOfRangeException(nameof(Placement));
    }

    private static SkinPointSnapshot[] DefaultSkin() =>
    [
        new(0.45f, 0),
        new(0.55f, 314),
        new(0.45f, 628),
        new(0.55f, 942),
    ];
}

