using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class OrganismVisualGeometryTests
{
    [Fact]
    public void SkinPointsScaleAndRotateWithAim()
    {
        SkinPointSnapshot[] skin =
        [
            new(0.5f, 0),
            new(0.5f, 314),
            new(0.5f, 628),
            new(0.5f, 942),
        ];

        var points = OrganismVisualGeometry.SkinPoints(skin, radius: 10, aim: 314);

        Assert.Equal(4, points.Length);
        Assert.All(points, point => Assert.InRange(
            MathF.Sqrt(point.X * point.X + point.Y * point.Y),
            4.99f,
            5.01f));
        Assert.InRange(points[0].X, 4.99f, 5.01f);
        Assert.InRange(points[0].Y, -0.01f, 0.01f);
    }

    [Fact]
    public void HeadingPointUsesDb2AimUnits()
    {
        var east = OrganismVisualGeometry.HeadingPoint(radius: 8, aim: 0);
        var north = OrganismVisualGeometry.HeadingPoint(radius: 8, aim: 314);

        Assert.InRange(east.X, 7.99f, 8.01f);
        Assert.InRange(east.Y, -0.01f, 0.01f);
        Assert.InRange(north.X, -0.01f, 0.01f);
        Assert.InRange(north.Y, 7.99f, 8.01f);
    }

    [Fact]
    public void NativeForwardSkinAndHeadingAgreeAndEyesUsePublishedAngles()
    {
        var skin = OrganismVisualGeometry.SkinPoints([new(1, 314)], radius: 8, aim: 0);
        var heading = OrganismVisualGeometry.HeadingPoint(radius: 8, aim: 0);
        Assert.Equal(heading, skin[0]);
        var eyes = VisionSnapshot.Default.Eyes.ToArray();
        eyes[4] = eyes[4] with { CenterRadians = 0.321f, HalfWidthRadians = 0.0025f };
        var vision = VisionSnapshot.Default with { Eyes = eyes };
        var sectors = OrganismVisualGeometry.EyeSectors(vision, aim: 0, radius: 8);
        Assert.InRange(sectors[4].StartRadians + sectors[4].SweepRadians / 2f, 0.3209f, 0.3211f);
        Assert.InRange(sectors[4].SweepRadians, 0.00499f, 0.00501f);
    }

    [Fact]
    public void EyeSectorsProduceNineArcsAndPreserveFocus()
    {
        var sectors = OrganismVisualGeometry.EyeSectors(
            VisionSnapshot.Default,
            aim: 0,
            radius: 8);

        Assert.Equal(9, sectors.Length);
        Assert.Equal(4, Array.FindIndex(sectors, sector => sector.Focused));
        Assert.All(sectors, sector => Assert.True(sector.Range > 8));
        Assert.All(sectors, sector => Assert.True(sector.SweepRadians > 0));
    }
}
