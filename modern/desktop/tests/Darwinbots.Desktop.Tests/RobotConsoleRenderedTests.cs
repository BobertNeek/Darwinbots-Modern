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
