using System.Runtime.InteropServices;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Media.Imaging;
using Avalonia.Platform;
using System.Text.Json;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class WorldShotRenderedTests
{
    [Fact]
    public async Task UnknownShotUsesOwnerColourAndWritesRenderedReceipt()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var ownerColour = 0xff13d7a5u;
            var owner = new OrganismSnapshot(
                7,
                3,
                [10f, 10f],
                [0f, 0f],
                875,
                19)
            {
                Phenotype = new VisualPhenotypeSnapshot(
                    7,
                    ownerColour,
                    [new SkinPointSnapshot(0.45f, 0), new SkinPointSnapshot(0.55f, 314)],
                    0),
            };
            var snapshot = new EngineSnapshot(12, 1, "CPU", [owner])
            {
                WorldSize = [100f, 100f],
                Shots =
                [
                    new ShotSnapshot(
                        new OrganismKey(owner.Slot, owner.Generation),
                        [36f, 61f],
                        [64f, 61f],
                        [0f, 0f],
                        42,
                        10,
                        0,
                        100,
                        10f,
                        false),
                ],
            };
            var viewport = new WorldViewport
            {
                ShowSelectedVision = false,
                ShowMovementVectors = false,
                ShowResourceGauges = false,
                RenderSkins = false,
            };
            viewport.SetSnapshot(snapshot);
            var window = new Window
            {
                WindowState = WindowState.Normal,
                Width = 640,
                Height = 480,
                Content = viewport,
            };
            try
            {
                window.Show();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var frame = Assert.IsAssignableFrom<Bitmap>(window.CaptureRenderedFrame());
                Assert.True(
                    ContainsColour(frame, new PixelRect(210, 280, 220, 30), 19, 215, 165),
                    "The unknown shot did not render with its owner's colour.");

                var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR")
                    ?? Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
                Directory.CreateDirectory(directory);
                using var stream = File.Create(Path.Combine(directory, "world-shot-owner-colour.png"));
                frame.Save(stream, new PngBitmapEncoderOptions());
            }
            finally
            {
                if (window.IsVisible) window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task ExistingDeathParticlesUseArgbPointsWhileCreationIsDisabledAndCorpsesRemainVisible()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var owner = new OrganismSnapshot(7, 3, [10, 10], [0, 0], 875, 19)
            {
                Phenotype = new VisualPhenotypeSnapshot(7, 0xff13d7a5, [], 0),
            };
            // The native loader has already converted original COLORREF red255,
            // blue16711680 and black0 to opaque ARGB. Public present0 is also black.
            uint[] colours = [0xffff0000, 0xff0000ff, 0xff000000, 0, 0xffffffff];
            var payload = JsonSerializer.Serialize(new
            {
                tick = 12, organisms = Array.Empty<object>(), world_size = new[] { 100f, 100f },
                death_poffs_enabled = false,
                shots = colours.Select((colour, index) => new
                {
                    owner = new { slot = 7, generation = 3 },
                    start = index == 4 ? new[] { 40f, 60f } : new[] { 10f, 70f },
                    end = index == 4 ? new[] { 60f, 60f } : new[] { 30f + index * 20f, 30f }, velocity = new[] { 0f, 0f },
                    kind = index == 4 ? -2 : -100, value = 0, age = 1, range = 10, energy = 0f, impact_flash = false, color = colour,
                }).ToArray(),
            });
            var snapshot = NativeSnapshotParser.Parse(payload, "CPU") with
            {
                Population = 1, Organisms = [owner],
                Corpses = [new CorpseSnapshot([80, 75], [0, 0], 10, 100, 2)],
            };
            Assert.False(snapshot.DeathPoffsEnabled);
            Assert.Equal(colours.Select(value => (uint?)value), snapshot.Shots.Select(shot => shot.Color));
            var viewport = new WorldViewport
            {
                ShowSelectedVision = false, ShowMovementVectors = false,
                ShowResourceGauges = false, RenderSkins = false,
            };
            viewport.SetSnapshot(snapshot);
            Assert.True(viewport.ShowDeathPoffs);
            var window = new Window { Width = 640, Height = 480, Content = viewport };
            try
            {
                window.Show();
                window.UpdateLayout();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var frame = Assert.IsAssignableFrom<Bitmap>(window.CaptureRenderedFrame());
                using var rgba = new WriteableBitmap(frame.PixelSize, frame.Dpi, PixelFormat.Rgba8888, AlphaFormat.Unpremul);
                byte[] pixels;
                int stride;
                using (var locked = rgba.Lock())
                {
                    // This documented overload converts channel order; accepting both
                    // orders here would let a red/blue rendering regression pass.
                    frame.CopyPixels(locked);
                    stride = locked.RowBytes;
                    pixels = new byte[stride * frame.PixelSize.Height];
                    Marshal.Copy(locked.Address, pixels, 0, pixels.Length);
                }
                bool HasColourAt(float x, float y, byte red, byte green, byte blue, int tolerance = 3)
                {
                    var point = viewport.TranslatePoint(new Point(viewport.Bounds.Width * x / 100, viewport.Bounds.Height * y / 100), window)
                        ?? throw new InvalidOperationException("Particle viewport is not attached to its window.");
                    var px = (int)Math.Floor(point.X * frame.PixelSize.Width / window.Bounds.Width);
                    var py = (int)Math.Floor(point.Y * frame.PixelSize.Height / window.Bounds.Height);
                    for (var row = Math.Max(0, py - 3); row <= Math.Min(frame.PixelSize.Height - 1, py + 3); row++)
                    for (var column = Math.Max(0, px - 3); column <= Math.Min(frame.PixelSize.Width - 1, px + 3); column++)
                    {
                        var offset = row * stride + column * 4;
                        if (Math.Abs(pixels[offset] - red) <= tolerance && Math.Abs(pixels[offset + 1] - green) <= tolerance
                            && Math.Abs(pixels[offset + 2] - blue) <= tolerance && pixels[offset + 3] >= 250) return true;
                    }
                    return false;
                }
                Assert.True(HasColourAt(30, 30, 255, 0, 0), "Saved native red particle was missing or channel-swapped.");
                Assert.True(HasColourAt(50, 30, 0, 0, 255), "Saved native blue particle was missing or channel-swapped.");
                Assert.True(HasColourAt(70, 30, 0, 0, 0), "Opaque native black particle was hidden.");
                Assert.True(HasColourAt(90, 30, 0, 0, 0), "Present colour0 was treated as absent or transparent.");
                Assert.False(HasColourAt(20, 50, 255, 0, 0), "A death particle was rendered as a travel line instead of a point.");
                Assert.True(HasColourAt(50, 60, 255, 255, 255, tolerance: 10), "A saved white energy shot used the old green fallback.");
                Assert.True(HasColourAt(80, 75, 133, 137, 130), "Turning creation off hid an existing corpse.");
                var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR")
                    ?? Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
                Directory.CreateDirectory(directory);
                using var stream = File.Create(Path.Combine(directory, "world-death-particle-colours.png"));
                frame.Save(stream, new PngBitmapEncoderOptions());
            }
            finally { if (window.IsVisible) window.Close(); }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task NativeFlashPaletteUsesPreviousPositionAndDisablingImpactsStillDrawsLiveShots()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            int[] kinds = [-1, -2, -3, -4, -5, -6, -7, 42, -100];
            uint[] palette = [0xffff0000, 0xffffffff, 0xff0000ff, 0xff00ff00, 0xffffff00, 0xffff00ff, 0xff00ffff, 0xff000000, 0xff000000];
            var owner = new OrganismSnapshot(7, 3, [5, 95], [0, 0], 875, 19);
            var snapshot = new EngineSnapshot(12, 1, "CPU", [owner])
            {
                WorldSize = [100, 100], DeathPoffsEnabled = false,
                Shots = kinds.Select((kind, index) => new ShotSnapshot(new(7, 3),
                    [12f + index * 10, 20], [12f + index * 10, 70], [0, 0],
                    kind, 0, 1, 100, 10, true, 0xff000000)).ToArray(),
            };
            var viewport = new WorldViewport
            {
                ShowSelectedVision = false, ShowMovementVectors = false,
                ShowResourceGauges = false, RenderSkins = false, RenderWaste = true, ShowShotImpacts = true,
            };
            viewport.SetSnapshot(snapshot);
            var window = new Window { Width = 640, Height = 480, Content = viewport };
            try
            {
                window.Show();
                window.UpdateLayout();
                bool HasInkAt(Bitmap frame, byte[] pixels, int stride, float x, float y, uint expected, int margin)
                {
                    var point = viewport.TranslatePoint(new Point(viewport.Bounds.Width * x / 100, viewport.Bounds.Height * y / 100), window)
                        ?? throw new InvalidOperationException("Shot viewport is not attached to its window.");
                    var px = (int)Math.Floor(point.X * frame.PixelSize.Width / window.Bounds.Width);
                    var py = (int)Math.Floor(point.Y * frame.PixelSize.Height / window.Bounds.Height);
                    var red = (byte)(expected >> 16);
                    var green = (byte)(expected >> 8);
                    var blue = (byte)expected;
                    // Exact RGBA channel identity, allowing edge antialiasing only.
                    // Dark components cannot be substituted for the opposite primary.
                    static bool Channel(byte actual, byte desired) => desired == 0 ? actual <= 150 : actual >= 245;
                    for (var row = Math.Max(0, py - margin); row <= Math.Min(frame.PixelSize.Height - 1, py + margin); row++)
                    for (var column = Math.Max(0, px - margin); column <= Math.Min(frame.PixelSize.Width - 1, px + margin); column++)
                    {
                        var offset = row * stride + column * 4;
                        if (Channel(pixels[offset], red) && Channel(pixels[offset + 1], green)
                            && Channel(pixels[offset + 2], blue) && pixels[offset + 3] >= 250) return true;
                    }
                    return false;
                }
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var flashFrame = Assert.IsAssignableFrom<Bitmap>(window.CaptureRenderedFrame());
                var (flashPixels, flashStride) = ReadRgba(flashFrame);
                for (var index = 0; index < kinds.Length; index++)
                {
                    var x = 12f + index * 10;
                    Assert.True(HasInkAt(flashFrame, flashPixels, flashStride, x, 20, palette[index], 11), $"Flash kind{kinds[index]} did not use its typed palette at Start/opos.");
                    Assert.False(HasInkAt(flashFrame, flashPixels, flashStride, x, 70, 0xff000000, 3), $"Flash kind{kinds[index]} was drawn at End/pos.");
                }
                viewport.ShowShotImpacts = false;
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var normalFrame = Assert.IsAssignableFrom<Bitmap>(window.CaptureRenderedFrame());
                var (normalPixels, normalStride) = ReadRgba(normalFrame);
                for (var index = 0; index < kinds.Length; index++)
                    Assert.True(HasInkAt(normalFrame, normalPixels, normalStride, 12f + index * 10, 70, 0xff000000, 3),
                        $"Impacts OFF hid the still-existing kind{kinds[index]} shot instead of drawing its saved normal color.");
                Assert.Equal(9, snapshot.Shots.Count);
                var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR")
                    ?? Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
                Directory.CreateDirectory(directory);
                using (var stream = File.Create(Path.Combine(directory, "world-shot-flash-palette.png")))
                    flashFrame.Save(stream, new PngBitmapEncoderOptions());
                using (var stream = File.Create(Path.Combine(directory, "world-shot-impacts-off-retains-shots.png")))
                    normalFrame.Save(stream, new PngBitmapEncoderOptions());
            }
            finally { if (window.IsVisible) window.Close(); }
        }, CancellationToken.None);
    }

    private static (byte[] Pixels, int Stride) ReadRgba(Bitmap bitmap)
    {
        using var rgba = new WriteableBitmap(bitmap.PixelSize, bitmap.Dpi, PixelFormat.Rgba8888, AlphaFormat.Unpremul);
        using var locked = rgba.Lock();
        bitmap.CopyPixels(locked);
        var pixels = new byte[locked.RowBytes * bitmap.PixelSize.Height];
        Marshal.Copy(locked.Address, pixels, 0, pixels.Length);
        return (pixels, locked.RowBytes);
    }

    private static bool ContainsColour(Bitmap bitmap, PixelRect region, byte expectedRed, byte expectedGreen, byte expectedBlue)
    {
        var stride = region.Width * 4;
        var length = stride * region.Height;
        var pointer = Marshal.AllocHGlobal(length);
        try
        {
            bitmap.CopyPixels(region, pointer, length, stride);
            var pixels = new byte[length];
            Marshal.Copy(pointer, pixels, 0, length);
            for (var offset = 0; offset < pixels.Length; offset += 4)
            {
                if (Matches(pixels[offset], pixels[offset + 1], pixels[offset + 2], expectedRed, expectedGreen, expectedBlue)
                    || Matches(pixels[offset + 2], pixels[offset + 1], pixels[offset], expectedRed, expectedGreen, expectedBlue))
                {
                    return true;
                }
            }

            return false;
        }
        finally
        {
            Marshal.FreeHGlobal(pointer);
        }
    }

    private static bool Matches(byte red, byte green, byte blue, byte expectedRed, byte expectedGreen, byte expectedBlue) =>
        red < 180 && green > 120 && blue > 80
            && Math.Abs(red - expectedRed) <= 170
            && Math.Abs(green - expectedGreen) <= 80
            && Math.Abs(blue - expectedBlue) <= 100;
}
