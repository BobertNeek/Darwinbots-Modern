namespace Darwinbots.Desktop.Core;

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
    public void Validate()
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(Name);
        ArgumentException.ThrowIfNullOrWhiteSpace(Dna);
        if (InitialCount <= 0) throw new ArgumentOutOfRangeException(nameof(InitialCount));
        if (InitialEnergy <= 0) throw new ArgumentOutOfRangeException(nameof(InitialEnergy));
        if (MinimumPopulation < 0) throw new ArgumentOutOfRangeException(nameof(MinimumPopulation));
        if (!float.IsFinite(MutationRate) || MutationRate < 0f)
            throw new ArgumentOutOfRangeException(nameof(MutationRate));
    }
}

