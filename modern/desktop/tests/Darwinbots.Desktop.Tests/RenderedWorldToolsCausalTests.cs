using System.Reflection;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class RenderedWorldToolsCausalTests
{
    [Fact]
    public async Task HistoricShapeDeletionUsesThePausedNativeWorldWithoutConfirmation()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu", Seed = 95, PopulationCapacity = 64,
            WorldWidth = 4_000, WorldHeight = 2_000,
        };
        uint[] ids = [78, 3, 240, 19, 700, 54, 7, 168, 42, 81, 130, 13, 502, 22];
        using var fixture = new NativeEngineClient(setup);
        for (var index = 0; index < ids.Length; index++)
        {
            var left = 100 + index * 260;
            fixture.AddObstacle(new ObstacleSnapshot(ids[index], [left, 500], [left + 180, 700]));
        }
        var save = fixture.Save();
        using var reference = new NativeEngineClient(setup);
        reference.Load(save);
        Assert.Equal(10, reference.RemoveNativeRandomObstacles());
        var survivors = reference.Snapshot().Obstacles.Select(shape => shape.Id).ToArray();
        Assert.Equal(ids.Where(survivors.Contains), survivors);
        var removedId = ids.First(id => !survivors.Contains(id));
        var storage = new AuditStorageService { OpenedSimulation = save };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, storage)
            {
                WindowState = WindowState.Normal,
                Width = 1_200,
                Height = 800,
            };
            window.Show();
            try
            {
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var load = menu.FindCommandItem(LegacyCommandId.FileLoadSimulation)!;
                await WaitFor(() => load.IsEnabled, () => "The simulation did not finish starting.");
                var session = TryField<SimulationSession>(window, "_session")!;
                var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                load.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => viewModel.Status == "LOADED" && session.LatestSnapshot.Obstacles.Count == ids.Length,
                    () => "The saved shapes were not loaded.");
                var before = session.LatestSnapshot;
                Assert.Equal(ids, before.Obstacles.Select(shape => shape.Id));

                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                var editShape = window.FindControl<Button>("EditShapeButton")!;
                var scaleX = viewport.Bounds.Width / before.WorldSize[0];
                var scaleY = viewport.Bounds.Height / before.WorldSize[1];
                if (viewport.FixedRatio) scaleX = scaleY = Math.Min(scaleX, scaleY);
                async Task SelectShape(uint id)
                {
                    var shape = Assert.Single(session.LatestSnapshot.Obstacles, value => value.Id == id);
                    var point = viewport.TranslatePoint(new Point(
                        (shape.Minimum[0] + shape.Maximum[0]) * 0.5 * scaleX * viewport.Zoom + viewport.Pan.X,
                        (shape.Minimum[1] + shape.Maximum[1]) * 0.5 * scaleY * viewport.Zoom + viewport.Pan.Y), window);
                    Assert.True(point.HasValue);
                    window.MouseDown(point.Value, MouseButton.Left, RawInputModifiers.None);
                    window.MouseUp(point.Value, MouseButton.Left, RawInputModifiers.None);
                    await WaitFor(() => editShape.IsEnabled, () => "The shape was not selected in the world viewport.");
                }
                await SelectShape(removedId);

                var deleteSelected = menu.FindCommandItem(LegacyCommandId.ObjectsDeleteShape)!;
                var delete = menu.FindCommandItem(LegacyCommandId.ObjectsDeleteTenRandomShapes)!;
                Assert.True(delete.IsEnabled);
                delete.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => session.LatestSnapshot.Obstacles.Count == ids.Length - 10
                        && !editShape.IsEnabled && !deleteSelected.IsEnabled,
                    () => "The menu did not remove ten shapes immediately and clear the removed selection.");
                Assert.DoesNotContain(window.OwnedWindows, owned => owned is DeleteConfirmationWindow);
                Assert.Equal(survivors, session.LatestSnapshot.Obstacles.Select(shape => shape.Id));

                await SelectShape(survivors[0]);
                await WaitFor(() => deleteSelected.IsEnabled, () => "Delete Shape did not recognize the selected shape.");
                deleteSelected.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => session.LatestSnapshot.Obstacles.Count == survivors.Length - 1
                        && !editShape.IsEnabled && !deleteSelected.IsEnabled,
                    () => "Delete Shape did not remove exactly the selected shape and clear its selection.");
                Assert.Equal(survivors[1..], session.LatestSnapshot.Obstacles.Select(shape => shape.Id));
                Assert.DoesNotContain(window.OwnedWindows, owned => owned is DeleteConfirmationWindow);

                await SelectShape(survivors[1]);
                var deleteAll = menu.FindCommandItem(LegacyCommandId.ObjectsDeleteAllShapes)!;
                Assert.True(deleteAll.IsEnabled);
                deleteAll.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => session.LatestSnapshot.Obstacles.Count == 0 && !editShape.IsEnabled && !deleteSelected.IsEnabled,
                    () => "Delete All Shapes did not immediately remove the remaining shapes and clear selection.");
                Assert.DoesNotContain(window.OwnedWindows, owned => owned is DeleteConfirmationWindow);
                Assert.Equal(before.Tick, session.LatestSnapshot.Tick);
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task ShapesToolCreatesWithLiveDimensionsAndResetsOnLoadedWorld()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu", Seed = 95, PopulationCapacity = 64,
            WorldWidth = 7_000, WorldHeight = 5_000,
        };
        using var savedWorld = new NativeEngineClient(setup with { WorldWidth = 1_100, WorldHeight = 800 });
        var storage = new AuditStorageService { OpenedSimulation = savedWorld.Save() };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, storage);
            window.Show();
            try
            {
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var create = menu.FindCommandItem(LegacyCommandId.ObjectsNewShape)!;
                var load = menu.FindCommandItem(LegacyCommandId.FileLoadSimulation)!;
                await WaitFor(() => load.IsEnabled, () => "The simulation did not finish starting.");
                var session = TryField<SimulationSession>(window, "_session")!;
                Assert.Equal([7_000f, 5_000f], session.LatestSnapshot.WorldSize);
                var tick = session.LatestSnapshot.Tick;

                async Task<ShapesWindow> OpenTool()
                {
                    create.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => window.OwnedWindows.OfType<ShapesWindow>().Any(tool => tool.IsVisible),
                        () => "New Shape did not show its modeless tool.");
                    return Assert.Single(window.OwnedWindows.OfType<ShapesWindow>(), tool => tool.IsVisible);
                }

                var tool = await OpenTool();
                Assert.Equal("Shapes", tool.Title);
                Assert.Same(tool, await OpenTool());
                Assert.Empty(session.LatestSnapshot.Obstacles);
                var width = tool.FindControl<Slider>("ShapeWidthSlider")!;
                var height = tool.FindControl<Slider>("ShapeHeightSlider")!;
                Assert.Equal(200d, width.Value);
                Assert.Equal(200d, height.Value);
                width.Value = 80;
                height.Value = 125;

                var add = tool.FindControl<Button>("AddShapeButton")!;
                var advanceGate = TryField<SemaphoreSlim>(window, "_advanceGate")!;
                for (var count = 1; count <= 2; count++)
                {
                    await WaitFor(() => add.IsEnabled, () => "The Add Shape button did not become available.");
                    if (count == 1)
                    {
                        await advanceGate.WaitAsync();
                        try
                        {
                            add.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                            Assert.False(add.IsEnabled);
                            width.Value = 120;
                            height.Value = 180;
                        }
                        finally { advanceGate.Release(); }
                    }
                    else
                    {
                        add.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    }
                    await WaitFor(() => session.LatestSnapshot.Obstacles.Count == count,
                        () => "Add Shape did not create one rectangle.");
                    width.Value = 80;
                    height.Value = 125;
                    Assert.True(tool.IsVisible);
                }
                Assert.All(session.LatestSnapshot.Obstacles, shape =>
                {
                    Assert.Equal(560f, shape.Maximum[0] - shape.Minimum[0], 2);
                    Assert.Equal(625f, shape.Maximum[1] - shape.Minimum[1], 2);
                });

                var random = menu.FindCommandItem(LegacyCommandId.ObjectsAddTenRandomShapes)!;
                Assert.True(random.IsEnabled);
                random.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => session.LatestSnapshot.Obstacles.Count == 12,
                    () => "Add Ten Random Shapes did not run with the tool open.");
                Assert.All(session.LatestSnapshot.Obstacles.Skip(2), shape =>
                {
                    Assert.InRange(shape.Maximum[0] - shape.Minimum[0], 0f, 560.001f);
                    Assert.InRange(shape.Maximum[1] - shape.Minimum[1], 0f, 625.001f);
                });
                Assert.Equal(tick, session.LatestSnapshot.Tick);

                tool.FindControl<Button>("CloseButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => !tool.IsVisible, () => "The Shapes tool did not close.");
                tool = await OpenTool();
                width = tool.FindControl<Slider>("ShapeWidthSlider")!;
                height = tool.FindControl<Slider>("ShapeHeightSlider")!;
                Assert.Equal(80d, width.Value);
                Assert.Equal(125d, height.Value);

                load.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => session.LatestSnapshot.WorldSize.SequenceEqual([1_100f, 800f])
                    && width.Value == 200 && height.Value == 200,
                    () => "Loading the saved world did not reset the visible Shapes controls.");
                Assert.Empty(session.LatestSnapshot.Obstacles);

                add = tool.FindControl<Button>("AddShapeButton")!;
                await WaitFor(() => add.IsEnabled, () => "The Shapes tool did not become available after load.");
                add.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => session.LatestSnapshot.Obstacles.Count == 1,
                    () => "Add Shape did not create a rectangle in the loaded world.");
                var loadedShape = Assert.Single(session.LatestSnapshot.Obstacles);
                Assert.Equal(220f, loadedShape.Maximum[0] - loadedShape.Minimum[0], 2);
                Assert.Equal(160f, loadedShape.Maximum[1] - loadedShape.Minimum[1], 2);
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task RandomShapesAndMazeCommandsUseTheLoadedWorldWhilePaused()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu", Seed = 95, PopulationCapacity = 64,
            WorldWidth = 7_000, WorldHeight = 5_000,
        };
        var loadedSetup = setup with { WorldWidth = 1_100, WorldHeight = 800 };
        using var savedWorld = new NativeEngineClient(loadedSetup);
        var storage = new AuditStorageService { OpenedSimulation = savedWorld.Save() };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, storage);
            window.Show();
            try
            {
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var load = menu.FindCommandItem(LegacyCommandId.FileLoadSimulation)!;
                await WaitFor(() => load.IsEnabled, () => "The simulation did not finish starting.");
                var session = TryField<SimulationSession>(window, "_session")!;
                Assert.Equal([7_000f, 5_000f], session.LatestSnapshot.WorldSize);

                load.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(
                    () => session.LatestSnapshot.WorldSize.SequenceEqual([1_100f, 800f]),
                    () => "The load command did not publish the saved field dimensions.");
                var tick = session.LatestSnapshot.Tick;

                var random = menu.FindCommandItem(LegacyCommandId.ObjectsAddTenRandomShapes)!;
                random.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => session.LatestSnapshot.Obstacles.Count == 10,
                    () => "The first random-shape command did not add ten rectangles.");
                random.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => session.LatestSnapshot.Obstacles.Count == 20,
                    () => "The second random-shape command did not add ten rectangles.");
                var rectangles = session.LatestSnapshot.Obstacles;
                Assert.Equal(20, rectangles.Select(shape =>
                    (shape.Minimum[0], shape.Minimum[1], shape.Maximum[0], shape.Maximum[1])).Distinct().Count());
                Assert.All(rectangles, shape =>
                {
                    Assert.InRange(shape.Minimum[0], 0f, shape.Maximum[0]);
                    Assert.InRange(shape.Minimum[1], 0f, shape.Maximum[1]);
                    Assert.InRange(shape.Maximum[0], 0f, 1_100f);
                    Assert.InRange(shape.Maximum[1], 0f, 800f);
                    Assert.InRange(shape.Maximum[0] - shape.Minimum[0], 0f, 220f);
                    Assert.InRange(shape.Maximum[1] - shape.Minimum[1], 0f, 160f);
                    Assert.InRange(shape.Color & 0xff, 0u, 1u);
                });

                menu.FindCommandItem(LegacyCommandId.ObjectsTrashCompactor)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => session.LatestSnapshot.Obstacles.Count == 22,
                    () => "The trash compactor command did not add two walls.");
                var walls = session.LatestSnapshot.Obstacles.Skip(20).ToArray();
                Assert.Equal([-999f, -80f], walls[0].Minimum);
                Assert.Equal([1_099f, -80f], walls[1].Minimum);
                Assert.Equal([2f, 0f], walls[0].Drift);
                Assert.Equal([-2f, 0f], walls[1].Drift);
                Assert.Equal(tick, session.LatestSnapshot.Tick);
                await session.StepAsync(1);
                var movedWalls = session.LatestSnapshot.Obstacles.Skip(20).ToArray();
                Assert.Equal([-997f, -80f], movedWalls[0].Minimum);
                Assert.Equal([1_097f, -80f], movedWalls[1].Minimum);
            }
            finally { window.Close(); }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task MainWindowRoutesWorldObjectsMutationAndGraphToolsWithOfflineRenderedReceipt()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            Seed = 95,
            Recording = new Db2RecordingOptions(SamplingInterval: 20),
            PopulationCapacity = 64,
            WorldWidth = 1_000,
            WorldHeight = 800,
            MetabolismCost = 0,
            VegetableEnergyPerTick = 0,
            SunlightEnergy = 0,
            Species =
            [
                new SpeciesImport(
                    "Rendered Bot",
                    "start\nstop",
                    false,
                    0xff3f8e34u,
                    1,
                    1_000,
                    0,
                    false,
                    0f),
            ],
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup);
            try
            {
                window.WindowState = WindowState.Normal;
                window.Width = 1_200;
                window.Height = 820;
                window.Show();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                await WaitFor(
                    () => TryField<SimulationSession>(window, "_session")?.LatestSnapshot.Organisms.Any(organism => !organism.Vegetable) == true,
                    () => "MainWindow did not publish a Rust-backed selected robot.");

                var session = TryField<SimulationSession>(window, "_session")!;
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var viewport = window.FindControl<WorldViewport>("Viewport")!;

                await session.StepAsync(20);
                await WaitFor(
                    () => session.LatestSnapshot.History.Count > 0,
                    () => "Rust-backed history was not published for the graph route.");

                var vectors = menu.FindCommandItem(LegacyCommandId.ViewDisplayMovementVectors)!;
                var impacts = menu.FindCommandItem(LegacyCommandId.ViewDisplayShotImpacts)!;
                var gauges = menu.FindCommandItem(LegacyCommandId.ViewDisplayResourceGuages)!;
                var backgroundImport = menu.FindCommandItem(LegacyCommandId.ViewImportBackground)!;
                var backgroundRemove = menu.FindCommandItem(LegacyCommandId.ViewRemoveBackground)!;
                Assert.True(vectors.IsVisible && vectors.IsEnabled);
                Assert.True(impacts.IsVisible && impacts.IsEnabled);
                Assert.True(gauges.IsVisible && gauges.IsEnabled);
                Assert.True(backgroundImport.IsVisible && backgroundImport.IsEnabled);
                Assert.True(backgroundRemove.IsVisible && backgroundRemove.IsEnabled);
                backgroundRemove.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => viewModel.Status == "BACKGROUND REMOVED", () => "Removing an empty background was not routed.");
                Assert.False(viewport.HasBackgroundImage);

                if (vectors.IsChecked) vectors.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                if (impacts.IsChecked) impacts.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                if (gauges.IsChecked) gauges.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                Assert.False(viewport.ShowMovementVectors || viewport.ShowShotImpacts || viewport.ShowResourceGauges);
                vectors.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                impacts.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                gauges.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(
                    () => viewport.ShowMovementVectors && viewport.ShowShotImpacts && viewport.ShowResourceGauges,
                    () => "View menu overlay commands did not change the real world viewport.");
                Assert.True(vectors.IsChecked);
                Assert.True(impacts.IsChecked);
                Assert.True(gauges.IsChecked);

                var objects = Assert.Single(menu.TopLevelItems.Where(item => string.Equals(item.Header?.ToString(), "Objects", StringComparison.Ordinal)));
                objects.IsSubMenuOpen = true;
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                Assert.True(objects.IsSubMenuOpen);
                Assert.Contains(objects.Items.OfType<MenuItem>(), item => string.Equals(item.Header?.ToString(), "Shots", StringComparison.Ordinal));
                Assert.Contains(objects.Items.OfType<MenuItem>(), item => string.Equals(item.Header?.ToString(), "Teleporters", StringComparison.Ordinal));
                Assert.Contains(objects.Items.OfType<MenuItem>(), item => string.Equals(item.Header?.ToString(), "Shapes", StringComparison.Ordinal));
                var newShape = menu.FindCommandItem(LegacyCommandId.ObjectsNewShape)!;
                Assert.True(newShape.IsVisible && newShape.IsEnabled);
                newShape.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(
                    () => window.OwnedWindows.OfType<ShapesWindow>().Any(tool => tool.IsVisible),
                    () => "Objects > Shapes > New Shape did not open the Shapes tool.");
                Assert.Single(window.OwnedWindows.OfType<ShapesWindow>(), tool => tool.IsVisible).Close();

                var selected = session.LatestSnapshot.Organisms.First(organism => !organism.Vegetable);
                viewport.SelectKey(new OrganismKey(selected.Slot, selected.Generation));
                await WaitFor(
                    () => menu.FindCommandItem(LegacyCommandId.RobotMutationRates)?.IsEnabled == true,
                    () => "The real Robot > Mutation Rates command did not follow viewport selection.");

                var mutationBefore = selected.MutationProfile;
                var globalMutationBefore = session.LatestSnapshot.MutationSettings;
                menu.FindCommandItem(LegacyCommandId.RobotMutationRates)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(
                    () => window.OwnedWindows.OfType<Window>().Any(IsMutationWindow),
                    () => "Robot > Mutation Rates did not open its player-reachable tool window.");
                var mutationWindow = Assert.IsType<MutationRatesWindow>(Assert.Single(window.OwnedWindows.Where(IsMutationWindow)));
                Assert.True(mutationWindow.IsVisible);
                mutationWindow.FindControl<ComboBox>("ClassSelector")!.SelectedIndex = 0;
                mutationWindow.FindControl<CheckBox>("Enabled")!.IsChecked = true;
                mutationWindow.FindControl<NumericUpDown>("OneIn")!.Value = 2;
                var expectedProfile = mutationBefore with { Point = mutationBefore.Point with { OneIn = 2 } };
                var applyMutation = mutationWindow.GetVisualDescendants().OfType<Button>()
                    .Single(button => button.Content?.ToString() == "APPLY");
                applyMutation.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(
                    () => !mutationWindow.IsVisible && session.LatestSnapshot.Organisms.Any(robot =>
                        robot.Slot == selected.Slot && robot.Generation == selected.Generation && robot.MutationProfile == expectedProfile),
                    () => "Robot mutation rates did not update the selected robot's authoritative profile.");
                Assert.Equal(globalMutationBefore, session.LatestSnapshot.MutationSettings);

                var graphsButton = window.FindControl<Button>("GraphsButton")!;
                graphsButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(
                    () => window.OwnedWindows.OfType<GraphSuiteWindow>().Any(),
                    () => "MainWindow did not open Graph Suite.");
                var graphWindow = Assert.Single(window.OwnedWindows.OfType<GraphSuiteWindow>());
                var graphView = graphWindow.FindControl<GraphSuiteView>("GraphView")!;
                var openGraph = graphView.FindControl<Button>("OpenGraphButton")!;
                graphWindow.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.AverageDnaLength);
                openGraph.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(
                    () => graphWindow.ViewModel.RunningGraphs.Count == 1,
                    () => "Graph Suite did not open the first live graph window.");
                graphWindow.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.Population);
                openGraph.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(
                    () => graphWindow.OwnedWindows.OfType<GraphSeriesWindow>().Count() == 2,
                    () => "Graph Suite did not open two live graph windows.");
                Assert.All(graphWindow.ViewModel.RunningGraphs, graph => Assert.NotEmpty(graph.Samples));

                var export = graphWindow.ViewModel.ExportCsv();
                Assert.Contains("Population", export, StringComparison.Ordinal);
                Assert.Contains("Average DNA length", export, StringComparison.Ordinal);
                foreach (var graph in graphWindow.ViewModel.RunningGraphs) graph.IsSelected = true;
                graphView.FindControl<Button>("JoinCompareButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.Contains("JOINED", graphWindow.ViewModel.Status, StringComparison.Ordinal);

                graphView.FindControl<Button>("ResetGraphsButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.All(graphWindow.ViewModel.RunningGraphs, graph => Assert.Empty(graph.Samples));

                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var frame = window.CaptureRenderedFrame();
                Assert.NotNull(frame);
                using var receipt = new MemoryStream();
                frame.Save(receipt, new Avalonia.Media.Imaging.PngBitmapEncoderOptions());
                Assert.NotEmpty(receipt.ToArray());
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task GlobalMutationMenuAppliesAndCancelsNativePolicyWithoutOverwritingRobotProfiles()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu", PopulationCapacity = 64, WorldWidth = 1_000, WorldHeight = 800,
            GlobalSettings = new GlobalSettings
            {
                Main = new GlobalMainSettings { EpigeneticAmplification = 2 },
                AdvancedMutation = new GlobalAdvancedMutationSettings { EpigeneticAmplification = 1.25 },
            },
            Species = [new("Control", "start\nstop", false, 0xff3f8e34u, 1, 1_000, 0, false, 0f)],
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService());
            window.Show();
            try
            {
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var command = menu.FindCommandItem(LegacyCommandId.EditGlobalMutationRates)!;
                await WaitFor(() => TryField<SimulationSession>(window, "_session")?.LatestSnapshot.Organisms.Count == 1
                    && command.IsEnabled, () => "The mutation menu did not become available.");
                var session = TryField<SimulationSession>(window, "_session")!;
                var robot = Assert.Single(session.LatestSnapshot.Organisms);
                var custom = robot.MutationProfile with { Point = new MutationRateUpdate(42, 3, 1) };
                await session.UpdateOrganismMutationProfileAsync(robot.Slot, robot.Generation, custom);
                var startingAdvanced = (await session.ReadConfigurationAsync())!.AdvancedMutation;

                async Task<MutationRatesWindow> Reopen()
                {
                    command.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => window.OwnedWindows.OfType<MutationRatesWindow>().Any(),
                        () => "Global Mutation Rates did not open the editor.");
                    return Assert.Single(window.OwnedWindows.OfType<MutationRatesWindow>());
                }

                var dialog = await Reopen();
                Assert.True(dialog.FindControl<Border>("GlobalMutationPanel")!.IsVisible);
                Assert.True(dialog.FindControl<CheckBox>("GlobalMutationsEnabled")!.IsChecked);
                Assert.Equal(1m, dialog.FindControl<NumericUpDown>("GlobalMultiplier")!.Value);
                Assert.Equal((int)GlobalMutationOscillationMode.Disabled, dialog.FindControl<ComboBox>("GlobalOscillationMode")!.SelectedIndex);
                Assert.NotNull(dialog.FindControl<NumericUpDown>("OneIn"));
                dialog.FindControl<CheckBox>("GlobalMutationsEnabled")!.IsChecked = false;
                dialog.FindControl<NumericUpDown>("GlobalMultiplier")!.Value = 2.25m;
                dialog.FindControl<ComboBox>("GlobalOscillationMode")!.SelectedIndex = (int)GlobalMutationOscillationMode.Sawtooth;
                dialog.FindControl<NumericUpDown>("GlobalHighCycles")!.Value = 25m;
                dialog.FindControl<NumericUpDown>("GlobalLowCycles")!.Value = 75m;
                dialog.FindControl<Button>("ApplyButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

                var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitFor(() => viewModel.Status == "GLOBAL MUTATION RATES APPLIED", () => viewModel.Status);
                var snapshot = session.LatestSnapshot;
                Assert.False(snapshot.MutationSettings.Enabled);
                Assert.Equal(2.25f, snapshot.MutationSettings.GlobalMultiplier);
                Assert.Equal(custom, Assert.Single(snapshot.Organisms).MutationProfile);
                Assert.Equal(startingAdvanced with
                {
                    OscillationMode = GlobalMutationOscillationMode.Sawtooth,
                    OscillationHighCycles = 25,
                    OscillationLowCycles = 75,
                }, (await session.ReadConfigurationAsync())!.AdvancedMutation);

                var applied = await session.SaveAsync();
                dialog = await Reopen();
                Assert.False(dialog.FindControl<CheckBox>("GlobalMutationsEnabled")!.IsChecked);
                Assert.Equal(2.25m, dialog.FindControl<NumericUpDown>("GlobalMultiplier")!.Value);
                Assert.Equal((int)GlobalMutationOscillationMode.Sawtooth, dialog.FindControl<ComboBox>("GlobalOscillationMode")!.SelectedIndex);
                Assert.Equal(25m, dialog.FindControl<NumericUpDown>("GlobalHighCycles")!.Value);
                Assert.Equal(75m, dialog.FindControl<NumericUpDown>("GlobalLowCycles")!.Value);
                dialog.FindControl<CheckBox>("GlobalMutationsEnabled")!.IsChecked = true;
                dialog.FindControl<NumericUpDown>("GlobalMultiplier")!.Value = 4m;
                dialog.FindControl<ComboBox>("GlobalOscillationMode")!.SelectedIndex = (int)GlobalMutationOscillationMode.Sine;
                dialog.FindControl<NumericUpDown>("GlobalHighCycles")!.Value = 50m;
                dialog.FindControl<NumericUpDown>("OneIn")!.Value = 777m;
                dialog.FindControl<Button>("CancelButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<MutationRatesWindow>().Count() == 0,
                    () => "Cancel did not close Global Mutation Rates.");
                Assert.Equal(applied, await session.SaveAsync());
                await session.StepAsync(1);
                Assert.Equal(16f, session.LatestSnapshot.MutationSettings.GlobalMultiplier);
            }
            finally
            {
                foreach (var owned in window.OwnedWindows.ToArray()) owned.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static bool IsMutationWindow(Window window) =>
        window.Title?.Contains("mutation", StringComparison.OrdinalIgnoreCase) == true;

    private static T? TryField<T>(object target, string name) where T : class =>
        target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)?.GetValue(target) as T;

    private static async Task WaitFor(Func<bool> condition, Func<string> diagnostic)
    {
        for (var attempt = 0; attempt < 500; attempt++)
        {
            AvaloniaHeadlessPlatform.ForceRenderTimerTick();
            if (condition()) return;
            await Task.Delay(20);
        }

        throw new Xunit.Sdk.XunitException(diagnostic());
    }
}
