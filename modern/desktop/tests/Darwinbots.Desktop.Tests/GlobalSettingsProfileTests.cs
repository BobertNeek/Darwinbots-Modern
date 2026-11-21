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
                    Assert.True(Named<CheckBox>(globals, "NewSeedOnLoad").IsChecked);
                    Assert.True(Named<CheckBox>(globals, "OldColors").IsChecked);
                    Named<CheckBox>(globals, "NewSeedOnLoad").IsChecked = false;
                    Named<CheckBox>(globals, "FixedRatio").IsChecked = true;
                    Named<CheckBox>(globals, "SafeMode").IsChecked = false;
                    Click(globals, "ApplyButton");
                    await WaitFor(() => Named<TextBlock>(setup, "SetupStatus").Text == "GLOBAL SETTINGS SAVED");
                    Click(setup, "CreateWorldButton");
                    Assert.NotNull(created);
                    Assert.True(created.GlobalSettings.Main.FixedRatio);
                    Assert.False(created.GlobalSettings.Main.NewSeedOnLoad);
                    Assert.Null(created.GlobalSettings.Main.OldColors);
                    Assert.False(created.ToEnvironmentUpdate().Recording!.SafeModeBackup);
                }
                finally { Close(setup); }

                var runtime = new MainWindow([], created!, new AuditEngineClientFactory(), storage);
                try
                {
                    runtime.Show();
                    var model = Assert.IsType<MainWindowViewModel>(runtime.DataContext);
                    await WaitFor(() => model.Status.StartsWith("WORLD READY", StringComparison.Ordinal));
                    var viewport = Named<WorldViewport>(runtime, "Viewport");
                    Assert.True(viewport.IsVisible);
                    Assert.True(viewport.OldColors);
                    Named<LegacyMenuBar>(runtime, "HistoricMenuBar")
                        .FindCommandItem(LegacyCommandId.EditGlobalSettings)!
                        .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    var globals = await GlobalDialog(runtime);
                    Named<CheckBox>(globals, "OldColors").IsChecked = false;
                    Named<CheckBox>(globals, "VideoOff").IsChecked = true;
                    Click(globals, "ApplyButton");
                    await WaitFor(() => model.Status == "GLOBAL SETTINGS SAVED");
                    Assert.True(viewport.IsVisible);
                    Assert.False(viewport.OldColors);
                }
                finally { Close(runtime); }

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
                    Assert.False(Named<CheckBox>(globals, "OldColors").IsChecked);
                    Assert.True(Named<CheckBox>(globals, "VideoOff").IsChecked);
                    Assert.False(Named<CheckBox>(globals, "SafeMode").IsChecked);
                    var beforeCancel = File.ReadAllText(Path.Combine(profile.FullName, "global-settings.json"));
                    Named<CheckBox>(globals, "OldColors").IsChecked = true;
                    Click(globals, "CancelButton");
                    await WaitFor(() => reopened.OwnedWindows.Count() == 0);
                    Assert.Equal(beforeCancel, File.ReadAllText(Path.Combine(profile.FullName, "global-settings.json")));
                    Click(reopened, "CreateWorldButton");
                    Assert.NotNull(restarted);
                }
                finally { Close(reopened); }

                var next = new MainWindow([], restarted!, new AuditEngineClientFactory(), reopenedStorage);
                try
                {
                    next.Show();
                    var model = Assert.IsType<MainWindowViewModel>(next.DataContext);
                    await WaitFor(() => model.Status.StartsWith("WORLD READY", StringComparison.Ordinal));
                    Assert.False(Named<WorldViewport>(next, "Viewport").IsVisible);
                    Assert.Equal("GRAPHICS OFF", Named<Button>(next, "ToolbarSlot14").Content);
                    Click(next, "TurboButton");
                    Assert.True(Named<WorldViewport>(next, "Viewport").IsVisible);
                }
                finally { Close(next); }
            }, CancellationToken.None);
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

    [Theory]
    [InlineData(null, null, true)]
    [InlineData("#FALSE#", false, false)]
    [InlineData("#TRUE#", true, true)]
    public async Task ConfiguredLegacyDirectorySuppliesOldColorsUntilExplicitOverride(
        string? eighthRecord, bool? legacyPreference, bool effectiveOldColors)
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-global-colors-");
        try
        {
            var legacy = profile.CreateSubdirectory("Original install");
            var legacyPath = Path.Combine(legacy.FullName, "Global.gset");
            var original = string.Join("\r\n", Enumerable.Repeat("#FALSE#", 7)) + "\r\n";
            if (eighthRecord is not null)
                original += eighthRecord + "\r\n";
            await File.WriteAllTextAsync(legacyPath, original);
            var storage = new AvaloniaDesktopStorageService(profile.FullName);
            var settings = new GlobalSettings { Main = new GlobalMainSettings { MainDirectory = legacy.FullName } };

            await storage.SaveGlobalSettingsAsync(settings);
            var imported = Assert.IsType<GlobalSettings>(storage.LoadGlobalSettings());
            Assert.Equal(legacyPreference, imported.Main.OldColors);
            Assert.Equal(effectiveOldColors, imported.Main.EffectiveOldColors);

            var overridden = settings with { Main = settings.Main with { OldColors = !effectiveOldColors } };
            await storage.SaveGlobalSettingsAsync(overridden);
            Assert.Equal(!effectiveOldColors, storage.LoadGlobalSettings()!.Main.EffectiveOldColors);
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
}
