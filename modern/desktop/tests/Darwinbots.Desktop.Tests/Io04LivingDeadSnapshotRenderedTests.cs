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

public sealed class Io04LivingDeadSnapshotRenderedTests
{
    [Fact]
    public async Task PlayerInspectorRendersLivingAndDeadSnapshotsWithStrictSelectionBoundaries()
    {
        var live = new OrganismSnapshot(
            3,
            2,
            [80f, 60f],
            [1.25f, -2.5f],
            777,
            14,
            Species: 1)
        {
            UniqueId = 31415,
            SimulationId = 2718,
            Memory = [new MemoryCellSnapshot(310, 777), new MemoryCellSnapshot(311, 42)],
            Mass = 101,
            Radius = 12.5f,
            DnaLength = 2,
            GeneCount = 2,
        };
        var corpse = new CorpseSnapshot([420f, 320f], [0f, 0f], 111, 222, 5, 1)
        {
            Dna = "DEAD-ONLY-DNA",
            UniqueId = 99001,
            Generation = 4,
            DnaLength = 1,
        };
        var snapshot = new EngineSnapshot(42, 1, "CPU", [live])
        {
            WorldSize = [640f, 480f],
            Species =
            [
                new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false),
                new SpeciesSnapshot("Living Audit Bot", false, 0xff239ac0, 0, false),
            ],
            RenderInstances = [new RenderInstanceSnapshot(3, [80f, 60f], 12.5f, 0xff239ac0)],
            Corpses = [corpse],
        };
        var engine = new RecordingEngineClient(snapshot);
        await using var session = new SimulationSession(engine);
        await session.StepAsync(0);

        Assert.Single(session.LatestSnapshot.Organisms);
        Assert.Single(session.LatestSnapshot.Corpses);
        Assert.Equal(live.UniqueId, session.LatestSnapshot.Organisms.Single().UniqueId);
        Assert.Equal(corpse.UniqueId, session.LatestSnapshot.Corpses.Single().UniqueId);

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions());
            DesktopTestApplication.UseSession(window, session);
            var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
            var viewport = window.FindControl<WorldViewport>("Viewport")!;
            viewModel.Update(session.LatestSnapshot);
            viewport.SetSnapshot(session.LatestSnapshot);
            viewport.SelectKey(new OrganismKey(live.Slot, live.Generation));
            Invoke(window, "RefreshLegacyMenuStates");

            try
            {
                window.WindowState = WindowState.Normal;
                window.Width = 1280;
                window.Height = 900;
                window.Show();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                Assert.True(viewport.Bounds.Width >= 500, $"Player width was {viewport.Bounds.Width}.");
                Assert.True(viewport.Bounds.Height >= 300, $"Player height was {viewport.Bounds.Height}.");
                using var playerFrame = window.CaptureRenderedFrame();
                Assert.NotNull(playerFrame);
                Assert.True(playerFrame.PixelSize.Width >= 500);
                Assert.True(playerFrame.PixelSize.Height >= 300);

                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                menu.FindCommandItem(LegacyCommandId.RobotShowInfo)!
                    .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<RobotDetailsWindow>().Any());

                var inspector = Assert.Single(window.OwnedWindows.OfType<RobotDetailsWindow>());
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                Assert.True(inspector.Bounds.Width >= 900, $"Inspector width was {inspector.Bounds.Width}.");
                Assert.True(inspector.Bounds.Height >= 560, $"Inspector height was {inspector.Bounds.Height}.");
                using var inspectorFrame = inspector.CaptureRenderedFrame();
                Assert.NotNull(inspectorFrame);
                Assert.True(inspectorFrame.PixelSize.Width >= 900);
                Assert.True(inspectorFrame.PixelSize.Height >= 560);

                inspector.SelectTab(RobotDetailsTab.Memory);
                inspector.UpdateLayout();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                var inspectorText = string.Join(
                    "\n",
                    inspector.GetVisualDescendants().OfType<TextBlock>().Select(block => block.Text)
                        .Concat(inspector.GetVisualDescendants().OfType<TextBox>().Select(box => box.Text)));
                Assert.Contains("LIVE ROBOT 3:2", inspectorText, StringComparison.Ordinal);
                Assert.Contains("ARRAY ID       3", inspectorText, StringComparison.Ordinal);
                Assert.Contains("UNIQUE ID      31,415", inspectorText, StringComparison.Ordinal);
                Assert.Contains("ENERGY         777", inspectorText, StringComparison.Ordinal);
                Assert.Contains("310         777", inspectorText, StringComparison.Ordinal);
                Assert.DoesNotContain("99,001", inspectorText, StringComparison.Ordinal);
                Assert.DoesNotContain("DEAD-ONLY-DNA", inspectorText, StringComparison.Ordinal);

                inspector.Close();
                viewModel.ClearSelection();
                viewport.SelectKey(null);
                Invoke(window, "RefreshLegacyMenuStates");
                var noSelection = menu.FindCommandItem(LegacyCommandId.RobotShowInfo)!;
                Assert.False(noSelection.IsEnabled);
                noSelection.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                Assert.Empty(window.OwnedWindows.OfType<RobotDetailsWindow>());

                viewModel.Select(new OrganismKey(999, 1));
                viewport.SelectKey(new OrganismKey(999, 1));
                Invoke(window, "RefreshLegacyMenuStates");
                var invalidSelection = menu.FindCommandItem(LegacyCommandId.RobotShowInfo)!;
                Assert.False(invalidSelection.IsEnabled);
                invalidSelection.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                Assert.Empty(window.OwnedWindows.OfType<RobotDetailsWindow>());
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                if (window.IsVisible) window.Close();
            }
        }, CancellationToken.None);
    }

    private static T Field<T>(object target, string name) =>
        (T)target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(target)!;

    private static void SetField(object target, string name, object? value) =>
        target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)!.SetValue(target, value);

    private static void Invoke(object target, string method) =>
        target.GetType().GetMethod(method, BindingFlags.Instance | BindingFlags.NonPublic)!
            .Invoke(target, null);

    private static async Task WaitFor(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(5);
        while (!condition() && DateTime.UtcNow < deadline) await Task.Delay(10);
        Assert.True(condition(), "Timed out waiting for the live inspector route.");
    }
}
