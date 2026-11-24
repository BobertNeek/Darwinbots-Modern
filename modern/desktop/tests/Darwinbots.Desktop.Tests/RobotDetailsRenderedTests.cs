using Avalonia.Headless;
using Avalonia.Controls;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class RobotDetailsRenderedTests
{
    [Fact]
    public async Task NativeRobotDetailsRendersAllMemoryAddressesInOriginalRows()
    {
        var robot = new OrganismSnapshot(7, 3, [100f, 200f], [0f, 0f], 875, 19)
        {
            Memory = [new(1, 32767), new(10, -5), new(11, 42), new(1000, -32768)],
            DebugText = "console diagnostic",
        };
        var snapshot = new EngineSnapshot(12, 1, "CPU", [robot]) { NativeCycle = 12 };

        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var window = new RobotDetailsWindow(snapshot, new OrganismKey(7, 3));
            try
            {
                window.Show();
                window.SelectTab(RobotDetailsTab.Memory);
                window.UpdateLayout();
                var memory = Assert.Single(window.GetVisualDescendants().OfType<TextBox>());
                var rows = Assert.IsType<string>(memory.Text).Split("\r\n", StringSplitOptions.None);
                Assert.Equal(101, rows.Length);
                Assert.Empty(rows[^1]);
                for (var row = 0; row < 100; row++)
                {
                    Assert.StartsWith($" {row * 10 + 1}:\t", rows[row], StringComparison.Ordinal);
                    Assert.Equal(10, rows[row].Count(character => character == '\t'));
                }
                Assert.Equal(" 1:\t 32767\t 0\t 0\t 0\t 0\t 0\t 0\t 0\t 0\t-5", rows[0]);
                Assert.Equal(" 11:\t 42\t 0\t 0\t 0\t 0\t 0\t 0\t 0\t 0\t 0", rows[1]);
                Assert.Equal(" 991:\t 0\t 0\t 0\t 0\t 0\t 0\t 0\t 0\t 0\t-32768", rows[99]);
                Assert.DoesNotContain("console diagnostic", memory.Text, StringComparison.Ordinal);
                Assert.Contains(window.GetVisualDescendants().OfType<TextBlock>(),
                    block => block.Text?.Contains("MEMORY VIEW    1, 10, 11, 1000", StringComparison.Ordinal) == true);

                window.SetSnapshot(snapshot with { Organisms = [robot with { Memory = [] }] });
                Assert.DoesNotContain("-32768", memory.Text, StringComparison.Ordinal);
                Assert.EndsWith(" 991:\t 0\t 0\t 0\t 0\t 0\t 0\t 0\t 0\t 0\t 0\r\n", memory.Text, StringComparison.Ordinal);

                window.SetSnapshot(snapshot with { NativeCycle = null });
                Assert.StartsWith("   1       32767", memory.Text, StringComparison.Ordinal);
                Assert.Contains($"{Environment.NewLine}DEBUG{Environment.NewLine}console diagnostic", memory.Text, StringComparison.Ordinal);
            }
            finally
            {
                if (window.IsVisible) window.Close();
            }
        }, CancellationToken.None);
    }

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
                var tabs = Assert.Single(window.GetVisualDescendants().OfType<TabControl>());
                Assert.DoesNotContain(tabs.Items.OfType<TabItem>(), tab => Equals(tab.Header, "MUTATION DETAILS"));
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

    [Fact]
    public async Task NativeMutationDetailsShowsTheCompleteLogAndFollowsRobotInfoFocus()
    {
        string[] log = ["Newest mutation", "Second mutation", "Third mutation", "Fourth mutation", "Oldest mutation"];
        var first = RecordingEngineClient.Organism with { MutationLog = log };
        var second = first with { Slot = 4, MutationLog = [] };
        var snapshot = RecordingEngineClient.CreateSnapshot() with
        {
            NativeCycle = 0,
            Population = 2,
            Organisms = [first, second],
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            await using var session = new SimulationSession(new RecordingEngineClient(snapshot));
            await session.RefreshSnapshotAsync();
            var focus = new OrganismKey(first.Slot, first.Generation);
            var window = new RobotDetailsWindow(session, focus, readFocus: () => focus);
            try
            {
                window.Show();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                var tabs = Assert.Single(window.GetVisualDescendants().OfType<TabControl>());
                tabs.SelectedItem = Assert.Single(tabs.Items.OfType<TabItem>(), tab => Equals(tab.Header, "MUTATION DETAILS"));
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                var details = Assert.Single(window.GetVisualDescendants().OfType<TextBox>(), box => box.Name == "RobotMutationDetails");
                Assert.True(details.IsReadOnly);
                Assert.Equal(string.Join(Environment.NewLine, log), details.Text);

                focus = new OrganismKey(second.Slot, second.Generation);
                await session.StepAsync();
                await Dispatcher.UIThread.InvokeAsync(() => Assert.Equal("No mutations", details.Text), DispatcherPriority.Background);
            }
            finally { window.Close(); }
        }, CancellationToken.None);
    }
}
