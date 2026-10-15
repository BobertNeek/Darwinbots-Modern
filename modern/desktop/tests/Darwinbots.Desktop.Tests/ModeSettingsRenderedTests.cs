using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Media.Imaging;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class ModeSettingsRenderedTests
{
    [Fact]
    public async Task RestartLeagueAndOfflineModeSurfacesRenderTogether()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var setup = new SetupWindow(new AuditStorageService());
            ShowForInteraction(setup, 1280, 820);
            try
            {
                var setupTabs = Named<TabControl>(setup, "SimulationSettingsTabs");
                setupTabs.SelectedIndex = 4;
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                var restartLeagueTab = Assert.IsType<TabItem>(setupTabs.Items[4]);
                Assert.Equal("RESTART AND LEAGUE", restartLeagueTab.Header?.ToString());
                Assert.Contains(
                    "Restart, Zerobot, and local league controls",
                    Assert.IsType<TextBlock>(restartLeagueTab.Content).Text,
                    StringComparison.Ordinal);

                await WaitFor(() => setup.OwnedWindows.OfType<GlobalSettingsWindow>().Any());
                var global = Assert.Single(setup.OwnedWindows.OfType<GlobalSettingsWindow>());
                try
                {
                    var globalTabs = Named<TabControl>(global, "Tabs");
                    globalTabs.SelectedIndex = 2;
                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    Assert.Equal("LEAGUE", Assert.IsType<TabItem>(globalTabs.Items[2]).Header?.ToString());
                    var offlineOwned = Named<CheckBox>(global, "LeagueOfflineOwned");
                    Assert.Equal("Owned by offline modes", offlineOwned.Content?.ToString());
                    Assert.True(offlineOwned.IsChecked);
                    Assert.False(offlineOwned.IsEnabled);

                    globalTabs.SelectedIndex = 3;
                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    var evolutionMode = Named<ComboBox>(global, "EvolutionMode");
                    var evolutionNames = evolutionMode.Items.Cast<object>().Select(item => item?.ToString()).ToArray();
                    Assert.Contains("SimpleSurvival", evolutionNames);
                    Assert.Contains("EcoSurvival", evolutionNames);
                    Assert.Equal("EVOLUTION", Assert.IsType<TabItem>(globalTabs.Items[3]).Header?.ToString());
                    evolutionMode.SelectedIndex = Array.IndexOf(evolutionNames, "SimpleSurvival");
                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    Assert.Equal("SimpleSurvival", evolutionMode.SelectedItem?.ToString());
                    evolutionMode.SelectedIndex = Array.IndexOf(evolutionNames, "EcoSurvival");
                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    Assert.Equal("EcoSurvival", evolutionMode.SelectedItem?.ToString());
                }
                finally
                {
                    global.Close();
                }

                await WaitFor(() => !global.IsVisible);
            }
            finally
            {
                setup.Close();
            }
        }, CancellationToken.None);

        var receipt = await DesktopTestApplication.Session.Dispatch(() =>
        {
            var factory = new AuditEngineClientFactory();
            var main = new MainWindow(
                [],
                new WorldSetupOptions
                {
                    TicksPerUpdate = 1,
                    Species = [new SpeciesImport("Rendered Species", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
                },
                factory,
                new AuditStorageService());
            ShowForInteraction(main, 1500, 1000);
            byte[] renderedFrame = [];
            try
            {
                var league = Named<Expander>(main, "LeagueExpander");
                league.IsExpanded = true;
                league.BringIntoView();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                var kinds = Named<ComboBox>(main, "LocalModeKindSelector");
                var kindLabels = kinds.Items.Cast<ComboBoxItem>().Select(item => item.Content?.ToString() ?? string.Empty).ToArray();
                Assert.Equal(
                    ["LEAGUE", "CONTEST", "SIMPLE SURVIVAL", "ECO SURVIVAL", "ZEROBOT"],
                    kindLabels);

                var presets = Named<ComboBox>(main, "LocalModePreset");
                var presetLabels = presets.Items.Cast<ComboBoxItem>().Select(item => item.Content?.ToString() ?? string.Empty).ToArray();
                Assert.Equal(
                    ["F1", "F2", "F3", "MULTIBOT", "SHORT BOT", "SHORT GENE", "SINGLE STORE"],
                    presetLabels);
                Assert.Equal(0, kinds.SelectedIndex);
                Assert.Equal("LEAGUE", (kinds.SelectedItem as ComboBoxItem)?.Content?.ToString());
                Assert.Equal(0, presets.SelectedIndex);
                Assert.Equal("F1", (presets.SelectedItem as ComboBoxItem)?.Content?.ToString());

                kinds.SelectedIndex = 1;
                presets.SelectedIndex = 2;
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                Assert.Equal("CONTEST", kinds.SelectedItem is ComboBoxItem contest ? contest.Content?.ToString() : null);
                Assert.Equal("F3", presets.SelectedItem is ComboBoxItem f3 ? f3.Content?.ToString() : null);

                kinds.SelectedIndex = 2;
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                Assert.Equal("SIMPLE SURVIVAL", kinds.SelectedItem is ComboBoxItem simple ? simple.Content?.ToString() : null);
                kinds.SelectedIndex = 3;
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                Assert.Equal("ECO SURVIVAL", kinds.SelectedItem is ComboBoxItem eco ? eco.Content?.ToString() : null);

                Assert.Equal(5m, Named<NumericUpDown>(main, "ModeMinimumRounds").Value);
                Assert.Equal(1m, Named<NumericUpDown>(main, "ModeMaximumRounds").Value);
                Assert.Equal(15_000m, Named<NumericUpDown>(main, "ModeMaximumCycles").Value);
                Assert.Equal(700m, Named<NumericUpDown>(main, "ModePopulationCap").Value);
                Assert.True(Named<CheckBox>(main, "ModeRestartOnExtinction").IsChecked);
                Assert.Equal("CONFIGURE", Named<Button>(main, "ConfigureModeButton").Content?.ToString());
                Assert.Equal("START", Named<Button>(main, "StartModeButton").Content?.ToString());
                Assert.Equal("RESTART", Named<Button>(main, "RestartModeButton").Content?.ToString());
                Assert.Equal("ADJUDICATE", Named<Button>(main, "AdjudicateModeButton").Content?.ToString());
                Assert.Contains("LEAGUE TAB", main.GetVisualDescendants().OfType<TextBlock>().Select(text => text.Text), StringComparer.Ordinal);

                renderedFrame = CaptureFrame(main);
            }
            finally
            {
                foreach (var ownedWindow in main.OwnedWindows.ToArray()) ownedWindow.Close();
                main.Close();
            }
            return renderedFrame;
        }, CancellationToken.None);
        WriteReceipt(receipt);
    }

    private static void ShowForInteraction(Window window, double width, double height)
    {
        window.WindowState = WindowState.Normal;
        window.Width = width;
        window.Height = height;
        window.Show();
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }

    private static T Named<T>(Control root, string name) where T : Control =>
        root.FindControl<T>(name) ?? throw new Xunit.Sdk.XunitException($"Missing {typeof(T).Name} named {name}.");

    private static void Click(Window window, Control control)
    {
        window.UpdateLayout();
        control.BringIntoView();
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        var point = control.TranslatePoint(
            new Point(Math.Max(1, control.Bounds.Width / 2), Math.Max(1, control.Bounds.Height / 2)),
            window);
        Assert.True(point.HasValue, $"{control.Name ?? control.GetType().Name} was not attached to the rendered window.");
        window.MouseDown(point!.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(point.Value, MouseButton.Left, RawInputModifiers.None);
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }

    private static async Task WaitFor(Func<bool> condition)
    {
        for (var attempt = 0; attempt < 500; attempt++)
        {
            AvaloniaHeadlessPlatform.ForceRenderTimerTick();
            if (condition()) return;
            await Task.Delay(20);
        }

        Assert.True(condition(), "Timed out waiting for the rendered route.");
    }

    private static byte[] CaptureFrame(Window window)
    {
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        using var frame = window.CaptureRenderedFrame();
        Assert.NotNull(frame);
        Assert.True(frame.PixelSize.Width >= 400, $"Rendered width was {frame.PixelSize.Width}.");
        Assert.True(frame.PixelSize.Height >= 300, $"Rendered height was {frame.PixelSize.Height}.");
        using var memory = new MemoryStream();
        frame.Save(memory, new PngBitmapEncoderOptions());
        return memory.ToArray();
    }

    private static void WriteReceipt(byte[] bytes)
    {
        Assert.True(bytes.Length > 24, $"Rendered PNG was too small: {bytes.Length} bytes.");
        Assert.Equal(
            new byte[] { 0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A },
            bytes[..8]);

        var auditDirectory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR");
        Assert.False(string.IsNullOrWhiteSpace(auditDirectory), "DARWINBOTS_GUI_AUDIT_DIR must name the durable GUI audit directory.");
        Directory.CreateDirectory(auditDirectory!);
        var output = Path.Combine(auditDirectory!, "set07-restart-league.png");
        using (var stream = new FileStream(output, FileMode.Create, FileAccess.Write, FileShare.Read, 64 * 1024, FileOptions.WriteThrough))
        {
            stream.Write(bytes);
            stream.Flush(true);
        }
        Assert.True(File.Exists(output), $"Rendered receipt was not written: {output}");
    }
}
