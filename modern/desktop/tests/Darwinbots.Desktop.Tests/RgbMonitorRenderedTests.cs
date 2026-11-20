using System.Reflection;
using Avalonia.Automation;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class RgbMonitorRenderedTests
{
    [Fact]
    public async Task ViewMenuConfiguresGlobalRgbWithoutSelectionAndRetainsLiveInspection()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu", PopulationCapacity = 64, WorldWidth = 1_100, WorldHeight = 800,
            Species = [new("Control", "start\nstop", false, 0xff3f8e34u, 1, 1_000, 0, false, 0f)],
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService());
            window.Show();
            try
            {
                var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitFor(() => viewModel.Population == 1,
                    () => viewModel.Status);
                var session = Field<SimulationSession>(window, "_session")!;
                var robot = Assert.Single(session.LatestSnapshot.Organisms);
                await session.SetOrganismMemoryAsync(robot.Slot, robot.Generation, 77, 50);
                var original = await session.SaveAsync();

                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var settings = menu.FindCommandItem(LegacyCommandId.ViewRgbMemoryMonitorSettings)!;
                var toggle = menu.FindCommandItem(LegacyCommandId.ViewRgbMemoryMonitor)!;
                viewModel.ClearSelection();
                viewport.SelectKey(null);
                Assert.Null(viewModel.SelectedKey);
                Assert.True(settings.IsEnabled && toggle.IsEnabled);
                Click(toggle);
                Assert.Equal("CONFIGURE RGB MONITOR SETTINGS FIRST", viewModel.Status);
                Assert.False(toggle.IsChecked);

                async Task<RgbMemoryMonitorWindow> OpenSettings()
                {
                    Click(settings);
                    await WaitFor(() => window.OwnedWindows.OfType<RgbMemoryMonitorWindow>().Any(dialog => dialog.CloseOnApply),
                        () => viewModel.Status);
                    return Assert.Single(window.OwnedWindows.OfType<RgbMemoryMonitorWindow>(), dialog => dialog.CloseOnApply);
                }

                var dialog = await OpenSettings();
                Assert.Equal(RgbMemoryMonitorSettings.LegacyDefaults, dialog.Settings);
                Number(dialog, "RED address").Value = 99;
                Click(DialogButton(dialog, "CANCEL"));
                await WaitFor(() => !dialog.IsVisible, () => "RGB settings did not close.");
                Click(toggle);
                Assert.Equal("CONFIGURE RGB MONITOR SETTINGS FIRST", viewModel.Status);

                dialog = await OpenSettings();
                Assert.Equal(1m, Number(dialog, "RED address").Value);
                Number(dialog, "RED address").Value = 77;
                Number(dialog, "RED ceiling").Value = 100;
                Click(DialogButton(dialog, "APPLY"));
                await WaitFor(() => viewModel.Status == "RGB SETTINGS APPLIED", () => viewModel.Status);
                var configured = RgbMemoryMonitorSettings.LegacyDefaults with { Red = new(77, 0, 100) };
                Assert.Null(Field<RgbMemoryMonitorSettings>(viewport, "_rgbMemoryMonitor"));
                Click(toggle);
                Assert.True(toggle.IsChecked);
                Assert.Equal(configured, Field<RgbMemoryMonitorSettings>(viewport, "_rgbMemoryMonitor"));
                Assert.Empty(window.OwnedWindows.OfType<RgbMemoryMonitorWindow>());

                dialog = await OpenSettings();
                Assert.Equal(77m, Number(dialog, "RED address").Value);
                Number(dialog, "RED address").Value = 88;
                Click(DialogButton(dialog, "CANCEL"));
                await WaitFor(() => !dialog.IsVisible, () => "RGB settings did not close.");
                Assert.Equal(configured, Field<RgbMemoryMonitorSettings>(viewport, "_rgbMemoryMonitor"));
                Assert.Equal(original, await session.SaveAsync());

                Click(toggle);
                Assert.False(toggle.IsChecked);
                var focus = new OrganismKey(robot.Slot, robot.Generation);
                viewModel.Select(focus);
                viewport.SelectKey(focus);
                Assert.Equal(focus, viewModel.SelectedKey);
                Click(toggle);
                await WaitFor(() => window.OwnedWindows.OfType<RgbMemoryMonitorWindow>().Any(),
                    () => viewModel.Status);
                var inspector = Assert.Single(window.OwnedWindows.OfType<RgbMemoryMonitorWindow>());
                Assert.False(inspector.CloseOnApply);
                await WaitFor(() => inspector.GetVisualDescendants().OfType<TextBlock>().Any(value =>
                    value.Text?.StartsWith("RGB  128", StringComparison.Ordinal) == true),
                    () => "The selected robot's initial RGB values did not appear.");
                await session.SetOrganismMemoryAsync(robot.Slot, robot.Generation, 77, 100);
                await WaitFor(() => inspector.GetVisualDescendants().OfType<TextBlock>().Any(value =>
                    value.Text?.StartsWith("RGB  255", StringComparison.Ordinal) == true),
                    () => "The selected robot's RGB live values did not update.");

                inspector.Close();
                Assert.True(toggle.IsChecked);
                Assert.Equal(configured, Field<RgbMemoryMonitorSettings>(viewport, "_rgbMemoryMonitor"));
                Click(toggle);
                Assert.False(toggle.IsChecked);
                Assert.Null(Field<RgbMemoryMonitorSettings>(viewport, "_rgbMemoryMonitor"));
            }
            finally
            {
                foreach (var owned in window.OwnedWindows.ToArray()) owned.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task LegacyPresetFilesStageSettingsWithoutChangingTheActiveMonitorUntilApply()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu", PopulationCapacity = 64, WorldWidth = 1_100, WorldHeight = 800,
            Species = [new("Control", "start\nstop", false, 0xff3f8e34u, 1, 1_000, 0, false, 0f)],
        };
        var preset = Convert.FromHexString("4D001302E703008318FB20006400E804007D");
        var storage = new AuditStorageService();
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, storage);
            window.Show();
            try
            {
                var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitFor(() => viewModel.Population == 1, () => viewModel.Status);
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var settings = menu.FindCommandItem(LegacyCommandId.ViewRgbMemoryMonitorSettings)!;
                var toggle = menu.FindCommandItem(LegacyCommandId.ViewRgbMemoryMonitor)!;
                var session = Field<SimulationSession>(window, "_session")!;
                var before = await session.SaveAsync();
                viewModel.ClearSelection();
                viewport.SelectKey(null);

                async Task<RgbMemoryMonitorWindow> OpenSettings()
                {
                    Click(settings);
                    await WaitFor(() => window.OwnedWindows.OfType<RgbMemoryMonitorWindow>().Any(), () => viewModel.Status);
                    return Assert.Single(window.OwnedWindows.OfType<RgbMemoryMonitorWindow>());
                }

                var dialog = await OpenSettings();
                Click(DialogButton(dialog, "APPLY"));
                await WaitFor(() => viewModel.Status == "RGB SETTINGS APPLIED", () => viewModel.Status);
                Click(toggle);
                Assert.True(toggle.IsChecked);
                Assert.Equal(RgbMemoryMonitorSettings.LegacyDefaults, Field<RgbMemoryMonitorSettings>(viewport, "_rgbMemoryMonitor"));

                dialog = await OpenSettings();
                storage.RgbMonitorPresetToOpen = [.. preset, 0xff];
                Click(DialogButton(dialog, "LOAD PRESET"));
                Assert.Equal(77m, Number(dialog, "RED address").Value);
                Assert.Equal(-32_000m, Number(dialog, "RED floor").Value);
                Assert.Equal(1_256m, Number(dialog, "GREEN ceiling").Value);
                Assert.Equal(RgbMemoryMonitorSettings.LegacyDefaults, dialog.Settings);
                Click(DialogButton(dialog, "SAVE PRESET"));
                Assert.Equal(preset, storage.SavedRgbMonitorPreset);

                Number(dialog, "RED address").Value = 98;
                storage.RgbMonitorPresetToOpen = null;
                Click(DialogButton(dialog, "LOAD PRESET"));
                Assert.Equal(98m, Number(dialog, "RED address").Value);
                storage.AcceptRgbMonitorPresetSave = false;
                Click(DialogButton(dialog, "SAVE PRESET"));
                Assert.Equal(preset, storage.SavedRgbMonitorPreset);

                storage.RgbMonitorPresetToOpen = preset[..^1];
                Click(DialogButton(dialog, "LOAD PRESET"));
                Assert.Contains(dialog.GetVisualDescendants().OfType<TextBlock>(),
                    label => label.Text?.StartsWith("RGB PRESET LOAD FAILED", StringComparison.Ordinal) == true);
                Assert.Equal(98m, Number(dialog, "RED address").Value);
                Click(DialogButton(dialog, "CANCEL"));
                await WaitFor(() => viewModel.Status == "CANCELED" && !window.OwnedWindows.Contains(dialog),
                    () => viewModel.Status);
                Assert.Equal(RgbMemoryMonitorSettings.LegacyDefaults, Field<RgbMemoryMonitorSettings>(viewport, "_rgbMemoryMonitor"));

                dialog = await OpenSettings();
                Assert.Equal(1m, Number(dialog, "RED address").Value);
                storage.RgbMonitorPresetToOpen = preset;
                Click(DialogButton(dialog, "LOAD PRESET"));
                Click(DialogButton(dialog, "APPLY"));
                await WaitFor(() => viewModel.Status == "RGB SETTINGS APPLIED", () => viewModel.Status);
                Assert.Equal(Db2RgbMonitorPresetCodec.Decode(preset), Field<RgbMemoryMonitorSettings>(viewport, "_rgbMemoryMonitor"));
                Assert.Equal(before, await session.SaveAsync());
            }
            finally
            {
                foreach (var owned in window.OwnedWindows.ToArray()) owned.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static NumericUpDown Number(Window window, string name) =>
        Assert.Single(window.GetVisualDescendants().OfType<NumericUpDown>(),
            input => AutomationProperties.GetName(input) == name);

    private static Button DialogButton(Window window, string name) =>
        Assert.Single(window.GetVisualDescendants().OfType<Button>(), button => Equals(button.Content, name));

    private static void Click(Control control) =>
        control.RaiseEvent(new RoutedEventArgs(control is MenuItem ? MenuItem.ClickEvent : Button.ClickEvent));

    private static T? Field<T>(object target, string name) where T : class =>
        target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)?.GetValue(target) as T;

    private static async Task WaitFor(Func<bool> condition, Func<string> diagnostic)
    {
        for (var attempt = 0; attempt < 500; attempt++)
        {
            AvaloniaHeadlessPlatform.ForceRenderTimerTick();
            if (condition()) return;
            await Task.Delay(20);
        }
        throw new Xunit.Sdk.XunitException(diagnostic());
    }
}
