using System.Reflection;
using System.Runtime.InteropServices;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Avalonia.Platform;
using Avalonia.Threading;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class NativeScenePaletteRenderedTests
{
    private const BindingFlags InternalInstance = BindingFlags.Instance | BindingFlags.NonPublic;
    private static readonly EngineSnapshot Seed42 = NativeSnapshotParser.Parse("""
        {"tick":10,"native_cycle":9,"organisms":[],
         "native_scene_palette":{"chart_argb":4293328884,"arena_argb":4278273318}}
        """, "CPU");
    private static readonly EngineSnapshot OlderNative = NativeSnapshotParser.Parse("""
        {"tick":10,"native_cycle":9,"organisms":[]}
        """, "CPU");
    private static readonly EngineSnapshot Restarted = Seed42 with
    {
        NativeScenePalette = new(0xffd1eaf5, 0xff123154),
    };

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public async Task ArenaUsesTheNativePaletteOrLaunchColorsAndKeepsThePortablePreference(bool launchOldColors)
    {
        Assert.Equal(new NativeScenePaletteSnapshot(0xffe6fff4, 0xff014526), Seed42.NativeScenePalette);
        Assert.Null(OlderNative.NativeScenePalette);

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var viewport = new WorldViewport { OldColors = !launchOldColors };
            typeof(WorldViewport).GetProperty("NativeOldColors", InternalInstance)!.SetValue(viewport, launchOldColors);
            var window = new Window { Width = 180, Height = 120, Content = viewport };
            try
            {
                window.Show();
                await AssertArena(Seed42, launchOldColors ? 0xff000040u : 0xff014526u);
                await AssertArena(Restarted, launchOldColors ? 0xff000040u : 0xff123154u);
                await AssertArena(OlderNative, launchOldColors ? 0xff000040u : 0xfff9f8f2u);
                await AssertArena(Seed42 with { NativeCycle = null }, launchOldColors ? 0xfff9f8f2u : 0xff000040u);
            }
            finally { window.Close(); }

            async Task AssertArena(EngineSnapshot snapshot, uint argb)
            {
                viewport.SetSnapshot(snapshot);
                Assert.Equal(Color.FromUInt32(argb), await RenderedColor(window, new Point(5, 5)));
            }
        }, CancellationToken.None);
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public async Task OpenChartBackdropsFollowCommittedWorldsOutsideTheHistoryInterval(bool launchOldColors)
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-native-scene-palette-");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var owner = new Window();
                try
                {
                    owner.Show();
                    var registry = new PersistentToolWindowRegistry(owner);
                    var recording = new LegacyGraphRecordingRun(profile.FullName, "palette-test");
                    var suite = registry.ShowOrActivate("graph-suite",
                        () => new GraphSuiteWindow(Seed42, recording, nativeOldColors: launchOldColors));
                    suite.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.Population);
                    var view = suite.FindControl<GraphSuiteView>("GraphView")!;
                    view.FindControl<Button>("OpenGraphButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    var graph = Assert.Single(suite.OwnedWindows.OfType<GraphSeriesWindow>());
                    var plot = graph.FindControl<Border>("ChartPlotFrame")!;
                    var saveBackdrop = graph.FindControl<Border>("SaveGraphBackdrop")!;
                    Assert.True(graph.FindControl<CheckBox>("SaveGraphData")!.IsVisible);
                    var initial = launchOldColors ? 0xffffffffu : 0xffe6fff4u;
                    await AssertNativeChart(initial);

                    registry.PublishSnapshot(Restarted, updateGraphs: false, updateGraphPalette: false);
                    await AssertNativeChart(initial);
                    registry.PublishSnapshot(Restarted, updateGraphs: false);
                    await AssertNativeChart(launchOldColors ? 0xffffffffu : 0xffd1eaf5u);

                    typeof(PersistentToolWindowRegistry).GetMethod("LoadGraphWorld", InternalInstance)!
                        .Invoke(registry, [Seed42, null]);
                    Assert.Same(graph, Assert.Single(suite.OwnedWindows.OfType<GraphSeriesWindow>()));
                    await AssertNativeChart(initial);

                    registry.PublishSnapshot(Seed42 with { NativeCycle = null }, updateGraphs: false);
                    Assert.Null(saveBackdrop.Background);
                    Assert.Equal(Color.Parse("#F4F1E8"), Assert.IsAssignableFrom<ISolidColorBrush>(plot.Background).Color);
                    Assert.Equal(Color.Parse("#F3F0E8"), await RenderedColor(graph, new Point(4, 4)));

                    async Task AssertNativeChart(uint argb)
                    {
                        var expected = Color.FromUInt32(argb);
                        Assert.Equal(expected, await RenderedColor(graph, new Point(4, 4)));
                        var insidePlot = plot.TranslatePoint(new Point(4, 4), graph)!.Value;
                        Assert.Equal(expected, await RenderedColor(graph, insidePlot));
                        Assert.Equal(expected, Assert.IsAssignableFrom<ISolidColorBrush>(saveBackdrop.Background).Color);
                    }
                }
                finally { owner.Close(); }
            }, CancellationToken.None);
        }
        finally { profile.Delete(recursive: true); }
    }

    private static async Task<Color> RenderedColor(Window window, Point point)
    {
        await Dispatcher.UIThread.InvokeAsync(() => { }, DispatcherPriority.Background);
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        using var frame = Assert.IsAssignableFrom<Bitmap>(window.CaptureRenderedFrame());
        using var rgba = new WriteableBitmap(frame.PixelSize, frame.Dpi, PixelFormat.Rgba8888, AlphaFormat.Unpremul);
        using var pixels = rgba.Lock();
        frame.CopyPixels(pixels);
        var x = (int)Math.Floor(point.X * frame.PixelSize.Width / window.Bounds.Width);
        var y = (int)Math.Floor(point.Y * frame.PixelSize.Height / window.Bounds.Height);
        var channels = new byte[4];
        Marshal.Copy(pixels.Address + y * pixels.RowBytes + x * 4, channels, 0, channels.Length);
        return Color.FromArgb(channels[3], channels[0], channels[1], channels[2]);
    }
}
