using System.Reflection;
using Avalonia.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class PositionalStartupSaveTests
{
    [Theory]
    [InlineData(true, false)]
    [InlineData(true, true)]
    [InlineData(false, false)]
    public async Task APositionalSaveStartsInTheLoadedSimulation(bool legacyFormat, bool useLegacySaveDirectory)
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-positional-save-");
        try
        {
            var legacyDirectory = Path.Combine(profile.FullName, "Legacy Main Directory");
            var saves = Directory.CreateDirectory(Path.Combine(legacyDirectory, "Saves"));
            var storage = new AvaloniaDesktopStorageService(profile.FullName);
            await storage.SaveGlobalSettingsAsync(new GlobalSettings { Main = new GlobalMainSettings { MainDirectory = legacyDirectory } });
            var name = $"Startup World {Guid.NewGuid():N}" + (legacyFormat ? ".SIM" : ".db3s");
            var path = Path.Combine(saves.FullName, name);
            using var engine = new NativeEngineClient(new WorldSetupOptions
            {
                Backend = "Cpu", Seed = 42, PopulationCapacity = 16, VegetablePopulationCap = 0,
                WorldWidth = 3000, WorldHeight = 2000, BrownianMotion = 0,
                Vegetation = Db2VegetationOptions.Default with { StartChloroplasts = 1234 },
            });
            engine.ImportSpecies(new("Startup Bot", "start stop", false, 0xff123456, 1, 1234, 0, false, 0));
            engine.Tick(3);
            var expected = engine.Snapshot();
            await File.WriteAllBytesAsync(path, legacyFormat ? engine.SaveLegacy() : engine.Save());

            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var opened = new List<Window>();
                var closed = new List<Window>();
                using var openedHandler = Window.WindowOpenedEvent.AddClassHandler<Window>((window, _) => opened.Add(window));
                using var closedHandler = Window.WindowClosedEvent.AddClassHandler<Window>((window, _) => closed.Add(window));
                var runtime = Assert.IsType<MainWindow>(CreateStartupWindow([useLegacySaveDirectory ? name : path], storage));
                try
                {
                    Assert.False(runtime.IsVisible);
                    Assert.Empty(closed);
                    runtime.Show();
                    var model = Assert.IsType<MainWindowViewModel>(runtime.DataContext);
                    await WaitFor(() => model.Status == "LOADED");
                    Assert.Equal(expected.NativeCycle, model.NativeCycle);
                    Assert.Equal(expected.Population, model.Population);
                    var session = Assert.IsType<SimulationSession>(typeof(MainWindow)
                        .GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(runtime));
                    var configuration = Assert.IsType<WorldConfiguration>(await session.ReadConfigurationAsync());
                    Assert.Equal(legacyFormat ? 0 : 1234,
                        configuration.Environment.Vegetation.StartChloroplasts);
                    Assert.Same(runtime, Assert.Single(opened));
                }
                finally
                {
                    runtime.Close();
                    await WaitFor(() => !runtime.IsVisible);
                }
            }, CancellationToken.None);
        }
        finally { profile.Delete(recursive: true); }
    }

    [Fact]
    public async Task SetupStaysAvailableForExistingStartupOptionsOrAnUnresolvableSave()
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-positional-errors-");
        try
        {
            var storage = new AvaloniaDesktopStorageService(profile.FullName);
            await storage.SaveGlobalSettingsAsync(new GlobalSettings { Main = new GlobalMainSettings { MainDirectory = profile.FullName } });
            await storage.SaveLastExitSimulationAsync(new byte[] { 1 });
            var saves = profile.CreateSubdirectory("Saves");
            File.WriteAllText(Path.Combine(saves.FullName, "Bot Parameter.sim"), "start\nstop");
            var missingPath = Path.Combine(profile.FullName, "missing", "Bot Parameter.sim");
            await DesktopTestApplication.Session.Dispatch(() =>
            {
                var resume = Assert.IsType<SetupWindow>(CreateStartupWindow([], storage));
                Assert.True(resume.FindControl<MenuItem>("ResumeMenuItem")!.IsVisible);
                resume.Close();

                var startupBot = Assert.IsType<SetupWindow>(CreateStartupWindow(["--BOT", "Bot Parameter.sim"], storage));
                Assert.False(startupBot.FindControl<MenuItem>("ResumeMenuItem")!.IsVisible);
                Assert.DoesNotContain("STARTUP LOAD FAILED", startupBot.FindControl<TextBlock>("SetupStatus")!.Text ?? string.Empty);
                startupBot.Close();

                var missing = Assert.IsType<SetupWindow>(CreateStartupWindow([missingPath], storage));
                Assert.Contains(missingPath, missing.FindControl<TextBlock>("SetupStatus")!.Text);
                missing.Close();

                var ambiguous = Assert.IsType<SetupWindow>(CreateStartupWindow(["a.sim", "b.db3s"], storage));
                Assert.Contains("Pass one simulation file", ambiguous.FindControl<TextBlock>("SetupStatus")!.Text);
                ambiguous.Close();
            }, CancellationToken.None);
        }
        finally { profile.Delete(recursive: true); }
    }

    private static Window CreateStartupWindow(string[] arguments, IDesktopStorageService storage) =>
        Assert.IsAssignableFrom<Window>(typeof(App).GetMethod("CreateStartupWindow", BindingFlags.Static | BindingFlags.NonPublic)!
            .Invoke(null, [arguments, storage, (Action<Window>)(_ => { })]));

    private static async Task WaitFor(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(10);
        while (!condition() && DateTime.UtcNow < deadline) await Task.Delay(10);
        Assert.True(condition(), "Timed out waiting for the startup simulation workflow.");
    }
}
