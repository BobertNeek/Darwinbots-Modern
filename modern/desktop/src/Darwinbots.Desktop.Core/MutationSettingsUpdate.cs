namespace Darwinbots.Desktop.Core;

public sealed record SpeciesMutationRateUpdate(uint Species, float Rate);

public sealed record MutationRateUpdate(double OneIn, double Mean, double StdDev)
{
    public static MutationRateUpdate Default { get; } = new(5_000, 3, 1);
}

public sealed record MutationCategoryDistributionUpdate(
    double Number,
    double Dereference,
    double Operator,
    double Logic,
    double Comparison,
    double Flow)
{
    public static MutationCategoryDistributionUpdate Default { get; } =
        new(100d / 6d, 100d / 6d, 100d / 6d, 100d / 6d, 100d / 6d, 100d / 6d);
}

public sealed record MutationProfileUpdate(
    MutationRateUpdate Point,
    MutationRateUpdate Point2,
    MutationRateUpdate Delta,
    MutationRateUpdate CopyError,
    MutationRateUpdate CopyError2,
    MutationRateUpdate Insertion,
    MutationRateUpdate Reversal,
    MutationRateUpdate MinorDeletion,
    MutationRateUpdate MajorDeletion,
    MutationRateUpdate Amplification,
    MutationRateUpdate Translocation,
    byte PointValueChangePercent,
    byte CopyErrorValueChangePercent,
    MutationCategoryDistributionUpdate PointCategoryDistribution,
    MutationCategoryDistributionUpdate CopyErrorCategoryDistribution)
{
    public static MutationProfileUpdate Default { get; } = new(
        new(5_000, 3, 1),
        new(5_000, 3, 1),
        new(5_000, 500, 150),
        new(5_000, 1, 0),
        new(5_000, 1, 0),
        new(5_000, 1, 0),
        new(5_000, 3, 1),
        new(5_000, 1, 0),
        new(5_000, 3, 1),
        new(5_000, 250, 75),
        new(5_000, 250, 75),
        80,
        80,
        MutationCategoryDistributionUpdate.Default,
        MutationCategoryDistributionUpdate.Default);
}

public sealed record MutationSettingsUpdate(
    bool Enabled,
    float GlobalMultiplier,
    IReadOnlyList<SpeciesMutationRateUpdate> SpeciesRates)
{
    public static MutationSettingsUpdate Default { get; } = new(true, 1f, []);
    public MutationProfileUpdate? Profile { get; init; }
}
