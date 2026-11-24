using System.Reflection;
using System.Text.Json;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Avalonia.Input;
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

public sealed class RobotConsoleRenderedTests
{
    [Fact]
    public async Task RobotMenuOpensConsoleAndRunsAuthoritativeCommands()
    {
        var factory = new AuditEngineClientFactory();
        var setup = new WorldSetupOptions
        {
            Species = [new SpeciesImport("Luna", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, factory, new AuditStorageService())
            {
                WindowState = WindowState.Normal,
                Width = 1_280,
                Height = 900,
            };

            try
            {
                window.Show();
                window.UpdateLayout();
                var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitForAsync(() => factory.Clients.TryPeek(out var client)
                    && client.Calls.Contains("ImportSpecies:Luna"));
                await WaitForAsync(() => viewModel.SelectedKey == new OrganismKey(3, 2));

                var engine = Assert.Single(factory.Clients);
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var robotMenu = menu.TopLevelItems.Single(item => Equals(item.Header, "Robot"));
                var openConsole = menu.FindCommandItem(LegacyCommandId.RobotOpenConsole)!;
                Assert.True(robotMenu.IsVisible);
                Assert.True(openConsole.IsEnabled);
                Assert.Equal("Open Console", openConsole.Header);
                Assert.Equal(new OrganismKey(3, 2), viewModel.SelectedKey);

                openConsole.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitForAsync(() => window.OwnedWindows.OfType<RobotConsoleWindow>().Any());
                var console = window.OwnedWindows.OfType<RobotConsoleWindow>().Single();
                console.Width = console.MinWidth;
                console.Height = console.MinHeight;
                console.UpdateLayout();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                Assert.Equal("Darwinbots — Robot Console 3:2", console.Title);
                var transcript = Assert.Single(console.GetVisualDescendants().OfType<TextBox>().Where(box => box.IsReadOnly));
                var command = Assert.Single(console.GetVisualDescendants().OfType<TextBox>().Where(box => !box.IsReadOnly));
                var buttons = console.GetVisualDescendants().OfType<Button>()
                    .ToDictionary(button => button.Content?.ToString() ?? string.Empty);
                Assert.Contains("Darwinbots robot console", transcript.Text, StringComparison.Ordinal);
                Assert.Contains("Type help for commands.", transcript.Text, StringComparison.Ordinal);
                Assert.Equal("command, for example: cycle 1 or set 50 100", command.PlaceholderText);
                var consoleArea = new Rect(console.Bounds.Size);
                foreach (var label in new[] { "RUN", "STEP", "EYES", "DEBUG", "DNA", "PLAY", "PAUSE" })
                {
                    var button = buttons[label];
                    var corner = button.TranslatePoint(new Point(button.Bounds.Width, button.Bounds.Height), console);
                    Assert.True(corner is { } point && consoleArea.Contains(point),
                        $"{label} is clipped at the minimum console size.");
                }

                Assert.True(console.Bounds.Width >= 560);
                Assert.True(console.Bounds.Height >= 420);
                Assert.True(transcript.Bounds.Width > 400);
                Assert.True(command.Bounds.Width > 100);
                using (var frame = Assert.IsAssignableFrom<Bitmap>(console.CaptureRenderedFrame()))
                {
                    Assert.True(frame.PixelSize.Width >= 560);
                    Assert.True(frame.PixelSize.Height >= 420);
                    using var png = new MemoryStream();
                    frame.Save(png, new PngBitmapEncoderOptions());
                    Assert.True(png.Length > 1_000, $"Rendered console PNG was only {png.Length} bytes.");
                }

                await RunAndWaitAsync(command, buttons["RUN"], transcript, "printmem 310", "mem[310] = 0");
                Assert.Contains("> printmem 310", transcript.Text, StringComparison.Ordinal);

                await RunAndWaitAsync(command, buttons["RUN"], transcript, "showdna", "start\nstop");
                Assert.Empty(window.OwnedWindows.OfType<RobotDetailsWindow>());

                await RunAndWaitAsync(command, buttons["RUN"], transcript, "set 310 123", "mem[310] = 123");
                Assert.Contains("Memory:3:2:310:123", engine.Calls);

                await RunAndWaitAsync(command, buttons["RUN"], transcript, "energy 4321", "energy = 4321");
                Assert.Contains("Energy:3:2:4321", engine.Calls);

                var callsBeforeInvalid = engine.Calls.ToArray();
                await RunAndWaitAsync(command, buttons["RUN"], transcript, "set nope 42", "ERROR: Unknown memory address 'nope'.");
                Assert.Contains("> set nope 42", transcript.Text, StringComparison.Ordinal);
                Assert.Equal(callsBeforeInvalid, engine.Calls.ToArray());

                var controller = (RobotConsoleController)typeof(RobotConsoleWindow)
                    .GetField("_controller", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .GetValue(console)!;
                using var cancelled = new CancellationTokenSource();
                cancelled.Cancel();
                var callsBeforeCancel = engine.Calls.ToArray();
                await Assert.ThrowsAsync<OperationCanceledException>(() =>
                    controller.ExecuteAsync("set 311 999", cancelled.Token));
                Assert.Equal(callsBeforeCancel, engine.Calls.ToArray());

                var callsBeforeClose = engine.Calls.ToArray();
                console.Close();
                Assert.False(console.IsVisible);
                Assert.Equal(callsBeforeClose, engine.Calls.ToArray());
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                if (window.IsVisible) window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task NativeConsoleCycleAndPlayReportEachCommittedGenePassInOrder()
    {
        const string dna = "cond\n*.robage 1 =\nstart\n1 501 store\nstop\ncond\n*.robage 0 =\nstart\n1 502 store\nstop";
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            TicksPerUpdate = 1,
            Species = [new SpeciesImport("Console Oracle", dna, false, 0xff239ac0, 1, 1_000, 0, false, 1)],
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService())
            {
                WindowState = WindowState.Normal, Width = 1_280, Height = 900,
            };
            try
            {
                window.Show();
                var model = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitForAsync(() => model.SelectedKey.HasValue && model.NativeCycle.HasValue);
                var session = (SimulationSession)typeof(MainWindow)
                    .GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window)!;
                var focus = model.SelectedKey!.Value;
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                menu.FindCommandItem(LegacyCommandId.RobotOpenConsole)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitForAsync(() => window.OwnedWindows.OfType<RobotConsoleWindow>().Any());
                var console = Assert.Single(window.OwnedWindows.OfType<RobotConsoleWindow>());
                console.UpdateLayout();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                var transcript = Assert.Single(console.GetVisualDescendants().OfType<TextBox>(), box => box.IsReadOnly);
                var command = Assert.Single(console.GetVisualDescendants().OfType<TextBox>(), box => !box.IsReadOnly);
                var buttons = console.GetVisualDescendants().OfType<Button>().ToDictionary(button => button.Content?.ToString() ?? "");
                const string header = "***ROBOT GENES EXECUTION***";
                var start = session.LatestSnapshot.Tick;

                await RunAndWaitAsync(command, buttons["RUN"], transcript, "cycle 2", $"cycle {start + 2} (+2)");
                var output = (transcript.Text ?? "").Replace("\r", "");
                Assert.Equal(2, PassCount());
                Assert.True(output.Contains($"{header}\n1 not executed\n2 executed\n\n{header}\n1 executed\n2 not executed\n",
                    StringComparison.Ordinal), output);
                Assert.Equal(start + 2, session.LatestSnapshot.Tick);

                buttons["PLAY"].RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitForAsync(() => PassCount() > 2);
                buttons["PAUSE"].RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitForAsync(() => model.Status == "PAUSED");
                // Let an already committed timer cycle settle before checking the ordinary toolbar step.
                await Task.Delay(150);
                var passesAfterPause = PassCount();
                var pausedTick = session.LatestSnapshot.Tick;
                window.FindControl<Button>("StepButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitForAsync(() => session.LatestSnapshot.Tick == pausedTick + 1);
                Assert.Equal(passesAfterPause, PassCount());

                await session.RemoveAsync(focus.Slot, focus.Generation);
                var beforeLoss = session.LatestSnapshot.Tick;
                await RunAndWaitAsync(command, buttons["RUN"], transcript, "cycle 2", $"cycle {beforeLoss + 2} (+2)");
                Assert.Single((transcript.Text ?? "").Split("Robot has died.", StringSplitOptions.None).Skip(1));
                Assert.Equal(passesAfterPause, PassCount());

                int PassCount() => (transcript.Text ?? "").Split(header, StringSplitOptions.None).Length - 1;
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                if (window.IsVisible) window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task NativeConsolePrintsTheRemovedRobotsFinalPassBeforeItsDeathNotice()
    {
        const string dna = "cond\n*.robage 1 =\nstart\n1 501 store\nstop\ncond\n*.robage 0 =\nstart\n1 502 store\nstop";
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            CorpseDecay = new(Enabled: false),
            GlobalSettings = new() { Costs = new() { DnaStore = 2 } },
            Species = [new SpeciesImport("Terminal Console Robot", dna, false, 0xff239ac0, 1, 1_000, 0, false, 1)],
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService());
            try
            {
                window.Show();
                var model = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitForAsync(() => model.SelectedKey.HasValue && model.NativeCycle.HasValue);
                var session = (SimulationSession)typeof(MainWindow)
                    .GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window)!;
                var focus = model.SelectedKey!.Value;
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                menu.FindCommandItem(LegacyCommandId.RobotOpenConsole)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitForAsync(() => window.OwnedWindows.OfType<RobotConsoleWindow>().Any());
                var console = Assert.Single(window.OwnedWindows.OfType<RobotConsoleWindow>());
                console.UpdateLayout();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                var transcript = Assert.Single(console.GetVisualDescendants().OfType<TextBox>(), box => box.IsReadOnly);
                var command = Assert.Single(console.GetVisualDescendants().OfType<TextBox>(), box => !box.IsReadOnly);
                var run = Assert.Single(console.GetVisualDescendants().OfType<Button>(), button => Equals(button.Content, "RUN"));
                var start = session.LatestSnapshot.Tick;
                const string header = "***ROBOT GENES EXECUTION***";
                await RunAndWaitAsync(command, run, transcript, "cycle 1", $"cycle {start + 1} (+1)");
                Assert.Contains($"{header}\n1 not executed\n2 executed", (transcript.Text ?? "").Replace("\r", ""),
                    StringComparison.Ordinal);

                await session.SetOrganismEnergyAsync(focus.Slot, focus.Generation, 1);
                await RunAndWaitAsync(command, run, transcript, "cycle 2", $"cycle {start + 3} (+2)");
                Assert.DoesNotContain(session.LatestSnapshot.Organisms,
                    actor => actor.Slot == focus.Slot && actor.Generation == focus.Generation);
                var output = (transcript.Text ?? "").Replace("\r", "");
                Assert.Contains($"> cycle 2\n\n{header}\n1 executed\n2 not executed\nRobot has died.\ncycle {start + 3} (+2)",
                    output, StringComparison.Ordinal);
                Assert.Equal(2, output.Split(header, StringSplitOptions.None).Length - 1);
                Assert.Single(output.Split("Robot has died.", StringSplitOptions.None).Skip(1));
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                if (window.IsVisible) window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task ConsoleCycleUsesTheOwnerForSourceRecoveryAndSafeModeCheckpoint()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            const string dna = "start\nstop\n";
            var missing = new NativeRestartSource(42, $"console-alga-{Guid.NewGuid():N}.txt", "Robots");
            var factory = new AuditEngineClientFactory(encodeTickInSave: true);
            var storage = new AuditStorageService
            {
                ResolveNativeRepopulationSource = (_, source) =>
                    Task.FromResult<DesktopTextFile?>(new DesktopTextFile(source.Filename, dna)),
            };
            var setup = new WorldSetupOptions
            {
                GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { SafeMode = true } },
                Species = [new SpeciesImport("Luna", dna, false, 0xff239ac0, 1, 1_000, 0, false, 1)],
            };
            var window = new MainWindow([], setup, factory, storage);
            try
            {
                window.Show();
                var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitForAsync(() => viewModel.SelectedKey == new OrganismKey(3, 2)
                    && viewModel.Status.Contains("WORLD READY", StringComparison.Ordinal));
                var engine = Assert.Single(factory.Clients);
                var session = (SimulationSession)typeof(MainWindow)
                    .GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window)!;
                engine.NativeCycle = 1999;
                engine.UnresolvedNativeRepopulationSources.Add(missing);
                await session.RefreshSnapshotAsync();
                engine.Calls.Clear();

                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                menu.FindCommandItem(LegacyCommandId.RobotOpenConsole)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitForAsync(() => window.OwnedWindows.OfType<RobotConsoleWindow>().Any());
                var console = Assert.Single(window.OwnedWindows.OfType<RobotConsoleWindow>());
                console.UpdateLayout();
                var transcript = Assert.Single(console.GetVisualDescendants().OfType<TextBox>().Where(box => box.IsReadOnly));
                var command = Assert.Single(console.GetVisualDescendants().OfType<TextBox>().Where(box => !box.IsReadOnly));
                var run = Assert.Single(console.GetVisualDescendants().OfType<Button>().Where(button => Equals(button.Content, "RUN")));

                await RunAndWaitAsync(command, run, transcript, "cycle 2", "cycle 2 (+2)");

                Assert.EndsWith("> cycle 2\ncycle 2 (+2)\n", transcript.Text, StringComparison.Ordinal);
                var request = Assert.Single(storage.NativeRepopulationSourceRequests);
                Assert.Same(window, request.Owner);
                Assert.Equal(missing, request.Source);
                var binding = Assert.Single(engine.NativeRestartBindings);
                Assert.True(binding.PreserveExisting);
                Assert.Equal(new NativeRestartBinding(missing.LineageId, dna), Assert.Single(binding.Sources));
                Assert.Equal(new[] { "Tick:1", "Save", "Tick:1" },
                    engine.Calls.Where(call => call.StartsWith("Tick:", StringComparison.Ordinal) || call == "Save"));
                Assert.Equal(1ul, BitConverter.ToUInt64(Assert.Single(storage.AutosaveWrites)));
                Assert.Equal(2ul, session.LatestSnapshot.Tick);
                Assert.Equal(2001, session.LatestSnapshot.NativeCycle);

                var load = typeof(MainWindow).GetMethod("LoadSimulationAsync", BindingFlags.Instance | BindingFlags.NonPublic)!;
                await (Task)load.Invoke(window, [new byte[] { 1 }])!;
                engine.Calls.Clear();
                await RunAndWaitAsync(command, run, transcript, "step", "This console belongs to a previous simulation.");
                Assert.DoesNotContain(engine.Calls, call => call.StartsWith("Tick:", StringComparison.Ordinal));
                Assert.Equal(2ul, session.LatestSnapshot.Tick);
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task NativeConsoleMemoryCommandsUseTheRobotsPrivateDefinitionsAfterSelectionMoves()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            PopulationCapacity = 64,
            Species =
            [
                new("Tracker", "def Counter 901\ndef Counter 902\ndef up 903\ndef Counter% 904\nstart stop",
                    false, 0xff239ac0, 1, 1_000, 0, false, 1),
                new("Other", "def Counter 911\nstart stop", false, 0xffbb832c, 1, 1_000, 0, false, 1),
            ],
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService());
            try
            {
                window.Show();
                var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitForAsync(() => viewModel.Population == 2
                    && viewModel.Status.Contains("WORLD READY", StringComparison.Ordinal));
                var session = Assert.IsType<SimulationSession>(typeof(MainWindow)
                    .GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window));
                var snapshot = session.LatestSnapshot;
                var tracker = Assert.Single(snapshot.Organisms,
                    robot => snapshot.Species[(int)robot.Species].Name == "Tracker");
                var other = Assert.Single(snapshot.Organisms,
                    robot => snapshot.Species[(int)robot.Species].Name == "Other");
                viewModel.Select(new OrganismKey(tracker.Slot, tracker.Generation));

                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                menu.FindCommandItem(LegacyCommandId.RobotOpenConsole)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitForAsync(() => window.OwnedWindows.OfType<RobotConsoleWindow>().Any());
                var console = Assert.Single(window.OwnedWindows.OfType<RobotConsoleWindow>());
                console.UpdateLayout();
                var transcript = Assert.Single(console.GetVisualDescendants().OfType<TextBox>(), box => box.IsReadOnly);
                var command = Assert.Single(console.GetVisualDescendants().OfType<TextBox>(), box => !box.IsReadOnly);
                var run = Assert.Single(console.GetVisualDescendants().OfType<Button>(), button => Equals(button.Content, "RUN"));

                viewModel.Select(new OrganismKey(other.Slot, other.Generation));
                await RunAndWaitAsync(command, run, transcript, "set .Counter 124", "mem[902] = 124");
                await RunAndWaitAsync(command, run, transcript, "? .Counter", "> ? .Counter\nmem[902] = 124");
                await RunAndWaitAsync(command, run, transcript, "set .up 21", "mem[903] = 21");
                await RunAndWaitAsync(command, run, transcript, "printmem .UP", "> printmem .UP\nmem[1] = 0");
                await RunAndWaitAsync(command, run, transcript, "set .Counter% -8", "mem[904] = -8");
                await RunAndWaitAsync(command, run, transcript, "printmem .COUNTER", "unknown memory address .COUNTER");

                var live = session.LatestSnapshot;
                var tracked = Assert.Single(live.Organisms, robot => robot.Slot == tracker.Slot);
                Assert.Equal(124, Assert.Single(tracked.Memory, cell => cell.Address == 902).Value);
                Assert.DoesNotContain(Assert.Single(live.Organisms, robot => robot.Slot == other.Slot).Memory,
                    cell => cell.Address == 911 && cell.Value == 124);
                Assert.Equal(snapshot.Tick, live.Tick);
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                if (window.IsVisible) window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task NativeConsoleDnaDistinguishesTheTypedCommandFromTheDnaButton()
    {
        const string firstDna = "def FirstAlias 501\ndef LastFirstAlias 501\nstart\n1101 501 store\nstop";
        const string secondDna = "def SecondAlias 502\ndef LastSecondAlias 502\nstart\n2202 502 store\nstop";
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            Species =
            [
                new SpeciesImport("First Console Robot", firstDna, false, 0xff239ac0, 1, 1_000, 0, false, 1),
                new SpeciesImport("Second Arena Robot", secondDna, false, 0xff349857, 1, 1_000, 0, false, 1),
            ],
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService())
            {
                WindowState = WindowState.Normal, Width = 1_280, Height = 900,
            };
            try
            {
                window.Show();
                var model = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitForAsync(() => model.Population == 2 && model.SelectedKey.HasValue && model.NativeCycle.HasValue);
                var session = (SimulationSession)typeof(MainWindow)
                    .GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window)!;
                var consoleFocus = model.SelectedKey!.Value;
                var other = Assert.Single(session.LatestSnapshot.Organisms, actor => actor.Slot != consoleFocus.Slot);
                var selectedFocus = new OrganismKey(other.Slot, other.Generation);
                var selectedDna = await session.ReadRobotDnaAsync(selectedFocus);
                var consoleDna = await session.ReadRobotDnaAsync(consoleFocus);
                var selectedMarkedDna = await session.ReadRobotDnaAsync(selectedFocus, 2);
                Assert.NotEqual(selectedDna, consoleDna);
                Assert.Contains(" '[<POSITION MARKER]", selectedMarkedDna);
                Assert.Contains("Alias store", consoleDna, StringComparison.OrdinalIgnoreCase);
                Assert.DoesNotContain("'#generation", consoleDna, StringComparison.OrdinalIgnoreCase);
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                menu.FindCommandItem(LegacyCommandId.RobotShowInfo)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitForAsync(() => window.OwnedWindows.OfType<RobotDetailsWindow>().Any());
                var info = Assert.Single(window.OwnedWindows.OfType<RobotDetailsWindow>());
                menu.FindCommandItem(LegacyCommandId.RobotOpenConsole)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitForAsync(() => window.OwnedWindows.OfType<RobotConsoleWindow>().Any());
                var console = Assert.Single(window.OwnedWindows.OfType<RobotConsoleWindow>());
                console.UpdateLayout();
                info.UpdateLayout();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                var transcript = Assert.Single(console.GetVisualDescendants().OfType<TextBox>(), box => box.IsReadOnly);
                var command = Assert.Single(console.GetVisualDescendants().OfType<TextBox>(), box => !box.IsReadOnly);
                var run = Assert.Single(console.GetVisualDescendants().OfType<Button>(), button => Equals(button.Content, "RUN"));
                var dnaButton = Assert.Single(console.GetVisualDescendants().OfType<Button>(), button => Equals(button.Content, "DNA"));
                var tabs = Assert.Single(info.GetVisualDescendants().OfType<TabControl>());
                var dnaTab = Assert.Single(tabs.Items.OfType<TabItem>(), item => Equals(item.Header, "DNA"));
                var dnaPanel = Assert.IsType<DockPanel>(Assert.IsType<Border>(dnaTab.Content).Child);
                var dna = Assert.Single(dnaPanel.Children.OfType<TextBox>());
                var marker = Assert.Single(dnaPanel.Children.OfType<Button>(), button => button.Name == "MarkRobotDnaLocation");
                tabs.SelectedItem = dnaTab;
                await WaitForAsync(() => dna.Text == consoleDna);

                var size = session.LatestSnapshot.WorldSize;
                var scaleX = viewport.Bounds.Width / size[0];
                var scaleY = viewport.Bounds.Height / size[1];
                if (viewport.FixedRatio) scaleX = scaleY = Math.Min(scaleX, scaleY);
                var robotPoint = new Point(other.Position[0] * scaleX * viewport.Zoom + viewport.Pan.X,
                    other.Position[1] * scaleY * viewport.Zoom + viewport.Pan.Y);
                var pointer = viewport.TranslatePoint(robotPoint, window);
                Assert.True(pointer.HasValue);
                window.MouseDown(pointer.Value, MouseButton.Left, RawInputModifiers.None);
                window.MouseUp(pointer.Value, MouseButton.Left, RawInputModifiers.None);
                Assert.Equal(selectedFocus, model.SelectedKey);
                await RunAndWaitAsync(command, run, transcript, "showdna", "ROBOT DNA OPEN");
                await WaitForAsync(() => dna.Text == selectedDna);
                Assert.Same(info, Assert.Single(window.OwnedWindows.OfType<RobotDetailsWindow>()));
                Assert.True(info.Topmost);
                Assert.Same(dnaTab, tabs.SelectedItem);
                Assert.Equal(selectedFocus, model.SelectedKey);
                Assert.True(ShowsFacts(consoleFocus));

                await session.StepAsync();
                await WaitForAsync(() => ShowsFacts(selectedFocus));
                Assert.Equal(selectedDna, dna.Text);

                await EnterMarkerAsync("-2.5tail", "APPLY", selectedMarkedDna);
                await session.StepAsync();
                Assert.Equal(selectedMarkedDna, dna.Text);
                await EnterMarkerAsync("1", "CANCEL", selectedDna);
                await EnterMarkerAsync("2", "APPLY", selectedMarkedDna);
                await EnterMarkerAsync("99999", "APPLY", selectedDna);
                await EnterMarkerAsync("2", "APPLY", selectedMarkedDna);
                await EnterMarkerAsync(string.Empty, "APPLY", selectedDna);
                await EnterMarkerAsync("2", "APPLY", selectedMarkedDna);
                for (var click = 0; click < 2; click++)
                {
                    window.MouseDown(pointer.Value, MouseButton.Left, RawInputModifiers.None);
                    window.MouseUp(pointer.Value, MouseButton.Left, RawInputModifiers.None);
                }
                await WaitForAsync(() => dna.Text == selectedDna);
                await EnterMarkerAsync("2", "APPLY", selectedMarkedDna);

                var beforeDnaButton = transcript.Text?.Length ?? 0;
                dnaButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitForAsync(() => dna.Text == consoleDna && transcript.Text is { } text
                    && text.Length > beforeDnaButton
                    && text.AsSpan(beforeDnaButton).Contains("ROBOT DNA OPEN", StringComparison.Ordinal));
                Assert.Equal(consoleFocus, model.SelectedKey);
                Assert.Same(dnaTab, tabs.SelectedItem);
                Assert.True(ShowsFacts(consoleFocus));

                model.ClearSelection();
                viewport.SelectKey(null);
                command.Text = "showdna";
                run.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitForAsync(() => info.IsVisible && dna.Text == "This Robot is dead.  No DNA available.");
                Assert.Null(model.SelectedKey);
                Assert.Same(dnaTab, tabs.SelectedItem);
                Assert.True(ShowsFacts(consoleFocus));

                bool ShowsFacts(OrganismKey actor) => info.GetVisualDescendants().OfType<TextBlock>()
                    .Any(label => label.Text?.StartsWith($"ARRAY ID       {actor.Slot:N0}{Environment.NewLine}",
                        StringComparison.Ordinal) == true);

                async Task EnterMarkerAsync(string input, string action, string expected)
                {
                    marker.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    await WaitForAsync(() => info.OwnedWindows.OfType<LegacyValueWindow>().Any());
                    var dialog = Assert.Single(info.OwnedWindows.OfType<LegacyValueWindow>());
                    Assert.Single(dialog.GetVisualDescendants().OfType<TextBox>()).Text = input;
                    Assert.Single(dialog.GetVisualDescendants().OfType<Button>(), button => Equals(button.Content, action))
                        .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    await WaitForAsync(() => !dialog.IsVisible && dna.Text == expected);
                }
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                if (window.IsVisible) window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task NativeConsoleRejectsCommandsAfterTheLoadMenuReusesTheSlotButSurvivesARejectedLoad()
    {
        const string oldDna = "start\n111 921 store\nstop";
        const string replacementDna = "start\n222 921 store\nstop";
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            Species = [new SpeciesImport("Console Survivor", oldDna, false, 0xff239ac0, 1, 1_000, 0, false, 1)],
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var picker = new AuditStorageService();
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, picker);
            try
            {
                window.Show();
                var model = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitForAsync(() => model.SelectedKey.HasValue && model.NativeCycle.HasValue
                    && model.Status.Contains("WORLD READY", StringComparison.Ordinal));
                var session = (SimulationSession)typeof(MainWindow)
                    .GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window)!;
                var focus = model.SelectedKey!.Value;
                var original = Assert.Single(session.LatestSnapshot.Organisms);
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var open = menu.FindCommandItem(LegacyCommandId.RobotOpenConsole)!;
                var load = menu.FindCommandItem(LegacyCommandId.FileLoadSimulation)!;
                Click(open);
                await WaitForAsync(() => window.OwnedWindows.OfType<RobotConsoleWindow>().Any());
                var previousConsole = Assert.Single(window.OwnedWindows.OfType<RobotConsoleWindow>());
                Assert.Contains("mem[901] = 17", await SubmitThroughConsoleAsync(previousConsole, "set 901 17"), StringComparison.Ordinal);

                picker.OpenedSimulation = [1];
                Click(load);
                await WaitForAsync(() => model.Status.Contains("FileLoadSimulation failed", StringComparison.Ordinal));
                Assert.True(previousConsole.IsVisible);
                Assert.Contains("mem[901] = 27", await SubmitThroughConsoleAsync(previousConsole, "set 901 27"), StringComparison.Ordinal);
                Assert.Equal(27, Cell(Assert.Single(session.LatestSnapshot.Organisms), 901));

                using (var source = NativeEngineClientFactory.Instance.Create(setup))
                {
                    source.Load(await session.SaveAsync());
                    source.ReplaceDna(focus.Slot, focus.Generation, replacementDna);
                    source.SetOrganismMemory(focus.Slot, focus.Generation, 901, 37);
                    source.SetOrganismEnergy(focus.Slot, focus.Generation, 700);
                    picker.OpenedSimulation = source.Save();
                }
                Click(load);
                await WaitForAsync(() => model.Status == "LOADED");
                var fresh = Assert.Single(session.LatestSnapshot.Organisms);
                Assert.Equal((focus.Slot, focus.Generation, original.UniqueId), (fresh.Slot, fresh.Generation, fresh.UniqueId));
                Assert.Equal((700, 37, 0), (fresh.Energy, Cell(fresh, 901), Cell(fresh, 921)));
                Assert.True(previousConsole.IsVisible);

                var responses = new[]
                {
                    await SubmitThroughConsoleAsync(previousConsole, "set 901 88"),
                    await SubmitThroughConsoleAsync(previousConsole, "energy 14"),
                    await SubmitThroughConsoleAsync(previousConsole, "execrob"),
                };
                var current = Assert.Single(session.LatestSnapshot.Organisms);
                Assert.Equal((700, 37, 0), (current.Energy, Cell(current, 901), Cell(current, 921)));
                Assert.All(responses, response => Assert.Contains("This console belongs to a previous simulation.",
                    response, StringComparison.Ordinal));

                Click(open);
                await WaitForAsync(() => window.OwnedWindows.OfType<RobotConsoleWindow>().Count() == 2);
                var newConsole = Assert.Single(window.OwnedWindows.OfType<RobotConsoleWindow>(), tool => tool != previousConsole);
                Assert.Contains("mem[901] = 55", await SubmitThroughConsoleAsync(newConsole, "set 901 55"), StringComparison.Ordinal);
                Assert.Equal(55, Cell(Assert.Single(session.LatestSnapshot.Organisms), 901));
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                if (window.IsVisible) window.Close();
            }
        }, CancellationToken.None);

        static void Click(MenuItem item) => item.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
        static int Cell(OrganismSnapshot actor, int address) => actor.Memory.SingleOrDefault(cell => cell.Address == address)?.Value ?? 0;
    }

