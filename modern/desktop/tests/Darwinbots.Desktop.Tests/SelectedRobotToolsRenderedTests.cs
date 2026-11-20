using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class SelectedRobotToolsRenderedTests
{
    [Fact]
    public async Task MakeNewSpeciesMenuConfirmsAndForksTheLiveDescendantBranch()
    {
        var options = new WorldSetupOptions
        {
            Backend = "Cpu", PopulationCapacity = 64,
            GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { NewSeed = false } },
        };
        using var engine = new NativeEngineClient(options);
        engine.ImportSpecies(new("Founder.txt", "start stop", false, 0xff239ac0, 2, 1000, 0, false, 0));
        var before = engine.Snapshot();
        var root = before.Organisms[0];
        var sibling = before.Organisms[1];
        var child = engine.CloneOrganism(root.Slot, root.Generation, [100f, 100f]);
        await using var simulation = new SimulationSession(engine);
        await simulation.RefreshSnapshotAsync();

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], options, new AuditEngineClientFactory(), new AuditStorageService());
            try
            {
                DesktopTestApplication.UseSession(window, simulation);
                window.Show();
                var model = (MainWindowViewModel)window.DataContext!;
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                var select = (Action<OrganismKey>)typeof(WorldViewport)
                    .GetField("OrganismKeySelected", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .GetValue(viewport)!;
                select(new OrganismKey(root.Slot, root.Generation));
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var action = menu.FindCommandItem(LegacyCommandId.RobotMakeNewSpecies)!;

                using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(5));
                action.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                Choose("NO");
                while (model.Status != "CANCELED")
                    await Task.Delay(10, timeout.Token);
                Assert.Equal(before.Species.Count, simulation.LatestSnapshot.Species.Count);

                action.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                Choose("YES");
                while (simulation.LatestSnapshot.Species.Count != before.Species.Count + 1)
                    await Task.Delay(10, timeout.Token);

                var after = simulation.LatestSnapshot;
                var newSpecies = (uint)after.Species.Count - 1;
                Assert.Matches(@"^Founder \d{1,5}$", after.Species[(int)newSpecies].Name);
                Assert.Equal(newSpecies, Assert.Single(after.Organisms, robot => robot.Slot == root.Slot).Species);
                Assert.Equal(newSpecies, Assert.Single(after.Organisms, robot => robot.Slot == child.Slot).Species);
                Assert.Equal(sibling.Species, Assert.Single(after.Organisms, robot => robot.Slot == sibling.Slot).Species);
                Assert.Equal(before.Tick, after.Tick);

                void Choose(string answer)
                {
                    var dialog = Assert.Single(window.OwnedWindows.OfType<LegacyConfirmationWindow>());
                    var button = Assert.Single(dialog.GetVisualDescendants().OfType<Button>(),
                        value => Equals(value.Content, answer));
                    button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
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
}
