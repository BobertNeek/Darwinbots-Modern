using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class SelectedRobotDiagnosticDomainTests
{
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

    [Fact]
    public void NativePhylogenyUsesAbsolutePrimaryParentsAndFindsTheEldestExistingParent()
    {
        var earlierParent = new OrganismKey(1, 0);
        var parent = new OrganismKey(2, 1);
        var focus = new OrganismKey(3, 2);
        var sibling = new OrganismKey(4, 1);
        var child = new OrganismKey(5, 0);
        var grandchild = new OrganismKey(6, 0);
        var secondParentOnly = new OrganismKey(7, 0);
        var founder = new OrganismKey(8, 0);
        var snapshot = new EngineSnapshot(42, 8, "CPU", [
            Robot(earlierParent, 10, 0), Robot(parent, 10, 0), Robot(focus, 20, 10, earlierParent),
            Robot(sibling, 25, 10), Robot(child, 30, 20, sibling), Robot(grandchild, 40, 30, child),
            Robot(secondParentOnly, 50, 25, sibling, focus), Robot(founder, 60, 0, focus),
        ]) { NativeCycle = 41 };

        Assert.Equal(new PhylogenyLink(focus, child), Assert.Single(PhylogenyModel.DescendantLinks(snapshot, focus, 1)));
        Assert.Equal(new PhylogenyLink[] { new(focus, child), new(child, grandchild) },
            PhylogenyModel.DescendantLinks(snapshot, focus, 2));
        Assert.Equal(new PhylogenyLink[] { new(parent, focus), new(parent, sibling), new(focus, child),
            new(sibling, secondParentOnly), new(child, grandchild) }, PhylogenyModel.WholeLineage(snapshot, focus));
        Assert.Empty(PhylogenyModel.DescendantLinks(snapshot, focus with { Generation = 9 }, 1));

        var portable = snapshot with { NativeCycle = null };
        Assert.Equal(new PhylogenyLink(focus, founder), Assert.Single(PhylogenyModel.DescendantLinks(portable, focus, 1)));

        static OrganismSnapshot Robot(OrganismKey key, ulong id, ulong parentId, OrganismKey? primary = null,
            OrganismKey? secondary = null) => new(key.Slot, key.Generation, [0, 0], [0, 0], 100, 1,
            Parents: [primary, secondary]) { UniqueId = id, ParentAbsoluteId = parentId };
    }
}
