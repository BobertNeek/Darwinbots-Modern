using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
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
    public async Task GeneActivationsFollowsSelectionAndLabelsTheFirstNativeGene()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            Seed = 95,
            PopulationCapacity = 16,
            Species =
            [
                new SpeciesImport("First", "start 11 100 store stop\ncond 0 1 > start 12 101 store stop",
                    false, 0xff3f8e34u, 1, 1_000, 0, false, 0f)
                {
                    Placement = new(SpeciesPlacementMode.Fixed, 500, 500),
                },
                new SpeciesImport("Second", "cond *100 0 > start 33 101 store stop\nstart 22 100 store stop",
                    false, 0xff704394u, 1, 1_000, 0, false, 0f)
                {
                    Placement = new(SpeciesPlacementMode.Fixed, 900, 900),
                },
            ],
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService());
            try
            {
                window.Show();
                var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitFor(() => viewModel.Population == 2, () => viewModel.Status);

                var session = TryField<SimulationSession>(window, "_session")!;
                await session.StepAsync();
                var snapshot = session.LatestSnapshot;
                var first = Assert.Single(snapshot.Organisms, robot => robot.ActivationBitmap.SequenceEqual([false, true, false]));
                var second = Assert.Single(snapshot.Organisms, robot => robot.ActivationBitmap.SequenceEqual([false, false, true]));

                viewModel.Select(new OrganismKey(first.Slot, first.Generation));
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var command = menu.FindCommandItem(LegacyCommandId.RobotViewGeneActivations)!;
                command.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<GeneActivationsWindow>().Any(), () => viewModel.Status);

                var viewer = Assert.Single(window.OwnedWindows.OfType<GeneActivationsWindow>());
                string?[] Text() => viewer.GetVisualDescendants().OfType<TextBlock>().Select(block => block.Text).ToArray();
                Assert.Contains("G1:ON  G2:off", Text());
                Assert.Single(viewer.GetVisualDescendants().OfType<Control>(), control => control.Name == "ActivationBands");

                var secondFocus = new OrganismKey(second.Slot, second.Generation);
                viewModel.Select(secondFocus);
                Assert.Contains("G1:off  G2:ON", Text());
                await WaitFor(() => session.LatestSnapshot.GeneActivationMeter?.Focus == secondFocus, () => viewModel.Status);
                Assert.Empty(session.LatestSnapshot.GeneActivationMeter!.Weighted);
                command.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                Assert.Same(viewer, Assert.Single(window.OwnedWindows.OfType<GeneActivationsWindow>()));

                await session.StepAsync();
                await WaitFor(() => Text().Contains("G1:ON  G2:ON"), () => string.Join(", ", Text()));
                var stepTick = session.LatestSnapshot.Tick;
                Assert.Equal(new[] { 20, 20 }, session.LatestSnapshot.GeneActivationMeter!.Weighted);
                await session.ExecuteRobotsAsync();
                Assert.Equal(stepTick, session.LatestSnapshot.Tick);
                Assert.Equal(new[] { 36, 36 }, session.LatestSnapshot.GeneActivationMeter!.Weighted);
                await session.RefreshSnapshotAsync();
                Assert.Equal(2ul, session.LatestSnapshot.GeneActivationMeter!.Executions);
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var frame = viewer.CaptureRenderedFrame();
                Assert.NotNull(frame);

                viewModel.ClearSelection();
                Assert.Contains("No robot focused", Text());
                Assert.DoesNotContain(Text(), line => line?.Contains("G1:", StringComparison.Ordinal) == true);
                Assert.True(viewer.IsVisible);
                await WaitFor(() => session.LatestSnapshot.GeneActivationMeter is { Focus: null }, () => viewModel.Status);
                Assert.Equal(new[] { 36, 36 }, session.LatestSnapshot.GeneActivationMeter!.Weighted);
                viewer.Close();
                await WaitFor(() => session.LatestSnapshot.GeneActivationMeter is null, () => viewModel.Status);
            }
            finally
            {
                window.Close();
            }
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
                var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
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
                Assert.True(backgroundRemove.IsVisible);
                Assert.False(backgroundRemove.IsEnabled);

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
                    () => viewModel.Status.Contains("OBSTACLE", StringComparison.Ordinal),
                    () => "Objects > Shapes > New Shape did not enter the real placement route.");

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
