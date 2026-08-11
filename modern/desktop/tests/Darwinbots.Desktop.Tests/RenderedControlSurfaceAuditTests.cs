using System.Collections.Concurrent;
using System.Reflection;
using Avalonia;
using Avalonia.Automation;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class RenderedControlSurfaceAuditTests
{
    private static readonly IReadOnlyDictionary<string, string[]> Coverage =
        new Dictionary<string, string[]>(StringComparer.Ordinal)
        {
            [nameof(SetupRenderedControlsDispatchThroughVisualTree)] =
            [
                "setup.file", "setup.file.new", "setup.file.open", "setup.file.recover", "setup.file.exit",
                "setup.view", "setup.view.advanced", "setup.help", "setup.help.about",
                "setup.mode.starter", "setup.mode.zerobot", "setup.mode.zerobot_veg",
                "setup.species.name", "setup.species.vegetable", "setup.species.count", "setup.species.energy",
                "setup.species.mutation", "setup.species.mutation_label", "setup.species.reseed",
                "setup.species.remove", "setup.species.add", "setup.backend", "setup.world.width",
                "setup.world.height", "setup.population_cap", "setup.vegetable_cap", "setup.speed",
                "setup.sustenance", "setup.auto_progression", "setup.advanced_button", "setup.create",
                "setup.recover_button",
            ],
            [nameof(RuntimeRenderedControlsDispatchThroughVisualTree)] =
            [
                "live.run", "live.pause", "live.step", "live.turbo", "live.import", "live.reset",
                "live.speed", "live.backend", "live.save", "live.load", "live.toroidal", "live.render_waste",
                "live.add_obstacle", "live.add_teleporter", "live.remove_feature", "live.live_physics",
                "live.show_vision", "live.metabolism", "live.plant_energy", "live.feeding_body",
                "live.apply_energy", "live.advance_zerobot", "live.viewport.select", "live.viewport.drag",
                "live.dna.edit", "live.move", "live.clone", "live.reproduce", "live.kill", "live.follow",
                "live.expander.world", "live.expander.parameters", "live.expander.environment",
                "live.expander.physics", "live.expander.senses", "live.expander.energy",
                "live.expander.history", "live.expander.statistics", "live.expander.experiments",
                "live.expander.league", "live.local_mode.kind", "live.local_mode.preset",
                "live.local_mode.minimum_rounds", "live.local_mode.maximum_rounds",
                "live.local_mode.maximum_cycles", "live.local_mode.population_cap",
                "live.local_mode.restart", "live.local_mode.info", "live.local_mode.viruses",
                "live.local_mode.sexual", "live.local_mode.configure", "live.local_mode.start",
                "live.local_mode.restart_button", "live.local_mode.adjudicate", "live.local_mode.reason",
            ],
            [nameof(AdvancedSettingsRenderedControlsRoundTripEveryField)] =
            [
                "advanced.metabolism", "advanced.vegetableenergy", "advanced.sunlight",
                "advanced.autospeciation", "advanced.speciationdistance", "advanced.toroidalworld",
                "advanced.maxvelocity", "advanced.movementefficiency", "advanced.gravityx", "advanced.gravityy",
                "advanced.surfacegravity", "advanced.staticfriction", "advanced.kineticfriction",
                "advanced.density", "advanced.viscosity", "advanced.elasticity", "advanced.drag",
                "advanced.brownian", "advanced.shotspeed", "advanced.shotrangemultiplier",
                "advanced.shotdecay", "advanced.energyshotsnodecay", "advanced.wasteshotsnodecay",
                "advanced.startchloroplasts", "advanced.maxplantenergy",
                "advanced.minimumchloroplastequivalents", "advanced.repopulationamount",
                "advanced.repopulationcooldown", "advanced.feedingtobody", "advanced.daytime",
                "advanced.daynightenabled", "advanced.cyclelength", "advanced.cancel", "advanced.apply",
            ],
            [nameof(GlobalSettingsRenderedSurfaceShowsFourTabs)] =
            [
                "global.main", "global.advanced_mutation", "global.league", "global.evolution",
                "global.cancel", "global.apply",
            ],
            [nameof(DnaEditorRenderedControlsDispatchThroughVisualTree)] =
            ["dna.editor", "dna.save", "dna.clone", "dna.apply"],
        };

    [Fact]
    public void InventoryDefinesEveryCurrentInteractiveSurfaceExactlyOnce()
    {
        var ids = Coverage.Values.SelectMany(value => value).ToArray();

        Assert.Equal(125, ids.Length);
        Assert.Equal(ids.Length, ids.Distinct(StringComparer.Ordinal).Count());
        Assert.All(ids, id => Assert.False(string.IsNullOrWhiteSpace(id)));
    }

    [Fact]
    public async Task ClassicResetControlRendersItsPlayerFacingPresentation()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), new AuditEngineClientFactory(), new AuditStorageService());
            try
            {
                ShowForInteraction(window, 1280, 900);
                var reset = Named<Button>(window, "ResetButton");

                Assert.Equal("RESET", reset.Content?.ToString());
                Assert.Equal("RESET SIMULATION", AutomationProperties.GetName(reset));
                Assert.Equal(
                    "Stop the current simulation and start a fresh local world.",
                    AutomationProperties.GetHelpText(reset));
                Capture(window, "run-01-classic-reset.png");
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }

            return 0;
        }, CancellationToken.None);
    }

    [Fact]
    public async Task SetupRenderedControlsDispatchThroughVisualTree()
    {
        var storage = new AuditStorageService
        {
            DnaFiles = [new DesktopTextFile("Audit_Bot.txt", "start\n10 .up store\nstop")],
            PickedSimulationPath = Path.Combine(Path.GetTempPath(), "audit-open.db3s"),
            LatestAutosave = Path.Combine(Path.GetTempPath(), "audit-autosave.db3s"),
        };
        var headless = DesktopTestApplication.Session;
        SetupWindow? window = null;
        var created = new List<WorldSetupOptions>();

        await headless.Dispatch(() =>
        {
            window = new SetupWindow(storage);
            window.WorldCreated += created.Add;
            ShowAndCapture(window, 1280, 820, "setup-initial.png");

            OpenAndCloseMenu(window, "FileMenu");
            OpenAndCloseMenu(window, "ViewMenu");
            OpenAndCloseMenu(window, "HelpMenu");

            Click(window, Named<RadioButton>(window, "ZerobotMode"));
            Assert.Single(window.Species);
            Click(window, Named<RadioButton>(window, "ZerobotVegetableMode"));
            Assert.Equal(2, window.Species.Count);
            Click(window, Named<RadioButton>(window, "StarterMode"));
            Assert.Equal(2, window.Species.Count);

            var firstRow = window.Species[0];
            var rowControls = Named<ItemsControl>(window, "SpeciesList").GetVisualDescendants().ToArray();
            var name = rowControls.OfType<TextBox>().First(control => ReferenceEquals(control.DataContext, firstRow));
            name.Focus();
            name.SelectAll();
            window.KeyTextInput("Audit Alga");
            Assert.Equal("Audit Alga", firstRow.Name);

            var rowChecks = rowControls.OfType<CheckBox>().Where(control => ReferenceEquals(control.DataContext, firstRow)).ToArray();
            var vegetable = rowChecks.Single(control => Equals(control.Content, "VEG"));
            var vegetableBefore = firstRow.Vegetable;
            Click(window, vegetable);
            Assert.Equal(!vegetableBefore, firstRow.Vegetable);
            var reseed = rowChecks.Single(control => control.Content is null);
            var reseedBefore = firstRow.Reseed;
            Click(window, reseed);
            Assert.Equal(!reseedBefore, firstRow.Reseed);

            var rowNumbers = rowControls.OfType<NumericUpDown>().Where(control => ReferenceEquals(control.DataContext, firstRow)).ToArray();
            Assert.Equal(2, rowNumbers.Length);
            AssertKeyboardIncrement(window, rowNumbers[0]);
            AssertKeyboardIncrement(window, rowNumbers[1]);

            var slider = rowControls.OfType<Slider>().First(control => ReferenceEquals(control.DataContext, firstRow));
            DragSlider(window, slider, 0.20);
            var lowMutationRate = firstRow.MutationRate;
            DragSlider(window, slider, 0.80);
            var highMutationRate = firstRow.MutationRate;
            Assert.True(
                highMutationRate > lowMutationRate + 40,
                $"Mutation slider did not traverse its range: low={lowMutationRate:0.##}, high={highMutationRate:0.##}.");
            var mutationLabel = rowControls.OfType<TextBlock>()
                .First(control => ReferenceEquals(control.DataContext, firstRow)
                    && control.Text?.Contains('%', StringComparison.Ordinal) == true);
            Assert.Contains('%', mutationLabel.Text!);

            var remove = rowControls.OfType<Button>().Last(control => control.Tag is SetupSpeciesRow);
            Click(window, remove);
            Assert.Single(window.Species);
            Click(window, Named<Button>(window, "AddDnaButton"));
            Assert.Equal(2, window.Species.Count);
            Assert.Equal("Audit_Bot", window.Species[^1].Name);

            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "WorldWidth"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "WorldHeight"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "PopulationCap"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "VegetablePopulationCap"));
            AssertComboSelection(window, Named<ComboBox>(window, "Backend"), 1);
            AssertComboSelection(window, Named<ComboBox>(window, "Speed"), 1);

            Click(window, Named<RadioButton>(window, "ZerobotMode"));
            var sustenance = Named<ComboBox>(window, "Sustenance");
            Assert.True(sustenance.IsEnabled);
            AssertComboSelection(window, sustenance, 1);
            var progression = Named<CheckBox>(window, "AutomaticProgression");
            var progressionBefore = progression.IsChecked;
            Click(window, progression);
            Assert.NotEqual(progressionBefore, progression.IsChecked);

            ActivateMenu(Named<MenuItem>(window, "OpenSaveMenuItem"));
            Assert.Equal(storage.PickedSimulationPath, created[^1].LoadSavePath);
            ActivateMenu(Named<MenuItem>(window, "RecoverMenuItem"));
            Assert.Equal(storage.LatestAutosave, created[^1].LoadSavePath);

            Click(window, Named<Button>(window, "AdvancedSettingsButton"));
            var advanced = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
            Click(advanced, Named<Button>(advanced, "ApplyButton"));
            Assert.Equal("ADVANCED SETTINGS APPLIED", Named<TextBlock>(window, "SetupStatus").Text);

            ActivateMenu(Named<MenuItem>(window, "AdvancedMenuItem"));
            advanced = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
            Click(advanced, Named<Button>(advanced, "CancelButton"));

            ActivateMenu(Named<MenuItem>(window, "AboutMenuItem"));
            var about = Assert.Single(window.OwnedWindows, value => value.Title == "About Darwinbots Modern");
            Click(about, about.GetVisualDescendants().OfType<Button>().Single());

            Click(window, Named<Button>(window, "CreateWorldButton"));
            Assert.Equal(StartingMode.Zerobots, created[^1].StartingMode);
            Click(window, Named<Button>(window, "RecoverButton"));
            Assert.Equal(storage.LatestAutosave, created[^1].LoadSavePath);

            ActivateMenu(Named<MenuItem>(window, "NewWorldMenuItem"));
            Assert.True(Named<RadioButton>(window, "StarterMode").IsChecked);
            Assert.Equal(3, Named<ComboBox>(window, "Speed").SelectedIndex);
            Capture(window, "setup-exercised.png");
            window.Close();

            var exitWindow = new SetupWindow(storage);
            ShowForInteraction(exitWindow, 1280, 820);
            ActivateMenu(Named<MenuItem>(exitWindow, "ExitMenuItem"));
            Assert.False(exitWindow.IsVisible);
        }, CancellationToken.None);
    }

    [Fact]
    public async Task MainWindowRendersSafemodeRecoveryAndRecoversAutosave()
    {
        var artifactPath = Path.Combine(Path.GetTempPath(), $"db24832-io05-recovery-{Guid.NewGuid():N}.db3s");
        File.WriteAllBytes(artifactPath, [9, 8, 7]);
        var storage = new AuditStorageService { LatestAutosave = artifactPath };
        var factory = new AuditEngineClientFactory();

        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var window = new MainWindow(
                    [],
                    new WorldSetupOptions
                    {
                        Species = [new SpeciesImport("IO-05 Recovery Species", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
                    },
                    factory,
                    storage);
                try
                {
                    ShowForInteraction(window, 1_200, 820);
                    var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                    await WaitFor(() => viewModel.Status.Contains("WORLD READY", StringComparison.Ordinal));

                    var menu = Named<LegacyMenuBar>(window, "HistoricMenuBar");
                    var safemode = menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.RecordingSafemodeBackup)!;
                    Assert.True(safemode.IsChecked);
                    safemode.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => safemode.IsChecked == false);
                    Assert.Equal("SAFEMODE BACKUP OFF", viewModel.Status);
                    safemode.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => safemode.IsChecked == true);
                    Assert.Equal("SAFEMODE BACKUP ON", viewModel.Status);

                    var handleError = typeof(MainWindow).GetMethod("HandleEngineErrorAsync", BindingFlags.Instance | BindingFlags.NonPublic)!;
                    await (Task)handleError.Invoke(window, [new AccessViolationException("IO-05 rendered recovery fault"), true])!;

                    var panel = Named<Border>(window, "RecoveryPanel");
                    var title = Named<TextBlock>(window, "RecoveryTitle");
                    var faultWindow = Named<TextBlock>(window, "RecoveryFaultWindowText");
                    var artifact = Named<TextBlock>(window, "RecoveryArtifactText");
                    var recover = Named<Button>(window, "RecoveryRecoverButton");
                    var ignore = Named<Button>(window, "RecoveryIgnoreButton");
                    Assert.True(panel.IsVisible);
                    Assert.Equal("SAFE MODE RECOVERY READY", title.Text);
                    Assert.Contains("FAULT WINDOW", faultWindow.Text, StringComparison.Ordinal);
                    Assert.Equal($"RECOVERY ARTIFACT · {Path.GetFileName(artifactPath)}", artifact.Text);
                    Assert.True(recover.IsVisible);
                    Assert.True(ignore.IsVisible);
                    Assert.Contains("RECOVERY READY", viewModel.Status, StringComparison.Ordinal);
                    Assert.Contains(
                        viewModel.Status,
                        window.GetVisualDescendants().OfType<TextBlock>().Select(value => value.Text));

                    Capture(window, "io05-autosave-safemode-recovery.png");
                    using var frame = window.CaptureRenderedFrame();
                    using var png = new MemoryStream();
                    frame.Save(png, new PngBitmapEncoderOptions());
                    var bytes = png.ToArray();
                    Assert.True(bytes.Length > 1_000, $"Rendered PNG was only {bytes.Length} bytes.");
                    Assert.Equal(new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 }, bytes.Take(8));

                    Click(window, recover);
                    await WaitFor(() => viewModel.Status.StartsWith("RECOVERED ·", StringComparison.Ordinal));
                    Assert.False(panel.IsVisible);
                    Assert.Equal($"RECOVERED · {Path.GetFileName(artifactPath)}", viewModel.Status);
                }
                finally
                {
                    window.Close();
                }
            }, CancellationToken.None);
        }
        finally
        {
            if (File.Exists(artifactPath)) File.Delete(artifactPath);
        }
    }

    [Fact]
    public async Task AdvancedSettingsRenderedControlsRoundTripEveryField()
    {
        var headless = DesktopTestApplication.Session;
        await headless.Dispatch(() =>
        {
            var snapshot = EngineSnapshot.Empty with
            {
                Species =
                [
                    new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false),
                    new SpeciesSnapshot("Animal Minimalis", false, 0xff239ac0, 0, false, 1f),
                    new SpeciesSnapshot("Alga Minimalis", true, 0xff4f9f45, 0, false, 0.5f),
                ],
            };
            var window = new AdvancedSettingsWindow(EnvironmentUpdate.Default, snapshot);
            ShowAndCapture(window, 860, 760, "advanced-initial.png");
            var tabs = window.GetVisualDescendants().OfType<TabControl>().Single();
            (int Tab, string[] Numeric, string[] Checks)[] groups =
            [
                (0, ["GlobalMutationRate"], ["MutationsEnabled"]),
                (1, ["Metabolism", "VegetableEnergy", "Sunlight", "StartChloroplasts", "MaxPlantEnergy", "MinimumChloroplastEquivalents", "RepopulationAmount", "RepopulationCooldown", "FeedingToBody", "CycleLength"], ["Daytime", "DayNightEnabled"]),
                (2, ["MaxVelocity", "MovementEfficiency", "GravityX", "GravityY", "SurfaceGravity", "StaticFriction", "KineticFriction", "Density", "Viscosity", "Elasticity", "Drag", "Brownian", "ShotSpeed", "ShotRangeMultiplier", "ShotDecay"], ["EnergyShotsNoDecay", "WasteShotsNoDecay"]),
                (3, ["SpeciationDistance"], ["AutoSpeciation", "ToroidalWorld"]),
            ];
            foreach (var group in groups)
            {
                tabs.SelectedIndex = group.Tab;
                window.UpdateLayout();
                foreach (var name in group.Numeric) AssertKeyboardIncrement(window, Named<NumericUpDown>(window, name), resetToMinimum: true);
                foreach (var name in group.Checks)
                {
                    var check = Named<CheckBox>(window, name);
                    var before = check.IsChecked;
                    Click(window, check);
                    Assert.True(before != check.IsChecked, $"{name} did not toggle from {before}.");
                }
            }

            var update = window.Update;
            Assert.NotEqual(EnvironmentUpdate.Default, update);
            Assert.NotEqual(MutationSettingsUpdate.Default, window.MutationUpdate);
            Assert.Equal(2, window.MutationUpdate.SpeciesRates.Count);
            Assert.True(Named<Border>(window, "DirtyBadge").IsVisible);
            Click(window, Named<Button>(window, "ApplyButton"));
            Assert.True(window.Accepted);
            Assert.False(window.IsVisible);

            var cancel = new AdvancedSettingsWindow(EnvironmentUpdate.Default);
            ShowForInteraction(cancel, 860, 760);
            Click(cancel, Named<Button>(cancel, "CancelButton"));
            Assert.False(cancel.Accepted);
            Assert.False(cancel.IsVisible);
        }, CancellationToken.None);
    }

    [Fact]
    public async Task GlobalSettingsRenderedSurfaceShowsFourTabs()
    {
        var headless = DesktopTestApplication.Session;
        await headless.Dispatch(() =>
        {
            var window = new GlobalSettingsWindow(new GlobalSettings
            {
                Main = new GlobalMainSettings { NewSeed = true },
                AdvancedMutation = new GlobalAdvancedMutationSettings { Delta2Enabled = true },
                Evolution = new GlobalEvolutionSettings { Mode = EvolutionMode.Zerobot },
            });
            ShowAndCapture(window, 1050, 780, "global-settings-four-tabs.png");
            var tabs = window.FindControl<TabControl>("Tabs")!;
            Assert.Equal(
                ["MAIN", "ADVANCED MUTATION", "LEAGUE", "EVOLUTION"],
                tabs.Items.Cast<TabItem>().Select(item => item.Header?.ToString() ?? string.Empty).ToArray());
            Assert.NotNull(window.FindControl<CheckBox>("NewSeed"));
            Assert.NotNull(window.FindControl<CheckBox>("Delta2Enabled"));
            Assert.NotNull(window.FindControl<CheckBox>("LeagueOfflineOwned"));
            Assert.NotNull(window.FindControl<ComboBox>("EvolutionMode"));
            window.Close();
        }, CancellationToken.None);
    }

    [Fact]
    public async Task DnaEditorRenderedControlsDispatchThroughVisualTree()
    {
        var engine = new AuditEngineClient();
        await using var simulation = new SimulationSession(engine);
        var storage = new AuditStorageService();
        var headless = DesktopTestApplication.Session;
        await headless.Dispatch(async () =>
        {
            var window = new DnaEditorWindow(simulation, engine.Organism, storage);
            ShowAndCapture(window, 760, 620, "dna-initial.png");
            var editor = Named<TextBox>(window, "Editor");
            await WaitFor(() =>
                engine.Calls.Contains("ExportDna")
                && editor.Text == "start\nstop");
            editor.Focus();
            editor.SelectAll();
            window.KeyTextInput("start\n20 .up store\nstop");
            await WaitFor(() => editor.Text?.Contains("20 .up store", StringComparison.Ordinal) == true);
            Click(window, Named<Button>(window, "ApplyDnaButton"));
            await WaitFor(() => engine.Calls.Contains("ReplaceDna:3:2"));
            Click(window, Named<Button>(window, "ApplyCloneButton"));
            await WaitFor(() =>
                engine.Calls.Contains("Clone:3:2")
                && engine.Calls.Contains("ReplaceDna:40:0"));
            Click(window, Named<Button>(window, "SaveBotButton"));
            await WaitFor(() => storage.SavedDna is not null);
            Capture(window, "dna-exercised.png");

            Assert.Contains("20 .up store", storage.SavedDna!, StringComparison.Ordinal);
            window.Close();
            return 0;
        }, CancellationToken.None);

    }

    [Fact]
    public async Task RuntimeRenderedControlsDispatchThroughVisualTree()
    {
        var factory = new AuditEngineClientFactory();
        var storage = new AuditStorageService
        {
            DnaFiles = [new DesktopTextFile("Imported_Hunter.txt", "start\n30 .up store\nstop")],
            OpenedSimulation = [9, 8, 7, 6],
        };
        var setup = new WorldSetupOptions
        {
            TicksPerUpdate = 1,
            Species = [new SpeciesImport("Audit Species", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
        };
        var headless = DesktopTestApplication.Session;
        await headless.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, factory, storage);
            ShowAndCapture(window, 1500, 900, "runtime-initial.png");
            var historicMenu = Named<LegacyMenuBar>(window, "HistoricMenuBar");
            foreach (var topLevel in historicMenu.TopLevelItems)
            {
                topLevel.IsSubMenuOpen = true;
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                Assert.True(topLevel.IsSubMenuOpen, $"Historic menu {topLevel.Header} did not open.");
                topLevel.IsSubMenuOpen = false;
            }
            await WaitFor(
                () => factory.Clients.TryPeek(out var client) && client.Calls.Contains("ImportSpecies:Audit Species"),
                () => factory.Clients.TryPeek(out var client) ? string.Join(", ", client.Calls) : "Engine client was not created.");
            var engine = factory.Clients.First();

            var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
            Assert.Equal(3U, viewModel.SelectedSlot);

            Click(window, Named<Button>(window, "RunButton"));
            Assert.True(Field<DispatcherTimer>(window, "_runTimer").IsEnabled);
            Click(window, Named<Button>(window, "PauseButton"));
            Assert.False(Field<DispatcherTimer>(window, "_runTimer").IsEnabled);
            Click(window, Named<Button>(window, "StepButton"));

            Click(window, Named<Button>(window, "TurboButton"));
            Assert.False(Named<WorldViewport>(window, "Viewport").IsVisible);
            Click(window, Named<Button>(window, "TurboButton"));
            Assert.True(Named<WorldViewport>(window, "Viewport").IsVisible);
            Click(window, Named<Button>(window, "PauseButton"));

            AssertComboSelection(window, Named<ComboBox>(window, "RuntimeSpeed"), 2);
            Click(window, Named<Button>(window, "BackendButton"));
            Click(window, Named<Button>(window, "ImportButton"));
            Click(window, Named<Button>(window, "SaveButton"));
            Click(window, Named<Button>(window, "LoadButton"));

            ToggleAndAssert(window, Named<CheckBox>(window, "ToroidalWorldToggle"));
            ToggleAndAssert(window, Named<CheckBox>(window, "RenderWasteToggle"));

            ClickExpander(window, Named<Expander>(window, "ParametersExpander"));
            ClickExpander(window, Named<Expander>(window, "PhysicsExpander"));
            Click(window, Named<Button>(window, "LiveAdvancedButton"));
            var advanced = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
            Named<NumericUpDown>(advanced, "Brownian").Value = 4;
            Click(advanced, Named<Button>(advanced, "ApplyButton"));

            ClickExpander(window, Named<Expander>(window, "SensesExpander"));
            ToggleAndAssert(window, Named<CheckBox>(window, "ShowVisionToggle"));
            ClickExpander(window, Named<Expander>(window, "EnergyExpander"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "LiveMetabolism"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "LivePlantEnergy"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "LiveFeedingToBody"));
            Click(window, Named<Button>(window, "ApplyEnergyButton"));

            ClickExpander(window, Named<Expander>(window, "HistoryExpander"));
            ClickExpander(window, Named<Expander>(window, "StatisticsExpander"));
            ClickExpander(window, Named<Expander>(window, "ExperimentsExpander"));
            Click(window, Named<Button>(window, "AdvanceZerobotButton"));

            AssertComboSelection(window, Named<ComboBox>(window, "LocalModeKindSelector"), 1);
            AssertComboSelection(window, Named<ComboBox>(window, "LocalModePreset"), 1);
            Named<NumericUpDown>(window, "ModeMinimumRounds").Value = 1;
            Named<NumericUpDown>(window, "ModeMaximumRounds").Value = 2;
            Named<NumericUpDown>(window, "ModeMaximumCycles").Value = 120;
            Named<NumericUpDown>(window, "ModePopulationCap").Value = 16;
            ToggleAndAssert(window, Named<CheckBox>(window, "ModeRestartOnExtinction"));
            ToggleAndAssert(window, Named<CheckBox>(window, "ModeProhibitInformation"));
            ToggleAndAssert(window, Named<CheckBox>(window, "ModeProhibitViruses"));
            ToggleAndAssert(window, Named<CheckBox>(window, "ModeProhibitSexual"));
            Named<TextBox>(window, "LocalModeReason").Text = "rendered adjudication";
            Click(window, Named<Button>(window, "ConfigureModeButton"));
            Click(window, Named<Button>(window, "StartModeButton"));
            Click(window, Named<Button>(window, "RestartModeButton"));
            Click(window, Named<Button>(window, "AdjudicateModeButton"));

            var viewport = Named<WorldViewport>(window, "Viewport");
            ClickAt(window, viewport, 0.5, 0.5);
            Assert.Equal(3U, viewModel.SelectedSlot);

            Click(window, Named<Button>(window, "MoveButton"));
            ClickAt(window, viewport, 0.82, 0.78);
            DragAt(window, viewport, 0.5, 0.5, 0.57, 0.58);

            Click(window, Named<Button>(window, "AddObstacleButton"));
            ClickAt(window, viewport, 0.30, 0.30);
            ClickAt(window, viewport, 0.40, 0.40);
            Click(window, Named<Button>(window, "AddTeleporterButton"));
            ClickAt(window, viewport, 0.60, 0.60);
            ClickAt(window, viewport, 0.70, 0.70);

            ClickAt(window, viewport, 0.09375, 0.125);
            Click(window, Named<Button>(window, "RemoveFeatureButton"));

            Click(window, Named<Button>(window, "CloneButton"));
            Click(window, Named<Button>(window, "ReproduceButton"));
            Click(window, Named<Button>(window, "FollowButton"));
            Click(window, Named<Button>(window, "FollowButton"));

            Click(window, Named<Button>(window, "EditDnaButton"));
            var editor = Assert.Single(window.OwnedWindows.OfType<DnaEditorWindow>());
            editor.Close();

            Click(window, Named<Button>(window, "KillButton"));
            ClickExpander(window, Named<Expander>(window, "EnvironmentExpander"));
            ClickExpander(window, Named<Expander>(window, "WorldExpander"));

            await WaitFor(() =>
                engine.Calls.Any(call => call.StartsWith("Tick:", StringComparison.Ordinal))
                && engine.Calls.Contains("Switch:Gpu")
                && engine.Calls.Contains("ImportSpecies:Imported_Hunter")
                && engine.Calls.Contains("Save")
                && engine.Calls.Contains("Load")
                && engine.Calls.Any(call => call.StartsWith("Move:3:2", StringComparison.Ordinal))
                && engine.Calls.Contains("AddObstacle")
                && engine.Calls.Contains("AddTeleporter")
                && engine.Calls.Contains("RemoveObstacle:7")
                && engine.Calls.Contains("Clone:3:2")
                && engine.Calls.Contains("Reproduce:3:2")
                && engine.Calls.Contains("Remove:3:2")
                && engine.Calls.Contains("ConfigureLocalMode")
                && engine.Calls.Contains("StartLocalMode")
                && engine.Calls.Contains("RestartLocalMode")
                && engine.Calls.Any(call => call.StartsWith("AdjudicateLocalRound:", StringComparison.Ordinal))
                && engine.EnvironmentUpdates.Count >= 3
                && storage.SavedSimulation is not null,
                () => string.Join(", ", engine.Calls));
            window.Height = 1200;
            AvaloniaHeadlessPlatform.ForceRenderTimerTick();
            Capture(window, "runtime-exercised.png");

            Click(window, Named<Button>(window, "ResetButton"));
            await WaitFor(() => factory.Clients.Count >= 2);
            return 0;
        }, CancellationToken.None);
    }

    [Fact]
    public async Task LegacyToolbarRoutesRemainingSlotsAndExposesAccessibleState()
    {
        var factory = new AuditEngineClientFactory();
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            Species = [new SpeciesImport("Audit Species", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
        };
        var callbackResult = new TaskCompletionSource<Exception?>(TaskCreationOptions.RunContinuationsAsynchronously);

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            try
            {
                var window = new MainWindow([], setup, factory, new AuditStorageService());
                try
                {
                    ShowForInteraction(window, 1280, 900);
                    await WaitFor(
                        () => factory.Clients.TryPeek(out var client) && client.Calls.Contains("ImportSpecies:Audit Species"),
                        () => "Audit engine did not import the toolbar test species.");

                    var toolbar = Named<UniformGrid>(window, "LegacyToolbar");
                    var slots = toolbar.GetVisualDescendants()
                        .Where(control => control.Name?.StartsWith("ToolbarSlot", StringComparison.Ordinal) == true)
                        .OrderBy(control => control.Name)
                        .ToArray();
                    Assert.Equal(24, slots.Length);
                    Assert.Equal(5, slots.OfType<Border>().Count());
                    Assert.Equal(18, slots.OfType<Button>().Count());
                    Assert.Single(slots.OfType<ComboBox>());
                    Assert.All(slots, slot => Assert.False(string.IsNullOrWhiteSpace(AutomationProperties.GetName(slot))));

                    var buttons = slots.OfType<Button>().ToArray();
                    var tabIndexes = buttons.Select(KeyboardNavigation.GetTabIndex).ToArray();
                    Assert.Equal(tabIndexes.OrderBy(index => index).ToArray(), tabIndexes);
                    Assert.Equal("NEW SIMULATION", AutomationProperties.GetName(buttons[0]));

                    var viewport = Named<WorldViewport>(window, "Viewport");
                    Assert.True(viewport.Focusable);
                    Assert.Equal("Simulation playfield", AutomationProperties.GetName(viewport));
                    Assert.Contains("click", AutomationProperties.GetHelpText(viewport), StringComparison.OrdinalIgnoreCase);
                    Assert.True(Named<Border>(window, "InternetOfflineOverlay").IsVisible);

                    var engine = factory.Clients.First();
                    Click(window, Named<Button>(window, "SunButton"));
                    await WaitFor(
                        () => engine.EnvironmentUpdates.Any(),
                        () => "Sun toolbar route did not publish an environment update.");
                    Assert.False(engine.EnvironmentUpdates.Last().Vegetation.Daytime);

                    Click(window, Named<Button>(window, "ToolbarSlot16"));
                    await WaitFor(
                        () => Named<Button>(window, "InsertSpeciesButton").Content?.ToString() == "CANCEL INSERTION",
                        () => "Insertion toolbar slot did not enter insertion mode.");
                    Assert.True(Named<Border>(window, "RuntimeOverlay").IsVisible);
                    Assert.Contains("INSERTION", Named<TextBlock>(window, "OverlayText").Text, StringComparison.OrdinalIgnoreCase);

                    Click(window, Named<Button>(window, "ToolbarSlot19"));
                    await WaitFor(
                        () => window.OwnedWindows.OfType<GraphSuiteWindow>().Any(),
                        () => "Graph toolbar slot did not open the graph suite.");
                    window.OwnedWindows.OfType<GraphSuiteWindow>().Single().Close();

                    Click(window, Named<Button>(window, "ContestIndicator"));
                    await WaitFor(
                        () => window.OwnedWindows.OfType<ContestResultsWindow>().Any(),
                        () => "Contest indicator did not open contest results.");
                    window.OwnedWindows.OfType<ContestResultsWindow>().Single().Close();

                    Click(window, Named<Button>(window, "ToolbarSlot24"));
                    Assert.True(Field<OfflineDiagnosticRecovery>(window, "_diagnostics").IgnoreErrors);
                    Assert.Contains("ON", Named<Button>(window, "ToolbarSlot24").Content?.ToString(), StringComparison.Ordinal);
                }
                finally
                {
                    foreach (var child in window.OwnedWindows.ToArray()) child.Close();
                    window.Close();
                }
            }
            catch (Exception exception)
            {
                callbackResult.TrySetResult(exception);
                return;
            }
            callbackResult.TrySetResult(null);
        }, CancellationToken.None);

        var callbackFailure = await callbackResult.Task.WaitAsync(TimeSpan.FromSeconds(10));
        Assert.Null(callbackFailure);
    }

    [Fact]
    public async Task MainWindowKeepsBottomDashboardVisibleAndUsableAt1500x900()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var window = new MainWindow([], new WorldSetupOptions());
            ShowForInteraction(window, 1500, 900);
            try
            {
                Capture(window, "shell-dashboard-1500x900.png");
                var surface = Named<Grid>(window, "MainSurfaceGrid");
                var dashboard = Named<ItemsControl>(window, "StatusInstrument");
                var theme = Named<ComboBox>(window, "ThemeSelector");
                var culture = Named<ComboBox>(window, "CultureSelector");

                Assert.True(dashboard.IsVisible);
                Assert.True(dashboard.Bounds.Height >= 52, $"Dashboard height was {dashboard.Bounds.Height}.");
                Assert.InRange(dashboard.Bounds.Bottom, 0, surface.Bounds.Height);
                Assert.InRange(theme.Bounds.Bottom, 0, surface.Bounds.Height);
                Assert.InRange(culture.Bounds.Bottom, 0, surface.Bounds.Height);
                Assert.True(theme.Bounds.Width >= 90);
                Assert.True(culture.Bounds.Width >= 120);
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task MainWindowRenderedSettingsWalkthroughCoversShellMenuRunEnergyAndMutation()
    {
        var factory = new AuditEngineClientFactory();
        var setup = new WorldSetupOptions
        {
            TicksPerUpdate = 1,
            Species = [new SpeciesImport("Luna", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, factory, new AuditStorageService());
            try
            {
                ShowForInteraction(window, 1_500, 900);
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                var surface = Named<Grid>(window, "MainSurfaceGrid");
                var dashboard = Named<ItemsControl>(window, "StatusInstrument");
                var menu = Named<LegacyMenuBar>(window, "HistoricMenuBar");

                await WaitFor(
                    () => factory.Clients.TryPeek(out var client) && client.Calls.Contains("ImportSpecies:Luna"),
                    () => factory.Clients.TryPeek(out var client) ? string.Join(", ", client.Calls) : "Engine client was not created.");
                var engine = factory.Clients.First();

                Assert.True(surface.IsVisible);
                Assert.True(dashboard.IsVisible);
                Assert.Contains("WORLD READY", viewModel.Status, StringComparison.Ordinal);
                Assert.True(Named<Border>(window, "InternetOfflineOverlay").IsVisible);

                var onlineButton = Named<Button>(window, "InternetButton");
                var statusBeforeOnlineInput = viewModel.Status;
                var callsBeforeOnlineInput = engine.Calls.ToArray();
                Assert.False(onlineButton.IsEnabled);
                Click(window, onlineButton);
                Assert.Equal(statusBeforeOnlineInput, viewModel.Status);
                Assert.Equal(callsBeforeOnlineInput, engine.Calls.ToArray());
                var onlineCommand = menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.InternetEnabled)!;
                Assert.False(onlineCommand.IsEnabled);

                var generalSettingsCommand = menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditGeneralSettings)!;
                ActivateMenu(generalSettingsCommand);
                var generalSettings = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                Assert.True(generalSettings.IsVisible);
                Named<NumericUpDown>(generalSettings, "Metabolism").Value = 7;
                Named<NumericUpDown>(generalSettings, "VegetableEnergy").Value = 33;
                Named<NumericUpDown>(generalSettings, "Sunlight").Value = 222;
                Named<CheckBox>(generalSettings, "Daytime").IsChecked = false;
                Named<CheckBox>(generalSettings, "DayNightEnabled").IsChecked = true;
                Named<NumericUpDown>(generalSettings, "CycleLength").Value = 240;
                var appliedEnvironment = generalSettings.Update;
                Click(generalSettings, Named<Button>(generalSettings, "ApplyButton"));
                await WaitFor(() => engine.EnvironmentUpdates.Count >= 1);
                Assert.Equal(appliedEnvironment, engine.EnvironmentUpdates.Single());
                Assert.Equal(appliedEnvironment, Field<EnvironmentUpdate>(window, "_liveEnvironment"));
                Assert.Contains("LIVE SETTINGS APPLIED", viewModel.Status, StringComparison.Ordinal);

                var mutationSettingsCommand = menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditGlobalMutationRates)!;
                ActivateMenu(mutationSettingsCommand);
                var mutationSettings = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                Assert.True(mutationSettings.IsVisible);
                Named<CheckBox>(mutationSettings, "MutationsEnabled").IsChecked = true;
                Named<NumericUpDown>(mutationSettings, "GlobalMutationRate").Value = 1.75m;
                var appliedMutation = mutationSettings.MutationUpdate;
                Click(mutationSettings, Named<Button>(mutationSettings, "ApplyButton"));
                await WaitFor(() => engine.MutationUpdates.Count >= 1);
                Assert.Equal(appliedMutation, engine.MutationUpdates.Single());
                Assert.Contains("LIVE SETTINGS APPLIED", viewModel.Status, StringComparison.Ordinal);

                AssertComboSelection(window, Named<ComboBox>(window, "RuntimeSpeed"), 2);
                Assert.Contains("5 TICKS/FRAME", viewModel.Status, StringComparison.Ordinal);
                AssertComboSelection(window, Named<ComboBox>(window, "RuntimeSpeed"), 3);
                Assert.Contains("MAXIMUM", viewModel.Status, StringComparison.Ordinal);
                Click(window, Named<Button>(window, "RunButton"));
                Assert.True(Field<DispatcherTimer>(window, "_runTimer").IsEnabled);
                Click(window, Named<Button>(window, "PauseButton"));
                Assert.False(Field<DispatcherTimer>(window, "_runTimer").IsEnabled);
                Click(window, Named<Button>(window, "StepButton"));

                var turbo = Named<Button>(window, "TurboButton");
                Click(window, turbo);
                Assert.False(Named<WorldViewport>(window, "Viewport").IsVisible);
                Assert.Contains("TURBO", viewModel.Status, StringComparison.Ordinal);
                Click(window, turbo);
                Assert.True(Named<WorldViewport>(window, "Viewport").IsVisible);

                Named<NumericUpDown>(window, "LiveMetabolism").Value = 6;
                Named<NumericUpDown>(window, "LivePlantEnergy").Value = 250;
                Named<NumericUpDown>(window, "LiveFeedingToBody").Value = 0.75m;
                Click(window, Named<Button>(window, "ApplyEnergyButton"));
                await WaitFor(() => engine.EnvironmentUpdates.Count >= 2);
                Assert.Contains("ENERGY APPLIED", viewModel.Status, StringComparison.Ordinal);

                var sun = Named<Button>(window, "SunButton");
                var sunBefore = sun.Content?.ToString();
                Click(window, sun);
                await WaitFor(() => !string.Equals(sun.Content?.ToString(), sunBefore, StringComparison.Ordinal));
                Assert.Contains("SUN", sun.Content?.ToString(), StringComparison.Ordinal);

                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var frame = window.CaptureRenderedFrame();
                Assert.NotNull(frame);
                Assert.True(frame.PixelSize.Width >= 400);
                Assert.True(frame.PixelSize.Height >= 300);
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task MainWindowMutationSurfaceRendersDefaultsAndPublishesGlobalAndSpeciesPressure()
    {
        var factory = new AuditEngineClientFactory();
        var setup = new WorldSetupOptions
        {
            TicksPerUpdate = 1,
            Species = [new SpeciesImport("Luna", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, factory, new AuditStorageService());
            try
            {
                ShowForInteraction(window, 1_200, 820);
                await WaitFor(
                    () => factory.Clients.TryPeek(out var client) && client.Calls.Contains("ImportSpecies:Luna"),
                    () => "The real MainWindow route did not create the Luna simulation.");

                var engine = factory.Clients.Single();
                var menu = Named<LegacyMenuBar>(window, "HistoricMenuBar");
                ActivateMenu(menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditGeneralSettings)!);
                await WaitFor(
                    () => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any(),
                    () => "MainWindow did not open the live settings surface.");

                var settings = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                Named<TabControl>(settings, "SettingsTabs").SelectedIndex = 0;
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                Assert.True(settings.IsVisible);
                Assert.True(Named<CheckBox>(settings, "MutationsEnabled").IsChecked);
                Assert.Equal(1m, Named<NumericUpDown>(settings, "GlobalMutationRate").Value);
                var speciesRows = Named<StackPanel>(settings, "SpeciesMutationRows");
                var speciesRate = Assert.Single(speciesRows.GetVisualDescendants().OfType<NumericUpDown>());
                Assert.Equal(0m, speciesRate.Value);
                Assert.Contains("GLOBAL MUTATION PRESSURE", settings.GetVisualDescendants().OfType<TextBlock>().Select(text => text.Text));
                Assert.Equal(
                    Avalonia.Media.Color.Parse("#F4F0E6"),
                    Assert.IsType<Avalonia.Media.SolidColorBrush>(settings.Background).Color);

                using var frame = settings.CaptureRenderedFrame();
                Assert.True(frame.PixelSize.Width >= 760);
                Assert.True(frame.PixelSize.Height >= 620);

                Named<NumericUpDown>(settings, "GlobalMutationRate").Value = 2.25m;
                speciesRate.Value = 12.5m;
                var staged = settings.MutationUpdate;
                Assert.Equal(2.25f, staged.GlobalMultiplier);
                Assert.Equal(12.5f, Assert.Single(staged.SpeciesRates).Rate);
                Click(settings, Named<Button>(settings, "ApplyButton"));
                await WaitFor(() => engine.MutationUpdates.Count == 1);
                var applied = engine.MutationUpdates.Single();
                Assert.Equal(2.25f, applied.GlobalMultiplier);
                Assert.Equal(12.5f, Assert.Single(applied.SpeciesRates).Rate);

                ActivateMenu(menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditGeneralSettings)!);
                await WaitFor(() => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any());
                var canceled = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                Named<TabControl>(canceled, "SettingsTabs").SelectedIndex = 0;
                Named<NumericUpDown>(canceled, "GlobalMutationRate").Value = 4m;
                Click(canceled, Named<Button>(canceled, "CancelButton"));
                Assert.Equal(1, engine.MutationUpdates.Count);
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static void ShowAndCapture(Window window, double width, double height, string evidenceName)
    {
        ShowForInteraction(window, width, height);
        Capture(window, evidenceName);
    }

    private static void ShowForInteraction(Window window, double width, double height)
    {
        window.WindowState = WindowState.Normal;
        window.Width = width;
        window.Height = height;
        window.Show();
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }

    private static void Capture(Window window, string evidenceName)
    {
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        using var frame = window.CaptureRenderedFrame();
        Assert.NotNull(frame);
        Assert.True(frame.PixelSize.Width >= 400, $"Rendered width was {frame.PixelSize.Width}.");
        Assert.True(frame.PixelSize.Height >= 300, $"Rendered height was {frame.PixelSize.Height}.");
        var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR");
        if (string.IsNullOrWhiteSpace(directory))
            directory = Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
        Directory.CreateDirectory(directory);
        using var stream = File.Create(Path.Combine(directory, evidenceName));
        frame.Save(stream, new PngBitmapEncoderOptions());
    }

    private static T Named<T>(Control root, string name) where T : Control =>
        root.FindControl<T>(name) ?? throw new Xunit.Sdk.XunitException($"Missing {typeof(T).Name} named {name}.");

    private static void Click(Window window, Control control)
    {
        StabilizeForInput(window, control);
        var local = new Point(Math.Max(1, control.Bounds.Width / 2), Math.Max(1, control.Bounds.Height / 2));
        var point = control.TranslatePoint(local, window);
        Assert.True(point.HasValue, $"{control.Name ?? control.GetType().Name} was not attached to the rendered window.");
        AssertPointInsideWindow(window, point.Value, control);
        window.MouseDown(point.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(point.Value, MouseButton.Left, RawInputModifiers.None);
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }

    private static void ClickAt(Window window, Control control, double fractionX, double fractionY)
    {
        StabilizeForInput(window, control);
        var local = new Point(control.Bounds.Width * fractionX, control.Bounds.Height * fractionY);
        var point = control.TranslatePoint(local, window);
        Assert.True(point.HasValue);
        AssertPointInsideWindow(window, point.Value, control);
        window.MouseDown(point.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(point.Value, MouseButton.Left, RawInputModifiers.None);
    }

    private static void DragAt(
        Window window,
        Control control,
        double startFractionX,
        double startFractionY,
        double endFractionX,
        double endFractionY)
    {
        StabilizeForInput(window, control);
        var start = control.TranslatePoint(
            new Point(control.Bounds.Width * startFractionX, control.Bounds.Height * startFractionY), window);
        var end = control.TranslatePoint(
            new Point(control.Bounds.Width * endFractionX, control.Bounds.Height * endFractionY), window);
        Assert.True(start.HasValue && end.HasValue);
        AssertPointInsideWindow(window, start.Value, control);
        AssertPointInsideWindow(window, end.Value, control);
        window.MouseDown(start.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseMove(end.Value, RawInputModifiers.LeftMouseButton);
        window.MouseUp(end.Value, MouseButton.Left, RawInputModifiers.None);
    }

    private static void DragSlider(Window window, Slider slider, double fraction)
    {
        StabilizeForInput(window, slider);
        var start = slider.TranslatePoint(new Point(2, slider.Bounds.Height / 2), window);
        var end = slider.TranslatePoint(new Point(slider.Bounds.Width * fraction, slider.Bounds.Height / 2), window);
        Assert.True(start.HasValue && end.HasValue);
        AssertPointInsideWindow(window, start.Value, slider);
        AssertPointInsideWindow(window, end.Value, slider);
        window.MouseDown(start.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseMove(end.Value, RawInputModifiers.LeftMouseButton);
        window.MouseUp(end.Value, MouseButton.Left, RawInputModifiers.None);
    }

    private static void ClickExpander(Window window, Expander expander)
    {
        var header = expander.GetVisualDescendants().OfType<ToggleButton>().First();
        StabilizeForInput(window, header);
        var before = expander.IsExpanded;
        var point = header.TranslatePoint(
            new Point(Math.Max(1, header.Bounds.Width / 2), Math.Max(1, header.Bounds.Height / 2)),
            window);
        Assert.True(point.HasValue);
        AssertPointInsideWindow(window, point.Value, header);
        window.MouseDown(point.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(point.Value, MouseButton.Left, RawInputModifiers.None);
        Assert.NotEqual(before, expander.IsExpanded);
    }

    private static void OpenAndCloseMenu(Window window, string name)
    {
        var menu = Named<MenuItem>(window, name);
        Click(window, menu);
        Assert.True(menu.IsSubMenuOpen);
        menu.IsSubMenuOpen = false;
    }

    private static void ActivateMenu(MenuItem item) =>
        item.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));

    private static void ToggleAndAssert(Window window, CheckBox check)
    {
        var before = check.IsChecked;
        Click(window, check);
        Assert.NotEqual(before, check.IsChecked);
    }

    private static void AssertKeyboardIncrement(Window window, NumericUpDown numeric, bool resetToMinimum = false)
    {
        StabilizeForInput(window, numeric);
        if (resetToMinimum) numeric.Value = numeric.Minimum;
        var before = numeric.Value;
        numeric.Focus();
        window.KeyPress(Key.Up, RawInputModifiers.None, PhysicalKey.ArrowUp, null);
        window.KeyRelease(Key.Up, RawInputModifiers.None, PhysicalKey.ArrowUp, null);
        Assert.True(numeric.Value > before, $"{numeric.Name} did not increment from {before}.");
    }

    private static void AssertComboSelection(Window window, ComboBox combo, int targetIndex)
    {
        StabilizeForInput(window, combo);
        combo.SelectedIndex = Math.Max(0, targetIndex - 1);
        combo.Focus();
        window.KeyPress(Key.Down, RawInputModifiers.None, PhysicalKey.ArrowDown, null);
        window.KeyRelease(Key.Down, RawInputModifiers.None, PhysicalKey.ArrowDown, null);
        Assert.Equal(targetIndex, combo.SelectedIndex);
    }

    private static void StabilizeForInput(Window window, Control control)
    {
        control.BringIntoView();
        Dispatcher.UIThread.RunJobs();
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        Dispatcher.UIThread.RunJobs();
        Assert.True(window.IsVisible, $"{window.Title} was not visible while targeting {control.Name ?? control.GetType().Name}.");
    }

    private static void AssertPointInsideWindow(Window window, Point point, Control control)
    {
        Assert.InRange(point.X, 0, window.ClientSize.Width);
        Assert.InRange(point.Y, 0, window.ClientSize.Height);
    }

    private static T Field<T>(object target, string name) =>
        (T)target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(target)!;

    private static async Task WaitFor(Func<bool> condition, Func<string>? details = null)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(8);
        while (!condition() && DateTime.UtcNow < deadline) await Task.Delay(10);
        Assert.True(condition(), $"Timed out waiting for rendered control dispatch. {details?.Invoke()}");
    }
}

internal sealed class AuditStorageService : IDesktopStorageService
{
    public IReadOnlyList<DesktopTextFile> DnaFiles { get; init; } = [];
    public string? PickedSimulationPath { get; init; }
    public byte[]? OpenedSimulation { get; init; }
    public DesktopSettings? SettingsToOpen { get; init; }
    public string? LatestAutosave { get; init; }
    public byte[]? SavedSimulation { get; private set; }
    public string? SavedDna { get; private set; }
    public byte[]? SavedLivingSnapshot { get; private set; }
    public List<byte[]> DeadSnapshotWrites { get; } = [];
    public bool AcceptOrganismSave { get; init; } = true;
    public string? SavedOrganismSuggestedFileName { get; private set; }
    public byte[]? SavedOrganism { get; private set; }
    public byte[]? Autosave { get; private set; }
    public DesktopSettings? SavedSettings { get; private set; }

    public Task<IReadOnlyList<DesktopTextFile>> OpenDnaFilesAsync(TopLevel owner) => Task.FromResult(DnaFiles);
    public Task<DesktopSettings?> OpenSettingsAsync(TopLevel owner) => Task.FromResult(SettingsToOpen);
    public Task<string?> PickSimulationPathAsync(TopLevel owner) => Task.FromResult(PickedSimulationPath);
    public Task<byte[]?> OpenSimulationAsync(TopLevel owner) => Task.FromResult(OpenedSimulation);

    public Task<bool> SaveSettingsAsync(TopLevel owner, DesktopSettings settings)
    {
        SavedSettings = settings;
        return Task.FromResult(true);
    }

    public Task<bool> SaveSimulationAsync(TopLevel owner, ReadOnlyMemory<byte> save)
    {
        SavedSimulation = save.ToArray();
        return Task.FromResult(true);
    }

    public Task<bool> SaveDnaAsync(TopLevel owner, string suggestedFileName, string dna)
    {
        SavedDna = dna;
        return Task.FromResult(true);
    }

    public async Task<bool> SaveLivingSnapshotAsync(TopLevel owner, string suggestedFileName, Func<Stream, Task> writeAsync)
    {
        using var stream = new MemoryStream();
        await writeAsync(stream);
        SavedLivingSnapshot = stream.ToArray();
        return true;
    }

    public async Task<bool> AppendDeadSnapshotAsync(Func<Stream, Task> writeAsync)
    {
        using var stream = new MemoryStream();
        await writeAsync(stream);
        DeadSnapshotWrites.Add(stream.ToArray());
        return true;
    }

    public async Task<bool> SaveOrganismAsync(TopLevel owner, string suggestedFileName, Func<Stream, Task> writeAsync)
    {
        SavedOrganismSuggestedFileName = suggestedFileName;
        if (!AcceptOrganismSave) return false;
        using var stream = new MemoryStream();
        await writeAsync(stream);
        SavedOrganism = stream.ToArray();
        return true;
    }

    public string? FindLatestAutosave() => LatestAutosave;

    public Task SaveAutosaveAsync(ReadOnlyMemory<byte> save)
    {
        Autosave = save.ToArray();
        return Task.CompletedTask;
    }
}

internal sealed class AuditEngineClientFactory : IEngineClientFactory
{
    public ConcurrentQueue<AuditEngineClient> Clients { get; } = [];

    public IEngineClient Create(WorldSetupOptions setup)
    {
        var client = new AuditEngineClient();
        Clients.Enqueue(client);
        return client;
    }
}

internal sealed class AuditEngineClient : IEngineClient
{
    private ulong _tick;
    private string _backend = "CPU";
    private EngineSnapshot _snapshot;
    private LocalModeConfig _localModeConfig = LocalModeConfig.Default;

    public AuditEngineClient()
    {
        Organism = new OrganismSnapshot(3, 2, [8_000f, 6_000f], [0f, 0f], 1_000, 10, Species: 1);
        _snapshot = new EngineSnapshot(0, 1, _backend, [Organism])
        {
            Species =
            [
                new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false),
                new SpeciesSnapshot("Audit Species", false, 0xff239ac0, 0, false),
            ],
            RenderInstances =
            [
                new RenderInstanceSnapshot(3, [8_000f, 6_000f], 12f, 0xff239ac0) { Generation = 2 },
            ],
            Obstacles = [new ObstacleSnapshot(7, [1_000f, 1_000f], [2_000f, 2_000f])],
            Teleporters = [new TeleporterSnapshot(8, [14_000f, 10_000f], 180f, [2_000f, 2_000f])],
        };
    }

    public OrganismSnapshot Organism { get; }
    public ConcurrentQueue<string> Calls { get; } = [];
    public ConcurrentQueue<EnvironmentUpdate> EnvironmentUpdates { get; } = [];
    public ConcurrentQueue<MutationSettingsUpdate> MutationUpdates { get; } = [];
    public string Backend => _backend;

    public EngineCapabilities Capabilities() => new(1, _backend, true, null);
    public void Tick(uint count = 1) { _tick += count; Calls.Enqueue($"Tick:{count}"); }
    public DnaImportReport ImportDna(string dna, float[]? position = null) { Calls.Enqueue("ImportDna"); return DnaImportReport.Compatible; }
    public DnaImportReport ImportSpecies(SpeciesImport species) { Calls.Enqueue($"ImportSpecies:{species.Name}"); return DnaImportReport.Compatible; }
    public DnaImportReport InsertSpeciesAt(SpeciesImport species, float[] position)
    {
        Calls.Enqueue($"InsertSpeciesAt:{species.Name}:{position[0]:0}:{position[1]:0}");
        return DnaImportReport.Compatible;
    }
    public void Remove(uint slot, uint generation) => Calls.Enqueue($"Remove:{slot}:{generation}");
    public void RemoveMany(IReadOnlyList<OrganismKey> organisms) => Calls.Enqueue($"RemoveMany:{string.Join(',', organisms.Select(value => $"{value.Slot}:{value.Generation}"))}");
    public void Move(uint slot, uint generation, float[] position) => Calls.Enqueue($"Move:{slot}:{generation}:{position[0]:0}:{position[1]:0}");
    public OrganismKey CloneOrganism(uint slot, uint generation, float[] position) { Calls.Enqueue($"Clone:{slot}:{generation}"); return new OrganismKey(40, 0); }
    public void ReplaceDna(uint slot, uint generation, string dna) => Calls.Enqueue($"ReplaceDna:{slot}:{generation}");
    public string ExportDna(uint slot, uint generation) { Calls.Enqueue("ExportDna"); return "start\nstop"; }
    public OrganismKey Reproduce(uint firstSlot, uint firstGeneration, uint? secondSlot, uint? secondGeneration, float[] position)
    {
        Calls.Enqueue($"Reproduce:{firstSlot}:{firstGeneration}");
        return new OrganismKey(41, 0);
    }
    public void SwitchBackend(string backend) { _backend = backend.ToUpperInvariant(); Calls.Enqueue($"Switch:{backend}"); }
    public void AddObstacle(ObstacleSnapshot obstacle) { Calls.Enqueue("AddObstacle"); _snapshot = _snapshot with { Obstacles = [.. _snapshot.Obstacles, obstacle] }; }
    public void UpdateObstacle(ObstacleSnapshot obstacle) { Calls.Enqueue("UpdateObstacle"); _snapshot = _snapshot with { Obstacles = _snapshot.Obstacles.Select(value => value.Id == obstacle.Id ? obstacle : value).ToArray() }; }
    public void RemoveObstacle(uint id) { Calls.Enqueue($"RemoveObstacle:{id}"); _snapshot = _snapshot with { Obstacles = _snapshot.Obstacles.Where(value => value.Id != id).ToArray() }; }
    public void AddTeleporter(TeleporterSnapshot teleporter) { Calls.Enqueue("AddTeleporter"); _snapshot = _snapshot with { Teleporters = [.. _snapshot.Teleporters, teleporter] }; }
    public void UpdateTeleporter(TeleporterSnapshot teleporter) { Calls.Enqueue("UpdateTeleporter"); _snapshot = _snapshot with { Teleporters = _snapshot.Teleporters.Select(value => value.Id == teleporter.Id ? teleporter : value).ToArray() }; }
    public void RemoveTeleporter(uint id) { Calls.Enqueue($"RemoveTeleporter:{id}"); _snapshot = _snapshot with { Teleporters = _snapshot.Teleporters.Where(value => value.Id != id).ToArray() }; }
    public void AddFileTeleporter(FileTeleporterSnapshot teleporter) { Calls.Enqueue("AddFileTeleporter"); _snapshot = _snapshot with { FileTeleporters = [.. _snapshot.FileTeleporters, teleporter] }; }
    public void UpdateFileTeleporter(FileTeleporterSnapshot teleporter) { Calls.Enqueue("UpdateFileTeleporter"); _snapshot = _snapshot with { FileTeleporters = _snapshot.FileTeleporters.Select(value => value.Id == teleporter.Id ? teleporter : value).ToArray() }; }
    public void RemoveFileTeleporter(uint id) { Calls.Enqueue($"RemoveFileTeleporter:{id}"); _snapshot = _snapshot with { FileTeleporters = _snapshot.FileTeleporters.Where(value => value.Id != id).ToArray() }; }
    public void SetBrownianMotion(float value) => Calls.Enqueue($"Brownian:{value}");
    public void UpdateEnvironment(EnvironmentUpdate update) { EnvironmentUpdates.Enqueue(update); Calls.Enqueue("Environment"); }
    public void UpdateMutationSettings(MutationSettingsUpdate update) { MutationUpdates.Enqueue(update); Calls.Enqueue("Mutations"); }
    public void SetTiesEnabled(bool enabled) => Calls.Enqueue($"Ties:{enabled}");
    public void SetSpeciesColor(uint species, uint color) => Calls.Enqueue($"Color:{species}:{color}");
    public void MakeNewSpecies(uint slot, uint generation, string name, uint color) => Calls.Enqueue($"NewSpecies:{slot}:{generation}:{name}");
    public void UpdateLegacyRestrictions(bool robotFixingAllowed, bool asexualReproductionAllowed) => Calls.Enqueue($"Restrictions:{robotFixingAllowed}:{asexualReproductionAllowed}");
    public void ConfigurePlayerBot(PlayerBotConfig config) => Calls.Enqueue("ConfigurePlayerBot");
    public void SetPlayerBotKey(uint slot, uint generation, int keyCode, bool pressed) => Calls.Enqueue($"PlayerBotKey:{slot}:{generation}:{keyCode}:{pressed}");
    public void SetOrganismMemory(uint slot, uint generation, ushort address, int value) => Calls.Enqueue($"Memory:{slot}:{generation}:{address}:{value}");
    public void SetOrganismEnergy(uint slot, uint generation, int energy) => Calls.Enqueue($"Energy:{slot}:{generation}:{energy}");
    public void UpdateRestrictionOptions(RestrictionOptions options) => Calls.Enqueue("RestrictionOptions");
    public void ConfigureLocalMode(LocalModeConfig config)
    {
        Calls.Enqueue("ConfigureLocalMode");
        _localModeConfig = config;
        _snapshot = _snapshot with { LocalMode = LocalModeSnapshotFor(config, LocalModeStatus.Configured, 0, 0, 0, []) };
    }
    public void StartLocalMode()
    {
        Calls.Enqueue("StartLocalMode");
        _snapshot = _snapshot with { LocalMode = LocalModeSnapshotFor(_localModeConfig, LocalModeStatus.Running, 1, 0, 0, []) };
    }
    public void RestartLocalMode()
    {
        Calls.Enqueue("RestartLocalMode");
        _snapshot = _snapshot with { LocalMode = LocalModeSnapshotFor(_localModeConfig, LocalModeStatus.Running, 1, 0, 1, []) };
    }
    public void AdjudicateLocalRound(uint? winner, string reason)
    {
        Calls.Enqueue($"AdjudicateLocalRound:{winner?.ToString() ?? "none"}");
        var result = new LocalModeResult(1, winner, 1, 1_000, reason);
        _snapshot = _snapshot with
        {
            LocalMode = LocalModeSnapshotFor(_localModeConfig, LocalModeStatus.Completed, 1, 0, 1, [result], new LocalModeAdjudication(winner, reason)),
        };
    }

    private static LocalModeSnapshot LocalModeSnapshotFor(
        LocalModeConfig config,
        LocalModeStatus status,
        uint round,
        ulong cycle,
        uint restarts,
        IReadOnlyList<LocalModeResult> results,
        LocalModeAdjudication? adjudication = null) => new(
        status,
        config.Kind,
        config.Preset,
        round,
        config.MaximumRounds,
        cycle,
        config.MaximumCycles,
        restarts,
        results,
        adjudication,
        config.Restrictions);
    public void SetOrganismAim(uint slot, uint generation, int aim) => Calls.Enqueue($"Aim:{slot}:{generation}:{aim}");
    public byte[] Save() { Calls.Enqueue("Save"); return [1, 2, 3, 4]; }
    public byte[] SaveWithoutMutations() { Calls.Enqueue("SaveWithoutMutations"); return [4, 3, 2, 1]; }
    public void Load(byte[] save) => Calls.Enqueue("Load");
    public EngineSnapshot Snapshot() => _snapshot with { Tick = _tick, Backend = _backend };
    public void Dispose() => Calls.Enqueue("Dispose");
}
