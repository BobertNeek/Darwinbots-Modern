using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Media.Imaging;
using Avalonia.Threading;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using System.Reflection;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class TeleporterRenderedTests
{
    [Fact]
    public async Task TeleporterEditorRendersTypesAndPolicyAndAppliesLocalEdit()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var window = new TeleporterEditorWindow(
                new TeleporterSnapshot(7, [100f, 200f], 40f, [800f, 900f])
                {
                    Capacity = 4,
                    BackflowLimit = 2,
                    RespectShapes = true,
                    Mode = "intrasim"
                })
            {
                WindowState = WindowState.Normal,
                Width = 560,
                Height = 780
            };

            window.Show();
            AvaloniaHeadlessPlatform.ForceRenderTimerTick();

            Assert.Equal(0, window.FindControl<ComboBox>("Mode")!.SelectedIndex);
            Assert.Equal(4m, window.FindControl<NumericUpDown>("Capacity")!.Value);
            Assert.Equal(2m, window.FindControl<NumericUpDown>("BackflowLimit")!.Value);
            Assert.True(window.FindControl<CheckBox>("RespectShapes")!.IsChecked);

            Capture(window, "teleporter-editor-exercised.png");

            window.FindControl<NumericUpDown>("Radius")!.Value = 55m;
            Click(window, window.FindControl<Button>("ApplyButton")!);

            Assert.NotNull(window.Result);
            Assert.Equal(55f, window.Result!.Radius);
            Assert.Equal("intrasim", window.Result.Mode);
            window.Close();
        }, CancellationToken.None);
    }

    [Fact]
    public async Task PlayerRouteRendersOfflineFileTeleporterEditorAndCommitsThroughSession()
    {
        var factory = new AuditEngineClientFactory(includeWorldObjects: false);
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var player = new MainWindow(
                [],
                new WorldSetupOptions(),
                factory,
                new AuditStorageService())
            {
                WindowState = WindowState.Normal,
                Width = 1100,
                Height = 820,
            };
            player.Show();
            try
            {
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                await WaitFor(() => ((Darwinbots.Desktop.ViewModels.MainWindowViewModel)player.DataContext!).Population == 1);
                var engine = factory.Clients.Single();

                var viewport = player.FindControl<Control>("Viewport")!;
                var source = new Point(viewport.Bounds.Width * 0.25, viewport.Bounds.Height * 0.30);
                var destination = new Point(viewport.Bounds.Width * 0.68, viewport.Bounds.Height * 0.62);
                Click(player, player.FindControl<Button>("AddTeleporterButton")!);
                ClickAt(player, viewport, source);
                ClickAt(player, viewport, destination);
                await WaitFor(() => Session(player)?.LatestSnapshot.Teleporters.Count == 1);

                ClickAt(player, viewport, source);
                await WaitFor(() => player.FindControl<Button>("EditTeleporterButton")!.IsEnabled);
                Click(player, player.FindControl<Button>("EditTeleporterButton")!);
                await WaitFor(() => player.OwnedWindows.OfType<TeleporterEditorWindow>().Any());

                var editor = player.OwnedWindows.OfType<TeleporterEditorWindow>().Single();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                var mode = editor.FindControl<ComboBox>("Mode")!;
                mode.SelectedIndex = 1;
                editor.FindControl<TextBox>("InboundPath")!.Text = Path.GetTempPath();
                editor.FindControl<NumericUpDown>("Capacity")!.Value = 6m;

                Assert.True(editor.FindControl<NumericUpDown>("CenterX")!.IsVisible);
                Assert.True(editor.FindControl<NumericUpDown>("CenterY")!.IsVisible);
                Assert.True(editor.FindControl<NumericUpDown>("DestinationX")!.IsVisible);
                Assert.True(editor.FindControl<NumericUpDown>("DestinationY")!.IsVisible);
                Assert.True(editor.FindControl<CheckBox>("TeleportVegetables")!.IsVisible);
                Assert.True(editor.FindControl<CheckBox>("TeleportCorpses")!.IsVisible);
                Assert.True(editor.FindControl<CheckBox>("TeleportHeterotrophs")!.IsVisible);
                Assert.True(editor.FindControl<NumericUpDown>("Capacity")!.IsVisible);
                Assert.True(mode.IsVisible);
                Assert.Equal("Inbound file", ((ComboBoxItem)mode.SelectedItem!).Content);
                Assert.Equal("APPLY", editor.FindControl<Button>("ApplyButton")!.Content);
                Assert.Equal("CANCEL", editor.FindControl<Button>("CancelButton")!.Content);

                CaptureReceipt(editor, "db24832-world08-teleporter-editor.png");
                Click(editor, editor.FindControl<Button>("ApplyButton")!);
                var result = Assert.IsType<TeleporterSnapshot>(editor.Result);
                await WaitFor(() => engine.Calls.Contains("UpdateTeleporter"));
                await WaitFor(() => Session(player)?.LatestSnapshot.Teleporters.Any(value =>
                    value.Id == result.Id && value.Mode == "inbound" && value.Capacity == 6) == true);
            }
            finally
            {
                foreach (var ownedWindow in player.OwnedWindows.ToArray())
                    ownedWindow.Close();
                player.Close();
            }
            return true;
        }, CancellationToken.None);
    }

    private static SimulationSession? Session(MainWindow window) =>
        typeof(MainWindow)
            .GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)
            ?.GetValue(window) as SimulationSession;

    private static async Task WaitFor(Func<bool> condition)
    {
        for (var attempt = 0; attempt < 200; attempt++)
        {
            if (condition()) return;
            await Task.Delay(25);
        }
        Assert.True(condition(), "Timed out waiting for the rendered player route.");
    }

    private static void ClickAt(Window window, Control control, Point point)
    {
        StabilizeForInput(window, control);
        var translated = control.TranslatePoint(point, window);
        Assert.True(translated.HasValue);
        window.MouseDown(translated.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(translated.Value, MouseButton.Left, RawInputModifiers.None);
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }

    private static void CaptureReceipt(Window window, string fileName)
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        DirectoryInfo? fallback = null;
        while (directory is not null)
        {
            if (directory.Name.Equals("parity-world08-rendered-95", StringComparison.OrdinalIgnoreCase))
            {
                fallback = directory;
                break;
            }
            if (fallback is null
                && File.Exists(Path.Combine(directory.FullName, "Cargo.toml"))
                && Directory.Exists(Path.Combine(directory.FullName, "modern")))
                fallback = directory;
            directory = directory.Parent;
        }
        directory = fallback ?? throw new DirectoryNotFoundException("Darwinbots worktree root was not found.");
        var receiptDirectory = Path.Combine(directory.FullName, "docs", "parity", "receipts");
        Directory.CreateDirectory(receiptDirectory);
        var path = Path.Combine(receiptDirectory, fileName);
        using var bitmap = new RenderTargetBitmap(new PixelSize((int)window.Bounds.Width, (int)window.Bounds.Height));
        bitmap.Render(window);
        using var stream = new MemoryStream();
        bitmap.Save(stream, new PngBitmapEncoderOptions());
        var bytes = stream.ToArray();
        Assert.True(bytes.Length > 8);
        File.WriteAllBytes(path, bytes);
    }

    private static void Click(Window window, Button button)
    {
        StabilizeForInput(window, button);
        var point = button.TranslatePoint(
            new Point(Math.Max(1, button.Bounds.Width / 2), Math.Max(1, button.Bounds.Height / 2)),
            window);
        Assert.True(point.HasValue);
        window.MouseDown(point.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(point.Value, MouseButton.Left, RawInputModifiers.None);
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }

    private static void StabilizeForInput(Window window, Control control)
    {
        Assert.True(window.IsVisible);
        control.BringIntoView();
        Dispatcher.UIThread.RunJobs();
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        Dispatcher.UIThread.RunJobs();
    }

    private static void Capture(Window window, string fileName)
    {
        var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR")
            ?? Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
        Directory.CreateDirectory(directory);
        var path = Path.Combine(directory, fileName);
        using var bitmap = new RenderTargetBitmap(new PixelSize((int)window.Bounds.Width, (int)window.Bounds.Height));
        bitmap.Render(window);
        bitmap.Save(path);
    }
}
