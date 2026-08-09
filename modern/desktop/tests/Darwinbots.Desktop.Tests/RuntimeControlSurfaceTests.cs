using System.Collections.Concurrent;
using System.Reflection;
using System.Text;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Threading;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class RuntimeControlSurfaceTests
{
    [Fact]
    public async Task HistoricShortcutMenuItemsOwnAndDispatchConflictingGestures()
    {
        var engine = new RecordingEngineClient();
        await using var simulation = new SimulationSession(engine);
        var storage = new AuditStorageService();
        await simulation.StepAsync(0);

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), new AuditEngineClientFactory(), storage);
            try
            {
                window.Show();
                SetField(window, "_session", simulation);
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                viewModel.Update(RecordingEngineClient.CreateSnapshot());
                viewModel.Select(3);
                window.FindControl<WorldViewport>("Viewport")!.SelectSlot(3);
                window.GetType().GetMethod("RefreshLegacyMenuStates", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .Invoke(window, null);

                var catalogGestures = Core.LegacyMenus.LegacyMenuCatalog.FlattenLeaves()
                    .Where(item => item.Shortcut is not null)
                    .Select(item => KeyGesture.Parse(item.Shortcut!))
                    .ToArray();
                Assert.Equal(28, catalogGestures.Length);
                foreach (var gesture in catalogGestures)
                {
                    Assert.Single(window.KeyBindings, binding => Equals(binding.Gesture, gesture));
                }

                var speed = window.FindControl<ComboBox>("RuntimeSpeed")!;
                speed.Focus();

                window.KeyPress(Key.F5, RawInputModifiers.None, PhysicalKey.F5, null);
                window.KeyRelease(Key.F5, RawInputModifiers.None, PhysicalKey.F5, null);
                await WaitFor(() => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any());
                Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>()).Close();

                window.KeyPress(Key.F8, RawInputModifiers.None, PhysicalKey.F8, null);
                window.KeyRelease(Key.F8, RawInputModifiers.None, PhysicalKey.F8, null);
                await WaitFor(() => window.OwnedWindows.OfType<RestrictionOptionsWindow>().Any());
                Assert.Single(window.OwnedWindows.OfType<RestrictionOptionsWindow>()).Close();

                window.KeyPress(Key.S, RawInputModifiers.Control, PhysicalKey.S, null);
                window.KeyRelease(Key.S, RawInputModifiers.Control, PhysicalKey.S, null);
                await WaitFor(() => storage.SavedDna == "start\nstop");

                window.KeyPress(Key.E, RawInputModifiers.Control, PhysicalKey.E, null);
                window.KeyRelease(Key.E, RawInputModifiers.Control, PhysicalKey.E, null);
                await WaitFor(() => engine.Calls.Contains("RemoveMany:3:2"), () => string.Join(", ", engine.Calls));
                Assert.Equal(["RemoveMany:3:2"], engine.Calls.Where(call => call.StartsWith("RemoveMany:", StringComparison.Ordinal)));
                return 0;
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task EditedGlobalSettingsReachNextWorldCreatePayloadAfterReset()
    {
        var factory = new SetupCapturingEngineClientFactory();
        var storage = new AuditStorageService();

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), factory, storage);
            window.Show();
            await WaitFor(() => factory.Setups.Count >= 1);
            Assert.Equal(1, factory.Setups.Count);
            var originalSetup = factory.Setups.Single();

            var dialog = new GlobalSettingsWindow(new GlobalSettings());
            dialog.FindControl<CheckBox>("Delta2Enabled")!.IsChecked = true;
            dialog.FindControl<NumericUpDown>("DnaUpkeep")!.Value = 2.5m;
            dialog.FindControl<CheckBox>("SaveAsTemporaryGene")!.IsChecked = true;
            Invoke(dialog, "Apply_Click");
            SetField(window, "_globalSettings", dialog.Settings);
            Assert.True(Field<GlobalSettings>(window, "_globalSettings").AdvancedMutation.Delta2Enabled);
            Assert.Equal(2.5, Field<GlobalSettings>(window, "_globalSettings").Costs.DnaUpkeep);
            Assert.False(originalSetup.GlobalSettings.AdvancedMutation.Delta2Enabled);
            Assert.Equal(0, originalSetup.GlobalSettings.Costs.DnaUpkeep);
            var pendingSetup = (WorldSetupOptions)typeof(MainWindow)
                .GetMethod("CreateNextWorldSetup", BindingFlags.Instance | BindingFlags.NonPublic)!
                .Invoke(window, null)!;
            Assert.True(pendingSetup.GlobalSettings.AdvancedMutation.Delta2Enabled);
            Assert.Equal(2.5, pendingSetup.GlobalSettings.Costs.DnaUpkeep);

            Invoke(window, "Reset_Click");
            await WaitFor(() => factory.Setups.Count >= 2);
            Assert.Equal(2, factory.Setups.Count);

            var nextWorld = factory.Setups.Last();
            Assert.NotSame(originalSetup, nextWorld);
            Assert.True(nextWorld.GlobalSettings.AdvancedMutation.Delta2Enabled);
            Assert.Equal(2.5, nextWorld.GlobalSettings.Costs.DnaUpkeep);
            Assert.True(nextWorld.GlobalSettings.AdvancedMutation.SaveAsTemporaryGene);
            window.Close();
        }, CancellationToken.None);
    }

    [Fact]
    public async Task HistoricMenuDispatchesRuntimeCommandsAndExposesUnavailableOnlineState()
    {
        var engine = new RecordingEngineClient();
        await using var simulation = new SimulationSession(engine);
        var headless = DesktopTestApplication.Session;

        await headless.Dispatch(async () =>
        {
            var window = RuntimeWindow(simulation);
            window.GetType().GetMethod("RefreshLegacyMenuStates", BindingFlags.Instance | BindingFlags.NonPublic)!
                .Invoke(window, null);
            var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
            var timer = Field<DispatcherTimer>(window, "_runTimer");
            var viewModel = Field<MainWindowViewModel>(window, "_viewModel");

            Invoke(window, "Run_Click");
            menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.FilePauseSimulation)!
                .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
            Assert.False(timer.IsEnabled);
            Assert.Equal("PAUSED", viewModel.Status);

            var vision = menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.ViewShowVisionGrid)!;
            Assert.True(vision.IsChecked);
            vision.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
            var viewport = window.FindControl<WorldViewport>("Viewport")!;
            Assert.False(viewport.ShowSelectedVision);

            menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.ViewDisplayMovementVectors)!
                .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
            menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.ViewDisplayShotImpacts)!
                .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
            menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.ViewDisplayResourceGuages)!
                .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
            menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.ObjectsHilightTeleporters)!
                .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
            menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EGridEnable)!
                .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
            menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EGridMedium)!
                .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
            Assert.True(viewport.ShowMovementVectors);
            Assert.False(viewport.ShowShotImpacts);
            Assert.True(viewport.ShowResourceGauges);
            Assert.True(viewport.HighlightTeleporters);
            Assert.True(viewport.EnergyGridEnabled);
            Assert.Equal(EnergyGridSize.Medium, viewport.EnergyGridSize);

            menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.ObjectsNewTeleporter)!
                .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
            Assert.Equal("Teleporter", Field<object>(window, "_environmentPlacement").ToString());

            menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.ObjectsDontDecayNrgShots)!
                .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
            menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.ObjectsDontDecayWasteShots)!
                .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
            menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.ObjectsDisableTies)!
                .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));

            menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.ObjectsHorizontalMaze)!
                .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));

            menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.RobotKill)!
                .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));

            var internet = menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.InternetEnabled)!;
            Assert.False(internet.IsEnabled);
            Assert.Contains("online", ToolTip.GetTip(internet)!.ToString()!, StringComparison.OrdinalIgnoreCase);
            await WaitFor(() =>
                engine.Calls.Contains("Remove:3:2")
                && engine.Calls.Count(call => call.StartsWith("AddObstacle:", StringComparison.Ordinal)) >= 4
                && engine.Calls.Count(call => call == "Environment") >= 2
                && engine.Calls.Contains("Ties:False"),
                () => string.Join(", ", engine.Calls));
            window.Close();
            return 0;
        }, CancellationToken.None);
    }

    [Fact]
    public async Task EveryHistoricMenuLeafIsPresentAndNoneUsesThePendingImplementationState()
    {
        var engine = new RecordingEngineClient();
        await using var simulation = new SimulationSession(engine);
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var window = RuntimeWindow(simulation);
            window.GetType().GetMethod("RefreshLegacyMenuStates", BindingFlags.Instance | BindingFlags.NonPublic)!.Invoke(window, null);
            var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
            foreach (var command in Enum.GetValues<Core.LegacyMenus.LegacyCommandId>())
            {
                var item = menu.FindCommandItem(command);
                Assert.NotNull(item);
                var reason = ToolTip.GetTip(item)?.ToString() ?? string.Empty;
                Assert.DoesNotContain("not implemented", reason, StringComparison.OrdinalIgnoreCase);
            }
            window.Close();
        }, CancellationToken.None);
    }

    [Fact]
    public async Task TransportSpeedDisplayPlacementAndManualSelectionControlsChangeLiveUiState()
    {
        var headless = DesktopTestApplication.Session;
        await headless.Dispatch(() =>
        {
            var window = new MainWindow([], new WorldSetupOptions { TicksPerUpdate = 1 });
            var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
            var timer = Field<DispatcherTimer>(window, "_runTimer");
            var viewport = window.FindControl<WorldViewport>("Viewport")!;

            Invoke(window, "Run_Click");
            Assert.True(timer.IsEnabled);
            Assert.Equal("RUNNING", viewModel.Status);
            Invoke(window, "Pause_Click");
            Assert.False(timer.IsEnabled);
            Assert.Equal("PAUSED", viewModel.Status);

            var speed = window.FindControl<ComboBox>("RuntimeSpeed")!;
            speed.SelectedIndex = 1;
            Assert.Equal(5U, Field<uint>(window, "_ticksPerUpdate"));
            speed.SelectedIndex = 2;
            Assert.Equal(20U, Field<uint>(window, "_ticksPerUpdate"));
            speed.SelectedIndex = 3;
            Assert.True(Field<bool>(window, "_maximumSpeed"));

            Invoke(window, "Turbo_Click");
            Assert.False(viewport.IsVisible);
            Assert.Equal("SHOW WORLD", window.FindControl<Button>("TurboButton")!.Content);
            Assert.Equal("TURBO · RENDERING SUSPENDED", viewModel.Status);
            Invoke(window, "Turbo_Click");
            Assert.True(viewport.IsVisible);
            Assert.Equal("TURBO", window.FindControl<Button>("TurboButton")!.Content);
            Invoke(window, "Pause_Click");

            window.FindControl<CheckBox>("RenderWasteToggle")!.IsChecked = false;
            Assert.False(viewport.RenderWaste);
            Assert.Equal("WASTE RENDERING OFF", viewModel.Status);
            window.FindControl<CheckBox>("ShowVisionToggle")!.IsChecked = false;
            Assert.False(viewport.ShowSelectedVision);

            Invoke(window, "AddObstacle_Click");
            Assert.Equal("Obstacle", Field<object>(window, "_environmentPlacement").ToString());
            Assert.Equal("OBSTACLE · CLICK FIRST CORNER", viewModel.Status);
            Invoke(window, "AddTeleporter_Click");
            Assert.Equal("Teleporter", Field<object>(window, "_environmentPlacement").ToString());
            Assert.Equal("TELEPORTER · CLICK SOURCE", viewModel.Status);

            viewModel.Update(RecordingEngineClient.CreateSnapshot());
            Invoke(window, "Move_Click");
            Assert.Equal(new OrganismKey(3, 2), Field<OrganismKey?>(window, "_moveOrganism"));
            Invoke(window, "Follow_Click");
            Assert.True(Field<bool>(window, "_following"));
            Assert.Equal("FOLLOWING 3:2", viewModel.Status);
            Invoke(window, "Follow_Click");
            Assert.False(Field<bool>(window, "_following"));
            Invoke(window, "AdvanceZerobot_Click");
            Assert.Equal("ZEROBOT PROGRESSION IS NOT ACTIVE IN THIS WORLD", viewModel.Status);
        }, CancellationToken.None);
    }

    [Fact]
    public async Task RuntimeButtonsDispatchTheExpectedEngineCommandsAndDnaTargets()
    {
        var engine = new RecordingEngineClient();
        await using var simulation = new SimulationSession(engine);
        var headless = DesktopTestApplication.Session;
        MainWindow? window = null;

        try
        {
            await headless.Dispatch(() =>
            {
                window = new MainWindow([], new WorldSetupOptions());
                SetField(window, "_session", simulation);
                Field<MainWindowViewModel>(window, "_viewModel").Update(RecordingEngineClient.CreateSnapshot());

                Invoke(window, "Step_Click");
                Invoke(window, "Clone_Click");
                Invoke(window, "Reproduce_Click");
                Invoke(window, "Kill_Click");
                Invoke(window, "SwitchBackend_Click");

                window.FindControl<NumericUpDown>("LiveMetabolism")!.Value = 7;
                window.FindControl<NumericUpDown>("LivePlantEnergy")!.Value = 55;
                window.FindControl<NumericUpDown>("LiveFeedingToBody")!.Value = 0.25m;
                Invoke(window, "ApplyEnergy_Click");
                window.FindControl<CheckBox>("ToroidalWorldToggle")!.IsChecked = false;

                var editor = new DnaEditorWindow(simulation, RecordingEngineClient.Organism);
                editor.FindControl<TextBox>("Editor")!.Text = "start\n10 .up store\nstop";
                Invoke(editor, "Apply_Click");
                Invoke(editor, "Clone_Click");
            }, CancellationToken.None);

            await WaitFor(() =>
                engine.Calls.Any(call => call.StartsWith("Tick:", StringComparison.Ordinal))
                && engine.Calls.Any(call => call.StartsWith("Clone:3:2", StringComparison.Ordinal))
                && engine.Calls.Any(call => call.StartsWith("Reproduce:3:2", StringComparison.Ordinal))
                && engine.Calls.Contains("Remove:3:2")
                && engine.Calls.Contains("Switch:Gpu")
                && engine.EnvironmentUpdates.Count >= 2
                && engine.DnaReplacements.Any(value => value.Slot == 3 && value.Generation == 2)
                && engine.DnaReplacements.Any(value => value.Slot == 40 && value.Generation == 0));

            Assert.Contains(engine.EnvironmentUpdates, update =>
                update.MetabolismCost == 7
                && update.Vegetation.MaxEnergyPerTick == 55
                && Math.Abs(update.Vegetation.FeedingToBody - 0.25f) < 0.001f);
            Assert.Contains(engine.EnvironmentUpdates, update => !update.ToroidalWorld);
        }
        finally
        {
            if (window is not null) await DesktopTestApplication.Session.Dispatch(() =>
            {
                SetField(window, "_session", null);
                window.Close();
                return Task.CompletedTask;
            }, CancellationToken.None);
        }
    }

    [Fact]
    public async Task ObstaclePlacementDispatchesTheDb2TwoCornerWorkflow()
    {
        var engine = new RecordingEngineClient();
        await using var simulation = new SimulationSession(engine);
        var headless = DesktopTestApplication.Session;

        await headless.Dispatch(() =>
        {
            var window = RuntimeWindow(simulation);
            var viewport = window.FindControl<WorldViewport>("Viewport")!;
            Invoke(window, "AddObstacle_Click");
            Raise<Action<float[]>>(viewport, "WorldClicked")([10f, 20f]);
            Raise<Action<float[]>>(viewport, "WorldClicked")([40f, 60f]);
        }, CancellationToken.None);
        await WaitFor(() => engine.Calls.Contains("AddObstacle:1"));
    }

    [Fact]
    public async Task SaveWithoutMutationsUsesTheMutationFreeEnginePath()
    {
        var engine = new RecordingEngineClient();
        await using var simulation = new SimulationSession(engine);
        var storage = new AuditStorageService();
        MainWindow? window = null;

        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                window = new MainWindow([], new WorldSetupOptions(), new AuditEngineClientFactory(), storage);
                SetField(window, "_session", simulation);
                Field<MainWindowViewModel>(window, "_viewModel").Update(RecordingEngineClient.CreateSnapshot());
                window.GetType().GetMethod("RefreshLegacyMenuStates", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .Invoke(window, null);
                var result = await (Task<Core.LegacyMenus.LegacyCommandResult>)window.GetType()
                    .GetMethod("SaveSimulationCommandAsync", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .Invoke(window, ["SAVED WITHOUT MUTATION SETTINGS", true])!;
                Assert.True(result.Succeeded, result.Status);
                Assert.Equal([9, 8, 7], storage.SavedSimulation);
                Assert.DoesNotContain("Save", engine.Calls);
            }, CancellationToken.None);
        }
        finally
        {
            if (window is not null) await DesktopTestApplication.Session.Dispatch(() =>
            {
                SetField(window, "_session", null);
                window.Close();
                return Task.CompletedTask;
            }, CancellationToken.None);
        }
    }

    [Fact]
    public async Task KillEntireOrganismRemovesExactlyTheCapturedTiedGroupOnce()
    {
        var first = RecordingEngineClient.Organism;
        var second = new OrganismSnapshot(4, 7, [140f, 200f], [0f, 0f], 900, 9, Species: 1);
        var unrelated = new OrganismSnapshot(8, 1, [800f, 600f], [0f, 0f], 700, 7, Species: 1);
        var snapshot = RecordingEngineClient.CreateSnapshot() with
        {
            Population = 3,
            Organisms = [first, second, unrelated],
            Ties = [new TieSnapshot(new OrganismKey(3, 2), new OrganismKey(4, 7), 40f)],
        };
        var engine = new RecordingEngineClient(snapshot);
        await using var simulation = new SimulationSession(engine);
        await simulation.StepAsync(0);
        MainWindowViewModel? selectedViewModel = null;
        MainWindow? window = null;

        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                window = new MainWindow([], new WorldSetupOptions());
                SetField(window, "_session", simulation);
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                selectedViewModel = viewModel;
                viewModel.Update(snapshot);
                viewModel.Select(3);
                window.FindControl<WorldViewport>("Viewport")!.SelectSlot(3);
                window.GetType().GetMethod("RefreshLegacyMenuStates", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .Invoke(window, null);
                var result = await (Task<Core.LegacyMenus.LegacyCommandResult>)window.GetType()
                    .GetMethod("KillSelectedOrganismAsync", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .Invoke(window, null)!;
                Assert.True(result.Succeeded, result.Status);
                Assert.Equal(["RemoveMany:3:2,4:7"], engine.Calls.Where(call => call.StartsWith("Remove", StringComparison.Ordinal)));
                Assert.Null(viewModel.SelectedSlot);
            }, CancellationToken.None);
        }
        finally
        {
            if (window is not null) await DesktopTestApplication.Session.Dispatch(() =>
            {
                SetField(window, "_session", null);
                window.Close();
                return Task.CompletedTask;
            }, CancellationToken.None);
        }
    }

    [Fact]
    public async Task KillEntireOrganismRejectsStaleSelectionInsteadOfDeletingReusedSlot()
    {
        var oldSnapshot = RecordingEngineClient.CreateSnapshot();
        var newer = RecordingEngineClient.Organism with { Generation = 3 };
        var newerSnapshot = oldSnapshot with { Organisms = [newer] };
        var engine = new RecordingEngineClient(newerSnapshot);
        await using var simulation = new SimulationSession(engine);
        await simulation.StepAsync(0);

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions());
            SetField(window, "_session", simulation);
            var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
            viewModel.Update(oldSnapshot);
            viewModel.Select(3);

            await (Task)window.GetType()
                .GetMethod("KillSelectedOrganismAsync", BindingFlags.Instance | BindingFlags.NonPublic)!
                .Invoke(window, null)!;
        }, CancellationToken.None);

        Assert.DoesNotContain(engine.Calls, call => call.StartsWith("Remove", StringComparison.Ordinal));
    }

    [Fact]
    public async Task SaveEntireOrganismMenuWritesTheExactTiedGroupDb2FrameThroughThePicker()
    {
        var selected = RecordingEngineClient.Organism;
        var tied = selected with { Slot = 4, Generation = 7 };
        var unrelated = selected with { Slot = 8, Generation = 1 };
        var snapshot = RecordingEngineClient.CreateSnapshot() with
        {
            Tick = 42,
            Population = 3,
            Organisms = [selected, tied, unrelated],
            Ties =
            [
                new(new OrganismKey(3, 2), new OrganismKey(4, 7), 40f),
                new(new OrganismKey(4, 7), new OrganismKey(99, 5), 60f),
            ],
        };
        var engine = new RecordingEngineClient(snapshot);
        engine.Tick(42);
        await using var simulation = new SimulationSession(engine);
        await simulation.StepAsync(0);
        var storage = new AuditStorageService();

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), new AuditEngineClientFactory(), storage);
            SimulationSession? windowSession = null;
            try
            {
                window.Show();
                windowSession = Field<SimulationSession>(window, "_session");
                SetField(window, "_session", simulation);
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                viewModel.Update(snapshot);
                viewModel.Select(3);
                window.FindControl<WorldViewport>("Viewport")!.SelectSlot(3);
                window.GetType().GetMethod("RefreshLegacyMenuStates", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .Invoke(window, null);

                window.FindControl<LegacyMenuBar>("HistoricMenuBar")!
                    .FindCommandItem(Core.LegacyMenus.LegacyCommandId.RobotSaveEntireOrganism)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));

                await WaitFor(() => storage.SavedOrganism is not null);
                Assert.Equal("organism-3.dbo", storage.SavedOrganismSuggestedFileName);
                var file = Db2OrganismFile.Decode(storage.SavedOrganism!);
                Assert.Equal(1, file.Version);
                Assert.Equal(42UL, file.Tick);

                var members = file.Members;
                Assert.Equal(2, members.Count);
                Assert.Equal((3U, 2U), (members[0].Slot, members[0].Generation));
                Assert.Equal((4U, 7U), (members[1].Slot, members[1].Generation));
                Assert.All(members, member => Assert.Equal("start\nstop", member.Dna));

                var tie = Assert.Single(file.Ties);
                Assert.Equal((3U, 2U), (tie.First.Slot, tie.First.Generation));
                Assert.Equal((4U, 7U), (tie.Second.Slot, tie.Second.Generation));
                var serializedSlots = members.Select(member => member.Slot)
                    .Concat(file.Ties.SelectMany(value => new[]
                    {
                        value.First.Slot,
                        value.Second.Slot,
                    }));
                Assert.DoesNotContain(8U, serializedSlots);
                Assert.DoesNotContain(99U, serializedSlots);
                Assert.Equal([(3U, 2U), (4U, 7U)], engine.DnaExports.ToArray());
            }
            finally
            {
                SetField(window, "_session", null);
                window.Close();
                if (windowSession is not null) await windowSession.DisposeAsync();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task TeleporterPlacementDispatchesTheDb2SourceDestinationWorkflow()
    {
        var engine = new RecordingEngineClient();
        await using var simulation = new SimulationSession(engine);
        var headless = DesktopTestApplication.Session;

        await headless.Dispatch(() =>
        {
            var window = RuntimeWindow(simulation);
            var viewport = window.FindControl<WorldViewport>("Viewport")!;
            Invoke(window, "AddTeleporter_Click");
            Raise<Action<float[]>>(viewport, "WorldClicked")([100f, 200f]);
            Raise<Action<float[]>>(viewport, "WorldClicked")([300f, 400f]);
        }, CancellationToken.None);
        await WaitFor(() => engine.Calls.Contains("AddTeleporter:1"));
    }

    [Fact]
    public async Task RemoveMoveAndDragDispatchStableOrganismAndFeatureIds()
    {
        var engine = new RecordingEngineClient();
        await using var simulation = new SimulationSession(engine);
        var headless = DesktopTestApplication.Session;

        await headless.Dispatch(() =>
        {
            var window = RuntimeWindow(simulation);
            SetField(window, "_selectedFeature", new WorldFeatureSelection(WorldFeatureKind.Obstacle, 1));
            Invoke(window, "RemoveFeature_Click");
            Invoke(window, "Move_Click");
            var viewport = window.FindControl<WorldViewport>("Viewport")!;
            Raise<Action<float[]>>(viewport, "WorldClicked")([500f, 600f]);
            Raise<Action<uint, float[]>>(viewport, "OrganismDragCompleted")(3, [700f, 800f]);
        }, CancellationToken.None);
        await WaitFor(() =>
            engine.Calls.Contains("RemoveObstacle:1")
            && engine.Calls.Contains("Move:3:2:500:600")
            && engine.Calls.Contains("Move:3:2:700:800"),
            () => string.Join(", ", engine.Calls));
    }

    [Fact]
    public async Task SnapshotRobotAndViewportRoutesUseLiveNativeState()
    {
        var snapshot = RecordingEngineClient.CreateSnapshot() with
        {
            Tick = 42,
            Corpses =
            [
                new CorpseSnapshot([4f, 5f], [0f, 0f], 0, 12, 3, 1)
                {
                    Dna = "start\nstop",
                    UniqueId = 9001,
                    Generation = 4,
                    DnaLength = 2,
                },
                new CorpseSnapshot([6f, 7f], [0f, 0f], 0, 12, 3, 1, Repopulating: true)
                {
                    Dna = "repopulating",
                    UniqueId = 9002,
                    Generation = 4,
                    DnaLength = 1,
                },
            ],
        };
        var engine = new RecordingEngineClient(snapshot);
        await using var simulation = new SimulationSession(engine);
        var storage = new AuditStorageService();

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), new AuditEngineClientFactory(), storage);
            try
            {
                SetField(window, "_session", simulation);
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                viewModel.Update(snapshot);
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                viewport.SetSnapshot(snapshot);
                viewport.SelectKey(new OrganismKey(3, 2));
                Invoke(window, "RefreshLegacyMenuStates");
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;

                Assert.NotNull(window.FindControl<Button>("ZoomInButton"));
                Assert.NotNull(window.FindControl<Button>("ZoomOutButton"));
                Assert.NotNull(window.FindControl<Button>("ZoomLockButton"));
                Assert.NotNull(viewport.ContextMenu);
                Assert.NotNull(typeof(WorldViewport).GetProperty("Zoom"));
                Assert.NotNull(typeof(WorldViewport).GetProperty("ZoomLocked"));
                Assert.NotNull(typeof(WorldViewport).GetEvent("OrganismDoubleClicked"));
                Assert.NotNull(typeof(MainWindow).GetMethod("RecordDeadSnapshotAsync", BindingFlags.Instance | BindingFlags.NonPublic));

                menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.RecordingSnapshotLiving)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => storage.SavedLivingSnapshot is not null);
                var living = Encoding.UTF8.GetString(storage.SavedLivingSnapshot!);
                Assert.StartsWith(Db2GenomeRecordFile.Header + "\r\n", living);
                Assert.Contains("start\nstop", living, StringComparison.Ordinal);

                menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.RobotShowInfo)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<RobotDetailsWindow>().Any());
                window.OwnedWindows.OfType<RobotDetailsWindow>().Single().Close();

                menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.RobotOpenConsole)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<RobotConsoleWindow>().Any());
                window.OwnedWindows.OfType<RobotConsoleWindow>().Single().Close();

                menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.RobotShowPhilogeny)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<PhylogenyWindow>().Any());
                window.OwnedWindows.OfType<PhylogenyWindow>().Single().Close();

                SetField(window, "_deadRecordingEnabled", true);
                SetField(window, "_excludeRepopulatingDead", true);
                var recorder = typeof(MainWindow).GetMethod("RecordDeadSnapshotAsync", BindingFlags.Instance | BindingFlags.NonPublic)!;
                await (Task)recorder.Invoke(window, [snapshot])!;
                var dead = Encoding.UTF8.GetString(storage.DeadSnapshotWrites.Single());
                Assert.StartsWith(Db2GenomeRecordFile.Header + "\r\n", dead);
                Assert.Contains("9001", dead, StringComparison.Ordinal);
                Assert.DoesNotContain("9002", dead, StringComparison.Ordinal);
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                SetField(window, "_session", null);
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static MainWindow RuntimeWindow(SimulationSession simulation)
    {
        var window = new MainWindow([], new WorldSetupOptions());
        SetField(window, "_session", simulation);
        Field<MainWindowViewModel>(window, "_viewModel").Update(RecordingEngineClient.CreateSnapshot());
        return window;
    }

    private static void Invoke(object target, string method, object? sender = null) =>
        target.GetType().GetMethod(method, BindingFlags.Instance | BindingFlags.NonPublic)!
            .Invoke(target, [sender, new RoutedEventArgs()]);

    private static T Field<T>(object target, string name) =>
        (T)target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(target)!;

    private static void SetField(object target, string name, object? value) =>
        target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)!.SetValue(target, value);

    private static T Raise<T>(object target, string eventName) where T : Delegate =>
        (T)target.GetType().GetField(eventName, BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(target)!;

    private static async Task WaitFor(Func<bool> condition, Func<string>? details = null)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(5);
        while (!condition() && DateTime.UtcNow < deadline) await Task.Delay(10);
        Assert.True(condition(), $"Timed out waiting for the serialized UI command batch. Calls: {details?.Invoke()}");
    }
}

