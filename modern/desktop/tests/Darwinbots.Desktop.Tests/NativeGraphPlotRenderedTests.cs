using System.Runtime.InteropServices;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
using Avalonia.Threading;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class NativeGraphPlotRenderedTests
{
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
