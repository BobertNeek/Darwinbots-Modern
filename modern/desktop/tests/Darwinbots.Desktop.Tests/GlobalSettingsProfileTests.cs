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

public sealed class GlobalSettingsProfileTests
{
    [Fact]
    public async Task GlobalDialogApplyPersistsAcrossSetupSessionsAndCancelKeepsTheSavedProfile()
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-global-settings-");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var storage = new AvaloniaDesktopStorageService(profile.FullName);
                var setup = new SetupWindow(storage);
                WorldSetupOptions? created = null;
                setup.WorldCreated += options => created = options;
                try
                {
                    setup.Show();
                    Click(setup, "GlobalSettingsButton");
                    var globals = await GlobalDialog(setup);
                    Assert.Equal(0m, Named<NumericUpDown>(globals, "StartChloroplasts").Value);
                    Assert.True(Named<CheckBox>(globals, "NewSeedOnLoad").IsChecked);
                    Named<CheckBox>(globals, "NewSeedOnLoad").IsChecked = false;
                    Named<CheckBox>(globals, "FixedRatio").IsChecked = true;
                    Named<CheckBox>(globals, "SafeMode").IsChecked = false;
                    Named<NumericUpDown>(globals, "StartChloroplasts").Value = 4321m;
                    Click(globals, "ApplyButton");
                    await WaitFor(() => Named<TextBlock>(setup, "SetupStatus").Text == "GLOBAL SETTINGS SAVED");
                    Click(setup, "CreateWorldButton");
                    Assert.NotNull(created);
                    Assert.True(created.GlobalSettings.Main.FixedRatio);
                    Assert.Equal(4321, created.GlobalSettings.Main.StartChloroplasts);
                    Assert.Equal(0, created.Vegetation.StartChloroplasts);
                    Assert.False(created.GlobalSettings.Main.NewSeedOnLoad);
                    Assert.False(created.ToEnvironmentUpdate().Recording!.SafeModeBackup);
                }
                finally { Close(setup); }

