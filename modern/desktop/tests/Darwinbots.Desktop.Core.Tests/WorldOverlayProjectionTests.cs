using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class WorldOverlayProjectionTests
{
    [Fact]
    public void HiddenEvolutionBaselinesDoNotProduceResourceGauges()
    {
        var visible = new OrganismSnapshot(1, 1, [100, 100], [0, 0], 500, 0);
        var hidden = visible with { Slot = 2, IsHidden = true };
        var projection = WorldOverlayProjection.Create([hidden, visible], true, true, true, false);
        Assert.Equal(3, projection.Gauges.Count);
        Assert.All(projection.Gauges, gauge => Assert.Equal(1U, gauge.Organism.Slot));
    }

    [Fact]
    public void ProjectsResourceGaugesAndViewportOverlayFlags()
    {
        var organism = new OrganismSnapshot(
            7,
            3,
            [100, 200],
            [1.5f, -0.5f],
            875,
            19,
            Body: 321,
            Shell: 3,
            Slime: 4,
            Venom: 5,
            Poison: 6);

        var projection = WorldOverlayProjection.Create(
            [organism],
            showMovementVectors: false,
            showShotImpacts: true,
            showResourceGauges: false,
            hasBackgroundImage: true);

        Assert.Collection(
            projection.Gauges,
            gauge => Assert.Equal(new OverlayGaugeProjection(
                new OrganismKey(7, 3),
                "ENERGY",
                875,
                "#45A23D",
                "ENERGY 875",
                0.04375), gauge),
            gauge => Assert.Equal(new OverlayGaugeProjection(
                new OrganismKey(7, 3),
                "BODY",
                321,
                "#E26722",
                "BODY 321",
                0.321), gauge),
            gauge => Assert.Equal(new OverlayGaugeProjection(
                new OrganismKey(7, 3),
                "DEFENSE",
                18,
                "#239AC0",
                "DEFENSE 18",
                0.018), gauge));

        Assert.False(projection.ShowMovementVectors);
        Assert.True(projection.ShowShotImpacts);
        Assert.False(projection.ShowResourceGauges);
        Assert.True(projection.HasBackgroundImage);
    }
}
