using System.Reflection;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Media.Imaging;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class World05ObstaclesMazeRenderedTests
{
    [Fact]
    public async Task RealObstaclePlacementEditorApplyAndMazeCommandProduceCanonicalSnapshotsAndReceipt()
    {
        var factory = new AuditEngineClientFactory();
        var setup = new WorldSetupOptions
        {
            WorldWidth = 16_000,
            WorldHeight = 12_000,
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var player = new MainWindow([], setup, factory, new AuditStorageService())
            {
                WindowState = WindowState.Normal,
                Width = 1_200,
                Height = 800,
            };
            player.Show();
            try
            {
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                await WaitFor(() => Session(player) is not null);
                var engine = factory.Clients.Single();
                var viewport = player.FindControl<WorldViewport>("Viewport")!;

                var firstCorner = new Point(viewport.Bounds.Width * 0.22, viewport.Bounds.Height * 0.24);
                var oppositeCorner = new Point(viewport.Bounds.Width * 0.38, viewport.Bounds.Height * 0.43);
                Click(player, player.FindControl<Button>("AddObstacleButton")!);
                ClickAt(player, viewport, firstCorner);
                ClickAt(player, viewport, oppositeCorner);
                await WaitFor(() => Session(player)?.LatestSnapshot.Obstacles.Count == 1);

                ClickAt(player, viewport, new Point(
                    (firstCorner.X + oppositeCorner.X) / 2,
                    (firstCorner.Y + oppositeCorner.Y) / 2));
                await WaitFor(() => player.FindControl<Button>("EditShapeButton")!.IsEnabled);
                Click(player, player.FindControl<Button>("EditShapeButton")!);
                await WaitFor(() => player.OwnedWindows.OfType<ShapeEditorWindow>().Any());

                var editor = player.OwnedWindows.OfType<ShapeEditorWindow>().Single();
                editor.FindControl<NumericUpDown>("MinimumX")!.Value = 125m;
                editor.FindControl<NumericUpDown>("MinimumY")!.Value = 135m;
                editor.FindControl<NumericUpDown>("MaximumX")!.Value = 925m;
                editor.FindControl<NumericUpDown>("MaximumY")!.Value = 935m;
                editor.FindControl<TextBox>("ColorText")!.Text = "#FF102030";
                editor.FindControl<NumericUpDown>("OpacityValue")!.Value = 0.65m;
                editor.FindControl<NumericUpDown>("DriftX")!.Value = 2.5m;
                editor.FindControl<NumericUpDown>("DriftY")!.Value = -3.5m;
                editor.FindControl<ComboBox>("VisionPolicy")!.SelectedIndex = 1;
                editor.FindControl<ComboBox>("ShotPolicy")!.SelectedIndex = 1;
                Click(editor, editor.GetVisualDescendants().OfType<Button>().Single(button => button.Content?.ToString() == "APPLY"));
                await WaitFor(() => engine.Calls.Contains("UpdateObstacle"));

                var edited = Session(player)!.LatestSnapshot.Obstacles.Single();
                Assert.Equal([125f, 135f], edited.Minimum);
                Assert.Equal([925f, 935f], edited.Maximum);
                Assert.Equal(0xFF102030u, edited.Color);
                Assert.Equal(0.65f, edited.Opacity);
                Assert.Equal([2.5f, -3.5f], edited.Drift);
                Assert.Equal("see_through", edited.VisionPolicy);
                Assert.Equal("pass_through", edited.ShotPolicy);

                var router = Assert.IsType<LegacyCommandRouter>(
                    typeof(MainWindow)
                        .GetField("_legacyCommands", BindingFlags.Instance | BindingFlags.NonPublic)!
                        .GetValue(player));
                await router.ExecuteAsync(LegacyCommandId.ObjectsTrashCompactor);
                await WaitFor(() => Session(player)?.LatestSnapshot.Obstacles.Count == 3);

                var obstacles = Session(player)!.LatestSnapshot.Obstacles.OrderBy(value => value.Id).ToArray();
                Assert.Equal(3, obstacles.Length);
                Assert.Equal(edited.Id, obstacles[0].Id);
                Assert.Equal([125f, 135f], obstacles[0].Minimum);
                Assert.Equal([925f, 935f], obstacles[0].Maximum);
                Assert.Equal(edited.Color, obstacles[0].Color);
                Assert.Equal([2.5f, -3.5f], obstacles[0].Drift);
                Assert.Equal(edited.Id + 1, obstacles[1].Id);
                Assert.Equal([-999f, -1200f], obstacles[1].Minimum);
                Assert.Equal([1f, 13200f], obstacles[1].Maximum);
                Assert.Equal([2f, 0f], obstacles[1].Drift);
                Assert.Equal(edited.Id + 2, obstacles[2].Id);
                Assert.Equal([15999f, -1200f], obstacles[2].Minimum);
                Assert.Equal([16999f, 13200f], obstacles[2].Maximum);
                Assert.Equal([-2f, 0f], obstacles[2].Drift);

                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var frame = player.CaptureRenderedFrame();
                Assert.NotNull(frame);
                Assert.True(player.IsVisible);
                Assert.True(frame.PixelSize.Width >= 900);
                Assert.True(frame.PixelSize.Height >= 600);
                using var png = new MemoryStream();
                frame.Save(png, new PngBitmapEncoderOptions());
                WriteReceipt(png.ToArray());
            }
            finally
            {
                foreach (var ownedWindow in player.OwnedWindows.ToArray())
                    ownedWindow.Close();
                player.Close();
            }
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
        Assert.True(condition(), "Timed out waiting for the WORLD-05 rendered obstacle route.");
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

    private static void Click(Window window, Control control)
    {
        StabilizeForInput(window, control);
        var point = control.TranslatePoint(
            new Point(Math.Max(1, control.Bounds.Width / 2), Math.Max(1, control.Bounds.Height / 2)),
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

    private static void WriteReceipt(byte[] bytes)
    {
        Assert.True(bytes.Length > 1_000, $"Rendered PNG was only {bytes.Length} bytes.");
        Assert.Equal(new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 }, bytes.Take(8));
        var receiptPath = ReceiptPath();
        Directory.CreateDirectory(Path.GetDirectoryName(receiptPath)!);
        File.WriteAllBytes(receiptPath, bytes);
        Assert.True(File.Exists(receiptPath), $"Rendered receipt was not written: {receiptPath}");
    }

    private static string ReceiptPath()
    {
        return Path.Combine(
            Path.GetTempPath(),
            "darwinbots-modern-control-audit",
            "db24832-world05-obstacles-maze.png");
    }
}
