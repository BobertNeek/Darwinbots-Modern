using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class DesktopControlRulesTests
{
    [Theory]
    [InlineData(0, 1u)]
    [InlineData(1, 5u)]
    [InlineData(2, 20u)]
    [InlineData(3, 100u)]
    public void SpeedSelectionUsesFixedDb2ThrottlesAndAnAdaptiveMaximumSeed(int selection, uint expected)
    {
        Assert.Equal(expected, DesktopControlRules.TicksForSelection(selection));
        Assert.Equal(selection >= 3, DesktopControlRules.IsMaximumSpeed(selection));
    }

    [Fact]
    public void PointerClickMustCrossAVisibleThresholdBeforeItBecomesADrag()
    {
        Assert.False(DesktopControlRules.IsDragGesture(2, 2));
        Assert.True(DesktopControlRules.IsDragGesture(4, 0));
    }

    [Fact]
    public void MaximumSpeedAdaptsBatchSizeWithoutCreatingUnboundedUiStalls()
    {
        Assert.True(DesktopControlRules.AdaptMaximumBatch(100, TimeSpan.FromMilliseconds(100), false) < 100);
        Assert.True(DesktopControlRules.AdaptMaximumBatch(100, TimeSpan.FromMilliseconds(4), false) > 100);
        Assert.InRange(DesktopControlRules.AdaptMaximumBatch(10_000, TimeSpan.FromMilliseconds(1), false), 1u, 2_000u);
        Assert.InRange(DesktopControlRules.AdaptMaximumBatch(20_000, TimeSpan.FromMilliseconds(1), true), 1u, 10_000u);
    }
}
