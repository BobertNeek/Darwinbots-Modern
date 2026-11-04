using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class FinalToolsVisualRenderedTests
{
    [Fact]
    public async Task ArenaCorpseRendersOnceAndOlderArchivedCorpsesRemainVisible()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var robot = new OrganismSnapshot(7, 3, [250f, 200f], [0f, 0f], 0, 1, Body: 100)
            { UniqueId = 77, Radius = 40 };
            var snapshot = new EngineSnapshot(2, 0, "CPU", [robot])
            {
                WorldSize = [600f, 400f],
                RenderInstances = [new RenderInstanceSnapshot(7, robot.Position, 40, 0xffffffff) { Generation = 3 }],
            };
            var viewport = new WorldViewport
            {
                Width = 480, Height = 360, RenderSkins = false,
                ShowMovementVectors = false, ShowResourceGauges = false, ShowDeathPoffs = true,
            };
            viewport.SetSnapshot(snapshot);
            var host = new Window { Width = 480, Height = 360, Content = viewport };
            host.Show();
            var robotFrame = CapturePng(host);
            viewport.SetSnapshot(snapshot with
            { Corpses = [new CorpseSnapshot(robot.Position, robot.Velocity, 0, 100, 1) { UniqueId = 77 }] });
            var corpseFrame = CapturePng(host);
            Assert.True(robotFrame.SequenceEqual(corpseFrame), "The small corpse projection must not paint a second body over the arena robot.");
            viewport.SetSnapshot(snapshot with
            { Corpses = [new CorpseSnapshot([100f, 200f], [0f, 0f], 0, 100, 1) { UniqueId = 88 }] });
            Assert.False(robotFrame.SequenceEqual(CapturePng(host)), "An older archived corpse still needs its visible body.");
            var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR")
                ?? Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
            Directory.CreateDirectory(directory);
            File.WriteAllBytes(Path.Combine(directory, "native-arena-corpse.png"), corpseFrame);
            host.Close();
        }, CancellationToken.None);
    }

    [Fact]
    public async Task SelectedEyeRayUsesCurrentEyeValueToChangeItsRenderedRange()
    {
        var focus = new OrganismKey(7, 3);
        var baseOrganism = new OrganismSnapshot(
            7,
            3,
            [50f, 50f],
            [0f, 0f],
            875,
            19)
        {
            Radius = 12.5f,
            Vision = new VisionSnapshot(
                0,
                [new EyeSnapshot(0, 35, 0f, 0.05f, 1_440f, 0)]),
        };
        var renderInstance = new RenderInstanceSnapshot(7, [50f, 50f], 12.5f, 0xff123456)
        {
            Generation = 3,
        };
        var wideVision = new EngineSnapshot(27, 1, "CPU", [baseOrganism])
        {
            RenderInstances = [renderInstance],
            WorldSize = [100f, 100f],
        };
        var observedVision = wideVision with
        {
            Organisms =
            [
                baseOrganism with
                {
                    Vision = new VisionSnapshot(
                        0,
                        [new EyeSnapshot(0, 35, 0f, 0.05f, 1_440f, 32_000)]),
                },
            ],
        };

        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var viewport = new WorldViewport
            {
                Width = 480,
                Height = 360,
                RenderSkins = false,
                ShowMovementVectors = false,
                ShowResourceGauges = false,
            };
            viewport.SetSnapshot(wideVision);
            viewport.SelectKey(focus);
            var viewportHost = new Window
            {
                Width = 480,
                Height = 360,
                Content = viewport,
            };
            viewportHost.Show();

            var fullRangeFrame = CapturePng(viewportHost);
            viewport.SetSnapshot(observedVision);
            var observedRangeFrame = CapturePng(viewportHost);
            var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR")
                ?? Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
            Directory.CreateDirectory(directory);
            File.WriteAllBytes(Path.Combine(directory, "native-vision-full-range.png"), fullRangeFrame);
            File.WriteAllBytes(Path.Combine(directory, "native-vision-observed-range.png"), observedRangeFrame);

            Assert.False(
                fullRangeFrame.SequenceEqual(observedRangeFrame),
                "A nonzero eye reading must change the selected eye ray in the rendered viewport.");

            viewportHost.Close();
        }, CancellationToken.None);
    }

    [Fact]
    public async Task LiveToolsAndWorldReceiptShowCompleteRobotFactsAndLegacyShotEncoding()
    {
        var focus = new OrganismKey(7, 3);
        var eyes = VisionSnapshot.Default.Eyes
            .Select(eye => eye with { CenterRadians = (eye.CenterRadians + 1.57f) % MathF.Tau })
            .ToArray();
        eyes[4] = new EyeSnapshot(314, 70, 3.14f, 0.175f + MathF.PI / 36f, 1440f, 875);
        var organism = new OrganismSnapshot(
            7,
            3,
            [100f, 200f],
            [1.5f, -0.5f],
            875,
            19,
            Species: 1,
            Body: 321,
            Waste: 8,
            Shell: 3,
            Slime: 4,
            Venom: 5,
            Poison: 6,
            Chloroplasts: 7,
            Aim: 314)
        {
            Touch = 4,
            Taste = -2,
            Mass = 333,
            Radius = 12.5f,
            Memory =
            [
                new MemoryCellSnapshot(310, 875),
                new MemoryCellSnapshot(311, 100),
                new MemoryCellSnapshot(312, 48),
            ],
            Phenotype = new VisualPhenotypeSnapshot(
                44,
                0xff123456,
                [
                    new SkinPointSnapshot(0.45f, 0),
                    new SkinPointSnapshot(0.72f, 314),
                    new SkinPointSnapshot(0.45f, 628),
                ],
                9),
            Vision = new VisionSnapshot(
                4,
                eyes)
        };
        var snapshot = new EngineSnapshot(27, 1, "CPU", [organism])
        {
            Species =
            [
                new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false),
                new SpeciesSnapshot("Audit Bot", false, 0xff123456, 0, false),
            ],
            RenderInstances =
            [new RenderInstanceSnapshot(7, [100f, 200f], 12.5f, 0xff123456)
            {
                Generation = 3,
                Aim = 314,
                Skin = organism.Phenotype.Skin,
            }],
            Shots =
            [new ShotSnapshot(focus, [20f, 40f], [80f, 40f], [40f, 0f], -1, 40, 1, 40, 40f, false)],
            Ties = [new TieSnapshot(focus, focus, 3f)],
            Corpses = [new CorpseSnapshot([60f, 40f], [0f, 0f], 0, 100, 1)],
            WorldSize = [600f, 400f],
        };

        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var viewport = new WorldViewport
            {
                Width = 480,
                Height = 360,
            };
            viewport.SetSnapshot(snapshot);
            viewport.SelectKey(focus);
            var viewportHost = new Window
            {
                Width = 480,
                Height = 360,
                Content = viewport,
            };
            viewportHost.Show();
            AvaloniaHeadlessPlatform.ForceRenderTimerTick();

            using (var frame = viewportHost.CaptureRenderedFrame())
            {
                var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR")
                    ?? Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
                Directory.CreateDirectory(directory);
                using var stream = File.Create(Path.Combine(directory, "final-tools-visual.png"));
                frame.Save(stream, new Avalonia.Media.Imaging.PngBitmapEncoderOptions());
            }

            var details = new RobotDetailsWindow(snapshot, focus, "start\nstop");
            details.Show();
            AvaloniaHeadlessPlatform.ForceRenderTimerTick();
            var visualText = string.Join(
                "\n",
                details.GetVisualDescendants().OfType<TextBlock>().Select(value => value.Text));

            Assert.Contains("SKIN", visualText, StringComparison.Ordinal);
            Assert.Contains("EYES", visualText, StringComparison.Ordinal);
            Assert.Contains("TOUCH / TASTE", visualText, StringComparison.Ordinal);
            Assert.Contains("TIES", visualText, StringComparison.Ordinal);
            Assert.Contains("start", visualText, StringComparison.Ordinal);
            Assert.Contains("310", visualText, StringComparison.Ordinal);

            details.Close();
            viewportHost.Close();
        }, CancellationToken.None);
    }

    private static byte[] CapturePng(Window window)
    {
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        using var frame = window.CaptureRenderedFrame();
        using var stream = new MemoryStream();
        frame.Save(stream, new Avalonia.Media.Imaging.PngBitmapEncoderOptions());
        return stream.ToArray();
    }
}
