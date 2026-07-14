using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class SelectionVisualTests
{
    [Theory]
    [InlineData(1.5f, 5.5f)]
    [InlineData(10f, 12.2f)]
    public void SelectionRingStaysVisibleWithoutChangingTheOrganismRadius(
        float renderedRadius,
        float expectedRingRadius)
    {
        Assert.Equal(expectedRingRadius, OrganismVisualGeometry.SelectionRingRadius(renderedRadius), 3);
    }
}
