using System.Reflection;
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

                await RunAndWaitAsync(command, buttons["RUN"], transcript, "showdna", "start\nstop");
                Assert.Empty(window.OwnedWindows.OfType<RobotDetailsWindow>());

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
    public async Task NativeConsoleDnaDistinguishesTheTypedCommandFromTheDnaButton()
    {
        const string firstDna = "start\n1101 501 store\nstop";
        const string secondDna = "start\n2202 502 store\nstop";
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
                var selectedDna = await session.ExportDnaAsync(selectedFocus.Slot, selectedFocus.Generation);
                var consoleDna = await session.ExportDnaAsync(consoleFocus.Slot, consoleFocus.Generation);
                Assert.NotEqual(selectedDna, consoleDna);
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
                var dna = Assert.IsType<TextBox>(Assert.IsType<Border>(dnaTab.Content).Child);

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
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                if (window.IsVisible) window.Close();
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
