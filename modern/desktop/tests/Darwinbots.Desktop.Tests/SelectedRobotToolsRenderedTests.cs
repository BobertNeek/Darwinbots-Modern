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
    public async Task EyeDesignerOpensWithoutAWorldAndExportsWithoutReplacingTheFocusedGenome()
    {
        var vision = VisionSnapshot.Default;
        vision.Eyes[0] = vision.Eyes[0] with { Direction = 12, Width = 34 };
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
            Vision = vision,
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
            var storage = new AuditStorageService();
            var player = new MainWindow([], new WorldSetupOptions(), new AuditEngineClientFactory(), storage);
            player.Opened -= typeof(MainWindow).GetMethod("Window_Opened", BindingFlags.Instance | BindingFlags.NonPublic)!
                .CreateDelegate<EventHandler>(player);
            var rgb = new RgbMemoryMonitorWindow(focus, RgbMemoryMonitorSettings.Default, _ => { });
            var phylogeny = new PhylogenyWindow(snapshot, focus);
            try
            {
                player.Show();
                var eye = OpenEyeDesigner(player);
                rgb.SetSnapshot(snapshot);
                rgb.Show();
                phylogeny.Show();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                AssertRendered(eye);
                AssertRendered(rgb);
                AssertRendered(phylogeny);

                var fields = eye.GetVisualDescendants().OfType<TextBox>().ToArray();
                Assert.Equal(18, fields.Length);
                Assert.All(fields, field => Assert.Equal("0", field.Text));
                Assert.Single(fields, field => field.Name == "Eye1Direction").Text = "17";
                Assert.Single(fields, field => field.Name == "Eye1Width").Text = "-45";
                await eye.WriteDnaAsync();
                Assert.Equal(EyeFile(firstDirection: 17, firstWidth: -45), storage.SavedDna);
                Assert.Null(storage.SavedDnaMutationProfile);

                DesktopTestApplication.UseSession(player, simulation);
                Assert.IsType<MainWindowViewModel>(player.DataContext).Select(focus);
                Assert.Same(eye, OpenEyeDesigner(player));
                await eye.WriteDnaAsync();
                Assert.Equal(EyeFile(firstDirection: 12, firstWidth: 34), storage.SavedDna);
                Assert.Null(storage.SavedDnaMutationProfile);
                Assert.Empty(engine.DnaReplacements);
            }
            finally
            {
                player.Close();
                if (rgb.IsVisible) rgb.Close();
                if (phylogeny.IsVisible) phylogeny.Close();
            }

            return 0;
        }, CancellationToken.None);
    }

    private static EyeDesignerWindow OpenEyeDesigner(MainWindow player)
    {
        var command = player.FindControl<LegacyMenuBar>("HistoricMenuBar")!.FindCommandItem(LegacyCommandId.EditEyeDesigner)!;
        Assert.True(command.IsEnabled);
        command.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
        return Assert.Single(player.OwnedWindows.OfType<EyeDesignerWindow>());
    }

    private static string EyeFile(int firstDirection, int firstWidth) => "Cond\r\n*.robage 0 =\r\nStart\r\n"
        + $"{firstDirection} .eye1dir store\r\n{firstWidth} .eye1width store\r\n'\r\n"
        + string.Concat(Enumerable.Range(2, 8).Select(eye => $"0 .eye{eye}dir store\r\n0 .eye{eye}width store\r\n'\r\n"))
        + "Stop\r\n";

    private static void AssertRendered(Window window)
    {
        using var frame = window.CaptureRenderedFrame();
        Assert.NotNull(frame);
        Assert.True(frame.PixelSize.Width >= 400, $"Rendered width was {frame.PixelSize.Width}.");
        Assert.True(frame.PixelSize.Height >= 300, $"Rendered height was {frame.PixelSize.Height}.");
    }
}
