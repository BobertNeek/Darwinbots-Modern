using System.Reflection;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class LastExitResumeRenderedTests
{
    [Fact]
    public async Task QuitSavesCurrentNativeWorldAndSetupResumesItSeparatelyFromRecovery()
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-exit-resume-");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var storage = new AvaloniaDesktopStorageService(profile.FullName);
                Assert.False(ResumeItem(new SetupWindow(storage)).IsVisible);

                var runtime = new MainWindow([], TestWorld(), NativeEngineClientFactory.Instance, storage);
                var resumed = default(MainWindow);
                var setup = default(SetupWindow);
                try
                {
                    runtime.Show();
                    await WaitFor(() => Status(runtime).StartsWith("WORLD READY", StringComparison.Ordinal));
                    var session = Session(runtime);
                    await storage.SaveAutosaveAsync(await session.SaveAsync());
                    var autosave = Assert.IsType<string>(storage.FindLatestAutosave());
                    var recoveryBytes = await File.ReadAllBytesAsync(autosave);

                    var organism = Assert.Single(session.LatestSnapshot.Organisms);
                    await session.SetOrganismEnergyAsync(organism.Slot, organism.Generation, 1200);
                    await session.StepAsync(3);
                    var configuration = (await session.ReadConfigurationAsync())!;
                    await session.UpdateEnvironmentAsync(configuration.Environment with { BrownianMotion = 0.125f, ShapesVisible = true });
                    await session.UpdateSpeciesSettingsAsync([Assert.Single(configuration.Species) with { InitialCount = 2 }]);
                    await session.SetTiesEnabledAsync(false);
                    await session.AddObstacleAsync(new ObstacleSnapshot(0, [100, 200], [500, 800]));
                    var expected = session.LatestSnapshot;

                    ClickRuntimeMenu(runtime, LegacyCommandId.FileQuit);
                    var firstUse = await QuitPromptAsync(runtime);
                    Assert.DoesNotContain(firstUse.GetVisualDescendants().OfType<Button>(), button => Equals(button.Content, "Cancel"));
                    await ChooseQuitAsync(firstUse, "No");
                    Assert.True(runtime.IsVisible);
                    Assert.False(storage.HasLastExitSettings);
                    Assert.Null(storage.FindLastExitSimulation());

                    ClickRuntimeMenu(runtime, LegacyCommandId.FileQuit);
                    await ChooseQuitAsync(await QuitPromptAsync(runtime), "Yes");
                    await WaitFor(() => !runtime.IsVisible);
                    var checkpoint = Assert.IsType<string>(storage.FindLastExitSimulation());
                    Assert.NotEqual(autosave, checkpoint);
                    Assert.Equal(autosave, storage.FindLatestAutosave());
                    Assert.Equal(recoveryBytes, await File.ReadAllBytesAsync(autosave));
                    Assert.Empty(Directory.GetFiles(Path.GetDirectoryName(checkpoint)!, "*.tmp"));
                    var settings = Assert.IsType<DesktopSettings>(storage.LoadLastExitSettings());
                    Assert.Equal(0.125f, settings.BrownianMotion);
                    Assert.False(settings.TiesEnabled);
                    Assert.True(settings.ShapesVisible);
                    Assert.Equal(new float[] { 100, 200 }, Assert.Single(settings.Obstacles).Minimum);
                    Assert.Equal("Resume Bot", Assert.Single(settings.Species).Name);
                    Assert.Equal(1000, settings.Species[0].InitialEnergy);
                    Assert.Equal(2, settings.Species[0].InitialCount);
                    Assert.Empty(Directory.GetFiles(profile.FullName, "*.tmp"));

                    var nextStorage = new AvaloniaDesktopStorageService(profile.FullName);
                    Assert.False(ResumeItem(new SetupWindow(nextStorage, allowLastExitResume: false)).IsVisible);
                    setup = new SetupWindow(nextStorage);
                    WorldSetupOptions? options = null;
                    setup.WorldCreated += value => options = value;
                    setup.Show();
                    Assert.True(ResumeItem(setup).IsVisible);
                    setup.FindControl<Button>("CreateWorldButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    var fresh = Assert.IsType<WorldSetupOptions>(options);
                    Assert.Null(fresh.LoadSavePath);
                    Assert.Equal(3000, fresh.WorldWidth);
                    Assert.Equal(0.125f, fresh.BrownianMotion);
                    Assert.False(fresh.TiesEnabled);
                    Assert.True(fresh.ShapesVisible);
                    Assert.Equal(new float[] { 500, 800 }, Assert.Single(fresh.Obstacles).Maximum);
                    ResumeItem(setup).RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    Assert.NotNull(options);
                    Assert.Equal(checkpoint, options.LoadSavePath);
                    Assert.False(options.IsAutosaveRecovery);

                    resumed = new MainWindow([], options, NativeEngineClientFactory.Instance, nextStorage);
                    resumed.Show();
                    setup.Close();
                    await WaitFor(() => Status(resumed) == "LOADED");
                    var restored = Session(resumed).LatestSnapshot;
                    Assert.Equal(expected.Tick, restored.Tick);
                    Assert.Equal(expected.Population, restored.Population);
                    Assert.Equal(Assert.Single(expected.Organisms).Energy, Assert.Single(restored.Organisms).Energy);
                }
                finally
                {
                    setup?.Close();
                    await CloseRuntimeAsync(resumed);
                    await CloseRuntimeAsync(runtime);
                }
            }, CancellationToken.None);
        }
        finally { profile.Delete(recursive: true); }
    }

    [Fact]
    public async Task ExistingSettingsCanBeKeptSavedOrCanceledWithoutLosingRunState()
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-quit-settings-");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var storage = new AvaloniaDesktopStorageService(profile.FullName);
                await storage.SaveLastExitSettingsAsync(DesktopSettings.FromWorldSetup(TestWorld() with { Seed = 111 }));
                var settingsPath = Path.Combine(profile.FullName, "last-exit-settings.json");
                var original = await File.ReadAllBytesAsync(settingsPath);
                var runtime = new MainWindow([], TestWorld(), NativeEngineClientFactory.Instance, storage);
                var next = default(MainWindow);
                try
                {
                    runtime.Show();
                    await WaitFor(() => Status(runtime).StartsWith("WORLD READY", StringComparison.Ordinal));
                    var editor = new Window { Title = "Settings editor" };
                    var editing = editor.ShowDialog(runtime);
                    runtime.Close();
                    Assert.True(editor.IsVisible);
                    Assert.False(ExitInFlight(runtime));
                    editor.Close();
                    await editing;

                    var timer = (DispatcherTimer)typeof(MainWindow)
                        .GetField("_runTimer", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(runtime)!;
                    var gate = (SemaphoreSlim)typeof(MainWindow)
                        .GetField("_advanceGate", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(runtime)!;
                    timer.Start();
                    await gate.WaitAsync();
                    try
                    {
                        ClickRuntimeMenu(runtime, LegacyCommandId.FileQuit);
                        runtime.Close();
                        Assert.True(ExitInFlight(runtime));
                        Assert.Null(FindQuitPrompt(runtime));
                    }
                    finally { gate.Release(); }
                    var dialog = await QuitPromptAsync(runtime);
                    Assert.False(timer.IsEnabled);
                    runtime.Close();
                    Assert.Same(dialog, await QuitPromptAsync(runtime));
                    await ChooseQuitAsync(dialog, "Cancel");
                    Assert.True(runtime.IsVisible);
                    Assert.True(timer.IsEnabled);
                    Assert.Null(storage.FindLastExitSimulation());
                    Assert.Equal(original, await File.ReadAllBytesAsync(settingsPath));

                    timer.Stop();
                    runtime.Close();
                    dialog = await QuitPromptAsync(runtime);
                    dialog.Close();
                    await WaitFor(() => !ExitInFlight(runtime));
                    Assert.True(runtime.IsVisible);
                    Assert.False(timer.IsEnabled);

                    runtime.Close();
                    await ChooseQuitAsync(await QuitPromptAsync(runtime), "No");
                    await WaitFor(() => !runtime.IsVisible);
                    Assert.Equal(original, await File.ReadAllBytesAsync(settingsPath));
                    Assert.NotNull(storage.FindLastExitSimulation());

                    next = new MainWindow([], TestWorld() with { Seed = 222 }, NativeEngineClientFactory.Instance, storage);
                    next.Show();
                    await WaitFor(() => Status(next).StartsWith("WORLD READY", StringComparison.Ordinal));
                    next.Close();
                    await ChooseQuitAsync(await QuitPromptAsync(next), "Yes");
                    await WaitFor(() => !next.IsVisible);
                    Assert.Equal(222UL, storage.LoadLastExitSettings()!.Seed);
                }
                finally
                {
                    await CloseRuntimeAsync(next);
                    await CloseRuntimeAsync(runtime);
                }
            }, CancellationToken.None);
        }
        finally { profile.Delete(recursive: true); }
    }

    [Fact]
    public async Task CleanSetupPreservesAnEmptyStartupRosterAndRecoversFromCorruptSettings()
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-exit-setup-");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var storage = new AvaloniaDesktopStorageService(profile.FullName);
                await storage.SaveLastExitSettingsAsync(DesktopSettings.FromWorldSetup(TestWorld() with { Species = [] }));
                var clean = new SetupWindow(storage);
                var bypass = new SetupWindow(storage, loadLastExitSettings: false);
                try
                {
                    Assert.NotEmpty(bypass.Species);
                    Assert.Empty(clean.Species);
                    Assert.False(ResumeItem(clean).IsVisible);
                    WorldSetupOptions? options = null;
                    clean.WorldCreated += value => options = value;
                    clean.Show();
                    clean.FindControl<Button>("CreateWorldButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    Assert.Empty(Assert.IsType<WorldSetupOptions>(options).Species);
                }
                finally { clean.Close(); bypass.Close(); }

                await File.WriteAllTextAsync(Path.Combine(profile.FullName, "last-exit-settings.json"), "{invalid");
                var fallback = new SetupWindow(storage);
                try
                {
                    Assert.True(storage.HasLastExitSettings);
                    Assert.NotEmpty(fallback.Species);
                    Assert.StartsWith("LAST EXIT SETTINGS LOAD FAILED", fallback.FindControl<TextBlock>("SetupStatus")!.Text);
                }
                finally { fallback.Close(); }
            }, CancellationToken.None);
        }
        finally { profile.Delete(recursive: true); }
    }

    [Fact]
    public async Task ResetAndNewKeepThePreviousLastExitCheckpoint()
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-exit-navigation-");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var storage = new AvaloniaDesktopStorageService(profile.FullName);
                var windows = new List<Window>();
                using var opened = Window.WindowOpenedEvent.AddClassHandler<Window>((window, _) => windows.Add(window));
                var previous = new MainWindow([], TestWorld(), NativeEngineClientFactory.Instance, storage);
                try
                {
                    previous.Show();
                    await WaitFor(() => Status(previous).StartsWith("WORLD READY", StringComparison.Ordinal));
                    await Session(previous).StepAsync(2);
                    await CloseRuntimeAsync(previous);
                    var checkpoint = Assert.IsType<string>(storage.FindLastExitSimulation());
                    var previousBytes = await File.ReadAllBytesAsync(checkpoint);

                    var current = new MainWindow([], TestWorld(), NativeEngineClientFactory.Instance, storage);
                    current.Show();
                    await WaitFor(() => Status(current).StartsWith("WORLD READY", StringComparison.Ordinal));
                    await Session(current).StepAsync(5);
                    current.FindControl<Button>("ResetButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    await WaitFor(() => !current.IsVisible && windows.OfType<MainWindow>().Count() == 3);
                    Assert.Equal(previousBytes, await File.ReadAllBytesAsync(checkpoint));

                    var reset = windows.OfType<MainWindow>().Last();
                    await WaitFor(() => Status(reset).StartsWith("WORLD READY", StringComparison.Ordinal));
                    ClickRuntimeMenu(reset, LegacyCommandId.FileNewSimulation);
                    await WaitFor(() => !reset.IsVisible && windows.OfType<SetupWindow>().Any());
                    Assert.Equal(previousBytes, await File.ReadAllBytesAsync(checkpoint));
                }
                finally
                {
                    foreach (var window in windows.Where(window => window.IsVisible).ToArray())
                    {
                        if (window is MainWindow main) await CloseRuntimeAsync(main);
                        else window.Close();
                    }
                }
            }, CancellationToken.None);
        }
        finally { profile.Delete(recursive: true); }
    }

    private static WorldSetupOptions TestWorld() => new()
    {
        Backend = "Cpu", Seed = 42, PopulationCapacity = 16, VegetablePopulationCap = 0,
        WorldWidth = 3000, WorldHeight = 2000, BrownianMotion = 0,
        Species = [new("Resume Bot", "start stop", false, 0xff123456, 1, 1000, 0, false, 0)],
    };

    private static MenuItem ResumeItem(SetupWindow window) => window.FindControl<MenuItem>("ResumeMenuItem")!;

    private static string Status(MainWindow window) => Assert.IsType<MainWindowViewModel>(window.DataContext).Status;

    private static SimulationSession Session(MainWindow window) => Assert.IsType<SimulationSession>(
        typeof(MainWindow).GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window));

    private static void ClickRuntimeMenu(MainWindow window, LegacyCommandId command) =>
        window.FindControl<LegacyMenuBar>("HistoricMenuBar")!.FindCommandItem(command)!
            .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));

    private static async Task CloseRuntimeAsync(MainWindow? window)
    {
        if (window is not { IsVisible: true }) return;
        window.Close();
        await WaitFor(() => !window.IsVisible || FindQuitPrompt(window) is not null);
        if (FindQuitPrompt(window) is { } prompt)
        {
            var hasCancel = prompt.GetVisualDescendants().OfType<Button>().Any(button => Equals(button.Content, "Cancel"));
            await ChooseQuitAsync(prompt, hasCancel ? "No" : "Yes");
        }
        await WaitFor(() => !window.IsVisible);
    }

    private static Window? FindQuitPrompt(MainWindow owner) =>
        owner.OwnedWindows.SingleOrDefault(window => window is { IsVisible: true, IsDialog: true, Title: "Warning!" });

    private static async Task<Window> QuitPromptAsync(MainWindow owner)
    {
        await WaitFor(() => FindQuitPrompt(owner) is not null);
        return FindQuitPrompt(owner)!;
    }

    private static bool ExitInFlight(MainWindow owner) => (bool)typeof(MainWindow)
        .GetField("_savingLastExit", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(owner)!;

    private static async Task ChooseQuitAsync(Window dialog, string choice)
    {
        var owner = Assert.IsType<MainWindow>(dialog.Owner);
        Assert.Single(dialog.GetVisualDescendants().OfType<Button>(), button => Equals(button.Content, choice))
            .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        await WaitFor(() => !owner.IsVisible || !ExitInFlight(owner));
    }

    private static async Task WaitFor(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(8);
        while (!condition() && DateTime.UtcNow < deadline) await Task.Delay(10);
        Assert.True(condition(), "Timed out waiting for the last-exit workflow.");
    }
}
