using System.Reflection;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Input.Platform;
using Avalonia.Interactivity;
using Avalonia.VisualTree;
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
    [InlineData(null)]
    [InlineData(12)]
    public async Task FindBestWithNoCandidateClearsOnlyNativeFocusAndKeepsNativeMarks(int? nativeCycle)
    {
        var robot = RecordingEngineClient.Organism;
        var key = new OrganismKey(robot.Slot, robot.Generation);
        var engine = new RecordingEngineClient(RecordingEngineClient.CreateSnapshot() with
        {
            NativeCycle = nativeCycle,
            Organisms = [robot with { Vegetable = true }],
        });
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(),
                new World09EngineFactory(engine), new AuditStorageService());
            try
            {
                window.Show();
                var model = ViewModel(window);
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                var info = menu.FindCommandItem(LegacyCommandId.RobotShowInfo)!;
                await WaitFor(() => model.SelectedKey == key && info.IsEnabled);
                menu.FindCommandItem(LegacyCommandId.RobotSelectEntireOrganism)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => model.Status == "ORGANISM SELECTED · 1 BOT(S)");

                menu.FindCommandItem(LegacyCommandId.RobotFindBest)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => model.Status == "NO LIVING ROBOTS");

                Assert.Equal(nativeCycle is null ? key : (OrganismKey?)null, model.SelectedKey);
                Assert.Equal(nativeCycle is null, info.IsEnabled);
                Assert.Equal(nativeCycle, model.NativeCycle);
                var marks = (HashSet<OrganismKey>)typeof(WorldViewport)
                    .GetField("_selectedKeys", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(viewport)!;
                Assert.Equal(key, Assert.Single(marks));
            }
            finally { window.Close(); }
        }, CancellationToken.None);
    }

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

                    Assert.Equal(playerBot ? secondKey : firstKey, viewModel.SelectedKey);
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

    [Theory]
    [InlineData(true)]
    [InlineData(false)]
    public async Task NativeRobotInfoFollowsTheNextCycleAndReopensTheSameModelessWindow(bool native)
    {
        var engine = new RecordingEngineClient(RecordingEngineClient.CreateSnapshot() with
        {
            NativeCycle = native ? 0 : null,
            Population = 2,
            WorldSize = [640f, 480f],
            Organisms =
            [
                RecordingEngineClient.Organism with { Position = [300f, 225f], Memory = [new(81, 101)] },
                new OrganismSnapshot(4, 1, [220f, 200f], [0f, 0f], 1_000, 10) { Memory = [new(81, 202)] },
            ],
        });
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), new World09EngineFactory(engine), new AuditStorageService())
            {
                WindowState = WindowState.Normal,
                Width = 1280,
                Height = 900,
            };
            try
            {
                window.Show();
                var viewModel = ViewModel(window);
                await WaitFor(() => viewModel.Population == 2);
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var command = menu.FindCommandItem(LegacyCommandId.RobotShowInfo)!;
                window.FindControl<Button>("PauseButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

                command.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<RobotDetailsWindow>().Any());
                var info = Assert.Single(window.OwnedWindows.OfType<RobotDetailsWindow>());
                Assert.Equal(native, info.Topmost);
                info.UpdateLayout();
                Assert.Equal(native, info.GetVisualDescendants().OfType<Button>()
                    .Any(button => button.Name == "NativeRobotInfoReproduce" && button.IsVisible));
                info.SelectTab(RobotDetailsTab.Memory);
                var memory = (TextBox)typeof(RobotDetailsWindow)
                    .GetField("_memory", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(info)!;
                Assert.Contains("101", memory.Text);

                ClickWorld(window, viewport, 220, 200);
                Assert.Equal(new OrganismKey(4, 1), viewModel.SelectedKey);
                Assert.Contains("3:2", info.Title);
                await Session(window)!.StepAsync();
                if (native)
                {
                    await WaitFor(() => info.Title!.EndsWith("4:1", StringComparison.Ordinal)
                        && memory.Text!.Contains("202", StringComparison.Ordinal) && engine.DnaExports.Contains((4u, 1u)));
                    Assert.Same(info, Assert.Single(window.OwnedWindows.OfType<RobotDetailsWindow>()));
                    ClickWorld(window, viewport, 560, 370);
                    Assert.False(info.IsVisible);
                    ClickWorld(window, viewport, 300, 225);
                    command.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => info.IsVisible && info.Title!.EndsWith("3:2", StringComparison.Ordinal));
                    Assert.Same(info, Assert.Single(window.OwnedWindows.OfType<RobotDetailsWindow>()));
                    Assert.Contains("101", memory.Text);

                    window.FindControl<Button>("RunButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    ClickWorld(window, viewport, 220, 200);
                    await WaitFor(() => info.Title!.EndsWith("4:1", StringComparison.Ordinal)
                        && memory.Text!.Contains("202", StringComparison.Ordinal));
                    ClickWorld(window, viewport, 560, 370);
                    await WaitFor(() => memory.Text == string.Empty);
                    Assert.True(info.IsVisible);
                }
                else
                {
                    Assert.Contains("3:2", info.Title);
                    Assert.Contains("101", memory.Text);
                    command.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => window.OwnedWindows.OfType<RobotDetailsWindow>().Count() == 2);
                    ClickWorld(window, viewport, 560, 370);
                    Assert.True(info.IsVisible);
                }
            }
            finally { window.Close(); }
        }, CancellationToken.None);
    }

    [Theory]
    [InlineData(true, true)]
    [InlineData(true, false)]
    [InlineData(false, true)]
    public async Task RobotInfoDoubleClickCopiesOnlyTheDisplayedNativeFounderFilename(bool native, bool hasFounderName)
    {
        const string firstName = "First, Mixed Case.TXT";
        const string secondName = "Second-Founder.txt";
        const string previousClipboard = "Previous clipboard contents";
        var first = RecordingEngineClient.Organism with { Position = [300f, 225f], FounderName = hasFounderName ? firstName : null };
        var second = new OrganismSnapshot(4, 1, [220f, 200f], [0f, 0f], 1_000, 10)
        {
            FounderName = hasFounderName ? secondName : null,
        };
        var engine = new RecordingEngineClient(RecordingEngineClient.CreateSnapshot() with
        {
            NativeCycle = native ? 0 : null,
            Population = 2,
            WorldSize = [640f, 480f],
            Organisms = [first, second],
        });

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), new World09EngineFactory(engine), new AuditStorageService())
            {
                WindowState = WindowState.Normal,
                Width = 1280,
                Height = 900,
            };
            try
            {
                window.Show();
                var viewModel = ViewModel(window);
                await WaitFor(() => viewModel.Population == 2);
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                var command = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!
                    .FindCommandItem(LegacyCommandId.RobotShowInfo)!;
                window.FindControl<Button>("PauseButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                command.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<RobotDetailsWindow>().Any());
                var info = Assert.Single(window.OwnedWindows.OfType<RobotDetailsWindow>());
                var labels = info.GetVisualDescendants().OfType<TextBlock>().ToArray();
                var founder = Assert.Single(labels, label => label.Name == "RobotFounderFilename");
                var subtitle = Assert.Single(labels, label => label.Text?.StartsWith("LIVE ROBOT 3:2", StringComparison.Ordinal) == true);
                var clipboard = Assert.IsAssignableFrom<IClipboard>(info.Clipboard);
                await clipboard.SetTextAsync(previousClipboard);

                if (!native || !hasFounderName)
                {
                    Assert.False(founder.IsVisible);
                    Assert.Contains("ANIMAL MINIMALIS", subtitle.Text);
                    DoubleClickLabel(info, subtitle);
                    Assert.Equal(previousClipboard, await clipboard.TryGetTextAsync());
                    return;
                }

                Assert.True(founder.IsVisible);
                Assert.Equal(firstName, founder.Text);
                DoubleClickLabel(info, subtitle);
                Assert.Equal(previousClipboard, await clipboard.TryGetTextAsync());

                ClickWorld(window, viewport, 220, 200);
                Assert.Equal(new OrganismKey(second.Slot, second.Generation), viewModel.SelectedKey);
                Assert.Equal(firstName, founder.Text);
                DoubleClickLabel(info, founder);
                Assert.Equal(firstName, await clipboard.TryGetTextAsync());

                await Session(window)!.StepAsync();
                await WaitFor(() => founder.Text == secondName && info.Title!.EndsWith("4:1", StringComparison.Ordinal));
                DoubleClickLabel(info, subtitle);
                Assert.Equal(firstName, await clipboard.TryGetTextAsync());
                DoubleClickLabel(info, founder);
                Assert.Equal(secondName, await clipboard.TryGetTextAsync());
            }
            finally { window.Close(); }
        }, CancellationToken.None);
    }

    private static void DoubleClickLabel(Window window, TextBlock label)
    {
        window.UpdateLayout();
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        var point = label.TranslatePoint(new Point(label.Bounds.Width / 2, label.Bounds.Height / 2), window);
        Assert.True(point.HasValue);
        window.MouseDown(point.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(point.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseDown(point.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(point.Value, MouseButton.Left, RawInputModifiers.None);
    }

    [Fact]
    public async Task NativeRobotInfoReproducesTheDisplayedParentWithoutAdvancingOrChangingFocus()
    {
        var first = RecordingEngineClient.Organism with { Position = [300f, 225f], UniqueId = 70 };
        var second = new OrganismSnapshot(4, 1, [220f, 200f], [0f, 0f], 1_000, 10) { UniqueId = 71 };
        var firstKey = new OrganismKey(first.Slot, first.Generation);
        var secondKey = new OrganismKey(second.Slot, second.Generation);
        var engine = new RecordingEngineClient(RecordingEngineClient.CreateSnapshot() with
        {
            NativeCycle = 20,
            Population = 2,
            WorldSize = [640f, 480f],
            Organisms = [first, second],
        }) { NativeRobotInfoResult = new OrganismKey(41, 1) };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), new World09EngineFactory(engine), new AuditStorageService())
            {
                WindowState = WindowState.Normal,
                Width = 1280,
                Height = 900,
            };
            try
            {
                window.Show();
                var model = ViewModel(window);
                await WaitFor(() => model.Population == 2 && model.SelectedKey.HasValue);
                window.FindControl<Button>("PauseButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var open = menu.FindCommandItem(LegacyCommandId.RobotShowInfo)!;
                open.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<RobotDetailsWindow>().Any());
                var info = Assert.Single(window.OwnedWindows.OfType<RobotDetailsWindow>());
                info.UpdateLayout();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                var button = Assert.Single(info.GetVisualDescendants().OfType<Button>(),
                    control => control.Name == "NativeRobotInfoReproduce");
                Assert.True(button.IsVisible);
                Assert.True(button.IsEnabled);
                Assert.Equal("Reproduce", button.Content);
                var session = Session(window)!;
                var start = session.LatestSnapshot.Tick;
                Assert.Equal(firstKey, model.SelectedKey);

                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                ClickWorld(window, viewport, 220, 200);
                Assert.Equal(secondKey, model.SelectedKey);
                Assert.Contains("3:2", info.Title);
                var gate = (SemaphoreSlim)typeof(MainWindow)
                    .GetField("_advanceGate", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window)!;
                await gate.WaitAsync();
                try
                {
                    ClickReproduce();
                    Assert.False(button.IsEnabled);
                    Assert.Empty(engine.NativeRobotInfoReproductions);
                }
                finally { gate.Release(); }

                await WaitFor(() => engine.NativeRobotInfoReproductions.Count == 1 && button.IsEnabled);
                Assert.Equal((firstKey, start, (int?)20), Assert.Single(engine.NativeRobotInfoReproductions));
                Assert.Contains(info.GetVisualDescendants().OfType<TextBlock>(), label => label.Text == "OFFSPRING BORN");
                Assert.Equal(start, session.LatestSnapshot.Tick);
                Assert.Equal(secondKey, model.SelectedKey);
                Assert.Contains("3:2", info.Title);
                Assert.True(info.IsVisible);
                Assert.False(info.IsDialog);

                await session.StepAsync();
                await WaitFor(() => info.Title!.EndsWith("4:1", StringComparison.Ordinal));
                engine.NativeRobotInfoResult = null;
                ClickReproduce();
                await WaitFor(() => engine.NativeRobotInfoReproductions.Count == 2 && button.IsEnabled);
                Assert.Equal((secondKey, start + 1, (int?)20), engine.NativeRobotInfoReproductions.Last());
                Assert.Contains(info.GetVisualDescendants().OfType<TextBlock>(), label => label.Text == "NO OFFSPRING");
                Assert.Equal(start + 1, session.LatestSnapshot.Tick);
                Assert.Equal(secondKey, model.SelectedKey);
                Assert.Same(info, Assert.Single(window.OwnedWindows.OfType<RobotDetailsWindow>()));
                Assert.DoesNotContain(engine.Calls, call => call.StartsWith("Reproduce:", StringComparison.Ordinal));

                void ClickReproduce()
                {
                    var center = new Point(button.Bounds.Width / 2, button.Bounds.Height / 2);
                    var pointer = button.TranslatePoint(center, info);
                    Assert.True(pointer.HasValue);
                    info.MouseDown(pointer.Value, MouseButton.Left, RawInputModifiers.None);
                    info.MouseUp(pointer.Value, MouseButton.Left, RawInputModifiers.None);
                }
            }
            finally { window.Close(); }
        }, CancellationToken.None);
    }

    [Theory]
    [InlineData(true)]
    [InlineData(false)]
    public async Task NativeRobotInfoConsoleUsesTheCurrentArenaFocusWithoutAdvancing(bool native)
    {
        var first = RecordingEngineClient.Organism with { Position = [300f, 225f] };
        var second = new OrganismSnapshot(4, 1, [220f, 200f], [0f, 0f], 1_000, 10);
        var secondKey = new OrganismKey(second.Slot, second.Generation);
        var engine = new RecordingEngineClient(RecordingEngineClient.CreateSnapshot() with
        {
            NativeCycle = native ? 20 : null,
            Population = 2,
            WorldSize = [640f, 480f],
            Organisms = [first, second],
        });
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(),
                new World09EngineFactory(engine), new AuditStorageService())
            {
                WindowState = WindowState.Normal,
                Width = 1280,
                Height = 900,
            };
            try
            {
                window.Show();
                var model = ViewModel(window);
                await WaitFor(() => model.Population == 2);
                window.FindControl<Button>("PauseButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                window.FindControl<LegacyMenuBar>("HistoricMenuBar")!
                    .FindCommandItem(LegacyCommandId.RobotShowInfo)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<RobotDetailsWindow>().Any());
                var info = Assert.Single(window.OwnedWindows.OfType<RobotDetailsWindow>());
                var console = info.GetVisualDescendants().OfType<Button>()
                    .SingleOrDefault(button => button.Name == "NativeRobotInfoOpenConsole");
                if (!native)
                {
                    Assert.Null(console);
                    return;
                }
                Assert.NotNull(console);
                Assert.True(console.IsVisible);
                Assert.Contains(info.GetVisualDescendants().OfType<Button>(),
                    button => button.Name == "NativeRobotInfoDescendants" && button.IsVisible);
                var session = Session(window)!;
                var initialTick = session.LatestSnapshot.Tick;

                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                ClickWorld(window, window.FindControl<WorldViewport>("Viewport")!, 220, 200);
                Assert.Equal(secondKey, model.SelectedKey);
                Assert.Contains("3:2", info.Title);
                console.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                var consoleWindow = Assert.Single(window.OwnedWindows.OfType<RobotConsoleWindow>());
                Assert.Contains("4:1", consoleWindow.Title);
                Assert.False(consoleWindow.IsDialog);
                console.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.Same(consoleWindow, Assert.Single(window.OwnedWindows.OfType<RobotConsoleWindow>()));
                Assert.Equal(secondKey, model.SelectedKey);

                model.ClearSelection();
                console.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.Same(consoleWindow, Assert.Single(window.OwnedWindows.OfType<RobotConsoleWindow>()));
                Assert.Contains(info.GetVisualDescendants().OfType<TextBlock>(), label => label.Text == "NO ROBOT SELECTED");
                Assert.Null(model.SelectedKey);
                Assert.Equal(initialTick, session.LatestSnapshot.Tick);
            }
            finally { window.Close(); }
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