internal sealed class SetupCapturingEngineClientFactory : IEngineClientFactory
{
    public List<WorldSetupOptions> Setups { get; } = [];

    public IEngineClient Create(WorldSetupOptions setup)
    {
        Setups.Add(setup);
        return new AuditEngineClient();
    }
}

internal sealed class RecordingEngineClient : IEngineClient
{
    public static readonly OrganismSnapshot Organism = new(
        3, 2, [100f, 200f], [0f, 0f], 1_000, 10, Species: 1);

    public ConcurrentQueue<string> Calls { get; } = [];
    public ConcurrentQueue<EnvironmentUpdate> EnvironmentUpdates { get; } = [];
    public ConcurrentQueue<MutationSettingsUpdate> MutationUpdates { get; } = [];
    public ConcurrentQueue<LocalModeConfig> LocalModeConfigs { get; } = [];
    public ConcurrentQueue<(uint Slot, uint Generation, string Dna)> DnaReplacements { get; } = [];
    public ConcurrentQueue<(uint Slot, uint Generation)> DnaExports { get; } = [];
    public string Backend { get; private set; } = "CPU";
    private ulong _tick;
    private readonly EngineSnapshot _snapshot;

    public RecordingEngineClient(EngineSnapshot? snapshot = null)
    {
        _snapshot = snapshot ?? CreateSnapshot();
    }

