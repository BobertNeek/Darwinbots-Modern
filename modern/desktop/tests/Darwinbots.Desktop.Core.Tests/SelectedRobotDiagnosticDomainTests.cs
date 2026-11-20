using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class SelectedRobotDiagnosticDomainTests
{
    [Fact]
    public void RgbChannelsUseTheOriginalIntegerRoundingAndLimits()
    {
        var robot = new OrganismSnapshot(1, 0, [0, 0], [0, 0], 100, 1)
        {
            Memory = [new MemoryCellSnapshot(1, 1), new MemoryCellSnapshot(2, 3), new MemoryCellSnapshot(3, 5)],
        };
        var settings = new RgbMemoryMonitorSettings(new(1, 0, 510), new(2, 0, 510), new(3, 0, 510));
        Assert.Equal(new RgbColor(0, 2, 2), RgbMemoryMonitor.ColorFor(robot, settings));
        Assert.Equal(new RgbColor(0, 255, 2), RgbMemoryMonitor.ColorFor(robot,
            settings with { Red = new(1, 10, 20), Green = new(2, 0, 1) }));
    }

    [Fact]
    public void EyeRgbAndPhylogenyModelsKeepTheSelectedRobotDiagnosticStateCausal()
    {
        var parent = new OrganismSnapshot(2, 1, [10f, 10f], [0f, 0f], 100, 1)
        {
            Phenotype = new VisualPhenotypeSnapshot(44, 0xff123456, [], 0),
        };
        var child = new OrganismSnapshot(7, 3, [20f, 10f], [0f, 0f], 200, 2, Parents: [new OrganismKey(2, 1), null])
        {
            Vision = VisionSnapshot.Default,
            Memory = [new MemoryCellSnapshot(310, 16), new MemoryCellSnapshot(311, 32), new MemoryCellSnapshot(312, 48)],
            Phenotype = new VisualPhenotypeSnapshot(44, 0xff654321, [], 0),
        };
        var snapshot = new EngineSnapshot(5, 2, "CPU", [parent, child]);

        var eyes = EyeDesignerModel.FromOrganism(child);
        Assert.Equal(9, eyes.Rows.Count);
        Assert.True(eyes.TrySet(4, "314", "70", out _));
        Assert.False(eyes.TrySet(4, "32001", "70", out _));
        Assert.Contains("314 .eye5dir store", eyes.WriteDna());
        Assert.Equal(9, eyes.Preview.Eyes.Length);

        var settings = RgbMemoryMonitorSettings.Preset("ENERGY / BODY / FDBODY");
        Assert.Equal(new RgbColor(128, 128, 128), RgbMemoryMonitor.ColorFor(child, settings with
        {
            Red = new RgbMemoryChannel(310, 0, 32),
            Green = new RgbMemoryChannel(311, 0, 64),
            Blue = new RgbMemoryChannel(312, 0, 96),
        }));
        Assert.Equal(new RgbColor(255, 255, 255), RgbMemoryMonitor.ColorFor(child, settings with
        {
            Red = new RgbMemoryChannel(310, 0, 16),
            Green = new RgbMemoryChannel(311, 0, 32),
            Blue = new RgbMemoryChannel(312, 0, 48),
        }));

        var lineage = PhylogenyModel.Build(snapshot, new OrganismKey(7, 3), 1);
        Assert.Contains(new OrganismKey(2, 1), lineage.Highlighted);
        Assert.Equal(0, Assert.Single(lineage.Nodes, node => node.IsFocused).Depth);
        Assert.Equal(-1, Assert.Single(lineage.Nodes, node => node.Key == new OrganismKey(2, 1)).Depth);
    }
}
