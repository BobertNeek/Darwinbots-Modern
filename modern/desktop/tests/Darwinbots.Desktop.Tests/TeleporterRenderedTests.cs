using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
using Avalonia.Threading;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using System.Reflection;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class TeleporterRenderedTests
{
    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public async Task TeleporterEditorRendersTypesAndPolicyAndAppliesLocalEdit(bool randomDestination)
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            using var engine = new NativeEngineClient(backend: "Cpu");
            engine.AddTeleporter(new TeleporterSnapshot(7, [100f, 200f], 40f, [800f, 900f])
            {
                Capacity = 4,
                BackflowLimit = 2,
                RandomDestination = randomDestination,
                RespectShapes = true
            });
            var original = Assert.Single(engine.Snapshot().Teleporters);
            Assert.Equal(randomDestination, original.RandomDestination);
            var window = new TeleporterEditorWindow(original)
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
            Assert.Equal(!randomDestination, window.FindControl<NumericUpDown>("DestinationX")!.IsEnabled);
            Assert.Equal(!randomDestination, window.FindControl<NumericUpDown>("DestinationY")!.IsEnabled);
            Assert.Equal(!randomDestination, window.FindControl<NumericUpDown>("BackflowLimit")!.IsEnabled);
            Assert.Equal(randomDestination, window.FindControl<TextBlock>("RandomDestinationNotice")!.IsVisible);

            if (!randomDestination) Capture(window, "teleporter-editor-exercised.png");

            window.FindControl<NumericUpDown>("Radius")!.Value = 55m;
            Click(window, window.FindControl<Button>("ApplyButton")!);

            Assert.NotNull(window.Result);
            Assert.Equal(55f, window.Result!.Radius);
            Assert.Equal("intrasim", window.Result.Mode);
            Assert.Equal(randomDestination, window.Result.RandomDestination);
            engine.UpdateTeleporter(window.Result);
            var updated = Assert.Single(engine.Snapshot().Teleporters);
            Assert.Equal(55f, updated.Radius);
            Assert.Equal(110f, updated.Width);
            Assert.Equal(110f, updated.Height);
            Assert.Equal(randomDestination, updated.RandomDestination);
            window.Close();
        }, CancellationToken.None);
    }

    [Fact]
    public async Task NativeRectangularTeleporterRetainsItsGeometryThroughTheRoutedEditorAndSave()
    {
        const float radius = 123.45678f;
        const float width = radius * 2f;
        const float height = width * 2f;
        var setup = new WorldSetupOptions { Backend = "Cpu", WorldWidth = 4_000, WorldHeight = 2_000, PopulationCapacity = 16 };
        var path = Path.Combine(Path.GetTempPath(), $"darwinbots-rectangular-teleporter-{Guid.NewGuid():N}.sim");
        try
        {
            using (var source = new NativeEngineClient(setup))
            {
                source.AddTeleporter(new TeleporterSnapshot(1, [1_000f, 600f], radius, [1_000f, 600f])
                {
                    Width = width, Height = height, RandomDestination = true,
                });
                await File.WriteAllBytesAsync(path, source.SaveLegacy());
            }

            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var player = new MainWindow([], setup with { LoadSavePath = path },
                    NativeEngineClientFactory.Instance, new AuditStorageService())
                {
                    WindowState = WindowState.Normal, Width = 1_100, Height = 820,
                };
                player.Show();
                try
                {
                    var model = Assert.IsType<MainWindowViewModel>(player.DataContext);
                    await WaitFor(() => model.Status == "LOADED");
                    var session = Assert.IsType<SimulationSession>(Session(player));
                    TeleporterSnapshot Current() => Assert.Single(session.LatestSnapshot.Teleporters);
                    AssertRectangle(Current(), width, height);

                    var viewport = player.FindControl<Control>("Viewport")!;
                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    ClickAt(player, viewport, new Point(viewport.Bounds.Width * 0.25, viewport.Bounds.Height * 0.3));
                    var edit = player.FindControl<Button>("EditTeleporterButton")!;
                    Assert.True(edit.IsEnabled);

                    async Task<TeleporterEditorWindow> OpenEditorAsync()
                    {
                        Click(player, edit);
                        await WaitFor(() => player.OwnedWindows.OfType<TeleporterEditorWindow>().Any());
                        Dispatcher.UIThread.RunJobs();
                        return Assert.Single(player.OwnedWindows.OfType<TeleporterEditorWindow>());
                    }

                    var editor = await OpenEditorAsync();
                    editor.FindControl<CheckBox>("TeleportCorpses")!.IsChecked = false;
                    Click(editor, editor.FindControl<Button>("ApplyButton")!);
                    await WaitFor(() => !Current().TeleportCorpses);
                    AssertRectangle(Current(), width, height);

                    using var reopened = new NativeEngineClient(setup);
                    reopened.Load(await session.SaveAsync());
                    var checkpoint = Assert.Single(reopened.Snapshot().Teleporters);
                    Assert.False(checkpoint.TeleportCorpses);
                    AssertRectangle(checkpoint, width, height);

                    editor = await OpenEditorAsync();
                    editor.FindControl<NumericUpDown>("Radius")!.Value = (decimal)(double)width;
                    Click(editor, editor.FindControl<Button>("ApplyButton")!);
                    await WaitFor(() => Current().Radius == width);
                    AssertRectangle(Current(), width * 2f, height * 2f);
                    reopened.Load(await session.SaveLegacyAsync());
                    AssertRectangle(Assert.Single(reopened.Snapshot().Teleporters), width * 2f, height * 2f);
                }
                finally
                {
                    foreach (var owned in player.OwnedWindows.ToArray()) owned.Close();
                    player.Close();
                }
            }, CancellationToken.None);
        }
        finally { File.Delete(path); }
    }

    [Fact]
    public async Task NativeTeleporterDoubleClickOpensPropertiesAndGivesRobotHitsPriority()
    {
        var setup = new WorldSetupOptions { Backend = "Cpu", WorldWidth = 4_000, WorldHeight = 2_000, PopulationCapacity = 16 };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var player = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService())
            {
                WindowState = WindowState.Normal, Width = 1_100, Height = 820,
            };
            player.Show();
            try
            {
                await WaitFor(() => Session(player)?.LatestSnapshot.NativeCycle is not null);
                var session = Session(player)!;
                await session.AddNativeTeleporterAsync(new NativeTeleporterOptions(
                    NativeTeleporterKind.Intrasim, 300, "", true, new(true, true, true), 10, 10));
                var viewport = player.FindControl<WorldViewport>("Viewport")!;
                viewport.FixedRatio = false;
                var portal = Assert.Single(session.LatestSnapshot.Teleporters);
                var world = session.LatestSnapshot.WorldSize;
                Point ToViewport(float[] position) =>
                    new(position[0] / world[0] * viewport.Bounds.Width, position[1] / world[1] * viewport.Bounds.Height);
                StabilizeForInput(player, viewport);

                DoubleClickAt(player, viewport, ToViewport(portal.Center));
                await WaitFor(() => player.OwnedWindows.OfType<TeleporterEditorWindow>().Any(window => window.IsVisible));
                var editor = Assert.Single(player.OwnedWindows.OfType<TeleporterEditorWindow>());
                Assert.Equal(0, editor.FindControl<ComboBox>("Mode")!.SelectedIndex);
                Assert.Equal(portal.Radius, (float)editor.FindControl<NumericUpDown>("Radius")!.Value!.Value);
                editor.FindControl<CheckBox>("TeleportCorpses")!.IsChecked = false;
                Click(editor, editor.FindControl<Button>("ApplyButton")!);
                await WaitFor(() => !editor.IsVisible && !Assert.Single(session.LatestSnapshot.Teleporters).TeleportCorpses);

                await session.ImportDnaAsync("start\nstop", portal.Center);
                var robot = Assert.Single(session.LatestSnapshot.Organisms);
                var left = portal.Center[0] > world[0] / 2;
                ClickAt(player, viewport, new Point(viewport.Bounds.Width * (left ? 0.15 : 0.85), viewport.Bounds.Height * 0.5));
                DoubleClickAt(player, viewport, ToViewport(robot.Position));
                await WaitFor(() => player.OwnedWindows.OfType<RobotDetailsWindow>().Any(window => window.IsVisible));
                var model = Assert.IsType<MainWindowViewModel>(player.DataContext);
                Assert.Equal(new OrganismKey(robot.Slot, robot.Generation), model.SelectedKey);
                Assert.DoesNotContain(player.OwnedWindows.OfType<TeleporterEditorWindow>(), window => window.IsVisible);
            }
            finally
            {
                foreach (var owned in player.OwnedWindows.ToArray()) owned.Close();
                player.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task NativeTeleporterPropertiesRunModelesslyAndRevalidateFocusedPortals()
    {
        var setup = new WorldSetupOptions { Backend = "Cpu", WorldWidth = 4_000, WorldHeight = 2_000, PopulationCapacity = 16 };
        var storage = new AuditStorageService();
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var player = new MainWindow([], setup, NativeEngineClientFactory.Instance, storage)
            {
                WindowState = WindowState.Normal, Width = 1_100, Height = 820,
            };
            player.Show();
            try
            {
                await WaitFor(() => Session(player)?.LatestSnapshot.NativeCycle is not null);
                var session = Session(player)!;
                var model = Assert.IsType<MainWindowViewModel>(player.DataContext);
                var viewport = player.FindControl<WorldViewport>("Viewport")!;
                viewport.FixedRatio = false;
                var options = new NativeTeleporterOptions(
                    NativeTeleporterKind.Intrasim, 300, "", true, new(true, true, true), 10, 3);
                await session.AddNativeTeleporterAsync(options);
                var firstId = Assert.Single(session.LatestSnapshot.Teleporters).Id;
                await session.AddNativeTeleporterAsync(options with { Capacity = 9 });
                var secondId = session.LatestSnapshot.Teleporters.Single(portal => portal.Id != firstId).Id;
                await session.DragNativeTeleporterAsync(firstId, [1_200, 920]);
                await session.DragNativeTeleporterAsync(secondId, [2_800, 920]);

                var editor = await OpenProperties(firstId);
                Assert.True(editor.Topmost);
                var before = session.LatestSnapshot.Tick;
                Click(player, player.FindControl<Button>("RunButton")!);
                await WaitFor(() => session.LatestSnapshot.Tick > before);
                Assert.True(editor.IsVisible);
                Click(player, player.FindControl<Button>("PauseButton")!);
                await WaitFor(() => model.Status == "PAUSED");

                Capacity(editor).Value = 4;
                ClickAt(player, viewport, PointFor(secondId));
                editor.Activate();
                await WaitFor(() => Capacity(editor).Value == 9);
                Assert.Same(editor, Assert.Single(player.OwnedWindows.OfType<TeleporterEditorWindow>()));
                Capacity(editor).Value = 11;
                Click(editor, editor.FindControl<Button>("ApplyButton")!);
                await WaitFor(() => Current(secondId).Capacity == 11);
                Assert.Equal(3, Current(firstId).Capacity);

                editor = await OpenProperties(secondId);
                Capacity(editor).Value = 12;
                Click(editor, editor.FindControl<Button>("CancelButton")!);
                await WaitFor(() => !editor.IsVisible);
                Assert.Equal(3, Current(firstId).Capacity);
                Assert.Equal(11, Current(secondId).Capacity);

                editor = await OpenProperties(firstId);
                await session.DragNativeTeleporterAsync(firstId, [1_380, 980]);
                var liveCenter = Current(firstId).Center;
                editor.FindControl<CheckBox>("TeleportCorpses")!.IsChecked = false;
                Click(editor, editor.FindControl<Button>("ApplyButton")!);
                await WaitFor(() => !Current(firstId).TeleportCorpses);
                Assert.Equal(liveCenter, Current(firstId).Center);
                Assert.True(Current(secondId).TeleportCorpses);

                editor = await OpenProperties(firstId);
                Capacity(editor).Value = 6;
                var menu = player.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                menu.FindCommandItem(LegacyCommandId.ObjectsDeleteTeleporter)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => !editor.IsVisible && session.LatestSnapshot.Teleporters.All(portal => portal.Id != firstId));
                Assert.Null(editor.Result);
                await session.AddNativeTeleporterAsync(options with { Capacity = 13 });
                await session.DragNativeTeleporterAsync(firstId, [1_230, 920]);

                storage.OpenedSimulation = await session.SaveAsync();
                editor = await OpenProperties(firstId);
                Capacity(editor).Value = 15;
                menu.FindCommandItem(LegacyCommandId.FileLoadSimulation)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => model.Status == "LOADED" && !editor.IsVisible);
                Assert.Null(editor.Result);
                Assert.Equal(13, Current(firstId).Capacity);
                Assert.Equal(11, Current(secondId).Capacity);

                await session.AddObstacleAsync(new ObstacleSnapshot(99, [1_920, 1_360], [2_080, 1_520]));
                editor = await OpenProperties(secondId);
                Capacity(editor).Value = 16;
                ClickAt(player, viewport, new Point(viewport.Bounds.Width * 0.5, viewport.Bounds.Height * 0.72));
                editor.Activate();
                await WaitFor(() => !editor.IsVisible);
                Assert.Null(editor.Result);
                Assert.Equal(11, Current(secondId).Capacity);

                TeleporterSnapshot Current(uint id) => session.LatestSnapshot.Teleporters.Single(portal => portal.Id == id);

                Point PointFor(uint id)
                {
                    var world = session.LatestSnapshot.WorldSize;
                    var center = Current(id).Center;
                    return new Point(center[0] / world[0] * viewport.Bounds.Width, center[1] / world[1] * viewport.Bounds.Height);
                }

                async Task<TeleporterEditorWindow> OpenProperties(uint id)
                {
                    StabilizeForInput(player, viewport);
                    // Move to a different hit before beginning each genuine double-click sequence.
                    ClickAt(player, viewport, new Point(viewport.Bounds.Width * 0.5, viewport.Bounds.Height * 0.72));
                    DoubleClickAt(player, viewport, PointFor(id));
                    await WaitFor(() => player.OwnedWindows.OfType<TeleporterEditorWindow>()
                        .Any(window => window.IsVisible && Capacity(window).Value == Current(id).Capacity));
                    return Assert.Single(player.OwnedWindows.OfType<TeleporterEditorWindow>());
                }
            }
            finally
            {
                foreach (var owned in player.OwnedWindows.ToArray()) owned.Close();
                player.Close();
            }
        }, CancellationToken.None);

        static NumericUpDown Capacity(Window editor) => editor.FindControl<NumericUpDown>("Capacity")!;
    }

    [Fact]
    public async Task NativeTeleporterDragsFromItsRectangleToThePointerBeforeRelease()
    {
        var setup = new WorldSetupOptions { Backend = "Cpu", WorldWidth = 4_000, WorldHeight = 2_000, PopulationCapacity = 16 };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var player = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService())
            {
                WindowState = WindowState.Normal, Width = 1_100, Height = 820,
            };
            player.Show();
            try
            {
                var menu = player.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var create = menu.FindCommandItem(LegacyCommandId.ObjectsNewTeleporter)!;
                await WaitFor(() => Session(player) is not null && create.IsEnabled);
                var session = Session(player)!;
                create.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => player.OwnedWindows.OfType<NewTeleporterWindow>().Any());
                var dialog = Assert.Single(player.OwnedWindows.OfType<NewTeleporterWindow>());
                dialog.FindControl<NumericUpDown>("PortalSize")!.Value = 300m;
                dialog.FindControl<CheckBox>("FixedPosition")!.IsChecked = true;
                dialog.FindControl<CheckBox>("TeleportCorpses")!.IsChecked = false;
                Click(dialog, dialog.FindControl<Button>("ApplyButton")!);
                await WaitFor(() => !dialog.IsVisible && session.LatestSnapshot.Teleporters.Count == 1);

                var viewport = player.FindControl<WorldViewport>("Viewport")!;
                var edit = player.FindControl<Button>("EditTeleporterButton")!;
                viewport.FixedRatio = false;
                StabilizeForInput(player, viewport);
                var before = session.LatestSnapshot;
                var original = Assert.Single(before.Teleporters);
                var originalTopLeft = Assert.IsType<float[]>(original.NativeTopLeft);
                Assert.Equal(150f, original.Width);
                Assert.Equal(300f, original.Height);
                Assert.True(original.FixedPosition);
                Assert.False(original.DriftHorizontal || original.DriftVertical);
                Assert.False(original.TeleportCorpses);
                Assert.False(edit.IsEnabled);

                var world = before.WorldSize;
                Point ViewportPoint(float x, float y) => new(
                    (double)x / world[0] * viewport.Bounds.Width, (double)y / world[1] * viewport.Bounds.Height);
                // This point is inside the native rectangle and outside the previous circular hit area.
                var start = ViewportPoint(original.Center[0], original.Center[1] + 112.5f);
                ClickAt(player, viewport, start);
                Assert.True(edit.IsEnabled);
                Assert.Equal(originalTopLeft, Assert.Single(session.LatestSnapshot.Teleporters).NativeTopLeft);

                var targetX = world[0] * (original.Center[0] < world[0] / 2f ? 0.65f : 0.35f);
                var targetY = world[1] * (original.Center[1] < world[1] / 2f ? 0.65f : 0.35f);
                var press = viewport.TranslatePoint(start, player)!.Value;
                var target = viewport.TranslatePoint(ViewportPoint(targetX, targetY), player)!.Value;
                player.MouseDown(press, MouseButton.Left, RawInputModifiers.Shift);
                player.MouseMove(target, RawInputModifiers.Shift | RawInputModifiers.LeftMouseButton);
                player.MouseUp(target, MouseButton.Left, RawInputModifiers.Shift);
                Assert.Equal(originalTopLeft, Assert.Single(session.LatestSnapshot.Teleporters).NativeTopLeft);

                player.MouseDown(press, MouseButton.Left, RawInputModifiers.None);
                player.MouseMove(target, RawInputModifiers.LeftMouseButton);
                await WaitFor(() => !original.Center.SequenceEqual(Assert.Single(session.LatestSnapshot.Teleporters).Center));

                var after = session.LatestSnapshot;
                var moved = Assert.Single(after.Teleporters);
                var movedTopLeft = Assert.IsType<float[]>(moved.NativeTopLeft);
                var toleranceX = world[0] / (float)viewport.Bounds.Width;
                var toleranceY = world[1] / (float)viewport.Bounds.Height;
                Assert.InRange(moved.Center[0], targetX - toleranceX, targetX + toleranceX);
                Assert.InRange(moved.Center[1], targetY - toleranceY, targetY + toleranceY);
                Assert.Equal(moved.Center[0] - 75f, movedTopLeft[0]);
                Assert.Equal(moved.Center[1] - 150f, movedTopLeft[1]);
                Assert.Equal((original.Id, original.Width, original.Height, original.Radius),
                    (moved.Id, moved.Width, moved.Height, moved.Radius));
                Assert.Equal((original.FixedPosition, original.DriftHorizontal, original.DriftVertical),
                    (moved.FixedPosition, moved.DriftHorizontal, moved.DriftVertical));
                Assert.Equal((original.TeleportVegetables, original.TeleportCorpses, original.TeleportHeterotrophs),
                    (moved.TeleportVegetables, moved.TeleportCorpses, moved.TeleportHeterotrophs));
                Assert.Equal(before.Tick, after.Tick);
                Assert.Equal(before.NativeCycle, after.NativeCycle);
                player.MouseUp(target, MouseButton.Left, RawInputModifiers.None);
            }
            finally
            {
                foreach (var owned in player.OwnedWindows.ToArray()) owned.Close();
                player.Close();
            }
        }, CancellationToken.None);
    }

    private static void AssertRectangle(TeleporterSnapshot teleporter, float width, float height)
    {
        Assert.Equal([1_000f, 600f], teleporter.Center);
        Assert.Equal(width / 2f, teleporter.Radius);
        Assert.Equal(width, teleporter.Width);
        Assert.Equal(height, teleporter.Height);
        Assert.True(teleporter.RandomDestination);
    }

    [Fact]
    public async Task ObjectsMenuCreatesOriginalTeleportersAndKeepsToolbarPlacement()
    {
        var setup = new WorldSetupOptions { Backend = "Cpu", WorldWidth = 4_000, WorldHeight = 2_000, PopulationCapacity = 16 };
        var transferDirectory = Directory.CreateTempSubdirectory("darwinbots teleporter ");
        try
        {
            var inboundDirectory = transferDirectory.CreateSubdirectory("inbound").FullName;
            var outboundDirectory = transferDirectory.CreateSubdirectory("outbound").FullName;
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var player = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService())
                {
                    WindowState = WindowState.Normal, Width = 1_100, Height = 820,
                };
                player.Show();
                try
                {
                    var menu = player.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                    var command = menu.FindCommandItem(LegacyCommandId.ObjectsNewTeleporter)!;
                    var toolbar = player.FindControl<Button>("AddTeleporterButton")!;
                    await WaitFor(() => Session(player) is not null && command.IsEnabled);
                    var session = Session(player)!;

                    async Task<NewTeleporterWindow> OpenDialog()
                    {
                        command.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                        await WaitFor(() => player.OwnedWindows.OfType<NewTeleporterWindow>().Any());
                        return Assert.Single(player.OwnedWindows.OfType<NewTeleporterWindow>());
                    }

                    var dialog = await OpenDialog();
                    Assert.True(toolbar.IsEffectivelyEnabled);
                    var mode = dialog.FindControl<ComboBox>("Mode")!;
                    Assert.Equal("Intrasim", ((ComboBoxItem)mode.SelectedItem!).Content);
                    Assert.Equal(3, mode.Items.Count);
                    Assert.Equal(300m, dialog.FindControl<NumericUpDown>("PortalSize")!.Value);
                    Assert.False(dialog.FindControl<CheckBox>("FixedPosition")!.IsChecked);
                    Assert.False(dialog.FindControl<TextBox>("FilePath")!.IsEffectivelyEnabled);
                    Assert.True(dialog.FindControl<CheckBox>("TeleportHeterotrophs")!.IsEffectivelyEnabled);
                    Assert.False(dialog.FindControl<NumericUpDown>("PollInterval")!.IsEffectivelyEnabled);
                    mode.SelectedIndex = 1;
                    Assert.Same(dialog, await OpenDialog());
                    Assert.Equal(1, mode.SelectedIndex);
                    Click(dialog, dialog.FindControl<Button>("CancelButton")!);
                    await WaitFor(() => !dialog.IsVisible);
                    Assert.Empty(session.LatestSnapshot.Teleporters);

                    var viewport = player.FindControl<Control>("Viewport")!;
                    Click(player, toolbar);
                    ClickAt(player, viewport, new Point(viewport.Bounds.Width * 0.25, viewport.Bounds.Height * 0.30));
                    ClickAt(player, viewport, new Point(viewport.Bounds.Width * 0.68, viewport.Bounds.Height * 0.62));
                    await WaitFor(() => session.LatestSnapshot.Teleporters.Count == 1);
                    var modern = Assert.Single(session.LatestSnapshot.Teleporters);
                    Assert.True(modern.Local && !modern.RandomDestination);
                    Assert.Equal(180f, modern.Radius);
                    Assert.NotEqual(modern.Center, modern.Destination);
                    await session.RemoveTeleporterAsync(modern.Id);
                    Assert.Empty(session.LatestSnapshot.Teleporters);

                    dialog = await OpenDialog();
                    Assert.Equal(0, dialog.FindControl<ComboBox>("Mode")!.SelectedIndex);
                    Click(dialog, dialog.FindControl<Button>("ApplyButton")!);
                    await WaitFor(() => !dialog.IsVisible && session.LatestSnapshot.Teleporters.Count == 1);
                    var local = Assert.Single(session.LatestSnapshot.Teleporters);
                    Assert.Equal((150f, 300f), (local.Width!.Value, local.Height!.Value));
                    Assert.InRange(local.Center[0], 75f, 3_925f);
                    Assert.InRange(local.Center[1], 150f, 1_850f);
                    Assert.True(local.Local && local.RandomDestination);
                    Assert.False(local.Inbound || local.Outbound || local.Internet);
                    Assert.True(local.DriftHorizontal && local.DriftVertical);

                    dialog = await OpenDialog();
                    dialog.FindControl<ComboBox>("Mode")!.SelectedIndex = 1;
                    var inboundPath = dialog.FindControl<TextBox>("FilePath")!;
                    var poll = dialog.FindControl<NumericUpDown>("PollInterval")!;
                    var capacity = dialog.FindControl<NumericUpDown>("Capacity")!;
                    Assert.True(inboundPath.IsEffectivelyEnabled);
                    Assert.True(poll.IsEffectivelyEnabled);
                    Assert.False(dialog.FindControl<CheckBox>("TeleportVegetables")!.IsEffectivelyEnabled);
                    Assert.Equal(10m, poll.Value);
                    Assert.Equal(10m, capacity.Value);
                    inboundPath.Text = inboundDirectory;
                    poll.Value = 3m;
                    capacity.Value = 12m;
                    dialog.FindControl<CheckBox>("FixedPosition")!.IsChecked = true;
                    Click(dialog, dialog.FindControl<Button>("ApplyButton")!);
                    await WaitFor(() => !dialog.IsVisible && session.LatestSnapshot.Teleporters.Count == 2);
                    var inbound = Assert.Single(session.LatestSnapshot.Teleporters, value => value.Inbound);
                    Assert.Equal(inboundDirectory, inbound.Path);
                    Assert.Equal((3u, 12, 12u), (inbound.PollInterval, inbound.Capacity, inbound.PollCountdown));
                    Assert.True(inbound.FixedPosition);
                    Assert.False(inbound.DriftHorizontal || inbound.DriftVertical);

                    dialog = await OpenDialog();
                    dialog.FindControl<ComboBox>("Mode")!.SelectedIndex = 2;
                    Assert.False(dialog.FindControl<NumericUpDown>("PollInterval")!.IsEffectivelyEnabled);
                    var outboundPath = dialog.FindControl<TextBox>("FilePath")!;
                    var excludesCorpses = dialog.FindControl<CheckBox>("TeleportCorpses")!;
                    Assert.True(outboundPath.IsEffectivelyEnabled);
                    Assert.True(excludesCorpses.IsEffectivelyEnabled);
                    outboundPath.Text = outboundDirectory;
                    excludesCorpses.IsChecked = false;
                    dialog.FindControl<NumericUpDown>("PortalSize")!.Value = 500m;
                    Click(dialog, dialog.FindControl<Button>("ApplyButton")!);
                    await WaitFor(() => !dialog.IsVisible && session.LatestSnapshot.Teleporters.Count == 3);
                    var outbound = Assert.Single(session.LatestSnapshot.Teleporters, value => value.Outbound);
                    Assert.Equal(outboundDirectory, outbound.Path);
                    Assert.Equal((250f, 500f), (outbound.Width!.Value, outbound.Height!.Value));
                    Assert.False(outbound.TeleportCorpses);

                    using var reopened = new NativeEngineClient(setup);
                    reopened.Load(await session.SaveLegacyAsync());
                    var afterSave = reopened.Snapshot().Teleporters;
                    Assert.Equal(3, afterSave.Count);
                    var reopenedLocal = Assert.Single(afterSave, value => value.Local);
                    Assert.False(reopenedLocal.Inbound || reopenedLocal.FixedPosition);
                    Assert.Equal(local.Center, reopenedLocal.Center);
                    Assert.Equal(inbound.Path, Assert.Single(afterSave, value => value.Inbound).Path);
                    var reopenedOutbound = Assert.Single(afterSave, value => value.Outbound);
                    Assert.Equal(outboundDirectory, reopenedOutbound.Path);
                    Assert.False(reopenedOutbound.TeleportCorpses);
                }
                finally
                {
                    foreach (var owned in player.OwnedWindows.ToArray()) owned.Close();
                    player.Close();
                }
            }, CancellationToken.None);
        }
        finally { transferDirectory.Delete(recursive: true); }
    }

    [Fact]
    public async Task QueuedSimulationLoadClosesNewTeleporterDraftBeforeApplyCanReachTheReplacement()
    {
        var setup = new WorldSetupOptions { Backend = "Cpu", WorldWidth = 4_000, WorldHeight = 2_000, PopulationCapacity = 16 };
        var storage = new AuditStorageService();
        using var replacement = new NativeEngineClient(setup);
        replacement.AddTeleporter(new TeleporterSnapshot(17, [500f, 500f], 50f, [800f, 800f]));
        var save = replacement.Save();

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var player = new MainWindow([], setup, NativeEngineClientFactory.Instance, storage);
            player.Show();
            try
            {
                var menu = player.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var create = menu.FindCommandItem(LegacyCommandId.ObjectsNewTeleporter)!;
                var load = menu.FindCommandItem(LegacyCommandId.FileLoadSimulation)!;
                await WaitFor(() => Session(player) is not null && create.IsEnabled && load.IsEnabled);
                var session = Session(player)!;
                create.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                var draft = Assert.Single(player.OwnedWindows.OfType<NewTeleporterWindow>());
                draft.FindControl<NumericUpDown>("PortalSize")!.Value = 500m;

                load.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                Assert.True(draft.IsVisible);
                Assert.Equal(500m, draft.FindControl<NumericUpDown>("PortalSize")!.Value);

                var gate = Assert.IsType<SemaphoreSlim>(typeof(MainWindow)
                    .GetField("_advanceGate", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(player));
                await gate.WaitAsync();
                try
                {
                    storage.OpenedSimulation = save;
                    load.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    draft.FindControl<Button>("ApplyButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                }
                finally { gate.Release(); }

                // Wait behind both tasks that were queued against the same session.
                await gate.WaitAsync();
                gate.Release();
                var model = Assert.IsType<MainWindowViewModel>(player.DataContext);
                await WaitFor(() => model.Status == "LOADED");
                Assert.False(draft.IsVisible);
                Assert.Same(session, Session(player));
                Assert.Equal(17u, Assert.Single(session.LatestSnapshot.Teleporters).Id);
            }
            finally
            {
                foreach (var owned in player.OwnedWindows.ToArray()) owned.Close();
                player.Close();
            }
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

    private static void DoubleClickAt(Window window, Control control, Point point)
    {
        StabilizeForInput(window, control);
        var translated = control.TranslatePoint(point, window);
        Assert.True(translated.HasValue);
        for (var click = 0; click < 2; click++)
        {
            window.MouseDown(translated.Value, MouseButton.Left, RawInputModifiers.None);
            window.MouseUp(translated.Value, MouseButton.Left, RawInputModifiers.None);
        }
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        Dispatcher.UIThread.RunJobs();
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
