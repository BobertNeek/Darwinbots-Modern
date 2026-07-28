namespace Darwinbots.Desktop.Core;

public sealed record SpeciesMutationRateUpdate(uint Species, float Rate);

public sealed record MutationSettingsUpdate(
    bool Enabled,
    float GlobalMultiplier,
    IReadOnlyList<SpeciesMutationRateUpdate> SpeciesRates)
{
    public static MutationSettingsUpdate Default { get; } = new(true, 1f, []);
}
