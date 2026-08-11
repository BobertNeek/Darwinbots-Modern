using System.Runtime.InteropServices;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Media.Imaging;
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
