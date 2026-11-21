using System.Reflection;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
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
                Assert.Contains("RUN", buttons.Keys);
                Assert.Contains("STEP", buttons.Keys);
                Assert.Contains("EYES", buttons.Keys);
                Assert.Contains("DEBUG", buttons.Keys);
                Assert.Contains("DNA", buttons.Keys);
                Assert.Contains("PLAY", buttons.Keys);
                Assert.Contains("PAUSE", buttons.Keys);

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

                await RunAndWaitAsync(command, buttons["RUN"], transcript, "set 310 123", "mem[310] = 123");
                Assert.Contains("Memory:3:2:310:123", engine.Calls);

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
        for (var attempt = 0; attempt < 200; attempt++)
        {
            if (predicate()) return;
            await Task.Delay(10);
        }

        Assert.True(predicate(), failure ?? "Timed out waiting for the rendered console route.");
    }
}
