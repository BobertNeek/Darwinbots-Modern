using System.Reflection;
using System.Text.Json.Nodes;
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

public sealed class GraphKeepUpdatingPreferenceTests
{
    [Fact]
    public async Task GraphClosePreferenceStagesUntilNextLaunchAndCancelKeepsIt()
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-graph-close-profile-");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var storage = new AvaloniaDesktopStorageService(profile.FullName);
                var runtime = Launch();
                try
                {
                    var globals = await OpenGlobalSettings(runtime);
                    var keepGraphs = Named<CheckBox>(globals, "KeepGraphsUpdatingOnClose");
                    Assert.Equal("Automatically assume to keep updating graphs", keepGraphs.Content);
                    Assert.False(keepGraphs.IsChecked);
                    keepGraphs.IsChecked = true;
                    Click(globals, "ApplyButton");
                    var model = Assert.IsType<MainWindowViewModel>(runtime.DataContext);
                    await WaitFor(() => model.Status == "GLOBAL SETTINGS SAVED");
                    Assert.True(storage.LoadGraphClosePreference());

                    var (suite, chart) = OpenPopulation(runtime);
                    RequestClose(chart);
                    await WaitFor(() => GraphPrompts(suite, chart).Any());
                    Assert.True(chart.IsVisible);
                    Assert.Single(GraphPrompts(suite, chart)).Close(false);
                    await WaitFor(() => !chart.IsVisible);
                }
                finally { await CloseRuntimeAsync(runtime); }

                var relaunched = Launch();
                try
                {
                    var globals = await OpenGlobalSettings(relaunched);
                    var keepGraphs = Named<CheckBox>(globals, "KeepGraphsUpdatingOnClose");
                    Assert.True(keepGraphs.IsChecked);
                    keepGraphs.IsChecked = false;
                    Click(globals, "CancelButton");
                    await WaitFor(() => !globals.IsVisible);
                    Assert.True(storage.LoadGraphClosePreference());

                    var (suite, chart) = OpenPopulation(relaunched);
                    RequestClose(chart);
                    await WaitFor(() => !chart.IsVisible);
                    Assert.Empty(GraphPrompts(suite, chart));
                    Assert.NotEmpty(suite.ViewModel.RunningGraphs);
                }
                finally { await CloseRuntimeAsync(relaunched); }

                MainWindow Launch()
                {
                    var world = new WorldSetupOptions
                    {
                        Species = [new("Graph Bot", "start stop", false, 0xff239ac0, 1, 1000, 0, false, 0)],
                        GlobalSettings = new GlobalSettings
                        {
                            Main = new GlobalMainSettings { KeepGraphsUpdatingOnClose = storage.LoadGraphClosePreference() },
                        },
                    };
                    var window = new MainWindow([], world, new AuditEngineClientFactory(), storage);
                    window.Show();
                    return window;
                }

                static async Task<GlobalSettingsWindow> OpenGlobalSettings(MainWindow window)
                {
                    var model = Assert.IsType<MainWindowViewModel>(window.DataContext);
                    await WaitFor(() => model.Status.StartsWith("WORLD READY", StringComparison.Ordinal));
                    Named<LegacyMenuBar>(window, "HistoricMenuBar").FindCommandItem(LegacyCommandId.EditGlobalSettings)!
                        .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    return await GlobalDialog(window);
                }

                static (GraphSuiteWindow Suite, GraphSeriesWindow Chart) OpenPopulation(MainWindow window)
                {
                    Click(window, "GraphsButton");
                    var suite = Assert.Single(window.OwnedWindows.OfType<GraphSuiteWindow>());
                    suite.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.Population);
                    Named<GraphSuiteView>(suite, "GraphView").FindControl<Button>("OpenGraphButton")!
                        .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    return (suite, Assert.Single(suite.OwnedWindows.OfType<GraphSeriesWindow>()));
                }

                static IEnumerable<LegacyConfirmationWindow> GraphPrompts(GraphSuiteWindow suite, GraphSeriesWindow chart) =>
                    chart.OwnedWindows.Concat(suite.OwnedWindows).OfType<LegacyConfirmationWindow>();

                static void RequestClose(Window chart) =>
                    typeof(Window).GetMethod("HandleClosing", BindingFlags.Instance | BindingFlags.NonPublic)!
                        .Invoke(chart, [WindowCloseReason.WindowClosing]);
            }, CancellationToken.None);
        }
        finally { profile.Delete(recursive: true); }
    }

    [Fact]
    public async Task GraphClosePreferenceImportsOriginalAndMergesAnExplicitFalseIntoExistingProfile()
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-graph-close-legacy-");
        try
        {
            var legacy = profile.CreateSubdirectory("Original install");
            var legacyPath = Path.Combine(legacy.FullName, "Global.gset");
            var lines = Enumerable.Repeat("#FALSE#", 52).ToArray();
            lines[50] = "#TRUE#";
            var original = string.Join("\r\n", lines) + "\r\n";
            await File.WriteAllTextAsync(legacyPath, original);
            var storage = new AvaloniaDesktopStorageService(profile.FullName);
            var profilePath = Path.Combine(profile.FullName, "global-settings.json");
            var initial = new JsonObject
            {
                ["main"] = new JsonObject { ["mainDirectory"] = legacy.FullName, ["startChloroplasts"] = 1234 },
                ["newerSettings"] = new JsonObject { ["enabled"] = true },
            };
            await File.WriteAllTextAsync(profilePath, initial.ToJsonString());

            Assert.True(storage.LoadGraphClosePreference());
            await storage.SaveGraphClosePreferenceAsync(false);
            Assert.False(storage.LoadGraphClosePreference());
            var saved = JsonNode.Parse(await File.ReadAllTextAsync(profilePath))!.AsObject();
            Assert.False(saved["main"]!["keepGraphsUpdatingOnClose"]!.GetValue<bool>());
            saved["main"]!.AsObject().Remove("keepGraphsUpdatingOnClose");
            Assert.True(JsonNode.DeepEquals(initial, saved));
            Assert.Equal(original, await File.ReadAllTextAsync(legacyPath));
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
        Assert.True(condition(), "Timed out waiting for the graph preference workflow.");
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
