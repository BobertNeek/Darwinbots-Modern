using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class GeneActivationMeterLayoutTests
{
    [Fact]
    public void BelowTwentyFiveGenesEachNumberIsCenteredInItsOwnColumn()
    {
        var labels = GeneActivationMeterLayout.Labels(24).ToArray();
        Assert.Equal(Enumerable.Range(1, 24), labels.Select(label => label.Number));
        Assert.All(labels, label => Assert.Null(label.Tick));
        Assert.Equal(0.5 / 24, labels[0].Center);
        Assert.Equal(23.5 / 24, labels[^1].Center);
    }

    [Fact]
    public void DenseLabelsPrintTheBoundaryNumberInTheFollowingColumn()
    {
        var labels = GeneActivationMeterLayout.Labels(25).ToArray();
        Assert.Equal(new[] { 5, 10, 15, 20 }, labels.Select(label => label.Number));
        Assert.Equal(5.0 / 25, labels[0].Tick);
        Assert.Equal(5.5 / 25, labels[0].Center);
        Assert.Equal(20.0 / 25, labels[^1].Tick);
        Assert.Equal(20.5 / 25, labels[^1].Center);
    }
}
