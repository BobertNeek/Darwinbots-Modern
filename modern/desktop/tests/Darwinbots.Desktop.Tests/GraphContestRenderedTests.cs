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
    public async Task PopulationGraphSavesOnlyAtTheOriginalDiskSegmentBoundaries()
    {
        var root = Directory.CreateTempSubdirectory("darwinbots-graph-auto-save-");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                const string stamp = "fixed-run-stamp";
                var directory = Path.Combine(root.FullName, stamp);
                var firstPath = Path.Combine(directory, "Populations1.gsave");
                var secondPath = Path.Combine(directory, "Populations2.gsave");
                SpeciesGraphSampleSnapshot Point(int value) =>
                    new("Alga.txt", 0xff123456u, new Dictionary<string, double> { ["population"] = value });
                var history = Enumerable.Range(1, 2_000).Select(value =>
                    new HistorySampleSnapshot((ulong)value, value, 0, 0, 0, 0, 0) { SpeciesGraphs = [Point(value)] }).ToArray();
                EngineSnapshot Through(int count) =>
                    EngineSnapshot.Empty with { Tick = (ulong)count, NativeCycle = count, Population = count, History = history[..count] };

                var suite = new GraphSuiteWindow(Through(1), new LegacyGraphRecordingRun(root.FullName, stamp));
                try
                {
                    suite.Show();
                    var view = suite.FindControl<GraphSuiteView>("GraphView")!;
                    suite.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.Population);
                    view.FindControl<Button>("OpenGraphButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    var graph = Assert.Single(suite.OwnedWindows.OfType<GraphSeriesWindow>());
                    await FlushGraphPresentationAsync();
                    var save = graph.FindControl<CheckBox>("SaveGraphData")!;
                    Assert.True(save.IsVisible);
                    Assert.False(save.IsChecked);
                    Assert.Equal("Save on d.p. 999", save.Content);
                    var point = save.TranslatePoint(new Point(save.Bounds.Width / 2, save.Bounds.Height / 2), graph)!.Value;
                    graph.MouseDown(point, MouseButton.Left, RawInputModifiers.None);
                    graph.MouseUp(point, MouseButton.Left, RawInputModifiers.None);
                    Assert.True(save.IsChecked);

                    suite.SetSnapshot(Through(998));
                    await FlushGraphPresentationAsync();
                    Assert.False(File.Exists(firstPath));
                    suite.SetSnapshot(Through(999));
                    await WaitFor(() => File.Exists(firstPath), () => "Population graph did not save data point 999.");
                    var first = File.ReadAllText(firstPath);
                    var firstSegment = Assert.Single(LegacyGraphJoin.Parse([first]));
                    Assert.Equal("normal", firstSegment.Title);
                    var alga = Assert.Single(firstSegment.Robots);
                    Assert.Equal("Alga", alga.Name);
                    Assert.Equal(0x00563412u, alga.Color);
                    Assert.Equal(Enumerable.Range(1, 999).Select(value => (double)value), alga.Values);

                    var repeat = Through(999);
                    var repeatedHistory = repeat.History.ToArray();
                    repeatedHistory[^1] = repeatedHistory[^1] with { Population = 99_999, SpeciesGraphs = [Point(99_999)] };
                    suite.SetSnapshot(repeat with { Population = 99_999, History = repeatedHistory });
                    await FlushGraphPresentationAsync();
                    Assert.Equal(first, File.ReadAllText(firstPath));
                    Assert.False(File.Exists(secondPath));

                    suite.SetSnapshot(Through(2_000));
                    await WaitFor(() => File.Exists(secondPath), () => "Population graph did not save its second segment.");
                    var second = Assert.Single(LegacyGraphJoin.Parse([File.ReadAllText(secondPath)]));
                    Assert.Equal(Enumerable.Range(1_002, 999).Select(value => (double)value), Assert.Single(second.Robots).Values);
                    Assert.Equal(first, File.ReadAllText(firstPath));
                    Assert.Equal(new[] { firstPath, secondPath }, Directory.GetFiles(directory, "*.gsave").Order(StringComparer.Ordinal));
                }
                finally { suite.Close(); }
            }, CancellationToken.None);
        }
        finally { root.Delete(recursive: true); }
    }

    [Fact]
    public async Task NativeGraphRecordingUsesCommittedSamplesAndDoesNotSeedAfterChartReset()
    {
        var root = Directory.CreateTempSubdirectory("darwinbots-native-graph-sampling-");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                const string stamp = "native-run";
                var directory = Path.Combine(root.FullName, stamp);
                var firstPath = Path.Combine(directory, "Populations1.gsave");
                var secondPath = Path.Combine(directory, "Populations2.gsave");
                var run = new LegacyGraphRecordingRun(root.FullName, stamp);
                SpeciesGraphSampleSnapshot Alga(int population) =>
                    new("Alga.txt", 0xff123456u, new Dictionary<string, double> { ["population"] = population });
                NativeGraphRecordingSample Sample(int tick, int population) => new((ulong)tick, tick, [Alga(population)], []);
                HistorySampleSnapshot History(int tick) =>
                    new((ulong)tick, 99_999, 0, 0, 0, 0, 0) { SpeciesGraphs = [Alga(99_999)] };
                var probe = Sample(1, 1);
                var samplesRequested = new List<GraphSamplingSettings?>();
                var opening = EngineSnapshot.Empty with
                {
                    Tick = 1, NativeCycle = 1, Population = 99_999, History = [History(1)],
                    Species = [new SpeciesSnapshot("Alga.txt", true, 0xff123456u, 0, false)],
                    Organisms = [new OrganismSnapshot(0, 0, [20f, 20f], [0f, 0f], 1_000, 10, Vegetable: true)],
                };
                var suite = new GraphSuiteWindow(opening, run, consumeNativeSample: (settings, consume) =>
                {
                    samplesRequested.Add(settings);
                    consume(probe);
                    return Task.CompletedTask;
                });
                try
                {
                    suite.Show();
                    suite.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.Population);
                    suite.FindControl<GraphSuiteView>("GraphView")!.FindControl<Button>("OpenGraphButton")!
                        .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    var graph = Assert.Single(suite.OwnedWindows.OfType<GraphSeriesWindow>());
                    var series = Assert.Single(suite.ViewModel.RunningGraphs);
                    await FlushGraphPresentationAsync();
                    Assert.Equal(new GraphSamplingSettings(), Assert.Single(samplesRequested));
                    Assert.True(series.UsesSampleOrder);
                    Assert.Equal(new GraphSample(1, 1), Assert.Single(series.Samples));
                    var save = graph.FindControl<CheckBox>("SaveGraphData")!;
                    Assert.True(save.IsVisible);
                    var point = save.TranslatePoint(new Point(save.Bounds.Width / 2, save.Bounds.Height / 2), graph)!.Value;
                    graph.MouseDown(point, MouseButton.Left, RawInputModifiers.None);
                    graph.MouseUp(point, MouseButton.Left, RawInputModifiers.None);
                    Assert.True(save.IsChecked);

                    suite.SetSnapshot(opening with { Tick = 999, NativeCycle = 999, History = [History(1), History(999)] });
                    Assert.Equal(new GraphSample(1, 1), Assert.Single(series.Samples));
                    suite.RecordNativeSamples(Enumerable.Range(2, 997).Select(tick => Sample(tick, tick)).ToArray(), run.Generation);
                    Assert.False(File.Exists(firstPath));
                    suite.RecordNativeSamples([Sample(999, 999)], run.Generation);
                    await WaitFor(() => File.Exists(firstPath), () => "The first 999 native graph samples did not produce a segment.");
                    Assert.Equal(Enumerable.Range(1, 999).Select(value => (double)value), Read(firstPath));
                    Assert.Equal(Enumerable.Range(1, 999).Select(value => new GraphSample((ulong)value, value)), series.Samples);
                    var csv = suite.ViewModel.ExportCsv();
                    Assert.Contains(",999,999\r\n", csv);
                    Assert.DoesNotContain("99999", csv);

                    probe = Sample(999, 99_999);
                    graph.FindControl<Button>("ResetGraphButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    await FlushGraphPresentationAsync();
                    Assert.Equal(new GraphSamplingSettings?[] { new(), null }, samplesRequested);
                    Assert.True(save.IsChecked);
                    Assert.Empty(series.Samples);
                    suite.RecordNativeSamples(Enumerable.Range(1_000, 998).Select(tick => Sample(tick, tick)).ToArray(), run.Generation);
                    Assert.False(File.Exists(secondPath));
                    suite.RecordNativeSamples([Sample(1_998, 1_998)], run.Generation);
                    await WaitFor(() => File.Exists(secondPath), () => "The chart reset did not start a fresh native segment.");
                    Assert.Equal(Enumerable.Range(1_000, 999).Select(value => (double)value), Read(secondPath));
                    Assert.Equal(new GraphSamplingSettings?[] { new(), null }, samplesRequested);

                    var imported = Assert.Single(suite.ViewModel.ImportLegacyGsave([File.ReadAllText(firstPath)]));
                    suite.SetSnapshot(opening with { Tick = 2_003, NativeCycle = 2_003, History = [History(2_003)] });
                    suite.RecordNativeSamples(Enumerable.Range(1_999, 5).Select(tick => Sample(tick, tick)).ToArray(), run.Generation);
                    Assert.Equal(Enumerable.Range(1_004, 1_000).Select(value => new GraphSample((ulong)value, value)), series.Samples);
                    Assert.False(imported.UsesSampleOrder);
                    Assert.Equal(Enumerable.Range(1, 999).Select(value => (double)value), imported.Samples.Select(sample => sample.Value));

                    suite.RecordNativeSamples([new NativeGraphRecordingSample(2_004, 2_004, [], [])], run.Generation);
                    Assert.Equal(1_000, series.Samples.Count);
                    Assert.Equal(new GraphSample(2_004, 0), series.Samples[^1]);

                    suite.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.CustomizableGraph1);
                    suite.FindControl<GraphSuiteView>("GraphView")!.FindControl<Button>("OpenGraphButton")!
                        .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    var custom = Assert.Single(suite.ViewModel.RunningGraphs.Where(value => value.UsesSampleOrder
                        && value.Metric == imported.Metric));
                    run.BeginRun("new-native-run");
                    probe = Sample(1, 42);
                    suite.SetSnapshot(opening);
                    Assert.Equal(new GraphSample(1, 42), Assert.Single(series.Samples));
                    Assert.Equal(new GraphSample(1, 42), Assert.Single(custom.Samples));
                    Assert.Equal(Enumerable.Range(1, 999).Select(value => (double)value), imported.Samples.Select(sample => sample.Value));
                }
                finally { suite.Close(); }

                static IReadOnlyList<double> Read(string path) =>
                    Assert.Single(Assert.Single(LegacyGraphJoin.Parse([File.ReadAllText(path)])).Robots).Values;
            }, CancellationToken.None);
        }
        finally { root.Delete(recursive: true); }
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public async Task NativeChartUpdatesAtTheSameTickAndKeepsRecordingWhenHidden(bool keepUpdatingOnClose)
    {
        var root = Directory.CreateTempSubdirectory("darwinbots-native-graph-gestures-");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                const string stamp = "native-gestures";
                var run = new LegacyGraphRecordingRun(root.FullName, stamp);
                run.Graph(GraphMetric.Population).SetEnabled(true);
                var segmentPath = Path.Combine(root.FullName, stamp, "Populations1.gsave");
                SpeciesGraphSampleSnapshot Alga(int value) =>
                    new("Alga.txt", 0xff123456u, new Dictionary<string, double> { ["population"] = value });
                NativeGraphRecordingSample Sample(int tick, int value) => new((ulong)tick, tick, [Alga(value)], []);
                var current = Sample(10, 1);
                var initial = EngineSnapshot.Empty with
                {
                    Tick = 10, NativeCycle = 10, Population = 1,
                    Species = [new SpeciesSnapshot("Alga.txt", true, 0xff123456u, 0, false)],
                    Organisms = [new OrganismSnapshot(0, 0, [20f, 20f], [0f, 0f], 1_000, 10, Vegetable: true)],
                };
                var suite = new GraphSuiteWindow(initial, run,
                    readNativeSample: () => Task.FromResult<NativeGraphRecordingSample?>(current),
                    keepUpdatingOnClose: keepUpdatingOnClose);
                try
                {
                    suite.Show();
                    suite.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.Population);
                    var open = suite.FindControl<GraphSuiteView>("GraphView")!.FindControl<Button>("OpenGraphButton")!;
                    open.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    var graph = Assert.Single(suite.OwnedWindows.OfType<GraphSeriesWindow>());
                    var series = Assert.Single(suite.ViewModel.RunningGraphs);
                    await FlushGraphPresentationAsync();
                    Assert.True(graph.FindControl<CheckBox>("SaveGraphData")!.IsChecked);
                    Assert.Equal(new GraphSample(10, 1), Assert.Single(series.Samples));
                    var update = graph.FindControl<Button>("UpdateGraphNowButton")!;
                    Assert.True(update.IsVisible);
                    var initialVisibleCount = series.Samples.Count;
                    var closed = false;
                    graph.Closed += (_, _) => closed = true;

                    current = Sample(10, 2);
                    update.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    await WaitFor(() => series.Samples.LastOrDefault()?.Value == 2d, () => "Update Now did not append the first point.");
                    current = Sample(10, 3);
                    update.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    await WaitFor(() => series.Samples.LastOrDefault()?.Value == 3d, () => "Update Now discarded a second point at the same tick.");
                    Assert.Equal(initialVisibleCount + 2, series.Samples.Count);
                    Assert.Equal(new[] { 2d, 3d }, series.Samples.TakeLast(2).Select(point => point.Value));

                    suite.RecordNativeSamples(Enumerable.Range(4, 995).Select(value => Sample(value + 7, value)).ToArray(), run.Generation);
                    Assert.False(File.Exists(segmentPath));
                    if (keepUpdatingOnClose)
                    {
                        RequestWindowClose();
                        await FlushGraphPresentationAsync();
                        Assert.Empty(Prompts());
                    }
                    else await RespondToWindowCloseAsync("YES");
                    await WaitFor(() => !graph.IsVisible, () => "Keeping the graph active did not hide its window.");
                    Assert.False(closed);
                    Assert.Same(series, Assert.Single(suite.ViewModel.RunningGraphs));

                    current = Sample(1_006, 999);
                    suite.RecordNativeSamples([current], run.Generation);
                    await WaitFor(() => File.Exists(segmentPath), () => "The hidden graph did not finish saving its segment.");
                    var segment = Assert.Single(LegacyGraphJoin.Parse([File.ReadAllText(segmentPath)]));
                    Assert.Equal(Enumerable.Range(1, 999).Select(value => (double)value), Assert.Single(segment.Robots).Values);

                    open.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    await WaitFor(() => graph.IsVisible, () => "Opening Population did not reopen the hidden graph.");
                    Assert.Same(graph, Assert.Single(suite.OwnedWindows.OfType<GraphSeriesWindow>()));
                    Assert.Same(series, Assert.Single(suite.ViewModel.RunningGraphs));
                    Assert.True(graph.FindControl<CheckBox>("SaveGraphData")!.IsChecked);

                    if (keepUpdatingOnClose) graph.Close();
                    else await RespondToWindowCloseAsync("NO");
                    await WaitFor(() => closed, () => "The graph window did not close.");
                    Assert.Empty(suite.ViewModel.RunningGraphs);

                    void RequestWindowClose() =>
                        typeof(Window).GetMethod("HandleClosing", BindingFlags.Instance | BindingFlags.NonPublic)!
                            .Invoke(graph, [WindowCloseReason.WindowClosing]);

                    IEnumerable<LegacyConfirmationWindow> Prompts() =>
                        graph.OwnedWindows.Concat(suite.OwnedWindows).OfType<LegacyConfirmationWindow>();

                    async Task RespondToWindowCloseAsync(string response)
                    {
                        RequestWindowClose();
                        await WaitFor(() => Prompts().Any(), () => "Closing the graph did not ask whether to keep updating.");
                        await FlushGraphPresentationAsync();
                        var prompt = Assert.Single(Prompts());
                        Assert.Contains(prompt.GetVisualDescendants().OfType<TextBlock>(),
                            text => text.Text == "Would you like to keep updating this graph?");
                        var choice = Assert.Single(prompt.GetVisualDescendants().OfType<Button>(), button => Equals(button.Content, response));
                        choice.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    }
                }
                finally { suite.Close(); }
            }, CancellationToken.None);
        }
        finally { root.Delete(recursive: true); }
    }

    [Fact]
    public async Task LoadingASimulationRestoresTheOpenGraphsSaveCheckbox()
    {
        var root = Directory.CreateTempSubdirectory("darwinbots-graph-settings-");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var storage = new AuditStorageService();
                var setup = new WorldSetupOptions
                {
                    Backend = "Cpu",
                    PopulationCapacity = 16,
                    GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { MainDirectory = root.FullName } },
                    Species = [new SpeciesImport("Alga.txt", "start\nstop", true, 0xff123456, 1, 1_000, 0, false, 1)],
                };
                var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, storage);
                try
                {
                    window.Show();
                    await WaitFor(() => TryField<SimulationSession>(window, "_session")?.LatestSnapshot.Population == 1,
                        () => "The native world did not finish startup.");
                    var session = TryField<SimulationSession>(window, "_session")!;
                    window.FindControl<Button>("GraphsButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    var suite = Assert.Single(window.OwnedWindows.OfType<GraphSuiteWindow>());
                    suite.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.Population);
                    suite.FindControl<GraphSuiteView>("GraphView")!.FindControl<Button>("OpenGraphButton")!
                        .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    var graph = Assert.Single(suite.OwnedWindows.OfType<GraphSeriesWindow>());
                    await FlushGraphPresentationAsync();
                    var checkbox = graph.FindControl<CheckBox>("SaveGraphData")!;
                    Assert.False(checkbox.IsChecked);
                    var point = checkbox.TranslatePoint(new Point(checkbox.Bounds.Width / 2, checkbox.Bounds.Height / 2), graph)!.Value;
                    void Toggle()
                    {
                        graph.MouseDown(point, MouseButton.Left, RawInputModifiers.None);
                        graph.MouseUp(point, MouseButton.Left, RawInputModifiers.None);
                    }

                    Toggle();
                    Assert.True(checkbox.IsChecked);
                    var enabled = Assert.IsType<NativeGraphRecordingState>(await session.ReadNativeGraphRecordingAsync());
                    Assert.False(string.IsNullOrWhiteSpace(enabled.RunStamp));
                    var populationRow = Assert.Single(enabled.Graphs, row => row.GraphIndex == 0);
                    Assert.Equal(new NativeGraphRecordingRow(0, true, 0), populationRow);
                    var checkpoint = await session.SaveAsync();
                    Assert.True(checkpoint.AsSpan().StartsWith("DB3S"u8));

                    Toggle();
                    Assert.False(checkbox.IsChecked);
                    var disabled = Assert.IsType<NativeGraphRecordingState>(await session.ReadNativeGraphRecordingAsync());
                    Assert.False(Assert.Single(disabled.Graphs, row => row.GraphIndex == 0).Enabled);

                    storage.OpenedSimulation = checkpoint;
                    var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                    menu.FindCommandItem(LegacyCommandId.FileLoadSimulation)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    var model = Assert.IsType<Darwinbots.Desktop.ViewModels.MainWindowViewModel>(window.DataContext);
                    await WaitFor(() => model.Status == "LOADED", () => model.Status);
                    Assert.Same(graph, Assert.Single(suite.OwnedWindows.OfType<GraphSeriesWindow>()));
                    Assert.True(checkbox.IsChecked);
                    var loaded = Assert.IsType<NativeGraphRecordingState>(await session.ReadNativeGraphRecordingAsync());
                    Assert.Equal(enabled.RunStamp, loaded.RunStamp);
                    Assert.Equal(populationRow, Assert.Single(loaded.Graphs, row => row.GraphIndex == 0));
                    Assert.Empty(root.EnumerateFiles("*.gsave", SearchOption.AllDirectories));
                }
                finally { window.Close(); }
            }, CancellationToken.None);
        }
        finally { root.Delete(recursive: true); }
    }

    [Fact]
    public async Task NativeSimulationColdOpenRestoresGraphLayoutAndCustomQuery()
    {
        var root = Directory.CreateTempSubdirectory("darwinbots-native-graph-layout-");
        try
        {
            var savePath = Path.Combine(root.FullName, "graphs.sim");
            var setup = new WorldSetupOptions
            {
                Backend = "Cpu",
                PopulationCapacity = 16,
                GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { MainDirectory = root.FullName } },
            };
            var population = new NativeGraphLayoutRow(0, true, 900, 1_350);
            var closed = new NativeGraphLayoutRow(2, false, 2_400, 1_800);
            var custom = new NativeGraphLayoutRow(15, true, 4_500, 300_000);
            var query = new NativeGraphCustomQuery(1, "pop 7 mult 2 add");
            using (var source = new NativeEngineClient(setup))
            {
                source.UpdateNativeGraphLayout([population, closed, custom], [query]);
                await File.WriteAllBytesAsync(savePath, source.SaveLegacy());
            }

            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var player = new MainWindow([], setup with { LoadSavePath = savePath },
                    NativeEngineClientFactory.Instance, new AuditStorageService());
                try
                {
                    player.Show();
                    var model = Assert.IsType<Darwinbots.Desktop.ViewModels.MainWindowViewModel>(player.DataContext);
                    await WaitFor(() => model.Status == "LOADED", () => model.Status);
                    var suite = Assert.Single(player.OwnedWindows.OfType<GraphSuiteWindow>());
                    await FlushGraphPresentationAsync();
                    var charts = suite.OwnedWindows.OfType<GraphSeriesWindow>()
                        .ToDictionary(chart => TryField<GraphSeriesViewModel>(chart, "_graph")!.Metric);
                    Assert.Equal(new[] { GraphMetric.Population, GraphMetric.CustomizableGraph1 }, charts.Keys.Order());
                    var populationChart = charts[GraphMetric.Population];
                    var customChart = charts[GraphMetric.CustomizableGraph1];
                    Assert.Equal(query.Expression, Assert.Single(suite.ViewModel.CustomMetrics, slot => slot.Slot == 1).Expression);
                    Assert.Equal(query.Expression, TryField<GraphSeriesViewModel>(customChart, "_graph")!.CustomExpression);
                    AssertPosition(populationChart, population);
                    AssertPosition(customChart, custom);

                    var session = TryField<SimulationSession>(player, "_session")!;
                    var restored = await session.ReadNativeGraphLayoutAsync();
                    Assert.NotNull(restored);
                    Assert.Equal(population, Assert.Single(restored.Graphs, row => row.GraphIndex == 0));
                    Assert.Equal(closed, Assert.Single(restored.Graphs, row => row.GraphIndex == 2));
                    Assert.Equal(custom, Assert.Single(restored.Graphs, row => row.GraphIndex == 15));
                    Assert.Equal(query.Expression, restored.CustomQueries[0]);

                    var newPosition = new PixelPoint(populationChart.Position.X + 20, populationChart.Position.Y + 20);
                    populationChart.Position = newPosition;
                    await FlushGraphPresentationAsync();
                    var scale = populationChart.Screens.ScreenFromPoint(newPosition)!.Scaling;
                    var moved = population with
                    {
                        Left = (int)Math.Round(newPosition.X * 15 / scale),
                        Top = (int)Math.Round(newPosition.Y * 15 / scale),
                    };
                    var movedLayout = await session.ReadNativeGraphLayoutAsync();
                    Assert.Equal(moved, Assert.Single(movedLayout!.Graphs, row => row.GraphIndex == 0));

                    typeof(Window).GetMethod("HandleClosing", BindingFlags.Instance | BindingFlags.NonPublic)!
                        .Invoke(populationChart, [WindowCloseReason.WindowClosing]);
                    await WaitFor(() => populationChart.OwnedWindows.OfType<LegacyConfirmationWindow>().Any(),
                        () => "The restored graph did not ask whether to keep updating.");
                    var prompt = Assert.Single(populationChart.OwnedWindows.OfType<LegacyConfirmationWindow>());
                    await FlushGraphPresentationAsync();
                    Assert.Single(prompt.GetVisualDescendants().OfType<Button>(), button => Equals(button.Content, "NO"))
                        .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    await WaitFor(() => !populationChart.IsVisible, () => "NO did not close the restored graph.");

                    var saved = await session.ReadNativeGraphLayoutAsync();
                    Assert.NotNull(saved);
                    Assert.Equal(moved with { Visible = false }, Assert.Single(saved.Graphs, row => row.GraphIndex == 0));
                    Assert.Equal(closed, Assert.Single(saved.Graphs, row => row.GraphIndex == 2));
                    Assert.Equal(custom, Assert.Single(saved.Graphs, row => row.GraphIndex == 15));
                    Assert.True(customChart.IsVisible);
                }
                finally { player.Close(); }
            }, CancellationToken.None);
        }
        finally { root.Delete(recursive: true); }

        static void AssertPosition(GraphSeriesWindow chart, NativeGraphLayoutRow layout)
        {
            Assert.True(chart.IsVisible);
            var screen = chart.Screens.Primary;
            Assert.NotNull(screen);
            var workArea = screen.WorkingArea;
            var width = (int)Math.Round(chart.Bounds.Width * screen.Scaling);
            var height = (int)Math.Round(chart.Bounds.Height * screen.Scaling);
            var left = Math.Clamp((int)Math.Round(layout.Left!.Value * screen.Scaling / 15),
                workArea.X, workArea.X + Math.Max(0, workArea.Width - width));
            var top = Math.Clamp((int)Math.Round(layout.Top!.Value * screen.Scaling / 15),
                workArea.Y, workArea.Y + Math.Max(0, workArea.Height - height));
            Assert.Equal(new PixelPoint(left, top), chart.Position);
        }
    }

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
    public async Task MainWindowRoutesToRustBackedGraphAndContestResultsWithoutInternetCommands()
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
                Assert.DoesNotContain(historicMenu.TopLevelItems, item => Equals(item.Header, "Internet"));

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
