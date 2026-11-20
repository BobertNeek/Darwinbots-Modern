using Avalonia.Controls;
using Avalonia.Headless;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class SelectedRobotToolsRenderedTests
{
    [Fact]
    public async Task SelectedRobotToolsRenderAndWriteDnaThroughExactFocus()
    {
        var organism = new OrganismSnapshot(
            7,
            3,
            [100f, 200f],
            [1f, -1f],
            875,
            19,
            Species: 1,
            Parents: [new OrganismKey(2, 1), null])
        {
            Memory = [
                new MemoryCellSnapshot(310, 16),
                new MemoryCellSnapshot(311, 32),
                new MemoryCellSnapshot(312, 48),
            ],
        };
        var snapshot = new EngineSnapshot(12, 1, "CPU", [organism])
        {
            Species = [new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false)],
        };
        var engine = new RecordingEngineClient(snapshot);
        await using var simulation = new SimulationSession(engine);
        await simulation.StepAsync(0);

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var focus = new OrganismKey(7, 3);
            var eye = new EyeDesignerWindow(simulation, focus);
            var rgb = new RgbMemoryMonitorWindow(focus, RgbMemoryMonitorSettings.Default, _ => { });
            var phylogeny = new PhylogenyWindow(snapshot, focus);
            try
            {
                eye.Show();
                rgb.SetSnapshot(snapshot);
                rgb.Show();
                phylogeny.Show();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                AssertRendered(eye);
                AssertRendered(rgb);
                AssertRendered(phylogeny);

                await eye.WriteDnaAsync();
                var replacement = Assert.Single(engine.DnaReplacements);
                Assert.Equal((7u, 3u), (replacement.Slot, replacement.Generation));
                Assert.Contains(".eye9width store", replacement.Dna, StringComparison.Ordinal);
            }
            finally
            {
                if (eye.IsVisible) eye.Close();
                if (rgb.IsVisible) rgb.Close();
                if (phylogeny.IsVisible) phylogeny.Close();
            }

            return 0;
        }, CancellationToken.None);
    }

    private static void AssertRendered(Window window)
    {
        using var frame = window.CaptureRenderedFrame();
        Assert.NotNull(frame);
        Assert.True(frame.PixelSize.Width >= 400, $"Rendered width was {frame.PixelSize.Width}.");
        Assert.True(frame.PixelSize.Height >= 300, $"Rendered height was {frame.PixelSize.Height}.");
    }
}
