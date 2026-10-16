using System.Collections.Concurrent;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.LogicalTree;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using System.Text.Json;
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
                DesktopTestApplication.UseSession(window, simulation);
                window.Show();
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
    public async Task RestrictionOptionsRouteRendersEveryOptionAndAppliesOnlyAfterApply()
    {
        var initial = new RestrictionOptions(
            new RestrictionColumn(KillMultibot: true, DisableChloroplasts: true, DisableDna: true, VirusImmune: true, DisableMovement: true),
            new RestrictionColumn(KillDisqualified: true, FixedInPlace: true, DisableReproduction: true, DisableMutations: true));
        var applied = new RestrictionOptions(
            new RestrictionColumn(KillDisqualified: true, FixedInPlace: true, DisableVision: true, DisableReproduction: true, DisableMutations: true),
            new RestrictionColumn(KillMultibot: true, DisableChloroplasts: true, DisableDna: true, VirusImmune: true, DisableMovement: true));
        var engine = new RecordingEngineClient(RecordingEngineClient.CreateSnapshot() with { RestrictionOptions = initial });
        await using var simulation = new SimulationSession(engine);
        await simulation.StepAsync(0);

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), new AuditEngineClientFactory(), new AuditStorageService());
            DesktopTestApplication.UseSession(window, simulation);
            window.Show();
            Field<MainWindowViewModel>(window, "_viewModel").Update(simulation.LatestSnapshot);
            Invoke(window, "RefreshLegacyMenuStates");
            try
            {
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var command = menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditRestrictionOverwrites)!;
                command.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<RestrictionOptionsWindow>().Any());
                var cancelDialog = Assert.Single(window.OwnedWindows.OfType<RestrictionOptionsWindow>());
                cancelDialog.Width = 760;
                cancelDialog.Height = 560;
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                var checks = cancelDialog.GetVisualDescendants().OfType<CheckBox>().ToArray();
                Assert.Equal(20, checks.Length);
                Assert.Equal(
                    [
                        "kill multibot", "kill disqualified", "disable chloroplasts", "fixed in place", "disable vision",
                        "disable DNA", "disable reproduction", "virus immune", "disable mutations", "disable movement sysvars",
                        "kill multibot", "kill disqualified", "disable chloroplasts", "fixed in place", "disable vision",
                        "disable DNA", "disable reproduction", "virus immune", "disable mutations", "disable movement sysvars",
                    ],
                    checks.Select(check => check.Content?.ToString()).ToArray());
                Assert.Equal(
                    [true, false, true, false, false, true, false, true, false, true, false, true, false, true, false, false, true, false, true, false],
                    checks.Select(check => check.IsChecked == true).ToArray());
                Assert.Equal(initial, cancelDialog.Options);
                Assert.Equal(["LOAD", "SAVE", "CANCEL", "APPLY"],
                    cancelDialog.GetVisualDescendants().OfType<Button>().Where(button => button is not CheckBox).Select(button => button.Content?.ToString()).ToArray());
                using (var frame = cancelDialog.CaptureRenderedFrame())
                {
                    Assert.True(frame.PixelSize.Width >= 640);
                    Assert.True(frame.PixelSize.Height >= 460);
                    var pixelRect = new PixelRect(0, 0, frame.PixelSize.Width, frame.PixelSize.Height);
                    var stride = pixelRect.Width * 4;
                    var pixelBytes = new byte[stride * pixelRect.Height];
                    var pixelBuffer = Marshal.AllocHGlobal(pixelBytes.Length);
                    try
                    {
                        frame.CopyPixels(pixelRect, pixelBuffer, pixelBytes.Length, stride);
                        Marshal.Copy(pixelBuffer, pixelBytes, 0, pixelBytes.Length);
                    }
                    finally { Marshal.FreeHGlobal(pixelBuffer); }
                    Assert.True(ContainsApproximateColor(pixelBytes, 0xF4, 0xF0, 0xE6));
                    Assert.True(ContainsApproximateColor(pixelBytes, 0x41, 0x6E, 0x35));
                }

                checks[0].IsChecked = false;
                checks[19].IsChecked = true;
                var cancel = cancelDialog.GetVisualDescendants().OfType<Button>().Single(button => button.Content?.ToString() == "CANCEL");
                cancel.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => !cancelDialog.IsVisible);
                Assert.False(cancelDialog.Accepted);
                Assert.Equal(initial, simulation.LatestSnapshot.RestrictionOptions);
                Assert.Null(engine.RestrictionOptionsUpdate);

                command.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<RestrictionOptionsWindow>().Any());
                var applyDialog = Assert.Single(window.OwnedWindows.OfType<RestrictionOptionsWindow>());
                var applyChecks = applyDialog.GetVisualDescendants().OfType<CheckBox>().ToArray();
                var appliedValues = new[] { false, true, false, true, true, false, true, false, true, false, true, false, true, false, false, true, false, true, false, true };
                for (var index = 0; index < applyChecks.Length; index++) applyChecks[index].IsChecked = appliedValues[index];
                Assert.Equal(applied, applyDialog.Options);
                var apply = applyDialog.GetVisualDescendants().OfType<Button>().Single(button => button.Content?.ToString() == "APPLY");
                apply.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => engine.RestrictionOptionsUpdate is not null);
                Assert.True(applyDialog.Accepted);
                Assert.Equal(applied, engine.RestrictionOptionsUpdate);
                Assert.Equal(applied, simulation.LatestSnapshot.RestrictionOptions);
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                SetField(window, "_session", null);
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
            Assert.False(viewport.ShowMovementVectors);
            Assert.False(viewport.ShowShotImpacts);
            Assert.False(viewport.ShowResourceGauges);
            Assert.False(viewport.HighlightTeleporters);
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
    public async Task DisplayOnlyBackgroundAndOverlayTogglesLeaveCanonicalSessionSaveUnchanged()
    {
        var engine = new RecordingEngineClient();
        await using var simulation = new SimulationSession(engine);
        await simulation.StepAsync(0);
        var beforeSnapshot = simulation.LatestSnapshot;
        var beforeSave = await simulation.SaveAsync();

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = RuntimeWindow(simulation);
            try
            {
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                viewport.SetSnapshot(beforeSnapshot);
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;

                using var backgroundStream = new MemoryStream(Convert.FromBase64String(
                    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII="));
                var background = new Avalonia.Media.Imaging.Bitmap(backgroundStream);
                viewport.SetBackgroundImage(background);
                SetField(window, "_hasBackgroundImage", true);
                Assert.True(viewport.HasBackgroundImage);
                Assert.True(viewport.OverlayProjection.HasBackgroundImage);
                window.GetType().GetMethod("RefreshLegacyMenuStates", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .Invoke(window, null);

                void InvokeMenu(Core.LegacyMenus.LegacyCommandId command) =>
                    menu.FindCommandItem(command)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));

                window.FindControl<CheckBox>("RenderWasteToggle")!.IsChecked = false;
                window.FindControl<CheckBox>("ShowVisionToggle")!.IsChecked = false;
                InvokeMenu(Core.LegacyMenus.LegacyCommandId.ViewShowVisionGrid);
                InvokeMenu(Core.LegacyMenus.LegacyCommandId.ViewDisplayMovementVectors);
                InvokeMenu(Core.LegacyMenus.LegacyCommandId.ViewDisplayShotImpacts);
                InvokeMenu(Core.LegacyMenus.LegacyCommandId.ViewDisplayResourceGuages);
                InvokeMenu(Core.LegacyMenus.LegacyCommandId.ObjectsHilightTeleporters);
                InvokeMenu(Core.LegacyMenus.LegacyCommandId.EGridEnable);
                InvokeMenu(Core.LegacyMenus.LegacyCommandId.EGridMedium);
                window.FindControl<Button>("ToolbarSlot11")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                window.FindControl<Button>("ToolbarSlot12")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                window.FindControl<Button>("ToolbarSlot13")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                window.FindControl<Button>("ToolbarSlot14")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                InvokeMenu(Core.LegacyMenus.LegacyCommandId.ViewRemoveBackground);

                Assert.False(viewport.HasBackgroundImage);
                Assert.True(viewport.ShowSelectedVision);
                Assert.False(viewport.ShowMovementVectors);
                Assert.False(viewport.ShowShotImpacts);
                Assert.False(viewport.ShowResourceGauges);
                Assert.False(viewport.HighlightTeleporters);
                Assert.True(viewport.EnergyGridEnabled);
                Assert.Equal(EnergyGridSize.Medium, viewport.EnergyGridSize);
                Assert.False(viewport.RenderWaste);
                Assert.True(viewport.FlickerMode);
                Assert.False(viewport.RenderSkins);
                Assert.False(viewport.ShowDeathPoffs);
                Assert.False(viewport.IsVisible);

                var afterSave = await simulation.SaveAsync();
                var afterSnapshot = simulation.LatestSnapshot;
                Assert.Equal(beforeSave, afterSave);
                Assert.Equal(beforeSnapshot.Tick, afterSnapshot.Tick);
                Assert.Equal(beforeSnapshot.Population, afterSnapshot.Population);
                Assert.Equal(beforeSnapshot.Backend, afterSnapshot.Backend);
                Assert.Equal(beforeSnapshot.Organisms, afterSnapshot.Organisms);
                Assert.Equal(["Tick:0", "Save", "Save"], engine.Calls.ToArray());
                return 0;
            }
            finally
            {
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task HistoricSettingsRoutesKeepDistinctWorkflowsPublishPhysicsAndLeaveOnlineControlsInert()
    {
        var engine = new RecordingEngineClient();
        await using var simulation = new SimulationSession(engine);

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = RuntimeWindow(simulation);
            window.Show();
            try
            {
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var general = menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditGeneralSettings)!;
                general.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any());
                var generalDialog = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                Assert.Equal(1, generalDialog.GetVisualDescendants().OfType<TabControl>().Single().SelectedIndex);
                generalDialog.Close();

                var physics = menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditPhysicsAndCosts)!;
                physics.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any());
                var physicsDialog = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                Assert.Equal(2, physicsDialog.GetVisualDescendants().OfType<TabControl>().Single().SelectedIndex);

                physicsDialog.FindControl<ComboBox>("PhysicsPreset")!.SelectedIndex = (int)Db2PhysicsPreset.Custom;
                physicsDialog.FindControl<Button>("PhysicsHelpButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                var physicsHelp = physicsDialog.FindControl<TextBlock>("PhysicsHelpText")!.Text!;
                Assert.Contains("Mass", physicsHelp, StringComparison.Ordinal);
                Assert.Contains("Bang", physicsHelp, StringComparison.Ordinal);
                Assert.NotNull(physicsDialog.FindControl<CheckBox>("ZeroMomentum"));
                physicsDialog.FindControl<NumericUpDown>("MaxVelocity")!.Value = 360;
                physicsDialog.FindControl<NumericUpDown>("MovementEfficiency")!.Value = 0.72m;
                physicsDialog.FindControl<NumericUpDown>("GravityX")!.Value = 7;
                physicsDialog.FindControl<NumericUpDown>("GravityY")!.Value = -3;
                physicsDialog.FindControl<NumericUpDown>("SurfaceGravity")!.Value = 3;
                physicsDialog.FindControl<NumericUpDown>("StaticFriction")!.Value = 0.8m;
                physicsDialog.FindControl<NumericUpDown>("KineticFriction")!.Value = 0.5m;
                physicsDialog.FindControl<NumericUpDown>("Density")!.Value = 0.0001m;
                physicsDialog.FindControl<NumericUpDown>("Viscosity")!.Value = 0.0002m;
                physicsDialog.FindControl<NumericUpDown>("Elasticity")!.Value = 1.5m;
                physicsDialog.FindControl<NumericUpDown>("Drag")!.Value = 0.25m;
                physicsDialog.FindControl<NumericUpDown>("Brownian")!.Value = 75;
                physicsDialog.FindControl<CheckBox>("PlanetEaters")!.IsChecked = true;
                physicsDialog.FindControl<NumericUpDown>("PlanetEatersG")!.Value = 2.5m;
                physicsDialog.FindControl<Button>("ApplyButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

                await WaitFor(() => engine.EnvironmentUpdates.Count == 1, () => string.Join(", ", engine.Calls));
                var update = engine.EnvironmentUpdates.Single();
                Assert.Equal([7f, -3f], update.Gravity);
                Assert.Equal(0.25f, update.Drag);
                Assert.Equal(0.5f, update.BrownianMotion);
                Assert.Equal(new Db2PhysicsOptions(360f, 0.72f, 3f, 0.8f, 0.5f, 0.0001d, 0.0002d, 1.5f, true, 2.5f), update.Physics);
                Assert.Contains("\"zero_momentum\":false", NativeCommandSerializer.SerializeEnvironment(update), StringComparison.Ordinal);

                var timer = Field<DispatcherTimer>(window, "_runTimer");
                window.FindControl<Button>("ToolbarSlot09")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.InternetEnabled)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                Assert.False(menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.InternetEnabled)!.IsEnabled);
                Assert.Equal(TimeSpan.FromMilliseconds(67), timer.Interval);
                Assert.DoesNotContain("Internet", engine.Calls);
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
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
                DesktopTestApplication.UseSession(window, simulation);
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
    public async Task LoadedConfiguredModePopulatesItsControlsWithoutAdvancingTheWorld()
    {
        var path = Path.Combine(Path.GetTempPath(), $"db24832-loaded-mode-{Guid.NewGuid():N}.db3s");
        LocalModeConfig expected;
        using (var engine = new NativeEngineClient(new WorldSetupOptions { Backend = "Cpu" }))
        {
            engine.ConfigureLocalMode(LocalModeConfig.Default with
            {
                Kind = LocalModeKind.Contest, Preset = LeaguePreset.F2,
                MinimumRounds = 2, MaximumRounds = 7, MaximumCycles = 123,
                MaximumPopulation = 37, RestartOnExtinction = false,
                SourceSpecies = 0, SelectionPopulationWeight = 0.25f,
                SelectionEnergyWeight = 0.75f, EnergyHandicap = 25,
                Restrictions = new LocalModeRestrictions(ProhibitViruses: true),
            });
            expected = Assert.IsType<LocalModeConfig>(engine.Snapshot().LocalMode.Configuration);
            File.WriteAllBytes(path, engine.Save());
        }
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var window = new MainWindow([], new WorldSetupOptions { Backend = "Cpu", LoadSavePath = path },
                    NativeEngineClientFactory.Instance, new AuditStorageService());
                SimulationSession? session = null;
                try
                {
                    window.Show();
                    await WaitFor(() => Field<MainWindowViewModel>(window, "_viewModel").Status == "LOADED");
                    session = Field<SimulationSession>(window, "_session");
                    Assert.Equal((int)expected.Kind, window.FindControl<ComboBox>("LocalModeKindSelector")!.SelectedIndex);
                    Assert.Equal((int)expected.Preset, window.FindControl<ComboBox>("LocalModePreset")!.SelectedIndex);
                    Assert.Equal((decimal)expected.MinimumRounds, window.FindControl<NumericUpDown>("ModeMinimumRounds")!.Value);
                    Assert.Equal((decimal)expected.MaximumRounds, window.FindControl<NumericUpDown>("ModeMaximumRounds")!.Value);
                    Assert.Equal((decimal)expected.MaximumCycles, window.FindControl<NumericUpDown>("ModeMaximumCycles")!.Value);
                    Assert.Equal((decimal)expected.MaximumPopulation, window.FindControl<NumericUpDown>("ModePopulationCap")!.Value);
                    Assert.Equal(expected.RestartOnExtinction, window.FindControl<CheckBox>("ModeRestartOnExtinction")!.IsChecked);
                    Assert.Equal(expected.Restrictions.ProhibitViruses, window.FindControl<CheckBox>("ModeProhibitViruses")!.IsChecked);
                    Assert.Equal(JsonSerializer.Serialize(expected), JsonSerializer.Serialize(Field<MainWindowViewModel>(window, "_viewModel").LocalMode.BuildConfiguration()));
                    Assert.Equal(0ul, session.LatestSnapshot.Tick);
                }
                finally
                {
                    SetField(window, "_session", null);
                    window.Close();
                    if (session is not null) await session.DisposeAsync();
                }
            }, CancellationToken.None);
        }
        finally { File.Delete(path); }
    }

    [Fact]
    public async Task LoadedNativeWorldHydratesControlsAndChangingTopologyPreservesItsRules()
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        const string relative = "modern/engine/tests/fixtures/db24832-original-user.sim";
        while (directory is not null && !File.Exists(Path.Combine(directory.FullName, relative))) directory = directory.Parent;
        Assert.NotNull(directory);
        var path = Path.Combine(directory.FullName, relative);
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var setup = new WorldSetupOptions { Backend = "Cpu", WorldWidth = 1000, WorldHeight = 1000, LoadSavePath = path };
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService());
            SimulationSession? session = null;
            try
            {
                window.Show();
                await WaitFor(() => Field<MainWindowViewModel>(window, "_viewModel").Status == "LOADED");
                session = Field<SimulationSession>(window, "_session");
                Assert.Equal(281ul, session.LatestSnapshot.Tick);
                Assert.Equal(196, session.LatestSnapshot.Population);
                Assert.False(window.FindControl<CheckBox>("ToroidalWorldToggle")!.IsChecked);
                var species = Field<System.Collections.ObjectModel.ObservableCollection<SpeciesImport>>(window, "_insertionSpecies");
                Assert.Equal(2, species.Count);
                Assert.All(species, value => Assert.False(string.IsNullOrWhiteSpace(value.Dna)));
                var before = await session.ReadConfigurationAsync();
                Assert.NotNull(before);
                var globals = Field<GlobalSettings>(window, "_globalSettings");
                Assert.Equal(before.Costs, globals.Costs);
                Assert.Equal(before.AdvancedMutation, globals.AdvancedMutation);
                Assert.Equal(before.BodyCutoff, globals.Main.BodyCutoff);
                Assert.Equal(before.GreedyReproduction, globals.Main.GreedyEnforcement);
                window.FindControl<CheckBox>("ToroidalWorldToggle")!.IsChecked = true;
                await WaitFor(() => Field<EnvironmentUpdate>(window, "_liveEnvironment").ToroidalWorld);
                var after = await session.ReadConfigurationAsync();
                Assert.NotNull(after);
                Assert.True(after.Environment.ToroidalWorld);
                Assert.Equal(before.Environment.Physics, after.Environment.Physics);
                Assert.Equal(before.Environment.Shots, after.Environment.Shots);
                Assert.Equal(before.Environment.Vegetation, after.Environment.Vegetation);
                Assert.Equal(before.Environment.CorpseDecay, after.Environment.CorpseDecay);
                Assert.Equal(before.GlobalCostMultiplier, after.GlobalCostMultiplier);
                Assert.Equal(281ul, session.LatestSnapshot.Tick);
            }
            finally
            {
                SetField(window, "_session", null);
                window.Close();
                if (session is not null) await session.DisposeAsync();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task SaveWithoutMutationsRenderedPlayerRoutePreservesLiveWorldAndDisablesOnlySavedMutationState()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            PopulationCapacity = 64,
            VegetablePopulationCap = 0,
            BrownianMotion = 0f,
            Species =
            [
                new SpeciesImport(
                    "IO02 Species",
                    "start\nstop",
                    false,
                    0xff239ac0,
                    1,
                    1_000,
                    0,
                    false,
                    3.5f),
            ],
        };
        var storage = new SaveRouteStorage();
        MainWindow? window = null;
        SimulationSession? session = null;

        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                window = new MainWindow([], setup, NativeEngineClientFactory.Instance, storage)
                {
                    WindowState = WindowState.Normal,
                    Width = 1_280,
                    Height = 900,
                };
                window.Show();

                await WaitFor(
                    () => window.GetType().GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window)
                        is SimulationSession candidate
                        && candidate.LatestSnapshot.Organisms.Count == 1
                        && Field<MainWindowViewModel>(window, "_viewModel").Status.Contains("WORLD READY", StringComparison.Ordinal),
                    () => "The native IO-02 player route did not publish its live organism.");

                session = Field<SimulationSession>(window, "_session");
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                var liveOrganism = Assert.Single(session.LatestSnapshot.Organisms);
                var mutationSettings = new MutationSettingsUpdate(
                    true,
                    2.5f,
                    [new SpeciesMutationRateUpdate(liveOrganism.Species, 3.5f)]);
                await session.UpdateMutationSettingsAsync(mutationSettings);
                await session.UpdateOrganismMutationProfileAsync(
                    liveOrganism.Slot,
                    liveOrganism.Generation,
                    MutationProfileUpdate.Default with
                    {
                        Point = new MutationRateUpdate(1_250, 3, 1),
                        Delta = new MutationRateUpdate(2_000, 500, 150),
                    });
                await session.StepAsync(0);
                await WaitFor(
                    () => session.LatestSnapshot.MutationSettings.Enabled
                        && Math.Abs(session.LatestSnapshot.MutationSettings.GlobalMultiplier - 2.5f) < 0.001f
                        && Math.Abs(Assert.Single(session.LatestSnapshot.Species, species => species.Name == "IO02 Species").MutationRate - 3.5f) < 0.001f
                        && Math.Abs(Assert.Single(session.LatestSnapshot.Organisms).MutationProfile.Point.OneIn - 1_250) < 0.001,
                    () => "The live mutation settings were not published before the save.");

                var liveBeforeSave = session.LatestSnapshot;
                var liveBeforeOrganism = Assert.Single(liveBeforeSave.Organisms);
                var saveMenu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!
                    .FindCommandItem(Core.LegacyMenus.LegacyCommandId.FileSaveWithoutMutations)!;
                Assert.Equal("Save Sim Without Mutations", saveMenu.Header?.ToString());
                Assert.True(saveMenu.IsVisible);
                Assert.True(saveMenu.IsEnabled);

                storage.AcceptSave = false;
                saveMenu.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(
                    () => storage.SaveRequests == 1 && viewModel.Status == "CANCELED",
                    () => $"Picker cancel did not reach the rendered status surface. Requests: {storage.SaveRequests}; Status: {viewModel.Status}");
                Assert.Null(storage.SavedSimulation);

                storage.AcceptSave = true;
                saveMenu.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(
                    () => storage.SaveRequests == 2
                        && storage.SavedSimulation is not null
                        && viewModel.Status == "SAVED WITHOUT MUTATION SETTINGS",
                    () => $"Picker success did not reach the rendered status surface. Requests: {storage.SaveRequests}; Status: {viewModel.Status}");

                var savedBytes = storage.SavedSimulation!;
                Assert.Equal("DB3S", Encoding.ASCII.GetString(savedBytes, 0, 4));
                Assert.Equal((ushort)4, BitConverter.ToUInt16(savedBytes, 4));
                Assert.Equal(savedBytes.Length - 10, BitConverter.ToInt32(savedBytes, 6));

                using (var payload = JsonDocument.Parse(savedBytes.AsMemory(10)))
                {
                    var root = payload.RootElement;
                    Assert.False(root.GetProperty("mutations_enabled").GetBoolean());
                    Assert.Equal(0f, root.GetProperty("global_mutation_multiplier").GetSingle());
                    Assert.All(
                        root.GetProperty("species").EnumerateArray(),
                        species => Assert.Equal(0f, species.GetProperty("mutation_rate").GetSingle()));
                    var savedSlot = Assert.Single(
                        root.GetProperty("slots").EnumerateArray(),
                        slot => slot.GetProperty("organism").ValueKind != JsonValueKind.Null);
                    Assert.False(
                        savedSlot.GetProperty("organism")
                            .GetProperty("flags")
                            .GetProperty("can_mutate")
                            .GetBoolean());
                }

                var liveAfterSave = session.LatestSnapshot;
                Assert.Equal(liveBeforeSave.Tick, liveAfterSave.Tick);
                Assert.Equal(liveBeforeSave.WorldSize, liveAfterSave.WorldSize);
                Assert.Equal(liveBeforeSave.MutationSettings, liveAfterSave.MutationSettings);
                Assert.Equal(liveBeforeSave.Species, liveAfterSave.Species);
                var liveAfterOrganism = Assert.Single(liveAfterSave.Organisms);
                Assert.Equal(liveBeforeOrganism.Slot, liveAfterOrganism.Slot);
                Assert.Equal(liveBeforeOrganism.Generation, liveAfterOrganism.Generation);
                Assert.Equal(liveBeforeOrganism.Position, liveAfterOrganism.Position);
                Assert.Equal(liveBeforeOrganism.Velocity, liveAfterOrganism.Velocity);
                Assert.Equal(liveBeforeOrganism.Energy, liveAfterOrganism.Energy);
                Assert.Equal(liveBeforeOrganism.MutationProfile, liveAfterOrganism.MutationProfile);

                using (var restored = new NativeEngineClient(setup with { Species = [] }))
                {
                    restored.Load(savedBytes);
                    var savedSnapshot = restored.Snapshot();
                    Assert.Equal(liveBeforeSave.Tick, savedSnapshot.Tick);
                    Assert.Equal(liveBeforeSave.WorldSize, savedSnapshot.WorldSize);
                    var savedOrganism = Assert.Single(savedSnapshot.Organisms);
                    Assert.Equal(liveBeforeOrganism.Slot, savedOrganism.Slot);
                    Assert.Equal(liveBeforeOrganism.Generation, savedOrganism.Generation);
                    Assert.Equal(liveBeforeOrganism.Position, savedOrganism.Position);
                    Assert.Equal(liveBeforeOrganism.Velocity, savedOrganism.Velocity);
                    Assert.Equal(liveBeforeOrganism.Energy, savedOrganism.Energy);
                    Assert.Equal(liveBeforeOrganism.MutationProfile, savedOrganism.MutationProfile);
                    Assert.False(savedSnapshot.MutationSettings.Enabled);
                    Assert.Equal(0f, savedSnapshot.MutationSettings.GlobalMultiplier);
                    Assert.All(savedSnapshot.Species, species => Assert.Equal(0f, species.MutationRate));
                }

                window.UpdateLayout();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var statusInstrument = window.FindControl<ItemsControl>("StatusInstrument")!;
                Assert.True(menu.Bounds.Width >= 1_000, $"Menu width was {menu.Bounds.Width}.");
                Assert.True(menu.Bounds.Height >= 20, $"Menu height was {menu.Bounds.Height}.");
                Assert.True(statusInstrument.IsVisible);
                Assert.True(statusInstrument.Bounds.Width >= 1_000, $"Status width was {statusInstrument.Bounds.Width}.");
                Assert.True(statusInstrument.Bounds.Height >= 20, $"Status height was {statusInstrument.Bounds.Height}.");
                var surfaceText = string.Join(
                    "\n",
                    window.GetVisualDescendants().OfType<TextBlock>()
                        .Select(block => block.Text)
                        .Where(text => !string.IsNullOrWhiteSpace(text)));
                Assert.Contains("SAVED WITHOUT MUTATION SETTINGS", surfaceText, StringComparison.Ordinal);
                Assert.Contains("SAVED WITHOUT MUTATION SETTINGS", viewModel.Status, StringComparison.Ordinal);

                using var frame = Assert.IsAssignableFrom<Avalonia.Media.Imaging.Bitmap>(window.CaptureRenderedFrame());
                Assert.True(frame.PixelSize.Width >= 1_000, $"Rendered width was {frame.PixelSize.Width}.");
                Assert.True(frame.PixelSize.Height >= 700, $"Rendered height was {frame.PixelSize.Height}.");
                var pixelRect = new PixelRect(0, 0, frame.PixelSize.Width, frame.PixelSize.Height);
                var stride = pixelRect.Width * 4;
                var pixelBytes = new byte[stride * pixelRect.Height];
                var pixelBuffer = Marshal.AllocHGlobal(pixelBytes.Length);
                try
                {
                    frame.CopyPixels(pixelRect, pixelBuffer, pixelBytes.Length, stride);
                    Marshal.Copy(pixelBuffer, pixelBytes, 0, pixelBytes.Length);
                }
                finally
                {
                    Marshal.FreeHGlobal(pixelBuffer);
                }
                Assert.True(ContainsApproximateColor(pixelBytes, 0xDC, 0xD8, 0xCD));
            }, CancellationToken.None);
        }
        finally
        {
            if (window is not null) await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var ownedSession = session
                    ?? window.GetType().GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window) as SimulationSession;
                SetField(window, "_session", null);
                window.Close();
                if (ownedSession is not null) await ownedSession.DisposeAsync();
            }, CancellationToken.None);
        }
    }

    [Fact]
    public async Task SaveLoadRenderedPlayerRouteRestoresLiveStateAndRejectsInvalidArtifactAtomically()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            PopulationCapacity = 64,
            VegetablePopulationCap = 0,
            BrownianMotion = 0f,
            Species =
            [
                new SpeciesImport(
                    "IO01 Species",
                    "start\nstop",
                    false,
                    0xff239ac0,
                    1,
                    1_000,
                    0,
                    false,
                    1f),
            ],
        };
        var storage = new SaveRouteStorage { AcceptSave = true };
        MainWindow? window = null;
        SimulationSession? session = null;

        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                window = new MainWindow([], setup, NativeEngineClientFactory.Instance, storage)
                {
                    WindowState = WindowState.Normal,
                    Width = 1_280,
                    Height = 900,
                };
                window.Show();

                await WaitFor(
                    () => window.GetType().GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window)
                        is SimulationSession candidate
                        && candidate.LatestSnapshot.Organisms.Count == 1
                        && Field<MainWindowViewModel>(window, "_viewModel").Status.Contains("WORLD READY", StringComparison.Ordinal),
                    () => "The native IO-01 player route did not publish its live organism.");

                session = Field<SimulationSession>(window, "_session");
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                var beforeSave = session.LatestSnapshot;

                window.FindControl<Button>("ToolbarSlot03")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(
                    () => storage.SavedSimulation is not null && viewModel.Status == "SAVED",
                    () => $"Save did not reach the rendered status surface. Requests: {storage.SaveRequests}; Status: {viewModel.Status}");
                var savedBytes = storage.SavedSimulation!;
                Assert.Equal("DB3S", Encoding.ASCII.GetString(savedBytes, 0, 4));

                await session.StepAsync(1);
                await WaitFor(
                    () => session.LatestSnapshot.Tick != beforeSave.Tick,
                    () => "The live session did not mutate after the save.");

                storage.OpenedSimulation = savedBytes;
                window.FindControl<Button>("ToolbarSlot02")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(
                    () => viewModel.Status == "LOADED" && session.LatestSnapshot.Tick == beforeSave.Tick,
                    () => $"Load did not restore the saved state. Tick: {session.LatestSnapshot.Tick}; Status: {viewModel.Status}");
                Assert.Equal(beforeSave.Population, session.LatestSnapshot.Population);
                Assert.Equal(JsonSerializer.Serialize(beforeSave.Organisms), JsonSerializer.Serialize(session.LatestSnapshot.Organisms));

                var stable = session.LatestSnapshot;
                storage.OpenedSimulation = [1, 2, 3];
                var invalid = await storage.OpenSimulationAsync(window);
                await Assert.ThrowsAnyAsync<Exception>(() => session.LoadAsync(invalid!));
                Assert.Equal(stable.Tick, session.LatestSnapshot.Tick);
                Assert.Equal(stable.Population, session.LatestSnapshot.Population);
                Assert.Equal(JsonSerializer.Serialize(stable.Organisms), JsonSerializer.Serialize(session.LatestSnapshot.Organisms));

                window.UpdateLayout();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                var surfaceText = string.Join(
                    "\n",
                    window.GetVisualDescendants().OfType<TextBlock>()
                        .Select(block => block.Text)
                        .Where(text => !string.IsNullOrWhiteSpace(text)));
                Assert.Contains("LOADED", surfaceText, StringComparison.Ordinal);
                using var frame = window.CaptureRenderedFrame();
                using var rendered = new MemoryStream();
                frame.Save(rendered, new Avalonia.Media.Imaging.PngBitmapEncoderOptions());
                var png = rendered.ToArray();
                Assert.True(png.AsSpan(0, 8).SequenceEqual(new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 }));
            }, CancellationToken.None);
        }
        finally
        {
            if (window is not null) await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var ownedSession = session
                    ?? window.GetType().GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window) as SimulationSession;
                SetField(window, "_session", null);
                window.Close();
                if (ownedSession is not null) await ownedSession.DisposeAsync();
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
                DesktopTestApplication.UseSession(window, simulation);
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
            DesktopTestApplication.UseSession(window, simulation);
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
    public async Task SaveEntireOrganismMenuWritesTheCapturedEngineGroupThroughThePicker()
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
                DesktopTestApplication.UseSession(window, simulation);
                window.Show();
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
                Assert.Equal(engine.ExportedOrganismPayload, storage.SavedOrganism);
                Assert.Equal([new OrganismKey(3, 2), new OrganismKey(4, 7)], Assert.Single(engine.ExportedGroups));
                Assert.Empty(engine.DnaExports);

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
    public async Task ImportAndSaveSelectedOrganismRoundTripsThroughNativeStorageRoute()
    {
        var fixture = SyntheticVb6Organism();
        var (storage, storageState) = OrganismRouteStorage.Create(fixture);
        var setup = new WorldSetupOptions
        {
            CorpseDecay = new Db2CorpseDecayOptions(Mode: Db2CorpseDecayMode.None),
        };
        var engine = NativeEngineClientFactory.Instance.Create(setup);
        await using var simulation = new SimulationSession(engine);
        await simulation.StepAsync(0);

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, storage);
            SimulationSession? windowSession = null;
            try
            {
                DesktopTestApplication.UseSession(window, simulation);
                window.Show();
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                window.FindControl<LegacyMenuBar>("HistoricMenuBar")!
                    .FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditInsertOrganism)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));

                await WaitFor(() => simulation.LatestSnapshot.Organisms.Any(organism =>
                    organism.Velocity.SequenceEqual([1.25f, -2.5f])
                    && organism.Energy == 777
                    && organism.Age == 91
                    && organism.Body == 321), () => viewModel.Status);
                var imported = Assert.Single(simulation.LatestSnapshot.Organisms, organism =>
                    organism.Velocity.SequenceEqual([1.25f, -2.5f])
                    && organism.Energy == 777
                    && organism.Age == 91
                    && organism.Body == 321);
                viewModel.Update(simulation.LatestSnapshot);
                viewModel.Select(imported.Slot);
                window.FindControl<WorldViewport>("Viewport")!.SelectSlot(imported.Slot);
                Assert.InRange(imported.Position[0], 60f, setup.WorldWidth - 60f);
                Assert.InRange(imported.Position[1], 60f, setup.WorldHeight - 60f);
                engine.SetOrganismEnergy(imported.Slot, imported.Generation, 1234);
                await simulation.StepAsync(0);
                Assert.Equal(1234, Assert.Single(simulation.LatestSnapshot.Organisms,
                    organism => organism.Slot == imported.Slot).Energy);
                window.GetType().GetMethod("RefreshLegacyMenuStates", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .Invoke(window, null);

                window.FindControl<LegacyMenuBar>("HistoricMenuBar")!
                    .FindCommandItem(Core.LegacyMenus.LegacyCommandId.RobotSaveEntireOrganism)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));

                await WaitFor(() => storageState.SavedOrganism is not null);
                var exported = ReadNativeOrganismGroup(storageState.SavedOrganism!);
                var member = Assert.Single(exported.Organisms);
                Assert.Equal(imported.Position, ReadNativeSelectedPosition(storageState.SavedOrganism!));
                Assert.Equal([1.25f, -2.5f], member.Velocity);
                Assert.Equal(1234, member.Energy);
                Assert.Equal(91UL, member.Age);
                Assert.Equal(321, member.Body);

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
    public async Task RenderedWholeOrganismRoundTripPreservesSelectedLiveNativeMember()
    {
        var fixture = new Db2OrganismFile(
            1,
            42,
            [
                new Db2OrganismMember(
                    1,
                    0,
                    "start\nstop",
                    0,
                    false,
                    false,
                    false,
                    [100f, 200f],
                    [1.5f, -0.5f],
                    875,
                    19,
                    7,
                    321),
                new Db2OrganismMember(
                    2,
                    0,
                    "start\nstop",
                    0,
                    false,
                    false,
                    false,
                    [138f, 226f],
                    [-0.5f, 1f],
                    650,
                    23,
                    9,
                    222),
            ],
            [new Db2OrganismTie(
                new Db2OrganismKey(1, 0),
                new Db2OrganismKey(2, 0),
                42.5f)]).Encode();
        var (storage, storageState) = OrganismRouteStorage.Create(fixture);
        var setup = new WorldSetupOptions
        {
            CorpseDecay = new Db2CorpseDecayOptions(Mode: Db2CorpseDecayMode.None),
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, storage)
            {
                WindowState = WindowState.Normal,
                Width = 1280,
                Height = 900,
            };
            try
            {
                window.Show();
                await WaitFor(() => typeof(MainWindow)
                    .GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .GetValue(window) is SimulationSession);
                var session = Field<SimulationSession>(window, "_session");
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                await WaitFor(() => menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditInsertOrganism)!.IsEnabled);

                menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditInsertOrganism)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => viewModel.Population == 2, () => viewModel.Status);

                var group = session.LatestSnapshot.Organisms
                    .Where(organism => organism.Position.SequenceEqual([100f, 200f])
                        || organism.Position.SequenceEqual([138f, 226f]))
                    .ToArray();
                Assert.Equal(2, group.Length);
                var originalOrganismKeys = group
                    .Select(organism => new OrganismKey(organism.Slot, organism.Generation))
                    .ToHashSet();
                var selected = group[^1];
                var selectedKey = new OrganismKey(selected.Slot, selected.Generation);
                viewModel.Select(selectedKey);
                window.FindControl<WorldViewport>("Viewport")!.SelectKey(selectedKey);
                Invoke(window, "RefreshLegacyMenuStates");
                Assert.Equal($"{selected.Slot}:{selected.Generation}", viewModel.SelectedId);
                Assert.Equal(selected.Energy, viewModel.SelectedEnergy);
                Assert.Equal(selected.Body, viewModel.SelectedBody);

                menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.RobotSaveEntireOrganism)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => storageState.SavedOrganism is not null);
                var exported = ReadNativeOrganismGroup(storageState.SavedOrganism!);
                Assert.Equal(2, exported.Organisms.Count);
                Assert.Equal(42.5f, Assert.Single(exported.Ties).RestLength);
                Assert.Equal(selected.Position, ReadNativeSelectedPosition(storageState.SavedOrganism!));

                static (float X, float Y, int Energy, ulong Age, ulong BirthCycle, int Body)[] RelativeState(
                    IReadOnlyList<OrganismSnapshot> organisms, OrganismSnapshot anchor) => organisms
                    .Select(robot => (robot.Position[0] - anchor.Position[0], robot.Position[1] - anchor.Position[1],
                        robot.Energy, robot.Age, robot.BirthCycle, robot.Body))
                    .OrderBy(value => value.Item1).ToArray();
                var expectedState = RelativeState(group, selected);
                Assert.Equal(expectedState, RelativeState(exported.Organisms, exported.Organisms[0]));

                storageState.SetFixture(storageState.SavedOrganism!);
                menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditInsertOrganism)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => session.LatestSnapshot.Organisms.Count == 4);
                await WaitFor(() => viewModel.Status == "IMPORTED ORGANISM · 2 BOT(S)");

                var roundTripped = session.LatestSnapshot.Organisms
                    .Where(robot => !originalOrganismKeys.Contains(new OrganismKey(robot.Slot, robot.Generation)))
                    .ToArray();
                Assert.Equal(2, roundTripped.Length);
                var insertedAnchor = Assert.Single(roundTripped, robot => robot.Energy == selected.Energy && robot.Age == selected.Age);
                Assert.InRange(insertedAnchor.Position[0], 60f, setup.WorldWidth - 60f);
                Assert.InRange(insertedAnchor.Position[1], 60f, setup.WorldHeight - 60f);
                Assert.Equal(expectedState, RelativeState(roundTripped, insertedAnchor));
                Assert.Equal($"{selected.Slot}:{selected.Generation}", viewModel.SelectedId);
                Assert.Equal(selected.Energy, viewModel.SelectedEnergy);
                Assert.Equal(selected.Body, viewModel.SelectedBody);

                using var frame = window.CaptureRenderedFrame();
                using var stream = new MemoryStream();
                frame.Save(stream, new Avalonia.Media.Imaging.PngBitmapEncoderOptions());
                var renderedPng = stream.ToArray();
                var pngSignature = new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 };
                Assert.True(renderedPng.Length >= 1024, $"Rendered frame was only {renderedPng.Length} bytes.");
                Assert.True(renderedPng.AsSpan(0, pngSignature.Length).SequenceEqual(pngSignature), "Rendered frame is not a PNG.");
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task MainWindowSelectsNonFirstTiedMemberHighlightsGroupAndSavesIt()
    {
        var first = RecordingEngineClient.Organism with { Position = [80f, 100f] };
        var second = RecordingEngineClient.Organism with
        {
            Slot = 4,
            Generation = 7,
            Position = [260f, 180f],
        };
        var firstKey = new OrganismKey(first.Slot, first.Generation);
        var secondKey = new OrganismKey(second.Slot, second.Generation);
        var snapshot = RecordingEngineClient.CreateSnapshot() with
        {
            Tick = 42,
            Population = 2,
            WorldSize = [400f, 300f],
            Organisms = [first, second],
            Ties = [new TieSnapshot(firstKey, secondKey, 42.5f)],
        };
        var engine = new RecordingEngineClient(snapshot);
        engine.Tick(42);
        await using var simulation = new SimulationSession(engine);
        await simulation.StepAsync(0);
        var storage = new AuditStorageService();

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow(
                [],
                new WorldSetupOptions { WorldWidth = 400, WorldHeight = 300 },
                new AuditEngineClientFactory(),
                storage)
            {
                WindowState = WindowState.Normal,
                Width = 1280,
                Height = 900,
            };
            SimulationSession? windowSession = null;
            try
            {
                DesktopTestApplication.UseSession(window, simulation);
                window.Show();

                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                viewModel.Update(snapshot);
                viewport.SetSnapshot(snapshot);
                viewModel.Select(secondKey);
                viewport.SelectKey(secondKey);
                Invoke(window, "RefreshLegacyMenuStates");
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                using var beforeFrame = window.CaptureRenderedFrame();
                using var beforeStream = new MemoryStream();
                beforeFrame.Save(beforeStream, new Avalonia.Media.Imaging.PngBitmapEncoderOptions());
                var beforePng = beforeStream.ToArray();

                menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.RobotSelectEntireOrganism)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => viewModel.Status == "ORGANISM SELECTED · 2 BOT(S)");

                Assert.Equal(
                    new[] { firstKey, secondKey },
                    Field<HashSet<OrganismKey>>(viewport, "_selectedKeys")
                        .OrderBy(value => value.Slot)
                        .ToArray());

                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var afterFrame = window.CaptureRenderedFrame();
                using var afterStream = new MemoryStream();
                afterFrame.Save(afterStream, new Avalonia.Media.Imaging.PngBitmapEncoderOptions());
                var afterPng = afterStream.ToArray();
                var pngSignature = new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 };
                Assert.True(beforePng.Length >= 1024, $"Rendered frame was only {beforePng.Length} bytes.");
                Assert.True(afterPng.Length >= 1024, $"Rendered frame was only {afterPng.Length} bytes.");
                Assert.True(afterPng.AsSpan(0, pngSignature.Length).SequenceEqual(pngSignature), "Rendered frame is not a PNG.");
                Assert.False(afterPng.AsSpan().SequenceEqual(beforePng), "Selecting the tied group did not change the rendered frame.");

                menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.RobotSaveEntireOrganism)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => storage.SavedOrganism is not null);
                Assert.Equal(engine.ExportedOrganismPayload, storage.SavedOrganism);
                Assert.Equal([secondKey, firstKey], Assert.Single(engine.ExportedGroups));
                Assert.Empty(engine.DnaExports);

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

        await headless.Dispatch(async () =>
        {
            var window = RuntimeWindow(simulation);
            window.Show();
            try
            {
                SetField(window, "_selectedFeature", new WorldFeatureSelection(WorldFeatureKind.Obstacle, 1));
                Invoke(window, "RemoveFeature_Click");
                await WaitFor(() => window.OwnedWindows.OfType<DeleteConfirmationWindow>().Any());
                var confirmation = window.OwnedWindows.OfType<DeleteConfirmationWindow>().Single();
                confirmation.FindControl<Button>("ConfirmButton")!
                    .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => engine.Calls.Contains("RemoveObstacle:1"));
                Invoke(window, "Move_Click");
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                Raise<Action<float[]>>(viewport, "WorldClicked")([500f, 600f]);
                Raise<Action<uint, float[]>>(viewport, "OrganismDragCompleted")(3, [700f, 800f]);
                await WaitFor(() =>
                    engine.Calls.Contains("Move:3:2:500:600")
                    && engine.Calls.Contains("Move:3:2:700:800"),
                    () => string.Join(", ", engine.Calls));
            }
            finally
            {
                foreach (var owned in window.OwnedWindows.ToArray()) owned.Close();
                SetField(window, "_session", null);
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task SnapshotRobotAndViewportRoutesUseLiveNativeState()
    {
        var snapshot = RecordingEngineClient.CreateSnapshot() with
        {
            Tick = 42,
            Species = [new("Unassigned", false, 0xff112233, 0, false), new("Café", false, 0xff445566, 0, false)],
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
                DesktopTestApplication.UseSession(window, simulation);
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                viewModel.Update(snapshot);
                window.Show();
                viewModel.Select(new OrganismKey(3, 2));
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
                Assert.Contains((byte)0xe9, storage.SavedLivingSnapshot!);
                var living = LegacyDnaText.Decode(storage.SavedLivingSnapshot!);
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
                Assert.Contains((byte)0xe9, storage.DeadSnapshotWrites.Single());
                var dead = LegacyDnaText.Decode(storage.DeadSnapshotWrites.Single());
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
        DesktopTestApplication.UseSession(window, simulation);
        Field<MainWindowViewModel>(window, "_viewModel").Update(RecordingEngineClient.CreateSnapshot());
        return window;
    }

    private static void Invoke(object target, string method, object? sender = null)
    {
        var handler = target.GetType().GetMethod(method, BindingFlags.Instance | BindingFlags.NonPublic)!;
        handler.Invoke(target, handler.GetParameters().Length == 0 ? null : [sender, new RoutedEventArgs()]);
    }

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

    private static bool ContainsApproximateColor(byte[] pixels, byte red, byte green, byte blue)
    {
        for (var index = 0; index + 3 < pixels.Length; index += 4)
        {
            var bgra = Close(pixels[index], blue)
                && Close(pixels[index + 1], green)
                && Close(pixels[index + 2], red);
            var rgba = Close(pixels[index], red)
                && Close(pixels[index + 1], green)
                && Close(pixels[index + 2], blue);
            if (bgra || rgba) return true;
        }

        return false;

        static bool Close(byte left, byte right) => Math.Abs(left - right) <= 24;
    }

    private static float[] ReadNativeSelectedPosition(byte[] bytes) =>
        // Native .dbo count, vegetable/wall/fixed words, then Single x/y.
        [BitConverter.ToSingle(bytes, 8), BitConverter.ToSingle(bytes, 12)];

    private static EngineSnapshot ReadNativeOrganismGroup(byte[] bytes)
    {
        Assert.InRange(BitConverter.ToInt16(bytes, 0), 1, 50);
        Assert.NotEqual("DB2O", Encoding.ASCII.GetString(bytes, 0, 4));
        using var engine = new NativeEngineClient(backend: "Cpu");
        engine.ImportOrganism(bytes);
        return engine.Snapshot();
    }

    private static byte[] SyntheticVb6Organism()
    {
        using var stream = new MemoryStream();
        using var writer = new BinaryWriter(stream, Encoding.UTF8, leaveOpen: true);
        writer.Write((short)1);
        foreach (var value in new short[] { 1, 0, 0 }) writer.Write(value);
        foreach (var value in new[] { 12.5f, 34.5f, 1.25f, -2.5f, 0f, 0f, 0f }) writer.Write(value);
        for (var tie = 0; tie < 11; tie++)
        {
            writer.Write((short)1);
            writer.Write((short)(tie == 0 ? 7 : 0));
            writer.Write((short)1);
            writer.Write(0f);
            writer.Write(0f);
            writer.Write((short)0);
            writer.Write(42);
            writer.Write(0);
            for (var value = 0; value < 7; value++) writer.Write((short)0);
        }
        writer.Write(777f);
        for (var value = 0; value < 50; value++)
        {
            writer.Write((short)0);
            writer.Write((short)0);
        }
        writer.Write((short)0);
        for (var value = 0; value <= 1000; value++) writer.Write((short)0);
        writer.Write((short)2);
        writer.Write((short)2);
        writer.Write((short)1);
        writer.Write((short)10);
        writer.Write((short)1);
        for (var value = 0; value < 21; value++) writer.Write(0f);
        foreach (var value in new short[] { 3, 4, 5 }) writer.Write(value);
        writer.Write(6);
        writer.Write(91);
        writer.Write(44);
        foreach (var value in new short[] { 1, 2, 2 }) writer.Write(value);
        for (var value = 0; value < 14; value++) writer.Write((short)0);
        writer.Write(0);
        writer.Write(321f);
        writer.Write(0f);
        writer.Write((short)0);
        for (var value = 0; value < 4; value++) writer.Write(0f);
        writer.Write((short)0);
        writer.Write((short)-1);
        writer.Write((short)0);
        WriteText(writer, "");
        WriteText(writer, "");
        writer.Write((short)0);
        writer.Write((short)0);
        for (var value = 0; value < 21; value++)
        {
            writer.Write(0f);
            writer.Write(0f);
        }
        for (var value = 0; value < 4; value++) writer.Write((short)0);
        writer.Write((short)7);
        for (var value = 0; value < 4; value++) writer.Write((short)0);
        writer.Write(0f);
        writer.Write(0f);
        for (var value = 0; value < 5; value++) writer.Write((short)0);
        for (var value = 0; value < 501; value++)
        {
            for (var component = 0; component < 3; component++) writer.Write(0);
        }
        writer.Write(0);
        writer.Write(0);
        writer.Write((short)0);
        for (var value = 0; value < 11; value++)
        {
            writer.Write((byte)0);
            writer.Write(0f);
            writer.Write(0f);
            writer.Write(0f);
        }
        writer.Write(0f);
        writer.Write(0f);
        for (var value = 0; value < 15; value++) writer.Write((short)0);
        writer.Write(new byte[50]);
        writer.Write((short)0);
        writer.Write((short)0);
        writer.Write((byte)0);
        writer.Write((byte)0);
        writer.Write((byte)0);
        writer.Write(0);
        writer.Write(0f);
        writer.Write(0f);
        writer.Write(new byte[] { 254, 254, 254 });
        writer.Flush();
        return stream.ToArray();
    }

    private static void WriteText(BinaryWriter writer, string value)
    {
        writer.Write((short)value.Length);
        writer.Write(Encoding.UTF8.GetBytes(value));
    }

    private class OrganismRouteStorage : DispatchProxy
    {
        private byte[] _fixture = [];

        public byte[]? SavedOrganism { get; private set; }

        public static (IDesktopStorageService Storage, OrganismRouteStorage State) Create(byte[] fixture)
        {
            var storage = DispatchProxy.Create<IDesktopStorageService, OrganismRouteStorage>();
            var state = (OrganismRouteStorage)(object)storage;
            state._fixture = fixture;
            return (storage, state);
        }

        public void SetFixture(byte[] fixture) => _fixture = fixture;

        protected override object? Invoke(MethodInfo? targetMethod, object?[]? args)
        {
            if (targetMethod is null) throw new MissingMethodException();
            if (targetMethod.Name == nameof(IDesktopStorageService.OpenOrganismAsync))
                return Task.FromResult<byte[]?>(_fixture);
            if (targetMethod.Name == nameof(IDesktopStorageService.SaveOrganismAsync))
                return SaveAsync((Func<Stream, Task>)args![2]!);
            if (targetMethod.ReturnType == typeof(Task)) return Task.CompletedTask;
            if (targetMethod.ReturnType.IsGenericType
                && targetMethod.ReturnType.GetGenericTypeDefinition() == typeof(Task<>))
            {
                var resultType = targetMethod.ReturnType.GetGenericArguments()[0];
                return typeof(Task).GetMethod(nameof(Task.FromResult))!
                    .MakeGenericMethod(resultType)
                    .Invoke(null, [resultType.IsValueType ? Activator.CreateInstance(resultType) : null]);
            }
            return targetMethod.ReturnType.IsValueType
                ? Activator.CreateInstance(targetMethod.ReturnType)
                : null;
        }

        private async Task<bool> SaveAsync(Func<Stream, Task> writeAsync)
        {
            await using var stream = new MemoryStream();
            await writeAsync(stream);
            SavedOrganism = stream.ToArray();
            return true;
        }
    }

    private sealed class SaveRouteStorage : IDesktopStorageService
    {
        public bool AcceptSave { get; set; }
        public int SaveRequests { get; private set; }
        public byte[]? SavedSimulation { get; private set; }
        public byte[]? OpenedSimulation { get; set; }
        public int OpenRequests { get; private set; }

        public Task<IReadOnlyList<DesktopTextFile>> OpenDnaFilesAsync(TopLevel owner) =>
            Task.FromResult<IReadOnlyList<DesktopTextFile>>([]);

        public Task<DesktopSettings?> OpenSettingsAsync(TopLevel owner) =>
            Task.FromResult<DesktopSettings?>(null);

        public Task<string?> PickSimulationPathAsync(TopLevel owner) =>
            Task.FromResult<string?>(null);

        public Task<byte[]?> OpenSimulationAsync(TopLevel owner)
        {
            OpenRequests++;
            return Task.FromResult(OpenedSimulation);
        }

        public Task<bool> SaveSettingsAsync(TopLevel owner, DesktopSettings settings) =>
            Task.FromResult(false);

        public Task<bool> SaveSimulationAsync(TopLevel owner, ReadOnlyMemory<byte> save)
        {
            SaveRequests++;
            if (AcceptSave) SavedSimulation = save.ToArray();
            return Task.FromResult(AcceptSave);
        }

        public MutationProfileUpdate? SavedDnaMutationProfile { get; private set; }
        public Task<bool> SaveDnaAsync(TopLevel owner, string suggestedFileName, string dna, MutationProfileUpdate? mutationProfile)
        {
            SavedDnaMutationProfile = mutationProfile;
            return Task.FromResult(false);
        }

        public Task<bool> SaveOrganismAsync(
            TopLevel owner,
            string suggestedFileName,
            Func<Stream, Task> writeAsync) => Task.FromResult(false);

        public string? FindLatestAutosave() => null;

        public Task SaveAutosaveAsync(ReadOnlyMemory<byte> save) => Task.CompletedTask;
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
    public ConcurrentQueue<OrganismKey[]> ExportedGroups { get; } = [];
    public byte[] ExportedOrganismPayload { get; } = Encoding.ASCII.GetBytes("engine-owned organism fixture");
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
    public byte[] ExportOrganismGroup(IReadOnlyList<OrganismKey> organisms)
    {
        ExportedGroups.Enqueue(organisms.ToArray());
        return ExportedOrganismPayload.ToArray();
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
    public void UpdateTeleporter(TeleporterSnapshot teleporter) => Calls.Enqueue($"UpdateTeleporter:{teleporter.Id}");
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
    public RestrictionOptions? RestrictionOptionsUpdate { get; private set; }
    public void UpdateRestrictionOptions(RestrictionOptions options)
    {
        RestrictionOptionsUpdate = options;
        Calls.Enqueue("RestrictionOptions");
    }
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
        return _snapshot with
        {
            Tick = _tick,
            Backend = Backend,
            RestrictionOptions = RestrictionOptionsUpdate ?? _snapshot.RestrictionOptions,
        };
    }
    public void Dispose() { }
}
