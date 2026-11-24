using System.Reflection;
using Avalonia.Controls;
using Avalonia.Interactivity;
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
                    var expected = session.LatestSnapshot;

                    ClickRuntimeMenu(runtime, LegacyCommandId.FileQuit);
                    await WaitFor(() => !runtime.IsVisible);
                    var checkpoint = Assert.IsType<string>(storage.FindLastExitSimulation());
                    Assert.NotEqual(autosave, checkpoint);
                    Assert.Equal(autosave, storage.FindLatestAutosave());
                    Assert.Equal(recoveryBytes, await File.ReadAllBytesAsync(autosave));
                    Assert.Empty(Directory.GetFiles(Path.GetDirectoryName(checkpoint)!, "*.tmp"));

                    var nextStorage = new AvaloniaDesktopStorageService(profile.FullName);
                    Assert.False(ResumeItem(new SetupWindow(nextStorage, allowLastExitResume: false)).IsVisible);
                    setup = new SetupWindow(nextStorage);
                    WorldSetupOptions? options = null;
                    setup.WorldCreated += value => options = value;
                    setup.Show();
                    Assert.True(ResumeItem(setup).IsVisible);
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
        await WaitFor(() => !window.IsVisible);
    }

    private static async Task WaitFor(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(8);
        while (!condition() && DateTime.UtcNow < deadline) await Task.Delay(10);
        Assert.True(condition(), "Timed out waiting for the last-exit workflow.");
    }
}