    public static EngineSnapshot CreateSnapshot() => new(0, 1, "CPU", [Organism])
    {
        Species =
        [
            new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false),
            new SpeciesSnapshot("Animal Minimalis", false, 0xff239ac0, 0, false),
        ],
    };

    public EngineCapabilities Capabilities() => new(1, Backend, true, null);
    public void Tick(uint count = 1) { _tick += count; Calls.Enqueue($"Tick:{count}"); }
    public DnaImportReport ImportDna(string dna, float[]? position = null) { Calls.Enqueue("ImportDna"); return new([]); }
    public DnaImportReport ImportSpecies(SpeciesImport species) { Calls.Enqueue("ImportSpecies"); return new([]); }
    public DnaImportReport InsertSpeciesAt(SpeciesImport species, float[] position)
    {
        Calls.Enqueue($"InsertSpeciesAt:{species.Name}:{position[0]:0}:{position[1]:0}");
        return DnaImportReport.Compatible;
    }
    public void Remove(uint slot, uint generation) => Calls.Enqueue($"Remove:{slot}:{generation}");
    public void RemoveMany(IReadOnlyList<OrganismKey> organisms) => Calls.Enqueue($"RemoveMany:{string.Join(',', organisms.Select(value => $"{value.Slot}:{value.Generation}"))}");
    public void Move(uint slot, uint generation, float[] position) => Calls.Enqueue($"Move:{slot}:{generation}:{position[0]:0}:{position[1]:0}");
    public OrganismKey CloneOrganism(uint slot, uint generation, float[] position)
    {
        Calls.Enqueue($"Clone:{slot}:{generation}");
        return new OrganismKey(40, 0);
    }
    public void ReplaceDna(uint slot, uint generation, string dna)
    {
        Calls.Enqueue($"ReplaceDna:{slot}:{generation}");
        DnaReplacements.Enqueue((slot, generation, dna));
    }
    public string ExportDna(uint slot, uint generation)
    {
        DnaExports.Enqueue((slot, generation));
        return "start\nstop";
    }
    public OrganismKey Reproduce(uint firstSlot, uint firstGeneration, uint? secondSlot, uint? secondGeneration, float[] position)
    {
        Calls.Enqueue($"Reproduce:{firstSlot}:{firstGeneration}:{secondSlot?.ToString() ?? "none"}");
        return new OrganismKey(41, 0);
    }
    public void SwitchBackend(string backend) { Backend = backend.ToUpperInvariant(); Calls.Enqueue($"Switch:{backend}"); }
    public void AddObstacle(ObstacleSnapshot obstacle) => Calls.Enqueue($"AddObstacle:{obstacle.Id}");
    public void UpdateObstacle(ObstacleSnapshot obstacle) => Calls.Enqueue($"UpdateObstacle:{obstacle.Id}");
    public void RemoveObstacle(uint id) => Calls.Enqueue($"RemoveObstacle:{id}");
    public void AddTeleporter(TeleporterSnapshot teleporter) => Calls.Enqueue($"AddTeleporter:{teleporter.Id}");
    public void RemoveTeleporter(uint id) => Calls.Enqueue($"RemoveTeleporter:{id}");
    public void AddFileTeleporter(FileTeleporterSnapshot teleporter) => Calls.Enqueue($"AddFileTeleporter:{teleporter.Id}");
    public void UpdateFileTeleporter(FileTeleporterSnapshot teleporter) => Calls.Enqueue($"UpdateFileTeleporter:{teleporter.Id}");
    public void RemoveFileTeleporter(uint id) => Calls.Enqueue($"RemoveFileTeleporter:{id}");
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
        LocalModeConfigs.Enqueue(config);
        Calls.Enqueue("ConfigureLocalMode");
    }
    public void StartLocalMode() => Calls.Enqueue("StartLocalMode");
    public void RestartLocalMode() => Calls.Enqueue("RestartLocalMode");
    public void AdjudicateLocalRound(uint? winner, string reason) => Calls.Enqueue($"AdjudicateLocalRound:{winner?.ToString() ?? "none"}");
    public void SetOrganismAim(uint slot, uint generation, int aim) => Calls.Enqueue($"Aim:{slot}:{generation}:{aim}");
    public byte[] Save() { Calls.Enqueue("Save"); return BitConverter.GetBytes(_tick); }
    public byte[] SaveWithoutMutations() { Calls.Enqueue("SaveWithoutMutations"); return [9, 8, 7]; }
    public void Load(byte[] save) => _tick = BitConverter.ToUInt64(save);
    public EngineSnapshot Snapshot()
    {
        return _snapshot with { Tick = _tick, Backend = Backend };
    }
    public void Dispose() { }
}
