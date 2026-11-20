using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class SelectedBotOverlayRenderedTests
{
    [Fact]
    public async Task PopulatedWorldDrawsGaugesOnlyForTheSelectedStableIdentity()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var bots = Enumerable.Range(0, 2132).Select(index => new OrganismSnapshot(
                (uint)index, 0, [20 + index % 50 * 12, 25 + index / 50 * 9], [0, 0],
                875, 0, Body: 321, Shell: 3, Slime: 4, Venom: 5, Poison: 6)).ToArray();
            var snapshot = new EngineSnapshot(108, bots.Length, "CPU", bots)
            { WorldSize = [640, 480] };
            var viewport = new WorldViewport
            {
                ShowMovementVectors = false, ShowSelectedVision = false,
                ShowResourceGauges = true, ShowDeathPoffs = false,
            };
            viewport.SetSnapshot(snapshot);
            var host = new Window { Width = 640, Height = 480, Content = viewport };
            host.Show();
            try
            {
                Assert.Empty(viewport.OverlayProjection.Gauges);
                var unselected = Capture(host);
                viewport.ShowResourceGauges = false;
                Assert.Equal(unselected, Capture(host));

                viewport.SelectKey(new OrganismKey(817, 0));
                viewport.ShowResourceGauges = true;
                Assert.Equal(3, viewport.OverlayProjection.Gauges.Count);
                Assert.All(viewport.OverlayProjection.Gauges,
                    gauge => Assert.Equal(new OrganismKey(817, 0), gauge.Organism));
                var selected = Capture(host);
                viewport.ShowResourceGauges = false;
                Assert.False(selected.SequenceEqual(Capture(host)), "The selected bot must retain its resource overlay.");

                viewport.ShowResourceGauges = true;
                bots[817] = bots[817] with { Generation = 1 };
                viewport.SetSnapshot(snapshot);
                Assert.Empty(viewport.OverlayProjection.Gauges);
                viewport.SelectKey(new OrganismKey(817, 1));
                Assert.Equal(3, viewport.OverlayProjection.Gauges.Count);
                Assert.All(viewport.OverlayProjection.Gauges,
                    gauge => Assert.Equal(new OrganismKey(817, 1), gauge.Organism));
                viewport.SelectKey(null);
                Assert.Empty(viewport.OverlayProjection.Gauges);
            }
            finally { host.Close(); }
        }, CancellationToken.None);
    }

    [Theory]
    [InlineData(0, 0)]
    [InlineData(1, 0)]
    [InlineData(2, 0)]
    [InlineData(3, 0)]
    [InlineData(0, 4)]
    [InlineData(1, 4)]
    [InlineData(2, 4)]
    [InlineData(3, 4)]
    public async Task CullingRetainsBodiesIntersectingEachEdgeAfterZoomAndPan(int edge, int zoomSteps)
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var viewport = new WorldViewport
            {
                ShowSelectedVision = false, ShowMovementVectors = false,
                ShowResourceGauges = false, ShowDeathPoffs = false, RenderSkins = false,
            };
            EngineSnapshot Scene(float[] position) => new(108, 1, "CPU",
                [new OrganismSnapshot(7, 0, position, [0, 0], 875, 0) { Radius = 40 }])
                { WorldSize = [480, 360], RenderInstances = [new RenderInstanceSnapshot(7, position, 40, 0xffed25b7)] };
            viewport.SetSnapshot(Scene([240, 180]));
            var host = new Window { Width = 480, Height = 360, Content = viewport };
            host.Show();
            try
            {
                for (var step = 0; step < zoomSteps; step++) viewport.ZoomIn();
                var width = viewport.Bounds.Width;
                var height = viewport.Bounds.Height;
                float[] AtScreen(double x, double y) =>
                    [(float)((x - viewport.Pan.X) / (width / 480 * viewport.Zoom)),
                     (float)((y - viewport.Pan.Y) / (height / 360 * viewport.Zoom))];
                viewport.SetSnapshot(Scene(AtScreen(-100, -100)));
                var outside = Capture(host);
                var center = edge switch
                {
                    0 => new Point(-20, height / 2),
                    1 => new Point(width + 20, height / 2),
                    2 => new Point(width / 2, -20),
                    _ => new Point(width / 2, height + 20),
                };
                viewport.SetSnapshot(Scene(AtScreen(center.X, center.Y)));
                Assert.False(outside.SequenceEqual(Capture(host)),
                    "A body intersecting the viewport edge must remain visible even when its center is outside.");
            }
            finally { host.Close(); }
        }, CancellationToken.None);
    }

    private static byte[] Capture(Window window)
    {
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        using var frame = window.CaptureRenderedFrame();
        Assert.NotNull(frame);
        using var stream = new MemoryStream();
        frame.Save(stream);
        return stream.ToArray();
    }
}
