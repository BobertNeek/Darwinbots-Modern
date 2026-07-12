using Darwinbots.Desktop.Core;

using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class SeededSpawnPlacementTests
{
    [Fact]
    public void SameSeedProducesSameScatteredPositions()
    {
        var first = new SeededSpawnPlacement(42).Next(100, 16_000f, 12_000f);
        var second = new SeededSpawnPlacement(42).Next(100, 16_000f, 12_000f);

        Assert.Equal(first, second);
        Assert.True(first.Select(position => position[0]).Distinct().Count() > 90);
        Assert.True(first.Select(position => position[1]).Distinct().Count() > 90);
    }

    [Fact]
    public void PositionsRespectWorldMargins()
    {
        var positions = new SeededSpawnPlacement(7).Next(1_000, 16_000f, 12_000f);

        Assert.All(positions, position =>
        {
            Assert.InRange(position[0], 60f, 15_940f);
            Assert.InRange(position[1], 60f, 11_940f);
        });
    }
}
