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

public sealed class NativeTeleporterHighlightsRenderedTests
{
    [Fact]
    public async Task NativeModeRingsUseDirectionColorsAndAnIndependentFocusRing()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var native = EngineSnapshot.Empty with
            {
                NativeCycle = 1,
                WorldSize = [600, 360],
                Teleporters =
                [
                    new TeleporterSnapshot(1, [110, 140], 35, [0, 0]) { Width = 70, Height = 60, Outbound = true },
                    new TeleporterSnapshot(2, [250, 140], 35, [0, 0]) { Width = 70, Height = 60, Local = false },
                    new TeleporterSnapshot(3, [390, 140], 35, [0, 0]) { Width = 70, Height = 60, Local = false, Outbound = true },
                ],
            };
            var viewport = new WorldViewport { OldColors = true, ToroidalWorld = false };
            viewport.SetSnapshot(native);
            viewport.SelectFeature(new WorldFeatureSelection(WorldFeatureKind.Teleporter, 1));
            var window = new Window { Width = 600, Height = 360, Content = viewport };
            try
            {
                window.Show();
                Assert.True(viewport.HighlightTeleporters);
                var nativePixels = Capture(window);
                Assert.True(HasPaint(nativePixels, 152, 130, IsYellow), "Missing yellow local mark.");
                Assert.True(HasPaint(nativePixels, 292, 130, IsGreen), "Missing green inbound mark.");
                Assert.True(HasPaint(nativePixels, 432, 130, IsRed), "Missing red outbound mark.");
                Assert.True(HasPaint(nativePixels, 159, 130, IsWhite), "Missing larger white focus mark.");

                viewport.HighlightTeleporters = false;
                var disabledPixels = Capture(window);
                Assert.False(HasPaint(disabledPixels, 152, 130, IsYellow));
                Assert.False(HasPaint(disabledPixels, 292, 130, IsGreen));
                Assert.False(HasPaint(disabledPixels, 432, 130, IsRed));
                Assert.True(HasPaint(disabledPixels, 159, 130, IsWhite));

                viewport.SetSnapshot(native with { NativeCycle = null });
                Assert.True(viewport.HighlightTeleporters);
                var portablePixels = Capture(window);
                Assert.True(HasPaint(portablePixels, 145, 140, IsOrange), "Portable highlight changed.");
                Assert.False(HasPaint(portablePixels, 159, 130, IsWhite));
            }
            finally { window.Close(); }
        }, CancellationToken.None);
    }

    private static (byte[] Pixels, int Stride) Capture(Window window)
    {
        window.UpdateLayout();
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        using var frame = Assert.IsAssignableFrom<Bitmap>(window.CaptureRenderedFrame());
        using var rgba = new WriteableBitmap(frame.PixelSize, frame.Dpi, PixelFormat.Rgba8888, AlphaFormat.Unpremul);
        using var locked = rgba.Lock();
        frame.CopyPixels(locked);
        var pixels = new byte[locked.RowBytes * frame.PixelSize.Height];
        Marshal.Copy(locked.Address, pixels, 0, pixels.Length);
        return (pixels, locked.RowBytes);
    }

    private static bool HasPaint((byte[] Pixels, int Stride) frame, int x, int y, Func<byte, byte, byte, bool> matches)
    {
        for (var row = y - 5; row <= y + 5; row++)
        for (var column = x - 2; column <= x + 2; column++)
        {
            var index = row * frame.Stride + column * 4;
            if (matches(frame.Pixels[index], frame.Pixels[index + 1], frame.Pixels[index + 2]))
                return true;
        }
        return false;
    }

    private static bool IsYellow(byte r, byte g, byte b) => r >= 105 && g >= 105 && b < 65;
    private static bool IsGreen(byte r, byte g, byte b) => r < 70 && g >= 105 && b < 65;
    private static bool IsRed(byte r, byte g, byte b) => r >= 105 && g < 70 && b < 65;
    private static bool IsWhite(byte r, byte g, byte b) => r >= 105 && g >= 105 && b >= 130 && Math.Abs(r - g) < 10;
    private static bool IsOrange(byte r, byte g, byte b) => r >= 185 && g >= 80 && g < 160 && b < 90;
}
