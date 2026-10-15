using System.Reflection;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class StartupMazeColorRenderedTests
{
    [Fact]
    public async Task StartupDnaMazePresetsAndColorEditorRenderPlayerRoute()
    {
        var dnaPath = Path.Combine(Path.GetTempPath(), $"startup-luna-{Guid.NewGuid():N}.txt");
        File.WriteAllText(dnaPath, "cond\nstart\nstop\n");

        try
        {
            var setup = new WorldSetupOptions
            {
                Backend = "Cpu",
                WorldWidth = 16_000,
                WorldHeight = 12_000,
            };
            var factory = NativeEngineClientFactory.Instance;

            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var player = new MainWindow(["--bot", dnaPath], setup, factory, new AuditStorageService())
                {
                    WindowState = WindowState.Normal,
                    Width = 1_200,
                    Height = 800,
                };
                player.Show();
                try
                {
                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    await WaitFor(() => Session(player) is not null
                        && ViewModel(player).Population == 1
                        && ViewModel(player).Status.Contains("IMPORTED 1 STARTUP BOT", StringComparison.OrdinalIgnoreCase));

                    var session = Session(player)!;
                    Assert.NotEmpty(session.LatestSnapshot.Organisms);
                    Assert.True(player.FindControl<WorldViewport>("Viewport")!.IsVisible);
                    Assert.True(player.FindControl<Border>("InternetOfflineOverlay")!.IsVisible);

                    var organism = session.LatestSnapshot.Organisms[0];
                    var key = new OrganismKey(organism.Slot, organism.Generation);
                    var viewport = player.FindControl<WorldViewport>("Viewport")!;
                    viewport.SelectKey(key);
                    ViewModel(player).Select(key);
                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    Dispatcher.UIThread.RunJobs();
                    Assert.NotNull(ViewModel(player).SelectedOrganism);

                    var router = Router(player);
                    foreach (var command in new[]
                    {
                        LegacyCommandId.ObjectsHorizontalMaze,
                        LegacyCommandId.ObjectsVerticalMaze,
                        LegacyCommandId.ObjectsSprial,
                        LegacyCommandId.ObjectsCheckerboard,
                        LegacyCommandId.ObjectsPolarIce,
                        LegacyCommandId.ObjectsTrashCompactor,
                    })
                    {
                        var before = session.LatestSnapshot.Obstacles.Count;
                        var result = await router.ExecuteAsync(command);
                        Assert.False(string.IsNullOrWhiteSpace(result.Status));
                        await WaitFor(() => Session(player)!.LatestSnapshot.Obstacles.Count > before);
                    }

                    Assert.True(session.LatestSnapshot.Obstacles.Count > 0);

                    var colorCommand = router.ExecuteAsync(LegacyCommandId.RobotChangeColor);
                    await WaitFor(() => player.OwnedWindows.OfType<RobotColorEditorWindow>().Any());

                    var editor = player.OwnedWindows.OfType<RobotColorEditorWindow>().Single();
                    editor.GetVisualDescendants().OfType<Slider>().Single(control => control.Name == "RedSlider").Value = 0x12;
                    editor.GetVisualDescendants().OfType<Slider>().Single(control => control.Name == "GreenSlider").Value = 0x34;
                    editor.GetVisualDescendants().OfType<Slider>().Single(control => control.Name == "BlueSlider").Value = 0x56;
                    Assert.Equal("#123456", editor.PreviewHex);
                    Click(editor, editor.GetVisualDescendants().OfType<Button>().Single(control => control.Name == "UseColorButton"));
                    await WaitFor(() => editor.Accepted);
                    var colorResult = await colorCommand;
                    Assert.False(string.IsNullOrWhiteSpace(colorResult.Status));
                    await WaitFor(() => Session(player)!.LatestSnapshot.Species.Any(species => species.Color == 0xff123456u));
                    Assert.Contains(Session(player)!.LatestSnapshot.Species, species => species.Color == 0xff123456u);

                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    using var frame = player.CaptureRenderedFrame();
                    Assert.NotNull(frame);
                    Assert.True(frame.PixelSize.Width >= 900);
                    Assert.True(frame.PixelSize.Height >= 600);
                }
                finally
                {
                    foreach (var ownedWindow in player.OwnedWindows.ToArray())
                        ownedWindow.Close();
                    player.Close();
                }
            }, CancellationToken.None);
        }
        finally
        {
            if (File.Exists(dnaPath)) File.Delete(dnaPath);
        }
    }

    private static SimulationSession? Session(MainWindow window) =>
        typeof(MainWindow)
            .GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)
            ?.GetValue(window) as SimulationSession;

    private static MainWindowViewModel ViewModel(MainWindow window) =>
        (MainWindowViewModel)typeof(MainWindow)
            .GetField("_viewModel", BindingFlags.Instance | BindingFlags.NonPublic)!
            .GetValue(window)!;

    private static LegacyCommandRouter Router(MainWindow window) =>
        Assert.IsType<LegacyCommandRouter>(typeof(MainWindow)
            .GetField("_legacyCommands", BindingFlags.Instance | BindingFlags.NonPublic)!
            .GetValue(window));

    private static async Task WaitFor(Func<bool> condition)
    {
        for (var attempt = 0; attempt < 200; attempt++)
        {
            if (condition()) return;
            await Task.Delay(25);
        }

        Assert.True(condition(), "Timed out waiting for the startup, maze, or color player route.");
    }

    private static void Click(Window window, Control control)
    {
        Assert.True(window.IsVisible);
        control.BringIntoView();
        Dispatcher.UIThread.RunJobs();
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        Dispatcher.UIThread.RunJobs();
        var point = control.TranslatePoint(
            new Point(Math.Max(1, control.Bounds.Width / 2), Math.Max(1, control.Bounds.Height / 2)),
            window);
        Assert.True(point.HasValue);
        window.MouseDown(point.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(point.Value, MouseButton.Left, RawInputModifiers.None);
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }
}
