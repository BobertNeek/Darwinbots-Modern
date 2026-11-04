using System.Diagnostics;
using System.Reflection;
using System.Text.Json;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class ViewportRenderProfileTests
{
    [ViewportProfileFact]
    public async Task MeasuresPopulatedZoomedViewportWithoutAdvancingSimulation()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var input = Environment.GetEnvironmentVariable("DARWINBOTS_VIEWPORT_PROFILE_SNAPSHOT");
            Assert.False(string.IsNullOrEmpty(input));
            var snapshot = JsonSerializer.Deserialize<EngineSnapshot>(File.ReadAllText(input))!;
            Assert.Equal(2132, snapshot.Population);
            var selected = snapshot.Organisms.First(bot => bot.Slot == 817);
            var viewport = new WorldViewport { ShowSelectedVision = false, Width = 1280, Height = 720 };
            viewport.SetSnapshot(snapshot);
            var host = new Window { Width = 1280, Height = 720, Content = viewport };
            host.Show();
            try
            {
                viewport.SelectKey(new OrganismKey(selected.Slot, selected.Generation));
                typeof(WorldViewport).GetMethod("SetZoomAt", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .Invoke(viewport, [9.32, new Point(viewport.Bounds.Width / 2, viewport.Bounds.Height / 2)]);
                viewport.FollowKey(new OrganismKey(selected.Slot, selected.Generation));
                var samples = new List<(double Apply, double Render, long Allocated)>();
                for (var frameIndex = 0; frameIndex < 10; frameIndex++)
                {
                    var allocated = GC.GetAllocatedBytesForCurrentThread();
                    var clock = Stopwatch.StartNew();
                    viewport.SetSnapshot(snapshot);
                    var apply = clock.Elapsed.TotalMilliseconds;
                    clock.Restart();
                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    using var frame = host.CaptureRenderedFrame();
                    Assert.NotNull(frame);
                    var render = clock.Elapsed.TotalMilliseconds;
                    if (frameIndex >= 3) samples.Add((apply, render, GC.GetAllocatedBytesForCurrentThread() - allocated));
                }
                static double Median(IEnumerable<double> values) => values.Order().ElementAt(3);
                var result = new
                {
                    scope = "CPU headless rendering of a recreated populated fixture, not simulation throughput or original-game speed",
                    snapshot.Tick, snapshot.Population, zoom = viewport.Zoom,
                    viewport_width = viewport.Bounds.Width, viewport_height = viewport.Bounds.Height,
                    gauges = viewport.OverlayProjection.Gauges.Count, samples = samples.Count,
                    ui_apply_median_ms = Median(samples.Select(s => s.Apply)),
                    render_capture_median_ms = Median(samples.Select(s => s.Render)),
                    ui_thread_allocated_median_bytes = Median(samples.Select(s => (double)s.Allocated)),
                };
                var output = Environment.GetEnvironmentVariable("DARWINBOTS_VIEWPORT_PROFILE_RESULT")!;
                File.WriteAllText(output, JsonSerializer.Serialize(result, new JsonSerializerOptions { WriteIndented = true }));
            }
            finally { host.Close(); }
        }, CancellationToken.None);
    }
}

public sealed class ViewportProfileFactAttribute : FactAttribute
{
    public ViewportProfileFactAttribute()
    {
        if (string.IsNullOrEmpty(Environment.GetEnvironmentVariable("DARWINBOTS_VIEWPORT_PROFILE_SNAPSHOT")))
            Skip = "Manual rendering profile: set DARWINBOTS_VIEWPORT_PROFILE_SNAPSHOT and DARWINBOTS_VIEWPORT_PROFILE_RESULT.";
    }
}
