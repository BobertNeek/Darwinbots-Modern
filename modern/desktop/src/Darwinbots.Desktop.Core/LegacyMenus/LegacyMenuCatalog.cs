namespace Darwinbots.Desktop.Core.LegacyMenus;

public static class LegacyMenuCatalog
{
    private static LegacyMenuItem L(
        string label,
        LegacyCommandId command,
        string icon,
        string? shortcut = null,
        LegacyMenuCheckKind check = LegacyMenuCheckKind.None) =>
        LegacyMenuItem.Leaf(label, command, icon, shortcut, check);

    private static LegacyMenuItem G(string label, string icon, params LegacyMenuItem[] children) =>
        LegacyMenuItem.Group(label, icon, children);

    private static LegacyMenuItem S() => LegacyMenuItem.Separator();

    public static IReadOnlyList<LegacyMenuItem> TopLevel { get; } =
    [
        LegacyMenuItem.Top("File",
            L("New Simulation", LegacyCommandId.FileNewSimulation, "new"),
            L("Load Simulation", LegacyCommandId.FileLoadSimulation, "open"),
            L("Save Simulation", LegacyCommandId.FileSaveSimulation, "save"),
            L("Save Sim Without Mutations", LegacyCommandId.FileSaveWithoutMutations, "save-clean"),
            L("Pause Sim (F12)", LegacyCommandId.FilePauseSimulation, "pause", "F12"),
            S(),
            L("Quit", LegacyCommandId.FileQuit, "quit")),

        LegacyMenuItem.Top("Edit",
            L("Insert Organism...", LegacyCommandId.EditInsertOrganism, "insert-bot"),
            S(),
            L("Species...", LegacyCommandId.EditSpecies, "species"),
            L("General Settings...", LegacyCommandId.EditGeneralSettings, "settings"),
            L("Physics and Costs...", LegacyCommandId.EditPhysicsAndCosts, "physics"),
            L("Global Mutation Rates...", LegacyCommandId.EditGlobalMutationRates, "mutation"),
            L("Restart and Leagues...", LegacyCommandId.EditRestartAndLeagues, "restart"),
            L("Internet Options...", LegacyCommandId.EditInternetOptions, "internet"),
            S(),
            L("Disallow robot from fixing or unfixing", LegacyCommandId.EditDisallowFixing, "lock", check: LegacyMenuCheckKind.Check),
            L("Disable asexual reproduction for non-repopulating robots", LegacyCommandId.EditDisableAsexualReproduction, "reproduction", check: LegacyMenuCheckKind.Check),
            S(),
            L("Automatically tag by name...", LegacyCommandId.EditAutomaticallyTagByName, "tag"),
            L("Enable Automatic Forking", LegacyCommandId.EditEnableAutomaticForking, "fork", check: LegacyMenuCheckKind.Check),
            S(),
            L("Settings for PlayerBot Mode...", LegacyCommandId.EditPlayerBotSettings, "player"),
            L("Enable PlayerBot Mode", LegacyCommandId.EditEnablePlayerBot, "player-toggle", check: LegacyMenuCheckKind.Check),
            S(),
            L("Go to eye designer...", LegacyCommandId.EditEyeDesigner, "eye"),
            S(),
            L("Restriction Overwrites", LegacyCommandId.EditRestrictionOverwrites, "restrictions")),

        LegacyMenuItem.Top("View",
            L("Import Background Picture", LegacyCommandId.ViewImportBackground, "image"),
            L("Remove Background Picture", LegacyCommandId.ViewRemoveBackground, "image-remove"),
            S(),
            L("Show Vision Grid", LegacyCommandId.ViewShowVisionGrid, "vision", check: LegacyMenuCheckKind.Check),
            L("Display Movement Vectors", LegacyCommandId.ViewDisplayMovementVectors, "vectors", check: LegacyMenuCheckKind.Check),
            L("Display Shot Impacts", LegacyCommandId.ViewDisplayShotImpacts, "impact", check: LegacyMenuCheckKind.Check),
            L("Display Resource Guages", LegacyCommandId.ViewDisplayResourceGuages, "gauges", check: LegacyMenuCheckKind.Check),
            S(),
            L("Settings for RGB Memory Monitor...", LegacyCommandId.ViewRgbMemoryMonitorSettings, "rgb-settings"),
            L("RGB Memory Monitor", LegacyCommandId.ViewRgbMemoryMonitor, "rgb", check: LegacyMenuCheckKind.Check)),

        LegacyMenuItem.Top("Robot",
            L("Show Robot Info", LegacyCommandId.RobotShowInfo, "info"),
            L("Show Philogeny", LegacyCommandId.RobotShowPhilogeny, "tree"),
            L("Mutation Rates", LegacyCommandId.RobotMutationRates, "mutation"),
            L("Change Color", LegacyCommandId.RobotChangeColor, "color"),
            S(),
            L("Open Console", LegacyCommandId.RobotOpenConsole, "console"),
            L("View Gene Activations", LegacyCommandId.RobotViewGeneActivations, "genes"),
            S(),
            L("Kill Robot", LegacyCommandId.RobotKill, "kill"),
            L("Save Robot's DNA", LegacyCommandId.RobotSaveDna, "dna-save"),
            L("Make New Species", LegacyCommandId.RobotMakeNewSpecies, "speciate"),
            S(),
            L("Select Entire Organism", LegacyCommandId.RobotSelectEntireOrganism, "select-organism"),
            L("Save Entire Organism", LegacyCommandId.RobotSaveEntireOrganism, "organism-save"),
            L("Kill Entire Organism", LegacyCommandId.RobotKillEntireOrganism, "organism-kill"),
            S(),
            L("Find Best", LegacyCommandId.RobotFindBest, "best")),

        LegacyMenuItem.Top("Objects",
            G("Shots", "shot",
                L("Don't decay nrg shots", LegacyCommandId.ObjectsDontDecayNrgShots, "energy-shot", check: LegacyMenuCheckKind.Check),
                L("Don't decay waste shots", LegacyCommandId.ObjectsDontDecayWasteShots, "waste-shot", check: LegacyMenuCheckKind.Check)),
            G("Ties", "tie",
                L("Disable Ties", LegacyCommandId.ObjectsDisableTies, "tie-disable", check: LegacyMenuCheckKind.Check)),
            G("Teleporters", "teleporter",
                L("New Teleporter...", LegacyCommandId.ObjectsNewTeleporter, "add"),
                L("Hilight Teleporters", LegacyCommandId.ObjectsHilightTeleporters, "highlight", check: LegacyMenuCheckKind.Check),
                L("Delete Teleporter", LegacyCommandId.ObjectsDeleteTeleporter, "delete"),
                L("Delete All Teleporters", LegacyCommandId.ObjectsDeleteAllTeleporters, "delete-all")),
            G("Shapes", "shape",
                L("New Shape...", LegacyCommandId.ObjectsNewShape, "add"),
                L("Add Ten Random Shapes", LegacyCommandId.ObjectsAddTenRandomShapes, "random-add"),
                L("Delete 10 Random Shapes", LegacyCommandId.ObjectsDeleteTenRandomShapes, "random-delete"),
                L("Delete Shape", LegacyCommandId.ObjectsDeleteShape, "delete"),
                L("Delete All Shapes", LegacyCommandId.ObjectsDeleteAllShapes, "delete-all")),
            G("Mazes", "maze",
                L("Simple Horizontal Maze", LegacyCommandId.ObjectsHorizontalMaze, "maze-horizontal"),
                L("Simple Vertical Maze", LegacyCommandId.ObjectsVerticalMaze, "maze-vertical"),
                L("Sprial", LegacyCommandId.ObjectsSprial, "maze-spiral"),
                L("Checkerboard", LegacyCommandId.ObjectsCheckerboard, "maze-checker"),
                L("Polar Ice", LegacyCommandId.ObjectsPolarIce, "maze-polar"),
                L("Trash Compactor", LegacyCommandId.ObjectsTrashCompactor, "maze-trash"))),

        LegacyMenuItem.Top("E-Grid",
            L("Enable", LegacyCommandId.EGridEnable, "grid", check: LegacyMenuCheckKind.Check),
            G("Grid Size", "grid-size",
                L("Large", LegacyCommandId.EGridLarge, "size-large", check: LegacyMenuCheckKind.Radio),
                L("Medium", LegacyCommandId.EGridMedium, "size-medium", check: LegacyMenuCheckKind.Radio),
                L("Small", LegacyCommandId.EGridSmall, "size-small", check: LegacyMenuCheckKind.Radio))),

        LegacyMenuItem.Top("Internet",
            L("Enabled", LegacyCommandId.InternetEnabled, "internet", check: LegacyMenuCheckKind.Check),
            L("Internet Teleporter...", LegacyCommandId.InternetTeleporter, "internet-teleporter")),

        LegacyMenuItem.Top("Recording",
            L("Safemode Backup", LegacyCommandId.RecordingSafemodeBackup, "backup", check: LegacyMenuCheckKind.Check),
            L("Snapshot of the living", LegacyCommandId.RecordingSnapshotLiving, "snapshot-living"),
            G("Snapshot of the dead", "snapshot-dead",
                L("Enable recording", LegacyCommandId.RecordingDeadEnable, "record", check: LegacyMenuCheckKind.Check),
                L("Exclude repopulating robots", LegacyCommandId.RecordingDeadExcludeRepopulating, "exclude", check: LegacyMenuCheckKind.Check))),

        LegacyMenuItem.Top("Help",
            L("DNA Help", LegacyCommandId.HelpDna, "help-dna"),
            S(),
            L("Robot Tag Information", LegacyCommandId.HelpRobotTagInformation, "help-tags"),
            L("Survival Info", LegacyCommandId.HelpSurvivalInfo, "help-survival"),
            S(),
            L("About", LegacyCommandId.HelpAbout, "about")),
    ];

    public static IEnumerable<LegacyMenuItem> FlattenLeaves() =>
        TopLevel.SelectMany(FlattenLeaves);

    private static IEnumerable<LegacyMenuItem> FlattenLeaves(LegacyMenuItem item)
    {
        if (item.IsLeaf)
        {
            yield return item;
            yield break;
        }

        foreach (var child in item.Children)
        foreach (var leaf in FlattenLeaves(child))
            yield return leaf;
    }
}
