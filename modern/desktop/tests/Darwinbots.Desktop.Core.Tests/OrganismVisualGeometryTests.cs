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
        var forward = OrganismVisualGeometry.HeadingPoint(radius: 8, aim: 0);
        var right = OrganismVisualGeometry.HeadingPoint(radius: 8, aim: 314);

        Assert.InRange(forward.X, -0.01f, 0.01f);
        Assert.InRange(forward.Y, 7.99f, 8.01f);
        Assert.InRange(right.X, 7.99f, 8.01f);
        Assert.InRange(right.Y, -0.01f, 0.01f);
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