                var engineFactory = new AuditEngineClientFactory();
                var runtime = new MainWindow([], created!, engineFactory, storage);
                try
                {
                    runtime.Show();
                    var model = Assert.IsType<MainWindowViewModel>(runtime.DataContext);
                    await WaitFor(() => model.Status.StartsWith("WORLD READY", StringComparison.Ordinal));
                    Assert.True(Named<WorldViewport>(runtime, "Viewport").IsVisible);
                    Named<LegacyMenuBar>(runtime, "HistoricMenuBar")
                        .FindCommandItem(LegacyCommandId.EditGlobalSettings)!
                        .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    var globals = await GlobalDialog(runtime);
                    Assert.Equal(4321m, Named<NumericUpDown>(globals, "StartChloroplasts").Value);
                    Named<CheckBox>(globals, "OldColors").IsChecked = true;
                    Named<CheckBox>(globals, "VideoOff").IsChecked = true;
                    Named<NumericUpDown>(globals, "StartChloroplasts").Value = 9100m;
                    Click(globals, "ApplyButton");
                    await WaitFor(() => model.Status == "GLOBAL SETTINGS SAVED");
                    Assert.True(Named<WorldViewport>(runtime, "Viewport").IsVisible);
                    Assert.Equal(0, Assert.Single(engineFactory.CreatedSetups).Vegetation.StartChloroplasts);
                    var engine = Assert.Single(engineFactory.Clients);
                    Assert.Equal(0, engine.EnvironmentUpdates.Last().Vegetation.StartChloroplasts);

                    Named<LegacyMenuBar>(runtime, "HistoricMenuBar")
                        .FindCommandItem(LegacyCommandId.EditGeneralSettings)!
                        .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => runtime.OwnedWindows.OfType<AdvancedSettingsWindow>().Any());
                    var advanced = Assert.Single(runtime.OwnedWindows.OfType<AdvancedSettingsWindow>());
                    Assert.Equal(0m, Named<NumericUpDown>(advanced, "StartChloroplasts").Value);
                    Named<NumericUpDown>(advanced, "StartChloroplasts").Value = 5678m;
                    Click(advanced, "ApplyButton");
                    await WaitFor(() => model.Status == "LIVE SETTINGS APPLIED");
                    Assert.Equal(5678, engine.EnvironmentUpdates.Last().Vegetation.StartChloroplasts);
                    Assert.Equal(9100, storage.LoadGlobalSettings()!.Main.StartChloroplasts);
                }
                finally { await CloseRuntimeAsync(runtime); }

                var reopenedStorage = new AvaloniaDesktopStorageService(profile.FullName);
                var reopened = new SetupWindow(reopenedStorage);
                WorldSetupOptions? restarted = null;
                reopened.WorldCreated += options => restarted = options;
                try
                {
                    reopened.Show();
                    Click(reopened, "GlobalSettingsButton");
                    var globals = await GlobalDialog(reopened);
                    Assert.True(Named<CheckBox>(globals, "FixedRatio").IsChecked);
                    Assert.False(Named<CheckBox>(globals, "NewSeedOnLoad").IsChecked);
                    Assert.True(Named<CheckBox>(globals, "OldColors").IsChecked);
                    Assert.True(Named<CheckBox>(globals, "VideoOff").IsChecked);
                    Assert.False(Named<CheckBox>(globals, "SafeMode").IsChecked);
                    Assert.Equal(9100m, Named<NumericUpDown>(globals, "StartChloroplasts").Value);
                    var beforeCancel = File.ReadAllText(Path.Combine(profile.FullName, "global-settings.json"));
                    Named<CheckBox>(globals, "OldColors").IsChecked = false;
                    Named<NumericUpDown>(globals, "StartChloroplasts").Value = 1234m;
                    Click(globals, "CancelButton");
                    await WaitFor(() => reopened.OwnedWindows.Count() == 0);
                    Assert.Equal(beforeCancel, File.ReadAllText(Path.Combine(profile.FullName, "global-settings.json")));
                    Click(reopened, "CreateWorldButton");
                    Assert.NotNull(restarted);
                    Assert.Equal(9100, restarted.Vegetation.StartChloroplasts);
                }
                finally { Close(reopened); }

                var next = new MainWindow([], restarted!, new AuditEngineClientFactory(), reopenedStorage);
                try
                {
                    next.Show();
                    var model = Assert.IsType<MainWindowViewModel>(next.DataContext);
                    await WaitFor(() => model.Status.StartsWith("WORLD READY", StringComparison.Ordinal));
                    Assert.True(Named<WorldViewport>(next, "Viewport").IsVisible);
                    Assert.Equal("GRAPHICS OFF", Named<Button>(next, "ToolbarSlot14").Content);
                    Assert.Equal("TURBO", Named<Button>(next, "TurboButton").Content);
                    Click(next, "ToolbarSlot14");
                    Assert.Equal("GRAPHICS ON", Named<Button>(next, "ToolbarSlot14").Content);
                    Assert.True(Named<WorldViewport>(next, "Viewport").IsVisible);
                }
                finally { await CloseRuntimeAsync(next); }
            }, CancellationToken.None);
        }
        finally { profile.Delete(recursive: true); }
    }

    [Fact]
    public async Task ConfiguredLegacyDirectorySuppliesStartingChloroplastsUntilAnExplicitProfileOverride()
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-global-settings-");
        try
        {
            var legacy = profile.CreateSubdirectory("Original install");
            var legacyPath = Path.Combine(legacy.FullName, "Global.gset");
            var original = string.Join("\r\n", Enumerable.Repeat("#FALSE#", 29)) + "\r\n1234.5\r\n3\r\n";
            await File.WriteAllTextAsync(legacyPath, original);
            var storage = new AvaloniaDesktopStorageService(profile.FullName);
            Assert.Null(storage.LoadGlobalSettings());

            var defaults = new GlobalSettings { Main = new GlobalMainSettings { MainDirectory = legacy.FullName } };
            await storage.SaveGlobalSettingsAsync(defaults);
            Assert.Equal(1234, storage.LoadGlobalSettings()!.Main.StartChloroplasts);

            await storage.SaveGlobalSettingsAsync(defaults with { Main = defaults.Main with { StartChloroplasts = 0 } });
            Assert.Equal(0, storage.LoadGlobalSettings()!.Main.StartChloroplasts);
            Assert.Equal(original, await File.ReadAllTextAsync(legacyPath));
        }
        finally { profile.Delete(recursive: true); }
    }

    [Fact]
    public async Task ConfiguredLegacyDirectorySuppliesTheSeparateLoadSeedPreference()
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-global-seed-");
        try
        {
            var legacy = profile.CreateSubdirectory("Original install");
            var legacyPath = Path.Combine(legacy.FullName, "Global.gset");
            const string original = "#TRUE#\r\n32100\r\n#FALSE#\r\n#TRUE#\r\n#FALSE#\r\n";
            await File.WriteAllTextAsync(legacyPath, original);
            var storage = new AvaloniaDesktopStorageService(profile.FullName);
            var settings = new GlobalSettings { Main = new GlobalMainSettings { MainDirectory = legacy.FullName } };

            await storage.SaveGlobalSettingsAsync(settings);
            var imported = Assert.IsType<GlobalSettings>(storage.LoadGlobalSettings());
            Assert.False(imported.Main.EffectiveNewSeedOnLoad);
            Assert.False(imported.Main.NewSeed);

            await storage.SaveGlobalSettingsAsync(settings with { Main = settings.Main with { NewSeedOnLoad = true } });
            Assert.True(storage.LoadGlobalSettings()!.Main.EffectiveNewSeedOnLoad);
            Assert.Equal(original, await File.ReadAllTextAsync(legacyPath));
        }
        finally { profile.Delete(recursive: true); }
    }

    [Fact]
    public async Task InvalidProfileUsesDefaultsWithoutOverwritingTheFileAtStartup()
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-global-settings-");
        try
        {
            const string invalid = """{"main":null}""";
            var path = Path.Combine(profile.FullName, "global-settings.json");
            await File.WriteAllTextAsync(path, invalid);
            await DesktopTestApplication.Session.Dispatch(() =>
            {
                var setup = new SetupWindow(new AvaloniaDesktopStorageService(profile.FullName));
                try
                {
                    Assert.StartsWith("GLOBAL SETTINGS LOAD FAILED", Named<TextBlock>(setup, "SetupStatus").Text);
                    WorldSetupOptions? created = null;
                    setup.WorldCreated += options => created = options;
                    Click(setup, "CreateWorldButton");
                    Assert.Equal(new GlobalSettings(), created!.GlobalSettings);
                    Assert.Equal(invalid, File.ReadAllText(path));
                }
                finally { Close(setup); }
            }, CancellationToken.None);
        }
        finally { profile.Delete(recursive: true); }
    }

    private static T Named<T>(Window owner, string name) where T : Control => owner.FindControl<T>(name)!;

    private static void Click(Window owner, string name) =>
        Named<Button>(owner, name).RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

    private static async Task<GlobalSettingsWindow> GlobalDialog(Window owner)
    {
        await WaitFor(() => owner.OwnedWindows.OfType<GlobalSettingsWindow>().Any());
        return Assert.Single(owner.OwnedWindows.OfType<GlobalSettingsWindow>());
    }

    private static async Task WaitFor(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(8);
        while (!condition() && DateTime.UtcNow < deadline) await Task.Delay(10);
        Assert.True(condition(), "Timed out waiting for the global settings workflow.");
    }

    private static void Close(Window owner)
    {
        foreach (var dialog in owner.OwnedWindows.ToArray()) dialog.Close();
        owner.Close();
    }

    private static async Task CloseRuntimeAsync(MainWindow owner)
    {
        // This fixture exercises the global profile with a fake engine, not graceful quit.
        typeof(MainWindow).GetField("_skipLastExitCheckpoint", System.Reflection.BindingFlags.Instance
            | System.Reflection.BindingFlags.NonPublic)!.SetValue(owner, true);
        Close(owner);
        await WaitFor(() => !owner.IsVisible);
    }
}