    [Fact]
    public async Task NativeConsoleRechecksAQueuedMemoryWriteAfterAddressResolution()
    {
        var factory = new AuditEngineClientFactory();
        var picker = new AuditStorageService { OpenedSimulation = "DB3S"u8.ToArray() };
        var setup = new WorldSetupOptions
        {
            Species = [new SpeciesImport("Console Survivor", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, factory, picker);
            using var releaseResolution = new ManualResetEventSlim();
            try
            {
                window.Show();
                var model = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitForAsync(() => model.SelectedKey.HasValue && model.Status.Contains("WORLD READY", StringComparison.Ordinal));
                var session = (SimulationSession)typeof(MainWindow)
                    .GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window)!;
                var engine = Assert.Single(factory.Clients);
                engine.NativeCycle = 1;
                await session.RefreshSnapshotAsync();
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                menu.FindCommandItem(LegacyCommandId.RobotOpenConsole)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitForAsync(() => window.OwnedWindows.OfType<RobotConsoleWindow>().Any());
                var console = Assert.Single(window.OwnedWindows.OfType<RobotConsoleWindow>());
                var resolving = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
                engine.BeforeResolveMemoryAddress = () =>
                {
                    resolving.TrySetResult();
                    if (!releaseResolution.Wait(TimeSpan.FromSeconds(10)))
                        throw new TimeoutException("The test did not release the memory resolver.");
                };
                engine.Calls.Clear();
                var command = SubmitThroughConsoleAsync(console, "set 901 88");
                await resolving.Task.WaitAsync(TimeSpan.FromSeconds(5));
                window.FindControl<Button>("LoadButton")!
                    .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                var loading = typeof(MainWindow).GetField("_loadingSimulation", BindingFlags.Instance | BindingFlags.NonPublic)!;
                await WaitForAsync(() => (bool)loading.GetValue(window)!);
                releaseResolution.Set();
                await WaitForAsync(() => model.Status == "LOADED");
                var response = await command;
                Assert.True(response.Contains("The simulation is loading.", StringComparison.Ordinal)
                    || response.Contains("This console belongs to a previous simulation.", StringComparison.Ordinal), response);
                Assert.Contains("Load", engine.Calls);
                Assert.DoesNotContain(engine.Calls, call => call.StartsWith("Memory:", StringComparison.Ordinal));
            }
            finally
            {
                releaseResolution.Set();
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                if (window.IsVisible) window.Close();
            }
        }, CancellationToken.None);
    }

    private static async Task<string> SubmitThroughConsoleAsync(RobotConsoleWindow console, string input)
    {
        console.UpdateLayout();
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        var transcript = Assert.Single(console.GetVisualDescendants().OfType<TextBox>(), box => box.IsReadOnly);
        var command = Assert.Single(console.GetVisualDescendants().OfType<TextBox>(), box => !box.IsReadOnly);
        var run = Assert.Single(console.GetVisualDescendants().OfType<Button>(), button => Equals(button.Content, "RUN"));
        var before = transcript.Text?.Length ?? 0;
        command.Text = input;
        run.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        await WaitForAsync(() => transcript.Text?.Length > before + input.Length + 3);
        return transcript.Text![before..];
    }

    [Fact]
    public async Task NativeConsoleEnergyChangesSingleWithoutWritingTheSensedMemory()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            await using var session = new SimulationSession(new NativeEngineClient(new WorldSetupOptions
            {
                Backend = "Cpu",
                PopulationCapacity = 32,
            }));
            await session.ImportDnaAsync("start *.nrg 50 store stop");
            Assert.NotNull(session.LatestSnapshot.NativeCycle);
            var robot = Assert.Single(session.LatestSnapshot.Organisms);
            await session.SetOrganismEnergyAsync(robot.Slot, robot.Generation, 1_000);
            var console = new RobotConsoleWindow(session, new OrganismKey(robot.Slot, robot.Generation));
            try
            {
                console.Show();
                console.UpdateLayout();
                var transcript = Assert.Single(console.GetVisualDescendants().OfType<TextBox>(), box => box.IsReadOnly);
                var submit = typeof(RobotConsoleWindow).GetMethod("RunCommandAsync",
                    BindingFlags.Instance | BindingFlags.NonPublic)!;
                var start = session.LatestSnapshot.Tick;
                await EnterEnergy("energy 32001", 32_001.0f);
                await EnterEnergy("energy -3.25", -3.25f);
                await EnterEnergy("energy", 0.0f);
                await EnterEnergy("energy &H400", 1_024.0f);
                await EnterEnergy("energy  999", 0.0f);
                await EnterEnergy("energy 1234.5tail 999", 1_234.5f);
                Assert.Equal(start, session.LatestSnapshot.Tick);
                Assert.Equal(1_000, Memory(310));

                await RunAsync("execrob");
                Assert.Equal(1_000, Memory(50));
                Assert.Equal(start, session.LatestSnapshot.Tick);

                await RunAsync("cycle 1");
                Assert.Equal(start + 1, session.LatestSnapshot.Tick);
                Assert.Equal(1_000, Memory(50));
                Assert.Equal(1_234, Memory(310));
                Assert.Equal(-234, Memory(203));
                Assert.Equal(234, Memory(204));

                Task RunAsync(string input) => (Task)submit.Invoke(console, [input, false])!;

                int Memory(ushort address) => Assert.Single(Assert.Single(session.LatestSnapshot.Organisms).Memory,
                    cell => cell.Address == address).Value;

                async Task EnterEnergy(string input, float expected)
                {
                    var previous = transcript.Text;
                    await RunAsync(input);
                    Assert.Equal(previous + $"> {input}\n", transcript.Text);

                    using var save = JsonDocument.Parse((await session.SaveAsync()).AsMemory(10));
                    var saved = Assert.Single(save.RootElement.GetProperty("slots").EnumerateArray(),
                        slot => slot.GetProperty("organism").ValueKind != JsonValueKind.Null);
                    var materials = saved.GetProperty("organism").GetProperty("materials");
                    Assert.Equal(expected, materials.GetProperty("energy").GetSingle());
                    Assert.Equal(1_000.0f, materials.GetProperty("previous_energy").GetSingle());
                    Assert.Equal(1_000, Memory(310));
                }
            }
            finally { console.Close(); }
        }, CancellationToken.None);
    }

    private static async Task RunAndWaitAsync(
        TextBox command,
        Button run,
        TextBox transcript,
        string input,
        string expected)
    {
        command.Text = input;
        run.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        await WaitForAsync(
            () => transcript.Text?.Contains(expected, StringComparison.Ordinal) == true,
            $"Console transcript did not contain {expected}.");
    }

    private static async Task WaitForAsync(Func<bool> predicate, string? failure = null)
    {
        for (var attempt = 0; attempt < 500; attempt++)
        {
            if (predicate()) return;
            await Task.Delay(10);
        }

        Assert.True(predicate(), failure ?? "Timed out waiting for the rendered console route.");
    }
}
