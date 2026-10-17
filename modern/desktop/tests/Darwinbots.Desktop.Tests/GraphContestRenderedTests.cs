using System.Reflection;
using System.Runtime.InteropServices;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class GraphContestRenderedTests
{
    [Fact]
    public async Task EvolutionDefaultGraphsOpenActualNativeSeriesWithoutReplacingManualCharts()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var snapshot = EngineSnapshot.Empty with { History = [
                new HistorySampleSnapshot(100, 5, 0, 0, 0, 0, 0) { SpeciesGraphs = [
                    new("Alga.txt", 0xff00a040, new Dictionary<string, double> { ["population"] = 2, ["average_mutations"] = 1.2, ["total_energy"] = 4.5 }),
                    new("Animal.txt", 0xffd03020, new Dictionary<string, double> { ["population"] = 3, ["average_mutations"] = 2.3, ["total_energy"] = 6.7 }),
                ] },
                new HistorySampleSnapshot(200, 5, 0, 0, 0, 0, 0) { SpeciesGraphs = [
                    new("Alga.txt", 0xff00a040, new Dictionary<string, double> { ["population"] = 4, ["average_mutations"] = 1.8, ["total_energy"] = 5.5 }),
                    new("Animal.txt", 0xffd03020, new Dictionary<string, double> { ["population"] = 1, ["average_mutations"] = 3.4, ["total_energy"] = 7.8 }),
                ] },
            ] };
            var suite = new GraphSuiteWindow(snapshot);
            try
            {
                suite.Show();
                Assert.Empty(suite.OwnedWindows.OfType<GraphSeriesWindow>());
                var view = suite.FindControl<GraphSuiteView>("GraphView")!;
                var open = view.FindControl<Button>("OpenGraphButton")!;
                suite.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.AverageAge);
                open.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                var manualWindow = Assert.Single(suite.OwnedWindows.OfType<GraphSeriesWindow>());
                var manual = suite.ViewModel.RunningGraphs.ToArray();
                var selected = GraphCatalog.For(GraphMetric.AverageDnaLength);
                suite.ViewModel.SelectedChoice = selected;

                suite.OpenEvolutionGraphs();
                Assert.Equal(4, suite.OwnedWindows.OfType<GraphSeriesWindow>().Count());
                Assert.Equal(8, suite.ViewModel.RunningGraphs.Count);
                Assert.Same(selected, suite.ViewModel.SelectedChoice);
                Assert.Contains(manualWindow, suite.OwnedWindows.OfType<GraphSeriesWindow>());
                Assert.All(manual, graph => Assert.Contains(graph, suite.ViewModel.RunningGraphs));
                foreach (var metric in new[] { GraphMetric.Population, GraphMetric.AverageMutations, GraphMetric.TotalEnergy })
                {
                    Assert.Equal(2, suite.ViewModel.RunningGraphs.Count(graph => graph.Metric == metric));
                    Assert.All(suite.ViewModel.RunningGraphs.Where(graph => graph.Metric == metric), graph =>
                    {
                        Assert.Equal(new ulong[] { 100, 200 }, graph.Samples.Select(sample => sample.Tick));
                        Assert.Equal(graph.SpeciesName == "Alga.txt" ? 0xff00a040u : 0xffd03020u, graph.Color);
                    });
                }
                Assert.Equal(new[] { 2d, 4d }, suite.ViewModel.RunningGraphs.Single(graph => graph.Metric == GraphMetric.Population && graph.SpeciesName == "Alga.txt").Samples.Select(sample => sample.Value));
                Assert.Equal(new[] { 2.3, 3.4 }, suite.ViewModel.RunningGraphs.Single(graph => graph.Metric == GraphMetric.AverageMutations && graph.SpeciesName == "Animal.txt").Samples.Select(sample => sample.Value));
                Assert.Equal(new[] { 6.7, 7.8 }, suite.ViewModel.RunningGraphs.Single(graph => graph.Metric == GraphMetric.TotalEnergy && graph.SpeciesName == "Animal.txt").Samples.Select(sample => sample.Value));
                var retained = suite.ViewModel.RunningGraphs.ToArray();
                suite.OpenEvolutionGraphs();
                Assert.Equal(retained, suite.ViewModel.RunningGraphs.ToArray());
                Assert.Equal(4, suite.OwnedWindows.OfType<GraphSeriesWindow>().Count());
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                Assert.All(suite.OwnedWindows.OfType<GraphSeriesWindow>(), graph =>
                {
                    Assert.True(graph.IsVisible);
                    Assert.True(graph.FindControl<GraphSeriesChart>("SeriesChart")!.Bounds.Width > 0);
                });
            }
            finally { suite.Close(); }
            await Task.CompletedTask;
        }, CancellationToken.None);
    }

    [Theory]
    [InlineData(LocalModeKind.SimpleSurvival, EvolutionMode.SimpleSurvival)]
    [InlineData(LocalModeKind.EcoSurvival, EvolutionMode.Eco)]
    public async Task PublishedEvolutionStartsRestartsAndLoadResultsPreserveDefaultGraphBoundaries(LocalModeKind kind, EvolutionMode mode)
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var factory = new AuditEngineClientFactory();
            var storage = new AuditStorageService { OpenedSimulation = [1, 2, 3, 4] };
            var window = new MainWindow([], new WorldSetupOptions(), factory, storage);
            try
            {
                window.Show();
                await WaitFor(() => TryField<SimulationSession>(window, "_session") is not null && !factory.Clients.IsEmpty, () => "Audit world was not initialized.");
                var session = TryField<SimulationSession>(window, "_session")!;
                var engine = factory.Clients.First();
                await session.RefreshSnapshotAsync();
                await FlushGraphPresentationAsync();
                var evolution = EvolutionGraphMode(kind, mode, true, "evolving");
                engine.LocalModeForRead = evolution with { Status = LocalModeStatus.Configured };
                await session.RefreshSnapshotAsync();
                await FlushGraphPresentationAsync();
                Assert.Empty(window.OwnedWindows.OfType<GraphSuiteWindow>());

                engine.LocalModeForRead = evolution;
                await session.StartLocalModeAsync();
                await FlushGraphPresentationAsync();
                var suite = Assert.Single(window.OwnedWindows.OfType<GraphSuiteWindow>());
                Assert.Equal(3, suite.OwnedWindows.OfType<GraphSeriesWindow>().Count());
                Assert.Equal(new[] { GraphMetric.Population, GraphMetric.AverageMutations, GraphMetric.TotalEnergy }, suite.ViewModel.RunningGraphs.Select(graph => graph.Metric));
                var population = suite.ViewModel.RunningGraphs.Single(graph => graph.Metric == GraphMetric.Population);
                var graphView = suite.FindControl<GraphSuiteView>("GraphView")!;
                suite.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.AverageAge);
                graphView.FindControl<Button>("OpenGraphButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                var manual = suite.ViewModel.RunningGraphs.Single(graph => graph.Metric == GraphMetric.AverageAge);
                var selected = GraphCatalog.For(GraphMetric.AverageDnaLength);
                suite.ViewModel.SelectedChoice = selected;

                suite.OwnedWindows.OfType<GraphSeriesWindow>().Single(graph => TryField<GraphSeriesViewModel>(graph, "_graph")!.Metric == GraphMetric.AverageMutations).Close();
                await session.RefreshSnapshotAsync();
                await FlushGraphPresentationAsync();
                Assert.DoesNotContain(suite.ViewModel.RunningGraphs, graph => graph.Metric == GraphMetric.AverageMutations);
                Assert.Equal(3, suite.OwnedWindows.OfType<GraphSeriesWindow>().Count());

                engine.LocalModeForRead = evolution with { RestartCount = 2 };
                await session.RestartLocalModeAsync();
                await FlushGraphPresentationAsync();
                Assert.Equal(4, suite.OwnedWindows.OfType<GraphSeriesWindow>().Count());
                Assert.Same(population, suite.ViewModel.RunningGraphs.Single(graph => graph.Metric == GraphMetric.Population));
                Assert.Same(manual, suite.ViewModel.RunningGraphs.Single(graph => graph.Metric == GraphMetric.AverageAge));
                Assert.Same(selected, suite.ViewModel.SelectedChoice);

                suite.OwnedWindows.OfType<GraphSeriesWindow>().Single(graph => TryField<GraphSeriesViewModel>(graph, "_graph")!.Metric == GraphMetric.TotalEnergy).Close();
                var buttonLoadCount = engine.Calls.Count(call => call == "Load");
                window.FindControl<Button>("LoadButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => engine.Calls.Count(call => call == "Load") == buttonLoadCount + 1 && suite.OwnedWindows.OfType<GraphSeriesWindow>().Count() == 4, () => "Loading the same evolving checkpoint did not reopen its missing default graph.");
                await FlushGraphPresentationAsync();
                Assert.Single(window.OwnedWindows.OfType<GraphSuiteWindow>());
                Assert.Same(population, suite.ViewModel.RunningGraphs.Single(graph => graph.Metric == GraphMetric.Population));
                Assert.Same(manual, suite.ViewModel.RunningGraphs.Single(graph => graph.Metric == GraphMetric.AverageAge));
                Assert.Same(selected, suite.ViewModel.SelectedChoice);
                await session.RefreshSnapshotAsync();
                await FlushGraphPresentationAsync();
                Assert.Equal(4, suite.OwnedWindows.OfType<GraphSeriesWindow>().Count());

                // A rejected engine Load publishes nothing. Its next ordinary
                // snapshot must not undo the player's closed-chart choice.
                suite.OwnedWindows.OfType<GraphSeriesWindow>().Single(graph => TryField<GraphSeriesViewModel>(graph, "_graph")!.Metric == GraphMetric.AverageMutations).Close();
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var loadItem = menu.FindCommandItem(LegacyCommandId.FileLoadSimulation)!;
                var viewModel = TryField<Darwinbots.Desktop.ViewModels.MainWindowViewModel>(window, "_viewModel")!;
                var failedLoadCount = engine.Calls.Count(call => call == "Load");
                var previousMode = session.LatestSnapshot.LocalMode;
                engine.RejectLoads = true;
                loadItem.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => viewModel.Status.Contains("Controlled audit load rejection.", StringComparison.Ordinal), () => viewModel.Status);
                Assert.Equal(failedLoadCount + 1, engine.Calls.Count(call => call == "Load"));
                Assert.Equal(previousMode, session.LatestSnapshot.LocalMode);
                await session.RefreshSnapshotAsync();
                await FlushGraphPresentationAsync();
                Assert.DoesNotContain(suite.ViewModel.RunningGraphs, graph => graph.Metric == GraphMetric.AverageMutations);
                Assert.Equal(3, suite.OwnedWindows.OfType<GraphSeriesWindow>().Count());
                Assert.Same(population, suite.ViewModel.RunningGraphs.Single(graph => graph.Metric == GraphMetric.Population));
                Assert.Same(manual, suite.ViewModel.RunningGraphs.Single(graph => graph.Metric == GraphMetric.AverageAge));
                Assert.Same(selected, suite.ViewModel.SelectedChoice);

                // Cancel both normal pickers without reaching engine.Load.
                engine.RejectLoads = false;
                storage.OpenedSimulation = null;
                var canceledLoadCount = engine.Calls.Count(call => call == "Load");
                loadItem.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => viewModel.Status == "CANCELED", () => viewModel.Status);
                window.FindControl<Button>("LoadButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await FlushGraphPresentationAsync();
                Assert.Equal(canceledLoadCount, engine.Calls.Count(call => call == "Load"));
                await session.RefreshSnapshotAsync();
                await FlushGraphPresentationAsync();
                Assert.DoesNotContain(suite.ViewModel.RunningGraphs, graph => graph.Metric == GraphMetric.AverageMutations);
                Assert.Equal(3, suite.OwnedWindows.OfType<GraphSeriesWindow>().Count());
                Assert.Same(population, suite.ViewModel.RunningGraphs.Single(graph => graph.Metric == GraphMetric.Population));
                Assert.Same(manual, suite.ViewModel.RunningGraphs.Single(graph => graph.Metric == GraphMetric.AverageAge));
                Assert.Same(selected, suite.ViewModel.SelectedChoice);

                // Successful legacy-menu load shares the button's success-only
                // boundary, opens the missing default once, and keeps all others.
                storage.OpenedSimulation = [1, 2, 3, 4];
                loadItem.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => viewModel.Status == "LOADED" && suite.OwnedWindows.OfType<GraphSeriesWindow>().Count() == 4, () => viewModel.Status);
                await FlushGraphPresentationAsync();
                Assert.Equal(canceledLoadCount + 1, engine.Calls.Count(call => call == "Load"));
                Assert.Equal(4, suite.ViewModel.RunningGraphs.Count);
                Assert.Single(suite.ViewModel.RunningGraphs.Where(graph => graph.Metric == GraphMetric.AverageMutations));
                Assert.Same(population, suite.ViewModel.RunningGraphs.Single(graph => graph.Metric == GraphMetric.Population));
                Assert.Same(manual, suite.ViewModel.RunningGraphs.Single(graph => graph.Metric == GraphMetric.AverageAge));
                Assert.Same(selected, suite.ViewModel.SelectedChoice);
                await session.RefreshSnapshotAsync();
                await FlushGraphPresentationAsync();
                Assert.Equal(4, suite.OwnedWindows.OfType<GraphSeriesWindow>().Count());
            }
            finally { window.Close(); }
        }, CancellationToken.None);
    }

    [Theory]
    [InlineData(LocalModeKind.SimpleSurvival, EvolutionMode.SimpleSurvival, false, "evolving")]
    [InlineData(LocalModeKind.EcoSurvival, EvolutionMode.Eco, false, "evolving")]
    [InlineData(LocalModeKind.Zerobot, EvolutionMode.Zerobot, true, "evolving")]
    [InlineData(LocalModeKind.SimpleSurvival, EvolutionMode.SimpleSurvival, true, "testing")]
    [InlineData(LocalModeKind.EcoSurvival, EvolutionMode.Eco, true, "testing")]
    [InlineData(LocalModeKind.Contest, EvolutionMode.SimpleSurvival, true, "evolving")]
    public async Task NoneligibleEvolutionSnapshotsKeepAutomaticGraphsClosedAndManualChooserUsable(LocalModeKind kind, EvolutionMode mode, bool autoGraphs, string phase)
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var factory = new AuditEngineClientFactory();
            var window = new MainWindow([], new WorldSetupOptions(), factory, new AuditStorageService());
            try
            {
                window.Show();
                await WaitFor(() => TryField<SimulationSession>(window, "_session") is not null && !factory.Clients.IsEmpty, () => "Audit world was not initialized.");
                var session = TryField<SimulationSession>(window, "_session")!;
                var engine = factory.Clients.First();
                engine.LocalModeForRead = EvolutionGraphMode(kind, mode, autoGraphs, phase);
                await session.RefreshSnapshotAsync();
                await FlushGraphPresentationAsync();
                Assert.Empty(window.OwnedWindows.OfType<GraphSuiteWindow>());
                window.FindControl<Button>("GraphsButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                var suite = Assert.Single(window.OwnedWindows.OfType<GraphSuiteWindow>());
                Assert.Empty(suite.ViewModel.RunningGraphs);
                suite.FindControl<GraphSuiteView>("GraphView")!.FindControl<Button>("OpenGraphButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                var manual = Assert.Single(suite.ViewModel.RunningGraphs);
                Assert.Equal(GraphMetric.Population, manual.Metric);
                Assert.Single(suite.OwnedWindows.OfType<GraphSeriesWindow>());
                await session.RefreshSnapshotAsync();
                await FlushGraphPresentationAsync();
                Assert.Same(manual, Assert.Single(suite.ViewModel.RunningGraphs));
                Assert.Single(suite.OwnedWindows.OfType<GraphSeriesWindow>());
            }
            finally { window.Close(); }
        }, CancellationToken.None);
    }

    private static LocalModeSnapshot EvolutionGraphMode(LocalModeKind kind, EvolutionMode mode, bool autoGraphs, string phase) =>
        LocalModeSnapshot.Idle with
        {
            Status = LocalModeStatus.Running,
            Kind = kind,
            RestartCount = 1,
            Configuration = LocalModeConfig.Default with
            {
                Kind = kind,
                StagedEvolution = new(mode, "start\nstop", 10, 100, false, 32_000, autoGraphs),
            },
            Evolution = new(0, phase, true, 10, 100, 0, 0, 0),
        };

    private static async Task FlushGraphPresentationAsync()
    {
        await Avalonia.Threading.Dispatcher.UIThread.InvokeAsync(() => { }, Avalonia.Threading.DispatcherPriority.Background);
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }

    [Fact]
    public async Task SpeciesGraphWindowRendersDistinctNativeColorsAndClosingStopsQueries()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var snapshot = EngineSnapshot.Empty with { History = [
                new HistorySampleSnapshot(100, 5, 0, 0, 0, 0, 0) { SpeciesGraphs = [
                    new("Alga.txt", 0xff00a040, new Dictionary<string, double> { ["population"] = 2 }),
                    new("Animal.txt", 0xffd03020, new Dictionary<string, double> { ["population"] = 3 }),
                ] },
                new HistorySampleSnapshot(200, 5, 0, 0, 0, 0, 0) { SpeciesGraphs = [
                    new("Alga.txt", 0xff00a040, new Dictionary<string, double> { ["population"] = 4 }),
                    new("Animal.txt", 0xffd03020, new Dictionary<string, double> { ["population"] = 1 }),
                ] },
            ] };
            var suite = new GraphSuiteWindow(snapshot);
            try
            {
                suite.Show();
                var settings = new List<GraphSamplingSettings>();
                suite.SamplingChanged += settings.Add;
                suite.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.Population);
                var view = suite.FindControl<GraphSuiteView>("GraphView")!;
                var button = view.FindControl<Button>("OpenGraphButton")!;
                button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                var graphWindow = Assert.Single(suite.OwnedWindows.OfType<GraphSeriesWindow>());
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var frame = graphWindow.CaptureRenderedFrame();
                Assert.NotNull(frame);
                var area = new PixelRect(0, 0, frame.PixelSize.Width, frame.PixelSize.Height);
                Assert.True(ReadColour(frame, area, 0, 160, 64).Count > 20, "Alga trace did not render in its native species color.");
                Assert.True(ReadColour(frame, area, 208, 48, 32).Count > 20, "Animal trace did not render in its native species color.");
                var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR") ?? Path.GetTempPath();
                Directory.CreateDirectory(directory);
                using (var stream = File.Create(Path.Combine(directory, "native-species-graph.png")))
                    frame.Save(stream, new PngBitmapEncoderOptions());
                graphWindow.Close();
                Assert.Empty(suite.ViewModel.RunningGraphs);
                suite.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.SlowGeneticDistance);
                button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.Equal(new GraphSamplingSettings(Slow: true), Assert.Single(settings));
                Assert.Single(suite.OwnedWindows.OfType<GraphSeriesWindow>()).Close();
                Assert.Equal(new GraphSamplingSettings(), settings[^1]);
            }
            finally { suite.Close(); }
            await Task.CompletedTask;
        }, CancellationToken.None);
    }

    [Fact]
    public async Task MainWindowRoutesToRustBackedGraphAndContestResultsWithOnlineCommandsDisabled()
    {
        var dna = new[] { "start\nstop", "start\n1\nstop", "start\n2\nstop", "start\n3\nstop", "start\n4\nstop" };
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            Seed = 24832,
            Recording = new Db2RecordingOptions(SamplingInterval: 20),
            PopulationCapacity = 1_000,
            WorldWidth = 2_000,
            WorldHeight = 1_500,
            MetabolismCost = 1,
            VegetableEnergyPerTick = 0,
            SunlightEnergy = 0,
            Species = dna.Select((value, index) => new SpeciesImport(
                $"Competitor {index + 1}",
                value,
                false,
                0xff224466u + (uint)(index * 0x001122),
                2,
                1_000,
                0,
                false,
                0f)).ToArray(),
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup);
            try
            {
                window.WindowState = WindowState.Normal;
                window.Width = 1280;
                window.Height = 900;
                window.Show();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                await WaitFor(
                    () => TryField<SimulationSession>(window, "_session") is { } ready
                        && ready.LatestSnapshot.Population == setup.Species.Sum(species => species.InitialCount)
                        && setup.Species.All(species => ready.LatestSnapshot.Species.Any(actual => actual.Name == species.Name)),
                    () => "MainWindow did not publish all Rust-backed imported species.");

                var session = TryField<SimulationSession>(window, "_session")!;
                await session.ConfigureLocalModeAsync(new LocalModeConfig(
                    LocalModeKind.Contest,
                    LeaguePreset.F1,
                    1,
                    1,
                    200,
                    1_000,
                    true,
                    null,
                    0.5f,
                    0.5f,
                    new()));
                await session.StartLocalModeAsync();
                await session.StepAsync(100);
                await session.AdjudicateLocalRoundAsync(1, "rendered route adjudication");
                await WaitFor(
                    () => session.LatestSnapshot.History.Count > 0 && session.LatestSnapshot.LocalMode.Results.Count > 0,
                    () => "Rust-backed history or local contest result was not published.");

                var populationBeforeInsert = session.LatestSnapshot.Population;
                var speciesBeforeInsert = session.LatestSnapshot.Species.Count;
                var insertionKeys = session.LatestSnapshot.Organisms.Select(robot => new OrganismKey(robot.Slot, robot.Generation)).ToHashSet();
                var insertButton = window.FindControl<Button>("InsertSpeciesButton")!;
                insertButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                var insertPoint = viewport.TranslatePoint(
                    new Point(viewport.Bounds.Width * 0.82, viewport.Bounds.Height * 0.78), window);
                Assert.True(insertPoint.HasValue, "The player viewport was not attached to MainWindow.");
                window.MouseDown(insertPoint.Value, MouseButton.Left, RawInputModifiers.None);
                window.MouseUp(insertPoint.Value, MouseButton.Left, RawInputModifiers.None);
                await WaitFor(
                    () => session.LatestSnapshot.Population == populationBeforeInsert + 1,
                    () => "The player insertion workflow must add exactly one robot.");
                Assert.Equal(speciesBeforeInsert, session.LatestSnapshot.Species.Count);
                var inserted = Assert.Single(session.LatestSnapshot.Organisms, robot => !insertionKeys.Contains(new OrganismKey(robot.Slot, robot.Generation)));
                var worldSize = session.LatestSnapshot.WorldSize;
                Assert.InRange(inserted.Position[0], worldSize[0] * 0.82f - 1, worldSize[0] * 0.82f + 1);
                Assert.InRange(inserted.Position[1], worldSize[1] * 0.78f - 1, worldSize[1] * 0.78f + 1);
                insertButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await session.StepAsync(100);
                await WaitFor(
                    () => session.LatestSnapshot.History.Count >= 2,
                    () => "The live player route did not publish two history samples.");

                var populationChart = window.FindControl<HistoryChart>("PopulationChart")!;
                var energyChart = window.FindControl<HistoryChart>("EnergyChart")!;
                var surfaceText = string.Join(
                    "\n",
                    window.FindControl<Grid>("MainSurfaceGrid")!
                        .GetVisualDescendants()
                        .OfType<TextBlock>()
                        .Select(value => value.Text)
                        .Where(value => !string.IsNullOrWhiteSpace(value)));
                Assert.Contains("POPULATION", surfaceText, StringComparison.Ordinal);
                Assert.Contains("ENERGY", surfaceText, StringComparison.Ordinal);
                Assert.Contains("organisms", surfaceText, StringComparison.Ordinal);
                Assert.Contains("total", surfaceText, StringComparison.Ordinal);
                Assert.Equal(HistoryMetric.Population, populationChart.Metric);
                Assert.Equal(HistoryMetric.Energy, energyChart.Metric);

                var graphsButton = window.FindControl<Button>("GraphsButton")!;
                graphsButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(
                    () => window.OwnedWindows.OfType<GraphSuiteWindow>().Any(),
                    () => "MainWindow did not open Graph Suite.");
                var graphWindow = Assert.Single(window.OwnedWindows.OfType<GraphSuiteWindow>());
                graphWindow.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.AverageDnaLength);
                var graphView = graphWindow.FindControl<GraphSuiteView>("GraphView")!;
                var openGraphButton = graphView.FindControl<Button>("OpenGraphButton")!;
                openGraphButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(
                    () => graphWindow.ViewModel.RunningGraphs.Count > 0,
                    () => "Graph Suite did not open a graph from the routed window.");

                graphWindow.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.Population);
                openGraphButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(
                    () => graphWindow.OwnedWindows.OfType<GraphSeriesWindow>().Count() == 2,
                    () => "Graph Suite did not open two live graph windows.");
                Assert.All(graphWindow.ViewModel.RunningGraphs, graph => Assert.NotEmpty(graph.Samples));

                var preUpdateTick = graphWindow.ViewModel.RunningGraphs
                    .SelectMany(graph => graph.Samples)
                    .Max(sample => sample.Tick);
                await session.StepAsync(100);
                await WaitFor(
                    () => graphWindow.ViewModel.RunningGraphs.All(graph => graph.Samples.Any(sample => sample.Tick > preUpdateTick)),
                    () => "Live graph windows did not receive the next Rust history sample.");

                var export = graphWindow.ViewModel.ExportCsv();
                Assert.StartsWith("series_id,metric,title,expression,tick,value\r\n", export, StringComparison.Ordinal);
                Assert.Contains("Population", export, StringComparison.Ordinal);
                Assert.Contains("Average DNA length", export, StringComparison.Ordinal);

                Assert.Equal(5, graphWindow.ViewModel.RunningGraphs.Count(graph => graph.Metric == GraphMetric.Population));
                Assert.All(graphWindow.ViewModel.RunningGraphs, graph => Assert.False(string.IsNullOrWhiteSpace(graph.SpeciesName)));
                var comparisonSpecies = graphWindow.ViewModel.RunningGraphs[0].SpeciesName;
                foreach (var graph in graphWindow.ViewModel.RunningGraphs)
                    graph.IsSelected = graph.SpeciesName == comparisonSpecies;
                graphView.FindControl<Button>("JoinCompareButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.Contains("JOINED", graphWindow.ViewModel.Status, StringComparison.Ordinal);

                var resetTick = graphWindow.ViewModel.RunningGraphs
                    .SelectMany(graph => graph.Samples)
                    .Max(sample => sample.Tick);
                graphView.FindControl<Button>("ResetGraphsButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.All(graphWindow.ViewModel.RunningGraphs, graph => Assert.Empty(graph.Samples));
                await session.StepAsync(100);
                await WaitFor(
                    () => graphWindow.ViewModel.RunningGraphs.All(graph => graph.Samples.Count > 0 && graph.Samples.Min(sample => sample.Tick) > resetTick),
                    () => "Reset graphs did not resume from a new live history segment.");

                var contestButton = window.FindControl<Button>("ContestResultsButton")!;
                contestButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(
                    () => window.OwnedWindows.OfType<ContestResultsWindow>().Any(),
                    () => "MainWindow did not open Contest Results.");
                var contestWindow = Assert.Single(window.OwnedWindows.OfType<ContestResultsWindow>());
                Assert.Equal(ContestResultSnapshot.FromSnapshot(session.LatestSnapshot).Competitors.Count, contestWindow.ViewModel.Competitors.Count);
                Assert.All(setup.Species, species => Assert.Contains(contestWindow.ViewModel.Competitors, competitor => competitor.Name == species.Name));
                Assert.Contains(contestWindow.ViewModel.Competitors, competitor => competitor.Wins > 0);

                var historicMenu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var onlineCommands = new[]
                {
                    LegacyCommandId.InternetEnabled,
                    LegacyCommandId.InternetTeleporter,
                    LegacyCommandId.EditInternetOptions,
                };
                var ownedWindowCount = window.OwnedWindows.Count;
                foreach (var command in onlineCommands)
                {
                    var item = historicMenu.FindCommandItem(command)!;
                    Assert.True(item.IsVisible);
                    Assert.False(item.IsEnabled);
                    Assert.Contains("online", ToolTip.GetTip(item)?.ToString() ?? string.Empty, StringComparison.OrdinalIgnoreCase);
                }
                Assert.Equal(ownedWindowCount, window.OwnedWindows.Count);

                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var frame = window.CaptureRenderedFrame();
                Assert.NotNull(frame);
                Assert.True(frame.PixelSize.Width >= 1_000);
                Assert.True(frame.PixelSize.Height >= 600);
                AssertChartPixels(frame, populationChart, window, 63, 142, 52, "population");
                AssertChartPixels(frame, energyChart, window, 216, 135, 36, "energy");
                var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR")
                    ?? Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
                Directory.CreateDirectory(directory);
                using var stream = File.Create(Path.Combine(directory, "graph-inline-history.png"));
                frame.Save(stream, new PngBitmapEncoderOptions());
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static T? TryField<T>(object target, string name) where T : class =>
        target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)?.GetValue(target) as T;

    private static void AssertChartPixels(
        Bitmap bitmap,
        Control chart,
        Window window,
        byte red,
        byte green,
        byte blue,
        string label)
    {
        var origin = chart.TranslatePoint(new Point(0, 0), window);
        Assert.True(origin.HasValue, $"The {label} chart was not attached to MainWindow.");
        var scaleX = bitmap.PixelSize.Width / window.Bounds.Width;
        var scaleY = bitmap.PixelSize.Height / window.Bounds.Height;
        var region = new PixelRect(
            Math.Clamp((int)Math.Floor(origin.Value.X * scaleX), 0, bitmap.PixelSize.Width - 1),
            Math.Clamp((int)Math.Floor(origin.Value.Y * scaleY), 0, bitmap.PixelSize.Height - 1),
            Math.Min(bitmap.PixelSize.Width, Math.Max(1, (int)Math.Ceiling(chart.Bounds.Width * scaleX))),
            Math.Min(bitmap.PixelSize.Height, Math.Max(1, (int)Math.Ceiling(chart.Bounds.Height * scaleY))));
        var series = ReadColour(bitmap, region, red, green, blue);
        Assert.True(series.Count > 8, $"The {label} chart did not render its series.");
        Assert.True(series.MaxY - series.MinY >= 6, $"The {label} chart series was flat.");
        Assert.True(ReadColour(bitmap, region, 221, 218, 209).Count > 8, $"The {label} chart axes were not rendered.");
    }

    private static (int Count, int MinY, int MaxY) ReadColour(
        Bitmap bitmap,
        PixelRect region,
        byte red,
        byte green,
        byte blue)
    {
        var stride = region.Width * 4;
        var length = stride * region.Height;
        var pointer = Marshal.AllocHGlobal(length);
        try
        {
            bitmap.CopyPixels(region, pointer, length, stride);
            var pixels = new byte[length];
            Marshal.Copy(pointer, pixels, 0, length);
            var count = 0;
            var minY = int.MaxValue;
            var maxY = int.MinValue;
            for (var offset = 0; offset < pixels.Length; offset += 4)
            {
                if (!Near(pixels[offset], pixels[offset + 1], pixels[offset + 2], red, green, blue)
                    && !Near(pixels[offset + 2], pixels[offset + 1], pixels[offset], red, green, blue))
                    continue;
                count++;
                var y = region.Y + offset / stride;
                minY = Math.Min(minY, y);
                maxY = Math.Max(maxY, y);
            }

            return (count, minY == int.MaxValue ? 0 : minY, maxY == int.MinValue ? 0 : maxY);
        }
        finally
        {
            Marshal.FreeHGlobal(pointer);
        }
    }

    private static bool Near(byte actualRed, byte actualGreen, byte actualBlue, byte red, byte green, byte blue) =>
        Math.Abs(actualRed - red) <= 18
        && Math.Abs(actualGreen - green) <= 18
        && Math.Abs(actualBlue - blue) <= 18;

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
