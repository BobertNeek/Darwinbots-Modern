using System.Runtime.InteropServices;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Media.Imaging;
using Avalonia.Platform;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class RgbMonitorPixelTests
{
    [Theory]
    [InlineData(true)]
    [InlineData(false)]
    public async Task MonitorDrawsAThinSquareAroundVisibleBodiesAndDisablingItRestoresTheFrame(bool useRenderInstances)
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            using var engine = new NativeEngineClient(new WorldSetupOptions
            {
                Backend = "Cpu", PopulationCapacity = 32, WorldWidth = 640, WorldHeight = 480,
            });
            engine.ImportDna("start\nstop", [200, 200]);
            engine.ImportDna("start\nstop", [460, 330]);
            var robots = engine.Snapshot().Organisms.OrderBy(robot => robot.Position[0]).ToArray();
            Assert.Equal(2, robots.Length);
            var visible = robots[0];
            var hidden = robots[1];
            engine.SetOrganismMemory(visible.Slot, visible.Generation, 77, 100);
            engine.SetOrganismMemory(hidden.Slot, hidden.Generation, 77, 100);
            var snapshot = engine.Snapshot();
            Assert.Equal("CPU", snapshot.Backend.ToUpperInvariant());
            var instance = Assert.Single(snapshot.RenderInstances, item => item.Slot == visible.Slot);
            Assert.NotEqual(0xffff0000u, useRenderInstances ? instance.Color : visible.Phenotype.Color);
            snapshot = snapshot with
            {
                Organisms = snapshot.Organisms.Select(robot => robot.Slot == hidden.Slot ? robot with { IsHidden = true } : robot).ToArray(),
                RenderInstances = useRenderInstances ? snapshot.RenderInstances : [],
                // This old color-absent packet still needs its owner's normal color.
                Shots = [new ShotSnapshot(new OrganismKey(visible.Slot, visible.Generation),
                    [330, 140], [390, 140], [0, 0], 42, 0, 1, 100, 10, false)],
            };

            var viewport = new WorldViewport
            {
                RenderSkins = false, ShowSelectedVision = false, ShowMovementVectors = false,
                ShowResourceGauges = false, ShowDeathPoffs = false,
            };
            viewport.SetSnapshot(snapshot);
            var window = new Window { Width = 640, Height = 480, Content = viewport };
            try
            {
                window.Show();
                window.UpdateLayout();
                var baseline = ReadPixels(window);
                viewport.SetRgbMemoryMonitor(new(new(77, 0, 100), new(78, 0, 100), new(79, 0, 100)));
                var monitored = ReadPixels(window);

                var scaleX = viewport.Bounds.Width / snapshot.WorldSize[0];
                var scaleY = viewport.Bounds.Height / snapshot.WorldSize[1];
                var logicalCenter = viewport.TranslatePoint(new Point(visible.Position[0] * scaleX, visible.Position[1] * scaleY), window)
                    ?? throw new InvalidOperationException("The viewport is not attached to the test window.");
                var radius = useRenderInstances ? Math.Clamp(instance.Radius * Math.Min(scaleX, scaleY), 1.5, 28) : 3.2;
                var outputScaleX = monitored.Size.Width / window.Bounds.Width;
                var outputScaleY = monitored.Size.Height / window.Bounds.Height;
                var center = new Point(logicalCenter.X * outputScaleX, logicalCenter.Y * outputScaleY);
                var halfWidth = radius * 1.1 * outputScaleX;
                var halfHeight = radius * 1.1 * outputScaleY;
                var changes = new List<(int X, int Y)>();
                for (var y = 0; y < monitored.Size.Height; y++)
                for (var x = 0; x < monitored.Size.Width; x++)
                {
                    var offset = y * monitored.Stride + x * 4;
                    var pixel = monitored.Bytes.AsSpan(offset, 4);
                    if (pixel.SequenceEqual(baseline.Bytes.AsSpan(offset, 4))) continue;
                    Assert.True(pixel.SequenceEqual(new byte[] { 255, 0, 0, 255 }), $"Unexpected outline color at ({x}, {y}).");
                    var dx = Math.Abs(x + 0.5 - center.X);
                    var dy = Math.Abs(y + 0.5 - center.Y);
                    Assert.True(dx <= halfWidth + 1 && dy <= halfHeight + 1
                        && (Math.Abs(dx - halfWidth) <= 1 || Math.Abs(dy - halfHeight) <= 1),
                        $"The monitor changed a pixel outside the expected outline at ({x}, {y}).");
                    changes.Add((x, y));
                }
                Assert.Equal(2, changes.Count(point => point.Y == (int)center.Y));
                Assert.Equal(2, changes.Count(point => point.X == (int)center.X));
                Assert.Contains(changes, point => Math.Abs(point.X + 0.5 - (center.X + halfWidth)) <= 1
                    && Math.Abs(point.Y + 0.5 - (center.Y + halfHeight)) <= 1);

                viewport.SetRgbMemoryMonitor(null);
                Assert.Equal(baseline.Bytes, ReadPixels(window).Bytes);
            }
            finally { window.Close(); }
        }, CancellationToken.None);
    }

    private static (PixelSize Size, int Stride, byte[] Bytes) ReadPixels(Window window)
    {
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        using var frame = Assert.IsAssignableFrom<Bitmap>(window.CaptureRenderedFrame());
        using var rgba = new WriteableBitmap(frame.PixelSize, frame.Dpi, PixelFormat.Rgba8888, AlphaFormat.Unpremul);
        using var locked = rgba.Lock();
        frame.CopyPixels(locked);
        var bytes = new byte[locked.RowBytes * frame.PixelSize.Height];
        Marshal.Copy(locked.Address, bytes, 0, bytes.Length);
        return (frame.PixelSize, locked.RowBytes, bytes);
    }
}
