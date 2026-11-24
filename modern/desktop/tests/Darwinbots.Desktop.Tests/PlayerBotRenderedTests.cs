using System.Collections.Concurrent;
using System.Globalization;
using System.Reflection;
using System.Runtime.InteropServices;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class PlayerBotRenderedTests
{
    [Fact]
    public async Task F1CompetitionDisablesPlayerBotAndEyeDesignerUntilLoadingAnOrdinaryWorld()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            PopulationCapacity = 64,
            Species =
            [
                new("First", "start\nstop", false, 0xff405060, 1, 1_000, 0, false, 0f),
                new("Second", "start\nstop", false, 0xff8090a0, 1, 1_000, 0, false, 0f),
            ],
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService());
            window.Show();
            try
            {
                var model = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitFor(() => model.Population == 2);
                var session = Field<SimulationSession>(window, "_session");
                var bot = session.LatestSnapshot.Organisms[0];
                var key = new OrganismKey(bot.Slot, bot.Generation);
                model.Select(key);
                window.FindControl<WorldViewport>("Viewport")!.SelectKey(key);
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var toggle = menu.FindCommandItem(LegacyCommandId.EditEnablePlayerBot)!;
                var eyeDesigner = menu.FindCommandItem(LegacyCommandId.EditEyeDesigner)!;
                var settings = menu.FindCommandItem(LegacyCommandId.EditPlayerBotSettings)!;
                await session.RefreshSnapshotAsync();
                var ordinarySave = await session.SaveAsync();
                await WaitFor(() => toggle.IsEnabled && eyeDesigner.IsEnabled);

                await session.ConfigureLocalModeAsync(LocalModeConfig.Default with { Kind = LocalModeKind.Contest });
                Assert.True(session.LatestSnapshot.NativeF1Enabled);
                await WaitFor(() => !toggle.IsEnabled && !eyeDesigner.IsEnabled);
                Assert.True(settings.IsEnabled);
                window.KeyPress(Key.F11, RawInputModifiers.None, PhysicalKey.F11, null);
                window.KeyRelease(Key.F11, RawInputModifiers.None, PhysicalKey.F11, null);
                await session.RefreshSnapshotAsync();
                Assert.False(session.LatestSnapshot.PlayerBot.Enabled);

                await session.LoadAsync(ordinarySave);
                Assert.False(session.LatestSnapshot.NativeF1Enabled);
                await WaitFor(() => toggle.IsEnabled && eyeDesigner.IsEnabled);
                window.KeyPress(Key.F11, RawInputModifiers.None, PhysicalKey.F11, null);
                window.KeyRelease(Key.F11, RawInputModifiers.None, PhysicalKey.F11, null);
                await WaitFor(() => session.LatestSnapshot.PlayerBot.Enabled);
            }
            finally { window.Close(); }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task HeldFieldPointerAimsFocusedAndShiftHighlightedRobotsUntilRelease()
    {
        var setup = new WorldSetupOptions { Backend = "Cpu", WorldWidth = 1000, WorldHeight = 1000, PopulationCapacity = 8 };
        await using var session = new SimulationSession(new NativeEngineClient(setup));
        await session.ImportDnaAsync("start\nstop", [300, 300]);
        await session.ImportDnaAsync("start\nstop", [600, 300]);
        var (focus, other) = (session.LatestSnapshot.Organisms[0], session.LatestSnapshot.Organisms[1]);

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService());
            DesktopTestApplication.UseSession(window, session);
            window.Show();
            try
            {
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                var focusPoint = WorldPoint(300, 300);
                window.MouseDown(focusPoint, MouseButton.Left, RawInputModifiers.None);
                window.MouseUp(focusPoint, MouseButton.Left, RawInputModifiers.None);
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                menu.FindCommandItem(LegacyCommandId.EditEnablePlayerBot)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => session.LatestSnapshot.PlayerBot.Enabled);

                var highlight = WorldPoint(600, 300);
                window.MouseDown(highlight, MouseButton.Left, RawInputModifiers.Shift);
                window.MouseUp(highlight, MouseButton.Left, RawInputModifiers.Shift);
                await session.StepAsync(0);
                Assert.Equal(new OrganismKey(focus.Slot, focus.Generation), session.LatestSnapshot.PlayerBot.Focus);

                var blankPoint = WorldPoint(140, 100);
                var target = WorldPoint(300, 50);
                window.MouseDown(blankPoint, MouseButton.Left, RawInputModifiers.None);
                await session.StepAsync();
                Assert.Equal(0, Aim(focus.Slot));
                Assert.Equal(0, Aim(other.Slot));

                window.MouseMove(target, RawInputModifiers.LeftMouseButton);
                await session.StepAsync();
                Assert.InRange(Aim(focus.Slot), 313, 315);
                Assert.InRange(Aim(other.Slot), 488, 490);

                window.MouseUp(target, MouseButton.Left, RawInputModifiers.None);
                await session.SetOrganismMemoryAsync(focus.Slot, focus.Generation, 19, 0);
                await session.SetOrganismMemoryAsync(other.Slot, other.Generation, 19, 0);
                await session.StepAsync();
                Assert.Equal(0, Aim(focus.Slot));
                Assert.Equal(0, Aim(other.Slot));

                int Aim(uint slot) => session.LatestSnapshot.Organisms.Single(robot => robot.Slot == slot).Aim;
                Point WorldPoint(float x, float y)
                {
                    var scaleX = viewport.Bounds.Width / setup.WorldWidth;
                    var scaleY = viewport.Bounds.Height / setup.WorldHeight;
                    if (viewport.FixedRatio) scaleX = scaleY = Math.Min(scaleX, scaleY);
                    return viewport.TranslatePoint(new Point(x * scaleX * viewport.Zoom + viewport.Pan.X,
                        y * scaleY * viewport.Zoom + viewport.Pan.Y), window)!.Value;
                }
            }
            finally
            {
                typeof(MainWindow).GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!.SetValue(window, null);
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task F11AndMappedKeysWorkWithNoFocusedRobotAndAfterFocusDies()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu", WorldWidth = 1000, WorldHeight = 1000, PopulationCapacity = 8,
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService());
            window.Show();
            try
            {
                await WaitFor(() => TryField<SimulationSession>(window, "_session") is not null);
                var session = Field<SimulationSession>(window, "_session");
                var viewModel = (MainWindowViewModel)window.DataContext!;
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var toggle = menu.FindCommandItem(LegacyCommandId.EditEnablePlayerBot)!;
                await WaitFor(() => toggle.IsEnabled);
                Assert.Null(viewModel.SelectedKey);
                await session.ConfigurePlayerBotAsync(new PlayerBotConfig(false, null,
                    [new PlayerBotBinding((int)Key.W, 19, 77)]));
                viewport.Focus();

                PressF11();
                await WaitFor(() => toggle.IsChecked && session.LatestSnapshot.PlayerBot.Enabled);
                Assert.Null(session.LatestSnapshot.PlayerBot.Focus);
                const string dna = "start\nstop";
                await session.ImportDnaAsync(dna, [500, 300]);
                await session.ImportDnaAsync(dna, [800, 300]);
                var robots = session.LatestSnapshot.Organisms;
                var highlighted = robots[0];
                var focus = robots[1];
                await WaitFor(() => viewModel.Population == 2);
                Assert.Null(viewModel.SelectedKey);
                window.KeyPress(Key.W, RawInputModifiers.None, PhysicalKey.W, null);
                await WaitFor(() => session.LatestSnapshot.PlayerBot.PressedKeys.SequenceEqual([(int)Key.W]));
                await session.StepAsync();
                Assert.Equal(0, Aim(highlighted.Slot));
                Assert.Equal(0, Aim(focus.Slot));

                ClickRobot(window, viewport, setup, highlighted, RawInputModifiers.Shift);
                await session.StepAsync();
                Assert.Null(viewModel.SelectedKey);
                Assert.Null(session.LatestSnapshot.PlayerBot.Focus);
                Assert.Equal(77, Aim(highlighted.Slot));
                Assert.Equal(0, Aim(focus.Slot));

                ClickRobot(window, viewport, setup, focus, RawInputModifiers.None);
                await WaitFor(() => session.LatestSnapshot.PlayerBot.Focus == new OrganismKey(focus.Slot, focus.Generation));
                await session.StepAsync();
                Assert.Equal(77, Aim(focus.Slot));
                await session.RemoveAsync(focus.Slot, focus.Generation);
                await WaitFor(() => viewModel.SelectedKey is null && session.LatestSnapshot.PlayerBot.Focus is null);
                Assert.True(toggle.IsChecked);
                window.KeyRelease(Key.W, RawInputModifiers.None, PhysicalKey.W, null);
                await WaitFor(() => session.LatestSnapshot.PlayerBot.PressedKeys.Count == 0);
                await session.SetOrganismMemoryAsync(highlighted.Slot, highlighted.Generation, 19, 0);
                ClickRobot(window, viewport, setup, highlighted, RawInputModifiers.Shift);
                await session.StepAsync();
                Assert.Equal(0, Aim(highlighted.Slot));

                PressF11();
                await WaitFor(() => !toggle.IsChecked && !session.LatestSnapshot.PlayerBot.Enabled);

                int Aim(uint slot) => session.LatestSnapshot.Organisms.Single(robot => robot.Slot == slot).Aim;
                void PressF11()
                {
                    window.KeyPress(Key.F11, RawInputModifiers.None, PhysicalKey.F11, null);
                    window.KeyRelease(Key.F11, RawInputModifiers.None, PhysicalKey.F11, null);
                }
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task FocusFollowsTheHighestSurvivingHighlightAfterManualAndNaturalDeaths()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu", WorldWidth = 1000, WorldHeight = 1000, PopulationCapacity = 8,
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService());
            window.Show();
            try
            {
                await WaitFor(() => TryField<SimulationSession>(window, "_session") is not null);
                var session = Field<SimulationSession>(window, "_session");
                var viewModel = (MainWindowViewModel)window.DataContext!;
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var toggle = menu.FindCommandItem(LegacyCommandId.EditEnablePlayerBot)!;
                var kill = menu.FindCommandItem(LegacyCommandId.RobotKill)!;
                viewport.ShowResourceGauges = true;

                const string dna = "start\nstop";
                await session.ImportDnaAsync(dna, [250, 300]);
                await session.ImportDnaAsync(dna, [500, 300]);
                await session.ImportDnaAsync("start\n100 .strbody store\nstop", [750, 300]);
                var robots = session.LatestSnapshot.Organisms;
                var first = robots[0];
                var lower = robots[1];
                var higher = robots[2];
                await WaitFor(() => viewModel.Population == 3);

                ClickRobot(window, viewport, setup, first, RawInputModifiers.None);
                toggle.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => toggle.IsChecked && Focused(first));
                ClickRobot(window, viewport, setup, higher, RawInputModifiers.Shift);
                ClickRobot(window, viewport, setup, lower, RawInputModifiers.Shift);
                await session.StepAsync(0);
                kill.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => viewModel.Population == 2 && Focused(higher));
                Assert.True(kill.IsEnabled);

                await session.SetOrganismEnergyAsync(higher.Slot, higher.Generation, 1);
                await session.StepAsync();
                await WaitFor(() => viewModel.Population == 1 && Focused(lower));
                Assert.True(toggle.IsChecked);

                bool Focused(OrganismSnapshot robot)
                {
                    var key = new OrganismKey(robot.Slot, robot.Generation);
                    return session.LatestSnapshot.PlayerBot.Focus == key
                        && viewModel.SelectedKey == key
                        && viewport.OverlayProjection.Gauges.FirstOrDefault()?.Organism == key;
                }
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Theory]
    [InlineData(true)]
    [InlineData(false)]
    public async Task DraggingAMarkedMemberMovesTheGroupBeforeRelease(bool playerBot)
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu", WorldWidth = 1000, WorldHeight = 1000, PopulationCapacity = 8,
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService());
            window.Show();
            try
            {
                await WaitFor(() => TryField<SimulationSession>(window, "_session") is not null);
                var session = Field<SimulationSession>(window, "_session");
                var viewModel = (MainWindowViewModel)window.DataContext!;
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                const string idle = "start\nstop";
                await session.ImportDnaAsync(playerBot ? idle : "cond *.numties 0 = start 41 .tie store stop", [250, 400]);
                await session.ImportDnaAsync(idle, [500, 400]);
                await session.ImportDnaAsync(idle, [820, 200]);
                if (!playerBot)
                {
                    await session.StepAsync(24);
                    Assert.Single(session.LatestSnapshot.Ties);
                }
                await WaitFor(() => viewModel.Population == 3);
                var (anchor, grabbed, outsider) = (session.LatestSnapshot.Organisms[0],
                    session.LatestSnapshot.Organisms[1], session.LatestSnapshot.Organisms[2]);
                ClickRobot(window, viewport, setup, outsider, RawInputModifiers.None);
                if (playerBot)
                {
                    menu.FindCommandItem(LegacyCommandId.EditEnablePlayerBot)!
                        .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => session.LatestSnapshot.PlayerBot.Enabled);
                    ClickRobot(window, viewport, setup, anchor, RawInputModifiers.Shift);
                    ClickRobot(window, viewport, setup, grabbed, RawInputModifiers.Shift);
                }
                ClickRobot(window, viewport, setup, anchor, RawInputModifiers.None);
                if (!playerBot)
                {
                    menu.FindCommandItem(LegacyCommandId.RobotSelectEntireOrganism)!
                        .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => viewModel.Status == "ORGANISM SELECTED · 2 BOT(S)");
                }

                window.MouseDown(Screen(grabbed.Position[0], grabbed.Position[1]), MouseButton.Left, RawInputModifiers.None);
                window.MouseMove(Screen(300, 650), RawInputModifiers.LeftMouseButton);
                await WaitFor(() => At(anchor.Slot, 300, 650));
                Assert.True(At(grabbed.Slot, 300 + grabbed.Position[0] - anchor.Position[0],
                    650 + grabbed.Position[1] - anchor.Position[1]));
                Assert.True(At(outsider.Slot, outsider.Position[0], outsider.Position[1]));
                window.MouseMove(Screen(340, 680), RawInputModifiers.LeftMouseButton);
                await WaitFor(() => At(anchor.Slot, 340, 680));
                window.MouseUp(Screen(450, 700), MouseButton.Left, RawInputModifiers.None);
                await session.RefreshSnapshotAsync();
                Assert.True(At(anchor.Slot, 340, 680));

                bool At(uint slot, float x, float y)
                {
                    var position = session.LatestSnapshot.Organisms.Single(robot => robot.Slot == slot).Position;
                    return Math.Abs(position[0] - x) < 1 && Math.Abs(position[1] - y) < 1;
                }

                Point Screen(float x, float y)
                {
                    var scaleX = viewport.Bounds.Width / setup.WorldWidth;
                    var scaleY = viewport.Bounds.Height / setup.WorldHeight;
                    if (viewport.FixedRatio) scaleX = scaleY = Math.Min(scaleX, scaleY);
                    return viewport.TranslatePoint(new Point(x * scaleX * viewport.Zoom + viewport.Pan.X,
                        y * scaleY * viewport.Zoom + viewport.Pan.Y), window)!.Value;
                }
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task RealPlayerBotKeyMapRoutesVisibleKeysToTheNextAuthoritativeCycle()
    {
        var factory = new PlayerBotTestEngineFactory();

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), factory, new AuditStorageService());
            window.Show();
            try
            {
                await WaitFor(() => TryField<SimulationSession>(window, "_session") is not null);
                var session = Field<SimulationSession>(window, "_session");
                await session.StepAsync(0);
                await WaitFor(() => Field<MainWindowViewModel>(window, "_viewModel").Population == 1);

                var focus = factory.Client.Focus;
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                viewModel.Select(focus);
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                viewport.SetSnapshot(session.LatestSnapshot);
                viewport.SelectKey(focus);
                window.GetType().GetMethod("RefreshLegacyMenuStates", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .Invoke(window, null);

                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                menu.FindCommandItem(LegacyCommandId.EditPlayerBotSettings)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<PlayerBotSettingsWindow>().Any());
                var dialog = Assert.Single(window.OwnedWindows.OfType<PlayerBotSettingsWindow>());

                AssertKeyMapSurface(dialog, "KEY MAP · PRESSED XOR INVERT WRITES MEMORY AFTER SENSE CLEAR");

                var fields = dialog.GetVisualDescendants().OfType<TextBox>().ToArray();
                Assert.Equal(3, fields.Length);
                var capture = dialog.GetVisualDescendants().OfType<Button>()
                    .Single(button => button.Content?.ToString() == "CAPTURE KEY");
                var add = dialog.GetVisualDescendants().OfType<Button>()
                    .Single(button => button.Content?.ToString() == "ADD");
                var invert = dialog.GetVisualDescendants().OfType<CheckBox>().Single();
                var status = Field<TextBlock>(dialog, "_status");

                CaptureAndAdd(dialog, capture, add, fields, invert, Key.W, PhysicalKey.W, 50, 123, false);
                CaptureAndAdd(dialog, capture, add, fields, invert, Key.Space, PhysicalKey.Space, 51, -77, false);
                Assert.Equal(2, dialog.Bindings.Count);
                Assert.Equal("BINDING ADDED", status.Text);

                fields[0].Text = "not-a-key";
                fields[1].Text = "52";
                fields[2].Text = "9";
                add.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.Equal("ENTER KEY, MEMORY, AND VALUE", status.Text);
                Assert.Equal(2, dialog.Bindings.Count);

                fields[0].Text = ((int)Key.W).ToString(CultureInfo.InvariantCulture);
                fields[1].Text = "52";
                fields[2].Text = "9";
                add.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.Equal("BINDING ADDED", status.Text);
                Assert.Equal(3, dialog.Bindings.Count);

                const string originalPreset = " 87 \r\n 50 \r\n 123 \r\nFalse\r\n 32 \r\n 51 \r\n-77 \r\nFalse\r\n 87 \r\n 52 \r\n 9 \r\nFalse\r\n";
                var encodePreset = typeof(PlayerBotSettingsWindow).GetMethod("EncodePreset", BindingFlags.Instance | BindingFlags.NonPublic)!;
                var loadPreset = typeof(PlayerBotSettingsWindow).GetMethod("LoadPreset", BindingFlags.Instance | BindingFlags.NonPublic)!;
                var expectedBindings = dialog.Bindings;
                Assert.Equal((originalPreset, true), encodePreset.Invoke(dialog, null));
                const string earlierModernPreset = "65535\r\n50\r\n123\r\nFalse\r\n";
                loadPreset.Invoke(dialog, [earlierModernPreset]);
                Assert.Equal((earlierModernPreset, false), encodePreset.Invoke(dialog, null));
                loadPreset.Invoke(dialog, [originalPreset]);
                Assert.Equal(expectedBindings, dialog.Bindings);
                Assert.IsType<InvalidDataException>(
                    Assert.Throws<TargetInvocationException>(() => loadPreset.Invoke(dialog, [originalPreset + " 87 \r\n"])).InnerException);
                Assert.Equal(expectedBindings, dialog.Bindings);
                AssertKeyMapSurface(dialog, "BINDING ADDED");

                dialog.GetVisualDescendants().OfType<Button>()
                    .Single(button => button.Content?.ToString() == "APPLY")
                    .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => !dialog.IsVisible && factory.Client.Configurations.Count == 1);
                Assert.Equal(
                    [
                        new PlayerBotBinding((int)Key.W, 50, 123),
                        new PlayerBotBinding((int)Key.Space, 51, -77),
                        new PlayerBotBinding((int)Key.W, 52, 9),
                    ],
                    factory.Client.Configurations.Single().Bindings);

                menu.FindCommandItem(LegacyCommandId.EditEnablePlayerBot)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => factory.Client.Configurations.Count == 2 && session.LatestSnapshot.PlayerBot.Enabled);
                Assert.Equal(focus, session.LatestSnapshot.PlayerBot.Focus);

                viewport.Focus();
                window.KeyPress(Key.W, RawInputModifiers.None, PhysicalKey.W, null);
                await WaitFor(() => session.LatestSnapshot.PlayerBot.PressedKeys.SequenceEqual([(int)Key.W]));
                Assert.Equal(0, MemoryValue(session.LatestSnapshot, 50));
                Assert.Equal("PlayerBotKey:W:True", factory.Client.LastKeyCommand);

                await session.StepAsync(1);
                await WaitFor(() => MemoryValue(session.LatestSnapshot, 50) == 123);
                Assert.Equal(9, MemoryValue(session.LatestSnapshot, 52));
                Assert.Equal("Tick:1", factory.Client.LastCommand);

                window.KeyRelease(Key.W, RawInputModifiers.None, PhysicalKey.W, null);
                await WaitFor(() => session.LatestSnapshot.PlayerBot.PressedKeys.Count == 0);
                Assert.Equal("PlayerBotKey:W:False", factory.Client.LastKeyCommand);
                await session.StepAsync(1);
                await WaitFor(() => session.LatestSnapshot.Tick == 2);
                Assert.Equal(123, MemoryValue(session.LatestSnapshot, 50));

                window.KeyPress(Key.Space, RawInputModifiers.None, PhysicalKey.Space, null);
                await WaitFor(() => session.LatestSnapshot.PlayerBot.PressedKeys.SequenceEqual([(int)Key.Space]));
                await session.StepAsync(1);
                await WaitFor(() => MemoryValue(session.LatestSnapshot, 51) == -77);
                Assert.Equal(-77, MemoryValue(session.LatestSnapshot, 51));
                window.KeyRelease(Key.Space, RawInputModifiers.None, PhysicalKey.Space, null);
                await WaitFor(() => session.LatestSnapshot.PlayerBot.PressedKeys.Count == 0);
                await session.StepAsync(1);
                await WaitFor(() => session.LatestSnapshot.Tick == 4);
                Assert.Equal(-77, MemoryValue(session.LatestSnapshot, 51));
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static void ClickRobot(MainWindow window, WorldViewport viewport, WorldSetupOptions setup,
        OrganismSnapshot robot, RawInputModifiers modifiers)
    {
        var scaleX = viewport.Bounds.Width / setup.WorldWidth;
        var scaleY = viewport.Bounds.Height / setup.WorldHeight;
        if (viewport.FixedRatio) scaleX = scaleY = Math.Min(scaleX, scaleY);
        var point = viewport.TranslatePoint(new Point(robot.Position[0] * scaleX * viewport.Zoom + viewport.Pan.X,
            robot.Position[1] * scaleY * viewport.Zoom + viewport.Pan.Y), window)!.Value;
        window.MouseDown(point, MouseButton.Left, modifiers);
        window.MouseUp(point, MouseButton.Left, modifiers);
    }

    private static void CaptureAndAdd(
        PlayerBotSettingsWindow dialog,
        Button capture,
        Button add,
        TextBox[] fields,
        CheckBox invert,
        Key key,
        PhysicalKey physicalKey,
        ushort memory,
        int value,
        bool inverted)
    {
        invert.IsChecked = inverted;
        capture.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        dialog.KeyPress(key, RawInputModifiers.None, physicalKey, null);
        dialog.KeyRelease(key, RawInputModifiers.None, physicalKey, null);
        Assert.Equal(((int)key).ToString(CultureInfo.InvariantCulture), fields[0].Text);
        fields[1].Text = memory.ToString(CultureInfo.InvariantCulture);
        fields[2].Text = value.ToString(CultureInfo.InvariantCulture);
        add.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
    }

    private static void AssertKeyMapSurface(PlayerBotSettingsWindow dialog, string statusText)
    {
        var surfaceText = string.Join("\n", dialog.GetVisualDescendants().OfType<TextBlock>().Select(block => block.Text));
        Assert.Contains("KEY MAP", surfaceText);
        Assert.Contains(statusText, surfaceText);
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        Assert.True(dialog.Bounds.Width >= 520, $"Rendered width was {dialog.Bounds.Width}.");
        Assert.True(dialog.Bounds.Height >= 420, $"Rendered height was {dialog.Bounds.Height}.");
        using var frame = Assert.IsAssignableFrom<Bitmap>(dialog.CaptureRenderedFrame());
        Assert.True(frame.PixelSize.Width >= 520);
        Assert.True(frame.PixelSize.Height >= 420);

        var rect = new PixelRect(0, 0, frame.PixelSize.Width, frame.PixelSize.Height);
        var stride = rect.Width * 4;
        var bytes = new byte[stride * rect.Height];
        var pointer = Marshal.AllocHGlobal(bytes.Length);
        try
        {
            frame.CopyPixels(rect, pointer, bytes.Length, stride);
            Marshal.Copy(pointer, bytes, 0, bytes.Length);
        }
        finally
        {
            Marshal.FreeHGlobal(pointer);
        }

        Assert.True(ContainsApproximateColor(bytes, 0xF4, 0xF0, 0xE6));
        Assert.True(ContainsApproximateColor(bytes, 0x41, 0x6E, 0x35));
    }

    private static bool ContainsApproximateColor(byte[] pixels, byte red, byte green, byte blue)
    {
        for (var index = 0; index + 3 < pixels.Length; index += 4)
        {
            var bgra = Close(pixels[index], blue) && Close(pixels[index + 1], green) && Close(pixels[index + 2], red);
            var rgba = Close(pixels[index], red) && Close(pixels[index + 1], green) && Close(pixels[index + 2], blue);
            if (bgra || rgba) return true;
        }
        return false;

        static bool Close(byte left, byte right) => Math.Abs(left - right) <= 24;
    }

    private static int MemoryValue(EngineSnapshot snapshot, ushort address) =>
        snapshot.Organisms.Single().Memory.Single(cell => cell.Address == address).Value;

    private static T Field<T>(object target, string name) =>
        (T)target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(target)!;

    private static T? TryField<T>(object target, string name) where T : class =>
        target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)?.GetValue(target) as T;

    private static async Task WaitFor(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(8);
        while (!condition() && DateTime.UtcNow < deadline) await Task.Delay(10);
        Assert.True(condition(), "Timed out waiting for PlayerBot UI or authoritative cycle state.");
    }
}

