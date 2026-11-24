using System.Runtime.InteropServices;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class NativeGraphPlotRenderedTests
{
    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public async Task SpeciesFirstAppearingInAFeedIsAddedToTheOpenNativePlots(bool startsEmpty)
    {
        var root = Directory.CreateTempSubdirectory("darwinbots-native-graph-species-");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                const uint algaColor = 0xffca4635u;
                const uint hunterColor = 0xff324fbdu;
                static SpeciesGraphSampleSnapshot Population(string name, uint color, int value) =>
                    new(name, color, new Dictionary<string, double> { ["population"] = value });

                var alga = Population("Alga.txt", algaColor, 3);
                var snapshot = EngineSnapshot.Empty with { Tick = 1, NativeCycle = 0 };
                if (!startsEmpty)
                    snapshot = snapshot with
                    {
                        Population = 1,
                        Species = [new SpeciesSnapshot("Alga.txt", true, algaColor, 0, false)],
                        Organisms = [new OrganismSnapshot(0, 0, [20f, 20f], [0f, 0f], 1_000, 10, Vegetable: true)],
                    };
                var run = new LegacyGraphRecordingRun(root.FullName, "native-run");
                var suite = new GraphSuiteWindow(snapshot, run, readNativeSample: () =>
                    Task.FromResult<NativeGraphRecordingSample?>(new(1, 0, startsEmpty ? [] : [alga], [])));
                try
                {
                    suite.Show();
                    suite.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.Population);
                    var view = suite.FindControl<GraphSuiteView>("GraphView")!;
                    view.FindControl<Button>("OpenGraphButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    var graph = Assert.Single(suite.OwnedWindows.OfType<GraphSeriesWindow>());

                    NativeGraphRecordingSample Feed(ulong tick, int population)
                    {
                        var hunter = Population("Hunter.txt", hunterColor, population);
                        return new(tick, (int)tick - 1, startsEmpty ? [hunter] : [alga, hunter], []);
                    }
                    suite.RecordNativeSamples([Feed(2, 0)], run.Generation);
                    Assert.DoesNotContain(suite.ViewModel.RunningGraphs, series => series.SpeciesName == "Hunter.txt");

                    suite.RecordNativeSamples([Feed(3, 7), Feed(4, 8)], run.Generation);
                    var hunter = Assert.Single(suite.ViewModel.RunningGraphs.Where(series => series.SpeciesName == "Hunter.txt"));
                    Assert.Equal(hunterColor, hunter.Color);
                    Assert.Equal(new[] { new GraphSample(1, 0), new GraphSample(2, 0), new GraphSample(3, 7), new GraphSample(4, 8) },
                        hunter.Samples);
                    Assert.Equal(startsEmpty ? 1 : 2, suite.ViewModel.RunningGraphs.Count);
                    Assert.Contains("Population · Hunter.txt,,4,8\r\n", suite.ViewModel.ExportCsv());

                    await Dispatcher.UIThread.InvokeAsync(() => { }, DispatcherPriority.Background);
                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    AssertPlotSpansWidth(graph, graph.FindControl<GraphSeriesChart>("SeriesChart")!);
                    AssertPlotSpansWidth(suite, view.FindControl<GraphSeriesChart>("SeriesChart")!);

                    graph.Close();
                    Assert.Empty(suite.ViewModel.RunningGraphs);
                }
                finally { suite.Close(); }
            }, CancellationToken.None);
        }
        finally { root.Delete(recursive: true); }
    }

    [Fact]
    public async Task ResetAllPromptsAndFeedsOneSharedPointWhileIndividualResetOnlyClears()
    {
        var root = Directory.CreateTempSubdirectory("darwinbots-native-graph-reset-");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                const ulong tick = 8;
                const int cycle = 7;
                const uint color = 0xff324fbdu;
                var first = new SpeciesGraphSampleSnapshot("Zulu.txt", color,
                    new Dictionary<string, double> { ["population"] = 1, ["species_diversity"] = 2 });
                var second = new SpeciesGraphSampleSnapshot("Alpha.txt", color,
                    new Dictionary<string, double> { ["population"] = 1, ["species_diversity"] = 3 });
                var openingSample = new NativeGraphRecordingSample(tick, cycle, [first, second], []);
                var snapshot = EngineSnapshot.Empty with { Tick = tick, NativeCycle = cycle, Population = 2 };
                var run = new LegacyGraphRecordingRun(root.FullName, "native-run");
                var resetFeeds = 0;
                var suite = new GraphSuiteWindow(snapshot, run,
                    readNativeSample: () => Task.FromResult<NativeGraphRecordingSample?>(openingSample),
                    consumeNativeResetSample: consume =>
                    {
                        resetFeeds++;
                        consume(openingSample);
                        return Task.CompletedTask;
                    });
                try
                {
                    suite.Show();
                    suite.ViewModel.ConfigureCustomMetric(1, "Custom test", "7");
                    var view = suite.FindControl<GraphSuiteView>("GraphView")!;
                    var open = view.FindControl<Button>("OpenGraphButton")!;
                    suite.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.Population);
                    open.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    var populationChart = Assert.Single(suite.OwnedWindows.OfType<GraphSeriesWindow>());
                    var populationSeries = Assert.Single(suite.ViewModel.RunningGraphs,
                        series => series.Metric == GraphMetric.Population && series.SpeciesName == first.Name);
                    foreach (var metric in new[] { GraphMetric.SpeciesDiversity, GraphMetric.CustomizableGraph1 })
                    {
                        suite.ViewModel.SelectedChoice = GraphCatalog.For(metric);
                        open.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    }

                    populationChart.FindControl<Button>("UpdateGraphNowButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    Assert.Equal(2, populationSeries.Samples.Count);

                    view.FindControl<Button>("ResetGraphsButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    var prompt = Assert.Single(suite.OwnedWindows.OfType<LegacyConfirmationWindow>());
                    Assert.Contains(prompt.GetVisualDescendants().OfType<TextBlock>(),
                        text => text.Text == "Are you sure you want to reset all graphs?");
                    prompt.GetVisualDescendants().OfType<Button>().Single(button => Equals(button.Content, "CANCEL"))
                        .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    await Dispatcher.UIThread.InvokeAsync(() => { }, DispatcherPriority.Background);
                    Assert.Equal(0, resetFeeds);
                    Assert.Equal(2, populationSeries.Samples.Count);

                    suite.ViewModel.SelectedChoice = suite.ViewModel.Choices.Single(choice => choice.Action == GraphMenuAction.ResetAllGraphs);
                    open.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    prompt = Assert.Single(suite.OwnedWindows.OfType<LegacyConfirmationWindow>());
                    prompt.GetVisualDescendants().OfType<Button>().Single(button => Equals(button.Content, "OK"))
                        .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    await Dispatcher.UIThread.InvokeAsync(() => { }, DispatcherPriority.Background);

                    Assert.Equal(1, resetFeeds);
                    Assert.Equal(new GraphSample(tick, 1), Assert.Single(populationSeries.Samples));
                    var diversity = Assert.Single(suite.ViewModel.RunningGraphs.Where(series => series.Metric == GraphMetric.SpeciesDiversity));
                    Assert.Equal(first.Name, diversity.SpeciesName);
                    Assert.Equal(new GraphSample(tick, 2), Assert.Single(diversity.Samples));
                    var custom = Assert.Single(suite.ViewModel.RunningGraphs.Where(series => series.Metric == GraphMetric.CustomizableGraph1));
                    Assert.Null(custom.SpeciesName);
                    Assert.Empty(custom.Samples);

                    populationChart.FindControl<Button>("ResetGraphButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    Assert.Empty(populationSeries.Samples);
                    Assert.Equal(1, resetFeeds);
                }
                finally { suite.Close(); }
            }, CancellationToken.None);
        }
        finally { root.Delete(recursive: true); }
    }

    [Fact]
    public async Task RepeatedPausedUpdatesDrawAHorizontalSeriesInBothPlots()
    {
        var root = Directory.CreateTempSubdirectory("darwinbots-native-graph-plot-");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                const ulong tick = 34;
                const int cycle = 33;
                const uint color = 0xff324fbdu;
                var species = new SpeciesGraphSampleSnapshot("Alga.txt", color,
                    new Dictionary<string, double> { ["population"] = 1 });
                var snapshot = EngineSnapshot.Empty with
                {
                    Tick = tick, NativeCycle = cycle, Population = 1,
                    Species = [new SpeciesSnapshot("Alga.txt", true, color, 0, false)],
                    Organisms = [new OrganismSnapshot(0, 0, [20f, 20f], [0f, 0f], 1_000, 10, Vegetable: true)],
                };
                var run = new LegacyGraphRecordingRun(root.FullName, "native-run");
                var suite = new GraphSuiteWindow(snapshot, run, readNativeSample: () =>
                    Task.FromResult<NativeGraphRecordingSample?>(new(tick, cycle, [species], [])));
                try
                {
                    suite.Show();
                    suite.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.Population);
                    var view = suite.FindControl<GraphSuiteView>("GraphView")!;
                    view.FindControl<Button>("OpenGraphButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    var graph = Assert.Single(suite.OwnedWindows.OfType<GraphSeriesWindow>());
                    var series = Assert.Single(suite.ViewModel.RunningGraphs);
                    Assert.Single(series.Samples);
                    await Dispatcher.UIThread.InvokeAsync(() => { }, DispatcherPriority.Background);
                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                    var update = graph.FindControl<Button>("UpdateGraphNowButton")!;
                    for (var updateIndex = 0; updateIndex < 4; updateIndex++)
                        update.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    Assert.Equal(Enumerable.Repeat(new GraphSample(tick, 1), 5), series.Samples);
                    await Dispatcher.UIThread.InvokeAsync(() => { }, DispatcherPriority.Background);
                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                    AssertPlotSpansWidth(graph, graph.FindControl<GraphSeriesChart>("SeriesChart")!);
                    AssertPlotSpansWidth(suite, view.FindControl<GraphSeriesChart>("SeriesChart")!);
                }
                finally { suite.Close(); }
            }, CancellationToken.None);
        }
        finally { root.Delete(recursive: true); }
    }

    private static void AssertPlotSpansWidth(Window window, GraphSeriesChart chart)
    {
        using var frame = Assert.IsAssignableFrom<Bitmap>(window.CaptureRenderedFrame());
        var origin = chart.TranslatePoint(new Point(0, 0), window)!.Value;
        var scaleX = frame.PixelSize.Width / window.Bounds.Width;
        var scaleY = frame.PixelSize.Height / window.Bounds.Height;
        var left = (int)Math.Floor(origin.X * scaleX);
        var top = (int)Math.Floor(origin.Y * scaleY);
        var width = Math.Min(frame.PixelSize.Width - left, (int)Math.Floor(chart.Bounds.Width * scaleX));
        var height = Math.Min(frame.PixelSize.Height - top, (int)Math.Floor(chart.Bounds.Height * scaleY));
        Assert.True(width > 20 && height > 20);
        var region = new PixelRect(left, top, width, height);
        var stride = width * 4;
        var pixels = new byte[stride * height];
        var buffer = GCHandle.Alloc(pixels, GCHandleType.Pinned);
        try { frame.CopyPixels(region, buffer.AddrOfPinnedObject(), pixels.Length, stride); }
        finally { buffer.Free(); }

        var firstColumn = width;
        var lastColumn = -1;
        for (var index = 0; index < pixels.Length; index += 4)
        {
            if (Math.Abs(pixels[index + 1] - 0x4f) > 16
                || Math.Abs(Math.Min(pixels[index], pixels[index + 2]) - 0x32) > 16
                || Math.Abs(Math.Max(pixels[index], pixels[index + 2]) - 0xbd) > 16) continue;
            var column = index % stride / 4;
            firstColumn = Math.Min(firstColumn, column);
            lastColumn = Math.Max(lastColumn, column);
        }
        Assert.True(lastColumn - firstColumn >= width / 2,
            $"The {window.Title} graph collapsed repeated samples to a single horizontal position.");
    }
}
