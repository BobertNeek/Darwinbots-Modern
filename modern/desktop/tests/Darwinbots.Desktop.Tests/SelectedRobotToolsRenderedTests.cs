using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class SelectedRobotToolsRenderedTests
{
    [Fact]
    public async Task AutoTagMenuEditsMatchingNativeRobotsAndKeepsCanceledChangesOut()
    {
        await using var simulation = new SimulationSession(new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu", PopulationCapacity = 64,
        }));
        const string dna = "start stop\n'#tag: Old tag";
        await simulation.ImportSpeciesAsync(new SpeciesImport("Founder.txt", dna, false, 0xff239ac0, 2, 1000, 0, false, 0));
        await simulation.ImportSpeciesAsync(new SpeciesImport("founder.txt", dna, false, 0xff239ac0, 1, 1000, 0, false, 0));

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), NativeEngineClientFactory.Instance, new AuditStorageService());
            DesktopTestApplication.UseSession(window, simulation);
            window.Show();
            try
            {
                var router = (LegacyCommandRouter)typeof(MainWindow)
                    .GetField("_legacyCommands", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window)!;
                const LegacyCommandId command = LegacyCommandId.EditAutomaticallyTagByName;
                Assert.False(router.GetState(command).Checked);

                var apply = router.ExecuteAsync(command);
                CompleteTagDialog(await WaitForTagDialog(window), "Founder.txt", "APPLY");
                CompleteTagDialog(await WaitForTagDialog(window), " leadéÿ", "APPLY");
                Assert.Equal("TAGGED · 2 ROBOT(S)", (await apply).Status);

                var cancel = router.ExecuteAsync(command);
                CompleteTagDialog(await WaitForTagDialog(window), "Founder.txt", "APPLY");
                CompleteTagDialog(await WaitForTagDialog(window), "Discard this", "CANCEL");
                Assert.True((await cancel).Canceled);

                var snapshot = simulation.LatestSnapshot;
                Assert.Equal(3, snapshot.Population);
                Assert.Equal(0ul, snapshot.Tick);
                foreach (var robot in snapshot.Organisms)
                {
                    var label = snapshot.Species[(int)robot.Species].Name;
                    var expected = label == "Founder.txt" ? " lead?ÿ" : "Old tag";
                    Assert.EndsWith($"'#tag: {expected}", await simulation.ExportDnaAsync(robot.Slot, robot.Generation), StringComparison.Ordinal);
                }
            }
            finally
            {
                foreach (var dialog in window.OwnedWindows.ToArray()) dialog.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

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

    private static async Task<LegacyValueWindow> WaitForTagDialog(MainWindow window)
    {
        for (var attempts = 0; attempts < 100; attempts++)
        {
            if (window.OwnedWindows.OfType<LegacyValueWindow>().SingleOrDefault(candidate => candidate.IsVisible) is { } dialog)
                return dialog;
            await Task.Delay(20);
        }
        throw new Xunit.Sdk.XunitException("The AutoTag value dialog did not open.");
    }

    private static void CompleteTagDialog(LegacyValueWindow dialog, string input, string action)
    {
        var controls = dialog.GetVisualDescendants().ToArray();
        Assert.Single(controls.OfType<TextBox>()).Text = input;
        Assert.Single(controls.OfType<Button>(), button => Equals(button.Content, action))
            .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
    }
}
