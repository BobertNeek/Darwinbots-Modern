using System.Reflection;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class World09CommandsRenderedTests
{
    [Theory]
    [InlineData(false, false)]
    [InlineData(true, false)]
    [InlineData(false, true)]
    public async Task EmptyArenaLeftClickClearsOrdinaryRobotFocus(bool playerBot, bool recovery)
    {
        var firstKey = new OrganismKey(3, 2);
        var secondKey = new OrganismKey(4, 1);
        var engine = new RecordingEngineClient(RecordingEngineClient.CreateSnapshot() with
        {
            Population = 2,
            WorldSize = [640f, 480f],
            Organisms =
            [
                RecordingEngineClient.Organism with { Position = [300f, 225f] },
                new OrganismSnapshot(4, 1, [220f, 200f], [0f, 0f], 1_000, 10),
            ],
            Obstacles = [new ObstacleSnapshot(1, [300f, 80f], [380f, 140f])],
            Teleporters = [new TeleporterSnapshot(2, [480f, 150f], 25f, [550f, 150f])],
            Ties = [new TieSnapshot(firstKey, secondKey, 120f)],
            PlayerBot = playerBot ? new PlayerBotSnapshot(true, secondKey, [], []) : PlayerBotSnapshot.Default,
        });
        var recoveryPath = recovery ? Path.GetTempFileName() : null;
        try
        {
            if (recoveryPath is not null) File.WriteAllBytes(recoveryPath, BitConverter.GetBytes(0UL));
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var setup = new WorldSetupOptions { LoadSavePath = recoveryPath, IsAutosaveRecovery = recovery };
                var window = new MainWindow([], setup, new World09EngineFactory(engine), new AuditStorageService())
                {
                    WindowState = WindowState.Normal,
                    Width = 1280,
                    Height = 900,
                };
                try
                {
                    window.Show();
                    var viewModel = ViewModel(window);
                    await WaitFor(() => viewModel.Population == 2 && (!recovery || viewModel.Status == "AUTOSAVE RECOVERED"));
                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    var viewport = window.FindControl<WorldViewport>("Viewport")!;
                    var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                    var robotInfo = menu.FindCommandItem(LegacyCommandId.RobotShowInfo)!;

                    Assert.Equal(firstKey, viewModel.SelectedKey);
                    ClickWorld(window, viewport, 220, 200);
                    Assert.Equal(secondKey, viewModel.SelectedKey);
                    Assert.True(robotInfo.IsEnabled);

                    ClickWorld(window, viewport, 340, 110);
                    Assert.Equal("OBSTACLE 1 SELECTED", viewModel.Status);
                    Assert.Equal(secondKey, viewModel.SelectedKey);
                    ClickWorld(window, viewport, 480, 150);
                    Assert.Equal("TELEPORTER 2 SELECTED", viewModel.Status);
                    Assert.Equal(secondKey, viewModel.SelectedKey);

                    ClickWorld(window, viewport, 560, 370, MouseButton.Right);
                    ClickWorld(window, viewport, 560, 370, MouseButton.Middle);
                    Assert.Equal(secondKey, viewModel.SelectedKey);

                    menu.FindCommandItem(LegacyCommandId.RobotSelectEntireOrganism)!
                        .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => viewModel.Status == "ORGANISM SELECTED · 2 BOT(S)");
                    ClickWorld(window, viewport, 560, 370);

                    OrganismKey? expectedFocus = playerBot || recovery ? secondKey : null;
                    Assert.Equal(expectedFocus, viewModel.SelectedKey);
                    Assert.Equal(expectedFocus.HasValue, robotInfo.IsEnabled);

                    var previousTick = viewModel.Tick;
                    await Session(window)!.StepAsync();
                    await WaitFor(() => viewModel.Tick > previousTick);
                    Assert.Equal(expectedFocus, viewModel.SelectedKey);
                    var marks = (HashSet<OrganismKey>)typeof(WorldViewport)
                        .GetField("_selectedKeys", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(viewport)!;
                    Assert.Equal([firstKey, secondKey], marks.OrderBy(value => value.Slot).ToArray());
                }
                finally
                {
                    window.Close();
                }
            }, CancellationToken.None);
        }
        finally
        {
            if (recoveryPath is not null) File.Delete(recoveryPath);
        }
    }

    [Fact]
    public async Task World09CommandsUseSelectedPlayerAndStayInertWithoutSelection()
    {
        var engine = new RecordingEngineClient(RecordingEngineClient.CreateSnapshot() with { WorldSize = [640f, 480f] });
        var factory = new World09EngineFactory(engine);

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), factory, new AuditStorageService())
            {
                Width = 1280,
                Height = 900,
            };
            try
            {
                window.Show();
                await WaitFor(() => Session(window) is not null && ViewModel(window).Population == 1);

                var move = window.FindControl<Button>("MoveButton")!;
                var clone = window.FindControl<Button>("CloneButton")!;
                var reproduce = window.FindControl<Button>("ReproduceButton")!;
                var kill = window.FindControl<Button>("KillButton")!;
                ViewModel(window).ClearSelection();

                foreach (var button in new[] { move, clone, reproduce, kill })
                    button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.Empty(engine.Calls);
                Assert.Equal("NONE", ViewModel(window).SelectedId);

                var key = new OrganismKey(3, 2);
                ViewModel(window).Select(key);
                window.FindControl<WorldViewport>("Viewport")!.SelectKey(key);

                move.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                RaiseWorldClick(window.FindControl<WorldViewport>("Viewport")!, [160f, 260f]);
                await WaitFor(() => engine.Calls.Contains("Move:3:2:160:260"));

                ViewModel(window).Select(key);
                clone.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => engine.Calls.Contains("Clone:3:2"));

                ViewModel(window).Select(key);
                kill.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => engine.Calls.Contains("Remove:3:2"));

                ViewModel(window).Select(key);
                reproduce.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => engine.Calls.Contains("Reproduce:3:2:none"));

                Assert.Equal(
                    ["Move:3:2:160:260", "Clone:3:2", "Remove:3:2", "Reproduce:3:2:none"],
                    engine.Calls.Where(call => call.StartsWith("Move:", StringComparison.Ordinal)
                        || call.StartsWith("Clone:", StringComparison.Ordinal)
                        || call.StartsWith("Remove:", StringComparison.Ordinal)
                        || call.StartsWith("Reproduce:", StringComparison.Ordinal)).ToArray());

                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var frame = window.CaptureRenderedFrame();
                Assert.True(frame.PixelSize.Width >= 1200 && frame.PixelSize.Height >= 800);
                Assert.All(new[] { move, clone, reproduce, kill }, button => Assert.True(button.IsVisible));
            }
            finally
            {
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static SimulationSession? Session(MainWindow window) =>
        typeof(MainWindow).GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)?.GetValue(window) as SimulationSession;

    private static MainWindowViewModel ViewModel(MainWindow window) =>
        (MainWindowViewModel)typeof(MainWindow).GetField("_viewModel", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window)!;

    private static void ClickWorld(Window window, WorldViewport viewport, double x, double y, MouseButton button = MouseButton.Left)
    {
        var scaleX = viewport.Bounds.Width / 640;
        var scaleY = viewport.Bounds.Height / 480;
        if (viewport.FixedRatio) scaleX = scaleY = Math.Min(scaleX, scaleY);
        var local = new Point(x * scaleX * viewport.Zoom + viewport.Pan.X, y * scaleY * viewport.Zoom + viewport.Pan.Y);
        var point = viewport.TranslatePoint(local, window);
        Assert.True(point.HasValue);
        window.MouseDown(point.Value, button, RawInputModifiers.None);
        window.MouseUp(point.Value, button, RawInputModifiers.None);
    }

    private static void RaiseWorldClick(WorldViewport viewport, float[] position) =>
        ((Action<float[]>)typeof(WorldViewport).GetField("WorldClicked", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(viewport)!).Invoke(position);

    private static async Task WaitFor(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(5);
        while (!condition() && DateTime.UtcNow < deadline) await Task.Delay(10);
        Assert.True(condition());
    }
}

internal sealed class World09EngineFactory(RecordingEngineClient engine) : IEngineClientFactory
{
    public IEngineClient Create(WorldSetupOptions setup) => engine;
}
