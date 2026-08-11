using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Media.Imaging;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class TeleporterRenderedTests
{
    [Fact]
    public async Task TeleporterEditorRendersTypesAndPolicyAndAppliesLocalEdit()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var window = new TeleporterEditorWindow(
                new TeleporterSnapshot(7, [100f, 200f], 40f, [800f, 900f])
                {
                    Capacity = 4,
                    BackflowLimit = 2,
                    RespectShapes = true,
                    Mode = "intrasim"
                })
            {
                WindowState = WindowState.Normal,
                Width = 560,
                Height = 780
            };

            window.Show();
            AvaloniaHeadlessPlatform.ForceRenderTimerTick();

            Assert.Equal(0, window.FindControl<ComboBox>("Mode")!.SelectedIndex);
            Assert.Equal(4m, window.FindControl<NumericUpDown>("Capacity")!.Value);
            Assert.Equal(2m, window.FindControl<NumericUpDown>("BackflowLimit")!.Value);
            Assert.True(window.FindControl<CheckBox>("RespectShapes")!.IsChecked);

            Capture(window, "teleporter-editor-exercised.png");

            window.FindControl<NumericUpDown>("Radius")!.Value = 55m;
            Click(window, window.FindControl<Button>("ApplyButton")!);

            Assert.NotNull(window.Result);
            Assert.Equal(55f, window.Result!.Radius);
            Assert.Equal("intrasim", window.Result.Mode);
            window.Close();
        }, CancellationToken.None);
    }

    private static void Click(Window window, Button button)
    {
        var point = button.TranslatePoint(
            new Point(Math.Max(1, button.Bounds.Width / 2), Math.Max(1, button.Bounds.Height / 2)),
            window);
        Assert.True(point.HasValue);
        window.MouseDown(point.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(point.Value, MouseButton.Left, RawInputModifiers.None);
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }

    private static void Capture(Window window, string fileName)
    {
        var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR")
            ?? Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
        Directory.CreateDirectory(directory);
        var path = Path.Combine(directory, fileName);
        using var bitmap = new RenderTargetBitmap(new PixelSize((int)window.Bounds.Width, (int)window.Bounds.Height));
        bitmap.Render(window);
        bitmap.Save(path);
    }
}
