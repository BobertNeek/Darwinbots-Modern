using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Media.Imaging;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class UiInstrumentsRenderedTests
{
    [Fact]
    public async Task PlayerRouteRendersStatusInstrumentAndStatisticsEventStatusProjection()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            TicksPerUpdate = 1,
            Species =
            [
                new SpeciesImport(
                    "Instrument Species",
                    "start\nstop",
                    false,
                    0xff239ac0,
                    1,
                    1_000,
                    0,
                    false,
                    1),
            ],
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, new AuditEngineClientFactory(), new AuditStorageService())
            {
                WindowState = WindowState.Normal,
                Width = 1_280,
                Height = 900,
            };

            try
            {
                window.Show();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitFor(
                    () => viewModel.SpeciesCount > 0
                        && viewModel.Status.StartsWith("WORLD READY", StringComparison.Ordinal),
                    "MainWindow did not publish the imported player-world state.");

                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var frame = Assert.IsAssignableFrom<Bitmap>(window.CaptureRenderedFrame());
                Assert.True(frame.PixelSize.Width >= 1_000, $"Rendered width was {frame.PixelSize.Width}.");
                Assert.True(frame.PixelSize.Height >= 700, $"Rendered height was {frame.PixelSize.Height}.");

                var mainSurface = window.FindControl<Grid>("MainSurfaceGrid")
                    ?? throw new Xunit.Sdk.XunitException("MainWindow is missing MainSurfaceGrid.");
                var statusInstrument = window.FindControl<ItemsControl>("StatusInstrument")
                    ?? throw new Xunit.Sdk.XunitException("MainWindow is missing StatusInstrument.");
                var statistics = window.FindControl<Expander>("StatisticsExpander")
                    ?? throw new Xunit.Sdk.XunitException("MainWindow is missing StatisticsExpander.");

                Assert.True(statusInstrument.IsVisible);
                Assert.True(viewModel.StatusPanels.Count >= 13);
                var statusText = VisibleText(statusInstrument);
                Assert.All(
                    viewModel.StatusPanels.Take(13),
                    panel => Assert.Contains(panel.Label, statusText, StringComparison.Ordinal));

                var surfaceText = VisibleText(mainSurface);
                Assert.Contains("EVENT LOG", surfaceText, StringComparison.Ordinal);
                Assert.Contains(viewModel.LifecycleSummary, surfaceText, StringComparison.Ordinal);
                Assert.Contains(viewModel.CombatSummary, surfaceText, StringComparison.Ordinal);
                Assert.Contains(viewModel.PlantSummary, surfaceText, StringComparison.Ordinal);
                Assert.Contains(viewModel.MutationSummary, surfaceText, StringComparison.Ordinal);
                Assert.Contains(viewModel.Status, surfaceText, StringComparison.Ordinal);

                Assert.True(statistics.IsVisible);
                Assert.True(
                    statistics.IsExpanded,
                    "Statistics must be open on the player route so GRAPH-04 is player-visible.");
                var statisticsText = VisibleText(statistics);
                Assert.Contains(viewModel.LifecycleSummary, statisticsText, StringComparison.Ordinal);
                Assert.Contains(viewModel.CombatSummary, statisticsText, StringComparison.Ordinal);
                Assert.Contains(viewModel.EnergyActivitySummary, statisticsText, StringComparison.Ordinal);
                Assert.Contains(viewModel.PlantSummary, statisticsText, StringComparison.Ordinal);
                Assert.Contains(viewModel.MutationSummary, statisticsText, StringComparison.Ordinal);
            }
            finally
            {
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static string VisibleText(Control root) => string.Join(
        "\n",
        root.GetVisualDescendants()
            .OfType<TextBlock>()
            .Select(value => value.Text)
            .Where(value => !string.IsNullOrWhiteSpace(value)));

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
}
