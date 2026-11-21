using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Input.Raw;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class RobotDetailsDescendantsTests
{
    [Fact]
    public async Task NativeDescendantsCountsTheCurrentFocusOnDemand()
    {
        var snapshot = NativeSnapshotParser.Parse("""
            {
              "native_cycle": 12,
              "organisms": [
                { "id": {"slot": 7, "generation": 3}, "position": [100, 100], "velocity": [0, 0],
                  "unique_id": 70, "parent_absolute_id": 0, "offspring": 12, "descendants": 99 },
                { "id": {"slot": 8, "generation": 2}, "position": [200, 100], "velocity": [0, 0],
                  "unique_id": 80, "parent_absolute_id": 70, "flags": {"exists": true, "corpse": true} },
                { "id": {"slot": 9, "generation": 0}, "position": [300, 100], "velocity": [0, 0],
                  "unique_id": 90, "parent_absolute_id": 80 },
                { "id": {"slot": 10, "generation": 0}, "position": [100, 200], "velocity": [0, 0],
                  "unique_id": 100, "parent_absolute_id": 70, "flags": {"exists": false} },
                { "id": {"slot": 11, "generation": 0}, "position": [200, 200], "velocity": [0, 0],
                  "unique_id": 110, "parent_absolute_id": 100 },
                { "id": {"slot": 12, "generation": 0}, "position": [300, 200], "velocity": [0, 0],
                  "unique_id": 120, "parent_absolute_id": 90, "parents": [null, {"slot": 7, "generation": 3}] }
              ]
            }
            """, "CPU");
        var selectedAncestor = new OrganismKey(7, 3);
        var corpseKey = new OrganismKey(8, 2);
        // A new robot reuses the corpse's slot. Its absolute identity cannot bridge to the orphan.
        var replacement = snapshot.Organisms[1] with { Generation = 3, UniqueId = 81, ParentAbsoluteId = 0, IsCorpse = false };
        var afterRemoval = snapshot with
        {
            Organisms = [snapshot.Organisms[0], replacement, .. snapshot.Organisms.Skip(2)],
        };
        Assert.Null(PhylogenyModel.CountNativeDescendants(afterRemoval, corpseKey));
        Assert.Equal(0, PhylogenyModel.CountNativeDescendants(afterRemoval, selectedAncestor));

        var engine = new RecordingEngineClient(snapshot);
        await using var session = new SimulationSession(engine);
        await session.RefreshSnapshotAsync();
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            OrganismKey? focus = selectedAncestor;
            var window = new RobotDetailsWindow(session, selectedAncestor, readFocus: () => focus);
            try
            {
                window.Show();
                window.UpdateLayout();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                var labels = window.GetVisualDescendants().OfType<TextBlock>().ToArray();
                var button = Assert.Single(window.GetVisualDescendants().OfType<Button>(),
                    control => control.Name == "NativeRobotInfoDescendants");
                var count = Assert.Single(labels, label => label.Name == "NativeRobotInfoDescendantCount");
                Assert.True(button.IsVisible);
                Assert.Equal("Descendants", button.Content);
                Assert.Equal("------", count.Text);
                Assert.Contains(labels, label => label.Text?.Contains("OFFSPRING      12") == true);
                Assert.DoesNotContain(labels, label => label.Text?.Contains("DESCENDANTS    99") == true);
                var center = button.TranslatePoint(new Point(button.Bounds.Width / 2, button.Bounds.Height / 2), window);
                Assert.NotNull(center);
                var startTick = session.LatestSnapshot.Tick;
                ClickDescendants();
                Assert.Equal("3", count.Text);

                focus = corpseKey;
                Assert.EndsWith("7:3", window.Title);
                Assert.Equal("3", count.Text);
                ClickDescendants();
                Assert.Equal("2", count.Text);

                focus = null;
                ClickDescendants();
                Assert.Equal("------", count.Text);
                Assert.Equal(startTick, session.LatestSnapshot.Tick);
                Assert.Empty(engine.Calls);

                void ClickDescendants()
                {
                    window.MouseDown(center.Value, MouseButton.Left, RawInputModifiers.None);
                    window.MouseUp(center.Value, MouseButton.Left, RawInputModifiers.None);
                }
            }
            finally { window.Close(); }
        }, CancellationToken.None);
    }

    [Fact]
    public void NativeCountStopsEnteringBranchesAfterTheSharedLimitInSlotOrder()
    {
        var ancestor = RecordingEngineClient.Organism with { Slot = 0, UniqueId = 1, ParentAbsoluteId = 0 };
        var chain = Enumerable.Range(1, 1_000).Select(slot => ancestor with
        {
            Slot = (uint)slot,
            UniqueId = (ulong)slot + 1,
            ParentAbsoluteId = (ulong)slot,
        });
        var sibling = ancestor with { Slot = 1_001, UniqueId = 1_002, ParentAbsoluteId = 1 };
        var siblingChild = ancestor with { Slot = 1_002, UniqueId = 1_003, ParentAbsoluteId = 1_002 };
        var snapshot = new EngineSnapshot(0, 1_003, "CPU", [ancestor, siblingChild, sibling, .. chain.Reverse()])
        {
            NativeCycle = 0,
        };

        // The first child chain reaches the bound. The root counts its later direct child,
        // but the later child's scan cannot start.
        Assert.Equal(1_001, PhylogenyModel.CountNativeDescendants(snapshot, new(ancestor.Slot, ancestor.Generation)));
    }

    [Fact]
    public async Task PortableRobotDetailsKeepTheLifetimeDescendantCounter()
    {
        var root = RecordingEngineClient.Organism with { Descendants = 37 };
        var snapshot = RecordingEngineClient.CreateSnapshot() with { Organisms = [root] };
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var window = new RobotDetailsWindow(snapshot, new(root.Slot, root.Generation));
            try
            {
                window.Show();
                window.UpdateLayout();
                Assert.Contains(window.GetVisualDescendants().OfType<TextBlock>(),
                    label => label.Text?.Contains("DESCENDANTS    37") == true);
                Assert.DoesNotContain(window.GetVisualDescendants().OfType<Button>(),
                    button => button.Name == "NativeRobotInfoDescendants" && button.IsEffectivelyVisible);
            }
            finally { window.Close(); }
        }, CancellationToken.None);
    }
}
