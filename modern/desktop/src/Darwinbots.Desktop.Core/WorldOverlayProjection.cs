namespace Darwinbots.Desktop.Core;

public sealed record OverlayGaugeProjection(
    OrganismKey Organism,
    string Label,
    int Value,
    string Color,
    string Text,
    double Ratio);

public sealed record WorldOverlayProjection(
    IReadOnlyList<OverlayGaugeProjection> Gauges,
    bool ShowMovementVectors,
    bool ShowShotImpacts,
    bool ShowResourceGauges,
    bool HasBackgroundImage)
{
    public static WorldOverlayProjection Create(
        IEnumerable<OrganismSnapshot> organisms,
        bool showMovementVectors,
        bool showShotImpacts,
        bool showResourceGauges,
        bool hasBackgroundImage)
    {
        var gauges = new List<OverlayGaugeProjection>();
        foreach (var organism in organisms)
        {
            if (organism.IsHidden) continue;
            var key = new OrganismKey(organism.Slot, organism.Generation);
            AddGauge(gauges, key, "ENERGY", Math.Max(0, organism.Energy), "#45A23D", 20_000);
            AddGauge(gauges, key, "BODY", Math.Max(0, organism.Body), "#E26722", 1_000);
            AddGauge(
                gauges,
                key,
                "DEFENSE",
                Math.Max(0, organism.Shell) +
                Math.Max(0, organism.Slime) +
                Math.Max(0, organism.Venom) +
                Math.Max(0, organism.Poison),
                "#239AC0",
                1_000);
        }

        return new WorldOverlayProjection(
            gauges,
            showMovementVectors,
            showShotImpacts,
            showResourceGauges,
            hasBackgroundImage);
    }

    private static void AddGauge(
        ICollection<OverlayGaugeProjection> gauges,
        OrganismKey organism,
        string label,
        int value,
        string color,
        int maximum)
    {
        gauges.Add(new OverlayGaugeProjection(
            organism,
            label,
            value,
            color,
            $"{label} {value:N0}",
            Math.Clamp(value / (double)maximum, 0.0, 1.0)));
    }
}
