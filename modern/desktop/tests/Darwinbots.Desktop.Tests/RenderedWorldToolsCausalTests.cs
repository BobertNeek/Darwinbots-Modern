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
    public async Task MainWindowRoutesWorldObjectsMutationAndGraphToolsWithOfflineRenderedReceipt()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            Seed = 95,
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

                var mutationBefore = ReadMutationSettings(session.LatestSnapshot);
                Assert.NotNull(mutationBefore);
                menu.FindCommandItem(LegacyCommandId.RobotMutationRates)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(
                    () => window.OwnedWindows.OfType<Window>().Any(IsMutationWindow),
                    () => "Robot > Mutation Rates did not open its player-reachable tool window.");
                var mutationWindow = Assert.Single(window.OwnedWindows.OfType<Window>().Where(IsMutationWindow));
                Assert.True(mutationWindow.IsVisible);

                var mutationTab = mutationWindow.GetVisualDescendants()
                    .OfType<TabItem>()
                    .FirstOrDefault(tab => tab.Header?.ToString()?.Contains("MUTATION", StringComparison.OrdinalIgnoreCase) == true);
                if (mutationTab is not null) mutationTab.IsSelected = true;
                var mutationInput = mutationWindow.GetVisualDescendants()
                    .OfType<NumericUpDown>()
                    .FirstOrDefault(input => input.IsEnabled && input.Maximum > input.Minimum);
                Assert.NotNull(mutationInput);
                var oldMutationValue = mutationInput.Value ?? mutationInput.Minimum;
                mutationInput.Value = oldMutationValue >= mutationInput.Maximum
                    ? Math.Max(mutationInput.Minimum, oldMutationValue - mutationInput.Increment)
                    : Math.Min(mutationInput.Maximum, oldMutationValue + mutationInput.Increment);
                var applyMutation = mutationWindow.GetVisualDescendants()
                    .OfType<Button>()
                    .FirstOrDefault(button => button.Content?.ToString()?.Contains("APPLY", StringComparison.OrdinalIgnoreCase) == true);
                Assert.NotNull(applyMutation);
                applyMutation.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(
                    () => !mutationWindow.IsVisible && !Equals(mutationBefore, ReadMutationSettings(session.LatestSnapshot)),
                    () => "Mutation tool did not publish its applied values through the authoritative session snapshot.");

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

    private static object? ReadMutationSettings(EngineSnapshot snapshot) =>
        snapshot.GetType().GetProperty("MutationSettings", BindingFlags.Instance | BindingFlags.Public)?.GetValue(snapshot);

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
