using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class Shell02PersistentToolsRenderedTests
{
    [Fact]
    public async Task MainWindowKeepsThreePersistentToolsOwnedVisibleAndInteractive()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            Species =
            [
                new SpeciesImport("Shell Audit Species", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1),
            ],
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, new AuditEngineClientFactory(), new AuditStorageService())
            {
                WindowState = WindowState.Normal,
                Width = 1_280,
                Height = 820,
            };

            try
            {
                window.Show();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                await WaitFor(
                    () => TryField<SimulationSession>(window, "_session")?.LatestSnapshot.Organisms.Count > 0,
                    "MainWindow did not start its player simulation.");
                var session = TryField<SimulationSession>(window, "_session")!;
                var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitFor(() => viewModel.Population > 0, "MainWindow did not publish the live population to its view model.");

                var graphsButton = window.FindControl<Button>("GraphsButton")!;
                graphsButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(
                    () => window.OwnedWindows.OfType<GraphSuiteWindow>().Any(),
                    "Graphs button did not open Graph Suite.");
                var graphWindow = Assert.Single(window.OwnedWindows.OfType<GraphSuiteWindow>());

                var contestButton = window.FindControl<Button>("ContestResultsButton")!;
                contestButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(
                    () => window.OwnedWindows.OfType<ContestResultsWindow>().Any(),
                    "Contest button did not open Contest Results.");
                var contestWindow = Assert.Single(window.OwnedWindows.OfType<ContestResultsWindow>());

                var selected = session.LatestSnapshot.Organisms[0];
                var focus = new OrganismKey(selected.Slot, selected.Generation);
                viewModel.Select(focus);
                Assert.NotNull(viewModel.SelectedOrganism);
                window.FindControl<WorldViewport>("Viewport")!.SelectKey(focus);
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var router = (LegacyCommandRouter)typeof(MainWindow)
                    .GetField("_legacyCommands", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .GetValue(window)!;
                menu.RefreshStates(router.GetState);
                menu.FindCommandItem(LegacyCommandId.RobotShowInfo)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(
                    () => window.OwnedWindows.OfType<RobotDetailsWindow>().Any(),
                    "Robot details menu route did not open its persistent tool.");
                var robotWindow = Assert.Single(window.OwnedWindows.OfType<RobotDetailsWindow>());

                var ownedTools = window.OwnedWindows.ToArray();
                Assert.Equal(3, ownedTools.Length);
                Assert.Contains(graphWindow, ownedTools);
                Assert.Contains(contestWindow, ownedTools);
                Assert.Contains(robotWindow, ownedTools);
                Assert.All(ownedTools, owned =>
                {
                    Assert.Same(window, owned.Owner);
                    Assert.True(owned.IsVisible);
                });
                Assert.True(window.IsEnabled);
                Assert.True(window.FindControl<WorldViewport>("Viewport")!.IsEnabled);

                graphsButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                var reopenedGraph = Assert.Single(window.OwnedWindows.OfType<GraphSuiteWindow>());
                Assert.Same(graphWindow, reopenedGraph);
                Assert.True(reopenedGraph.IsVisible);
                Assert.Equal(3, window.OwnedWindows.Count);

                var benignInput = window.FindControl<Button>("ToolbarSlot12")!;
                window.Activate();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                Assert.True(benignInput.Focus());
                Press(window, Key.Space, PhysicalKey.Space);
                Assert.Contains("OFF", benignInput.Content?.ToString(), StringComparison.Ordinal);

                graphWindow.Width = 860;
                graphWindow.Height = 620;
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var frame = graphWindow.CaptureRenderedFrame();
                Assert.NotNull(frame);
                Assert.True(frame.PixelSize.Width >= 700, $"Rendered width was {frame.PixelSize.Width}.");
                Assert.True(frame.PixelSize.Height >= 500, $"Rendered height was {frame.PixelSize.Height}.");
                using var png = new MemoryStream();
                frame.Save(png, new PngBitmapEncoderOptions());
                var bytes = png.ToArray();
                Assert.True(bytes.Length > 1_000, $"Rendered PNG was only {bytes.Length} bytes.");
                Assert.Equal(new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 }, bytes.Take(8));

                if (Environment.GetEnvironmentVariable("DB24832_EXTRACT_SHELL02_RECEIPT") == "1")
                    throw new InvalidOperationException("DB24832_SHELL02_PNG_BASE64:" + Convert.ToBase64String(bytes));

                var receiptPath = ReceiptPath();
                Directory.CreateDirectory(Path.GetDirectoryName(receiptPath)!);
                File.WriteAllBytes(receiptPath, bytes);
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray())
                    if (ownedWindow.IsVisible) ownedWindow.Close();
                if (window.IsVisible) window.Close();
            }

            return true;
        }, CancellationToken.None);
    }

    private static void Press(Window window, Key key, PhysicalKey physicalKey)
    {
        window.KeyPress(key, RawInputModifiers.None, physicalKey, null);
        window.KeyRelease(key, RawInputModifiers.None, physicalKey, null);
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }

    private static async Task WaitFor(Func<bool> condition, string diagnostic)
    {
        for (var attempt = 0; attempt < 500; attempt++)
        {
            AvaloniaHeadlessPlatform.ForceRenderTimerTick();
            if (condition()) return;
            await Task.Delay(20);
        }

        throw new Xunit.Sdk.XunitException(diagnostic);
    }

    private static T? TryField<T>(object target, string name) where T : class =>
        target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)?.GetValue(target) as T;

    private static string ReceiptPath()
    {
        for (var directory = new DirectoryInfo(AppContext.BaseDirectory); directory is not null; directory = directory.Parent)
        {
            var gitPath = Path.Combine(directory.FullName, ".git");
            if (File.Exists(gitPath) || Directory.Exists(gitPath))
                return Path.Combine(directory.FullName, "docs", "parity", "receipts", "db24832-shell02-persistent-tools.png");
        }

        throw new DirectoryNotFoundException("Could not locate the worktree root for the SHELL-02 receipt.");
    }
}
