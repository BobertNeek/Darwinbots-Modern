using Darwinbots.Desktop.Core.LegacyMenus;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class LegacyMenuCatalogTests
{
    [Fact]
    public void CatalogMatchesHistoricalTopLevelOrder()
    {
        Assert.Equal(
            ["File", "Edit", "View", "Robot", "Objects", "E-Grid", "Internet", "Recording", "Help"],
            LegacyMenuCatalog.TopLevel.Select(item => item.Label));
    }

    [Fact]
    public void EveryActionableLeafHasAUniqueCommandId()
    {
        var leaves = LegacyMenuCatalog.FlattenLeaves().ToArray();

        Assert.NotEmpty(leaves);
        Assert.DoesNotContain(leaves, item => item.Command is null);
        Assert.Equal(leaves.Length, leaves.Select(item => item.Command).Distinct().Count());
    }

    [Fact]
    public void CatalogContainsTheCompleteHistoricalNestedSurfaces()
    {
        AssertSubmenu("Objects", "Shots", "Don't decay nrg shots", "Don't decay waste shots");
        AssertSubmenu("Objects", "Ties", "Disable Ties");
        AssertSubmenu("Objects", "Teleporters", "New Teleporter...", "Hilight Teleporters", "Delete Teleporter", "Delete All Teleporters");
        AssertSubmenu("Objects", "Shapes", "New Shape...", "Add Ten Random Shapes", "Delete 10 Random Shapes", "Delete Shape", "Delete All Shapes");
        AssertSubmenu("Objects", "Mazes", "Simple Horizontal Maze", "Simple Vertical Maze", "Sprial", "Checkerboard", "Polar Ice", "Trash Compactor");
        AssertSubmenu("E-Grid", "Grid Size", "Large", "Medium", "Small");
        AssertSubmenu("Recording", "Snapshot of the dead", "Enable recording", "Exclude repopulating robots");
    }

    [Fact]
    public void MutationRatesRemainExplicitNumericCommands()
    {
        Assert.Equal(
            LegacyCommandId.EditGlobalMutationRates,
            FindLeaf("Edit", "Global Mutation Rates...").Command);
        Assert.Equal(
            LegacyCommandId.RobotMutationRates,
            FindLeaf("Robot", "Mutation Rates").Command);
    }

    [Fact]
    public void CheckAndRadioStateMatchesHistoricalBehavior()
    {
        Assert.Equal(LegacyMenuCheckKind.Check, FindLeaf("View", "Show Vision Grid").CheckKind);
        Assert.Equal(LegacyMenuCheckKind.Check, FindLeaf("Internet", "Enabled").CheckKind);
        Assert.Equal(LegacyMenuCheckKind.Radio, FindLeaf("E-Grid", "Grid Size", "Large").CheckKind);
        Assert.Equal(LegacyMenuCheckKind.Radio, FindLeaf("E-Grid", "Grid Size", "Medium").CheckKind);
        Assert.Equal(LegacyMenuCheckKind.Radio, FindLeaf("E-Grid", "Grid Size", "Small").CheckKind);
    }

    [Fact]
    public void HistoricalVisibilityAndShortcutsArePreserved()
    {
        var expectedShortcuts = new Dictionary<LegacyCommandId, string>
        {
            [LegacyCommandId.FileNewSimulation] = "F1",
            [LegacyCommandId.FileLoadSimulation] = "F2",
            [LegacyCommandId.FileSaveSimulation] = "F3",
            [LegacyCommandId.FileSaveWithoutMutations] = "Ctrl+F3",
            [LegacyCommandId.FilePauseSimulation] = "F12",
            [LegacyCommandId.FileQuit] = "Ctrl+Q",
            [LegacyCommandId.EditInsertOrganism] = "Ctrl+I",
            [LegacyCommandId.EditSpecies] = "Ctrl+X",
            [LegacyCommandId.EditGeneralSettings] = "Ctrl+P",
            [LegacyCommandId.EditPhysicsAndCosts] = "F5",
            [LegacyCommandId.EditGlobalMutationRates] = "Ctrl+G",
            [LegacyCommandId.EditRestartAndLeagues] = "Ctrl+F",
            [LegacyCommandId.EditEnablePlayerBot] = "F11",
            [LegacyCommandId.EditRestrictionOverwrites] = "F8",
            [LegacyCommandId.RobotShowInfo] = "Ctrl+R",
            [LegacyCommandId.RobotShowPhilogeny] = "Ctrl+T",
            [LegacyCommandId.RobotMutationRates] = "Ctrl+M",
            [LegacyCommandId.RobotChangeColor] = "Ctrl+C",
            [LegacyCommandId.RobotOpenConsole] = "Ctrl+Y",
            [LegacyCommandId.RobotViewGeneActivations] = "Ctrl+Z",
            [LegacyCommandId.RobotKill] = "Ctrl+K",
            [LegacyCommandId.RobotSaveDna] = "Ctrl+S",
            [LegacyCommandId.RobotSelectEntireOrganism] = "Ctrl+F1",
            [LegacyCommandId.RobotSaveEntireOrganism] = "Ctrl+F2",
            [LegacyCommandId.RobotKillEntireOrganism] = "Ctrl+E",
            [LegacyCommandId.RobotFindBest] = "Ctrl+B",
            [LegacyCommandId.HelpDna] = "Ctrl+D",
            [LegacyCommandId.HelpAbout] = "Ctrl+A",
        };
        var actualShortcuts = LegacyMenuCatalog.FlattenLeaves()
            .Where(item => item.Shortcut is not null)
            .ToDictionary(item => item.Command!.Value, item => item.Shortcut!);

        Assert.Equal(expectedShortcuts, actualShortcuts);

        var eGrid = Assert.Single(LegacyMenuCatalog.TopLevel, item => item.Label == "E-Grid");
        Assert.False(eGrid.VisibleByDefault);
        Assert.All(LegacyMenuCatalog.TopLevel.Where(item => item != eGrid), item => Assert.True(item.VisibleByDefault));
    }

    private static void AssertSubmenu(string topLevel, string submenu, params string[] expectedChildren)
    {
        var parent = Find(topLevel, submenu);
        Assert.Equal(expectedChildren, parent.Children.Select(item => item.Label));
    }

    private static LegacyMenuItem FindLeaf(params string[] path)
    {
        var item = Find(path);
        Assert.True(item.IsLeaf);
        return item;
    }

    private static LegacyMenuItem Find(params string[] path)
    {
        IReadOnlyList<LegacyMenuItem> level = LegacyMenuCatalog.TopLevel;
        LegacyMenuItem? found = null;
        foreach (var segment in path)
        {
            found = Assert.Single(level, item => item.Label == segment);
            level = found.Children;
        }

        return found ?? throw new InvalidOperationException("Menu path was empty.");
    }
}
