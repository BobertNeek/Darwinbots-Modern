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
                    Named<CheckBox>(globals, "FixedRatio").IsChecked = true;
                    Named<CheckBox>(globals, "SafeMode").IsChecked = false;
                    Click(globals, "ApplyButton");
                    await WaitFor(() => Named<TextBlock>(setup, "SetupStatus").Text == "GLOBAL SETTINGS SAVED");
                    Click(setup, "CreateWorldButton");
                    Assert.NotNull(created);
                    Assert.True(created.GlobalSettings.Main.FixedRatio);
                    Assert.False(created.ToEnvironmentUpdate().Recording!.SafeModeBackup);
                }
                finally { Close(setup); }

                var runtime = new MainWindow([], created!, new AuditEngineClientFactory(), storage);
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
                    Named<CheckBox>(globals, "OldColors").IsChecked = true;
                    Named<CheckBox>(globals, "VideoOff").IsChecked = true;
                    Click(globals, "ApplyButton");
                    await WaitFor(() => model.Status == "GLOBAL SETTINGS SAVED");
                    Assert.True(Named<WorldViewport>(runtime, "Viewport").IsVisible);
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
                    Assert.True(Named<CheckBox>(globals, "OldColors").IsChecked);
                    Assert.True(Named<CheckBox>(globals, "VideoOff").IsChecked);
                    Assert.False(Named<CheckBox>(globals, "SafeMode").IsChecked);
                    var beforeCancel = File.ReadAllText(Path.Combine(profile.FullName, "global-settings.json"));
                    Named<CheckBox>(globals, "OldColors").IsChecked = false;
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
                finally { await CloseRuntimeAsync(next); }
            }, CancellationToken.None);
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
        Close(owner);
        await WaitFor(() => !owner.IsVisible);
    }
}
