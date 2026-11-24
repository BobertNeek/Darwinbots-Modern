using System.Reflection;
using System.Runtime.InteropServices;
using System.Text.Json.Nodes;
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
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
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

    [Fact]
    public async Task ExplicitFalseOverridesTheOriginalOldColorsRecordAndPreservesOtherPreferences()
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-old-colors-profile-");
        try
        {
            var legacy = profile.CreateSubdirectory("Original install");
            var records = new string[8];
            Array.Fill(records, "#FALSE#");
            records[7] = "#TRUE#";
            var legacyPath = Path.Combine(legacy.FullName, "Global.gset");
            await File.WriteAllLinesAsync(legacyPath, records);
            var profilePath = Path.Combine(profile.FullName, "global-settings.json");
            var initial = new JsonObject
            {
                ["main"] = new JsonObject { ["mainDirectory"] = legacy.FullName, ["keepGraphsUpdatingOnClose"] = true },
            };
            await File.WriteAllTextAsync(profilePath, initial.ToJsonString());
            var storage = new AvaloniaDesktopStorageService(profile.FullName);
            Assert.True(storage.LoadOldColorsPreference());

            var settings = Assert.IsType<GlobalSettings>(storage.LoadGlobalSettings());
            await storage.SaveGlobalSettingsAsync(settings with { Main = settings.Main with { OldColors = false } });
            Assert.False(storage.LoadOldColorsPreference());
            var saved = Assert.IsType<GlobalSettings>(storage.LoadGlobalSettings());
            Assert.False(saved.Main.OldColors);
            Assert.True(saved.Main.KeepGraphsUpdatingOnClose);
            Assert.Equal(legacy.FullName, saved.Main.MainDirectory);
            Assert.Equal(records, await File.ReadAllLinesAsync(legacyPath));
        }
        finally { profile.Delete(recursive: true); }
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public async Task SavingOldColorsUpdatesPortableImmediatelyAndNativeOnRelaunch(bool native)
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-old-colors-runtime-");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var storage = new AvaloniaDesktopStorageService(profile.FullName);
                IEngineClientFactory factory = native ? NativeEngineClientFactory.Instance : new AuditEngineClientFactory();
                var world = new WorldSetupOptions
                {
                    Backend = "Cpu", Seed = 44, PopulationCapacity = 64, WorldWidth = 1_600, WorldHeight = 1_200,
                    Species = [new("Color probe", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 0)],
                };
                var windows = new List<Window>();
                using var opened = Window.WindowOpenedEvent.AddClassHandler<Window>((window, _) => windows.Add(window));
                var runtime = new MainWindow([], world, factory, storage);
                try
                {
                    runtime.Show();
                    await WaitFor(() => Model(runtime).Status.StartsWith("WORLD READY", StringComparison.Ordinal));
                    Assert.Equal(native, Model(runtime).NativeCycle.HasValue);
                    Assert.Equal(native, runtime.FindControl<WorldViewport>("Viewport")!.OldColors);
                    runtime.FindControl<LegacyMenuBar>("HistoricMenuBar")!
                        .FindCommandItem(LegacyCommandId.EditGlobalSettings)!
                        .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => runtime.OwnedWindows.OfType<GlobalSettingsWindow>().Any());
                    var dialog = Assert.Single(runtime.OwnedWindows.OfType<GlobalSettingsWindow>());
                    var oldColors = dialog.FindControl<CheckBox>("OldColors")!;
                    Assert.Equal(native, oldColors.IsChecked);
                    oldColors.IsChecked = !native;
                    Click(dialog, "ApplyButton");
                    await WaitFor(() => Model(runtime).Status == "GLOBAL SETTINGS SAVED");
                    Assert.Equal(!native, storage.LoadOldColorsPreference());
                    Assert.True(runtime.FindControl<WorldViewport>("Viewport")!.OldColors);
                    if (!native) return;

                    Click(runtime, "ResetButton");
                    await WaitFor(() => windows.OfType<MainWindow>().Count() == 2);
                    var replacement = Assert.Single(windows.OfType<MainWindow>(), window => window != runtime);
                    await WaitFor(() => Model(replacement).NativeCycle.HasValue);
                    Assert.True(replacement.FindControl<WorldViewport>("Viewport")!.OldColors);

                    var setup = new SetupWindow(new AvaloniaDesktopStorageService(profile.FullName));
                    WorldSetupOptions? reopened = null;
                    setup.WorldCreated += options => reopened = options;
                    Click(setup, "CreateWorldButton");
                    Assert.False(Assert.IsType<WorldSetupOptions>(reopened).GlobalSettings.Main.OldColors);
                    setup.Close();
                    var relaunched = new MainWindow([], world with { GlobalSettings = reopened!.GlobalSettings }, factory, storage);
                    relaunched.Show();
                    await WaitFor(() => Model(relaunched).NativeCycle.HasValue);
                    Assert.False(relaunched.FindControl<WorldViewport>("Viewport")!.OldColors);
                }
                finally
                {
                    foreach (var window in windows.Where(window => window.IsVisible).Reverse().ToArray())
                        window.Close();
                }
            }, CancellationToken.None);
        }
        finally { profile.Delete(recursive: true); }

        static MainWindowViewModel Model(MainWindow window) => Assert.IsType<MainWindowViewModel>(window.DataContext);
        static void Click(Window window, string name) =>
            window.FindControl<Button>(name)!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

        static async Task WaitFor(Func<bool> condition)
        {
            var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(8);
            while (!condition() && DateTime.UtcNow < deadline) await Task.Delay(10);
            Assert.True(condition());
        }
    }

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
