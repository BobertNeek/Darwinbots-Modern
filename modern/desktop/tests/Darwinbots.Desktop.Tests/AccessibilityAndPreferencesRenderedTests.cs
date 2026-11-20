using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Media.Imaging;
using Avalonia.Styling;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class AccessibilityAndPreferencesRenderedTests
{
    [Fact]
    public async Task CompactHighDpiWindowKeepsPlayfieldVisibleAndAppliesThemeCultureAndNumericInput()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var compactWindow = new MainWindow([], new WorldSetupOptions(), new AuditEngineClientFactory(), new AuditStorageService())
            {
                WindowState = WindowState.Normal,
                Width = 760,
                Height = 520,
            };

            try
            {
                compactWindow.Show();
                compactWindow.SetRenderScaling(1.5);
                compactWindow.UpdateLayout();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                var viewport = compactWindow.FindControl<WorldViewport>("Viewport")!;
                Assert.InRange(compactWindow.ClientSize.Width, 740, 780);
                Assert.True(viewport.Bounds.Width > 400);
                Assert.True(viewport.InputHitRadius < 14);

                var toolbar = compactWindow.FindControl<ScrollViewer>("LegacyToolbarScroller")!;
                Assert.True(toolbar.Extent.Width > toolbar.Viewport.Width);

                compactWindow.FindControl<ComboBox>("ThemeSelector")!.SelectedIndex = 2;
                Assert.Equal(ThemeVariant.Dark, Application.Current!.RequestedThemeVariant);

                compactWindow.FindControl<ComboBox>("CultureSelector")!.SelectedIndex = 1;
                Assert.Equal("OBERFLÄCHE", compactWindow.FindControl<TextBlock>("InterfaceTitle")!.Text);

                using var frame = Assert.IsAssignableFrom<Bitmap>(compactWindow.CaptureRenderedFrame());
                Assert.True(frame.PixelSize.Width >= 1_000);
                Assert.True(frame.PixelSize.Height >= 700);
                var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR")
                    ?? Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
                Directory.CreateDirectory(directory);
                using var stream = File.Create(Path.Combine(directory, "accessibility-preferences.png"));
                frame.Save(stream, new PngBitmapEncoderOptions());
            }
            finally
            {
                compactWindow.Close();
            }

            var inputWindow = new MainWindow([], new WorldSetupOptions(), new AuditEngineClientFactory(), new AuditStorageService())
            {
                WindowState = WindowState.Normal,
                Width = 940,
                Height = 650,
            };

            try
            {
                inputWindow.Show();
                inputWindow.SetRenderScaling(1.5);
                inputWindow.UpdateLayout();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                Assert.True(inputWindow.FindControl<Border>("SpeciesPanel")!.IsVisible);

                inputWindow.FindControl<Expander>("EnergyExpander")!.IsExpanded = true;
                inputWindow.UpdateLayout();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                UiPreferencesController.ApplyNumericCulture(inputWindow);

                var numeric = inputWindow.FindControl<NumericUpDown>("LiveFeedingToBody")!;
                numeric.ApplyTemplate();
                var textBox = Assert.Single(numeric.GetVisualDescendants().OfType<TextBox>());
                textBox.Focus();
                textBox.SelectAll();
                textBox.Text = "0,75";
                Assert.Equal(0.75m, numeric.Value);
            }
            finally
            {
                inputWindow.Close();
                DesktopUiPreferences.Reset();
            }
        }, CancellationToken.None);
    }
}
