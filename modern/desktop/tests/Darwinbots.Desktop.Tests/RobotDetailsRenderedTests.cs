using Avalonia.Headless;
using Avalonia.Controls;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class RobotDetailsRenderedTests
{
    [Fact]
    public async Task RobotDetailsRendersLiveFactsMemoryAndActivationTimeline()
    {
        var organism = new OrganismSnapshot(
            7,
            3,
            [100f, 200f],
            [1.5f, -0.5f],
            875,
            19,
            Species: 1,
            Parents: [new OrganismKey(2, 1), null])
        {
            CurrentGene = 2,
            ConditionCount = 3,
            ActivationBitmap = [false, true, false],
            ActivationHistory =
            [
                new ActivationFrameSnapshot(11, [false, true, false]),
                new ActivationFrameSnapshot(12, [true, false, true]),
            ],
            Memory = [new MemoryCellSnapshot(310, 875), new MemoryCellSnapshot(311, 100)],
            Mass = 101,
            Radius = 12.5f,
            PermanentWaste = 4,
            Offspring = 6,
            Descendants = 14,
            SimulationId = 22,
            UniqueId = 99,
            VirusTimer = 8,
            NewMutations = 2,
            TotalMutations = 9,
            DnaLength = 17,
            GeneCount = 3,
        };
        var snapshot = new EngineSnapshot(12, 1, "CPU", [organism])
        {
            Shots =
            [
                new ShotSnapshot(new(7, 3), [10, 10], [20, 10], [0, 0], -1, 0, 1, 10, 1, false),
                new ShotSnapshot(new(7, 3), [10, 20], [20, 20], [0, 0], -7, 0, 1, 10, 1, false) { Stored = true },
                new ShotSnapshot(new(7, 3), [10, 30], [20, 30], [0, 0], -1, 0, 1, 10, 1, true) { Alive = false },
                new ShotSnapshot(new(8, 3), [10, 40], [20, 40], [0, 0], -1, 0, 1, 10, 1, false),
            ],
            Species = [
                new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false),
                new SpeciesSnapshot("Audit Bot", false, 0xff239ac0, 0, false),
            ],
        };

        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var window = new RobotDetailsWindow(snapshot, new OrganismKey(7, 3), "cond\nstart\nstop");
            try
            {
                window.WindowState = Avalonia.Controls.WindowState.Normal;
                window.Show();
                Avalonia.Headless.AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                Assert.Equal("LIVE · TICK 12 · 2 GENE FRAMES", window.ActivationSummary);
                // Keep the prior live-owned count: include its stored packet,
                // exclude the dead flash and another robot's live packet.
                Assert.Contains(window.GetVisualDescendants().OfType<TextBlock>(),
                    block => block.Text?.Contains("SHOTS          2", StringComparison.Ordinal) == true);
                using var frame = window.CaptureRenderedFrame();
                Assert.NotNull(frame);
                Assert.True(frame.PixelSize.Width >= 900);
                Assert.True(frame.PixelSize.Height >= 560);
                var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR")
                    ?? Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
                Directory.CreateDirectory(directory);
                using var stream = File.Create(Path.Combine(directory, "robot-details.png"));
                frame.Save(stream, new Avalonia.Media.Imaging.PngBitmapEncoderOptions());
                window.Close();
            }
            finally
            {
                if (window.IsVisible) window.Close();
            }
        }, CancellationToken.None);
    }
}