internal sealed class PlayerBotTestEngineFactory : IEngineClientFactory
{
    public PlayerBotAuthorityEngineClient Client { get; } = new();

    public IEngineClient Create(WorldSetupOptions setup) => Client;
}

internal sealed class PlayerBotAuthorityEngineClient : IEngineClient
{
    private readonly OrganismKey _focus = new(7, 3);
    private readonly HashSet<int> _pressed = [];
    private EngineSnapshot _snapshot;
    private PlayerBotConfig _config = PlayerBotConfig.Disabled;
    private ulong _tick;

    public PlayerBotAuthorityEngineClient()
    {
        var organism = new OrganismSnapshot(7, 3, [100f, 200f], [0f, 0f], 1_000, 10, Species: 1)
        {
            Memory = [new MemoryCellSnapshot(50, 0), new MemoryCellSnapshot(51, 0)],
        };
        _snapshot = new EngineSnapshot(0, 1, "CPU", [organism])
        {
            Species = [
                new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false),
                new SpeciesSnapshot("Player Species", false, 0xff239ac0, 0, false),
            ],
        };
    }

    public OrganismKey Focus => _focus;
    public ConcurrentQueue<PlayerBotConfig> Configurations { get; } = [];
    public ConcurrentQueue<string> Commands { get; } = [];
    public string LastCommand => Commands.Last();
    public string LastKeyCommand => Commands.Last(command => command.StartsWith("PlayerBotKey:", StringComparison.Ordinal));
    public string Backend => "CPU";

    public EngineCapabilities Capabilities() => new(1, Backend, true, null);

    public void Tick(uint count = 1)
    {
        Commands.Enqueue($"Tick:{count}");
        for (var cycle = 0u; cycle < count; cycle++)
        {
            _tick++;
            var organism = _snapshot.Organisms.Single();
            var memory = organism.Memory.ToDictionary(cell => cell.Address, cell => cell.Value);
            foreach (var binding in _config.Bindings.Where(binding => _pressed.Contains(binding.KeyCode)))
                memory[binding.MemoryAddress] = binding.Value;
            _snapshot = _snapshot with
            {
                Tick = _tick,
                Organisms = [organism with
                {
                    Memory = memory.OrderBy(cell => cell.Key).Select(cell => new MemoryCellSnapshot(cell.Key, cell.Value)).ToArray(),
                }],
                PlayerBot = CurrentPlayerBot(),
            };
        }
    }

    public void ConfigurePlayerBot(PlayerBotConfig config)
    {
        _config = config;
        Configurations.Enqueue(config);
        Commands.Enqueue($"ConfigurePlayerBot:{config.Enabled}");
        _snapshot = _snapshot with { PlayerBot = CurrentPlayerBot() };
    }

    public void SetPlayerBotKey(uint slot, uint generation, int keyCode, bool pressed)
    {
        _config = _config with { Focus = new OrganismKey(slot, generation) };
        SetPlayerBotKeyState(keyCode, pressed);
    }

    public void SetPlayerBotKeyState(int keyCode, bool pressed)
    {
        if (pressed) _pressed.Add(keyCode);
        else _pressed.Remove(keyCode);
        Commands.Enqueue($"PlayerBotKey:{(Key)keyCode}:{pressed}");
        _snapshot = _snapshot with { PlayerBot = CurrentPlayerBot() };
    }

    private PlayerBotSnapshot CurrentPlayerBot() =>
        new(_config.Enabled, _config.Focus, _config.Bindings.ToArray(), _pressed.OrderBy(key => key).ToArray());

    public DnaImportReport ImportDna(string dna, float[]? position = null) => DnaImportReport.Compatible;
    public DnaImportReport ImportSpecies(SpeciesImport species) => DnaImportReport.Compatible;
    public DnaImportReport InsertSpeciesAt(SpeciesImport species, float[] position) => DnaImportReport.Compatible;
    public void Remove(uint slot, uint generation) { }
    public void RemoveMany(IReadOnlyList<OrganismKey> organisms) { }
    public void Move(uint slot, uint generation, float[] position) { }
    public OrganismKey CloneOrganism(uint slot, uint generation, float[] position) => new(100, 0);
    public void ReplaceDna(uint slot, uint generation, string dna) { }
    public string ExportDna(uint slot, uint generation) => string.Empty;
    public OrganismKey Reproduce(uint firstSlot, uint firstGeneration, uint? secondSlot, uint? secondGeneration, float[] position) => new(101, 0);
    public void SwitchBackend(string backend) { }
    public void AddObstacle(ObstacleSnapshot obstacle) { }
    public void UpdateObstacle(ObstacleSnapshot obstacle) { }
    public void RemoveObstacle(uint id) { }
    public void AddTeleporter(TeleporterSnapshot teleporter) { }
    public void UpdateTeleporter(TeleporterSnapshot teleporter) { }
    public void RemoveTeleporter(uint id) { }
    public void AddFileTeleporter(FileTeleporterSnapshot teleporter) { }
    public void UpdateFileTeleporter(FileTeleporterSnapshot teleporter) { }
    public void RemoveFileTeleporter(uint id) { }
    public void SetBrownianMotion(float value) { }
    public void UpdateMutationSettings(MutationSettingsUpdate update) { }
    public void SetTiesEnabled(bool enabled) { }
    public void SetSpeciesColor(uint species, uint color) { }
    public void MakeNewSpecies(uint slot, uint generation, string name, uint color) { }
    public void UpdateLegacyRestrictions(bool robotFixingAllowed, bool asexualReproductionAllowed) { }
    public void SetOrganismMemory(uint slot, uint generation, ushort address, int value) { }
    public void SetOrganismEnergy(uint slot, uint generation, int energy) { }
    public void UpdateRestrictionOptions(RestrictionOptions options) { }
    public void ConfigureLocalMode(LocalModeConfig config) { }
    public void StartLocalMode() { }
    public void RestartLocalMode() { }
    public void AdjudicateLocalRound(uint? winner, string reason) { }
    public void SetOrganismAim(uint slot, uint generation, int aim) { }
    public void UpdateEnvironment(EnvironmentUpdate update) { }
    public byte[] Save() => [];
    public byte[] SaveWithoutMutations() => [];
    public void Load(byte[] save) { }
    public EngineSnapshot Snapshot() => _snapshot with { Tick = _tick };
    public void Dispose() { }
}
