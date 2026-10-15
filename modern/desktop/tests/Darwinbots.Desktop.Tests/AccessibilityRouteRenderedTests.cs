using Avalonia.Automation;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class AccessibilityRouteRenderedTests
{
    [Fact]
    public async Task KeyboardRouteTraversesAccessibleMenuToolbarAndViewToggles()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), new AuditEngineClientFactory(), new AuditStorageService())
            {
                WindowState = WindowState.Normal,
                Width = 1280,
                Height = 900,
            };

            try
            {
                window.Show();
                window.UpdateLayout();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var viewMenu = menu.TopLevelItems.Single(item => Equals(item.Header, "View"));
                var visionCommand = menu.FindCommandItem(LegacyCommandId.ViewShowVisionGrid)!;
                Assert.Equal("View", AutomationProperties.GetName(viewMenu));
                Assert.Contains("menu", AutomationProperties.GetHelpText(viewMenu), StringComparison.OrdinalIgnoreCase);
                Assert.Equal("Show Vision Grid", AutomationProperties.GetName(visionCommand));
                Assert.Contains("vision", AutomationProperties.GetHelpText(visionCommand), StringComparison.OrdinalIgnoreCase);

                var visionBefore = window.FindControl<CheckBox>("ShowVisionToggle")!.IsChecked;
                visionCommand.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                Assert.NotEqual(visionBefore, window.FindControl<CheckBox>("ShowVisionToggle")!.IsChecked);

                var toolbar = window.FindControl<UniformGrid>("LegacyToolbar")!;
                var toolbarButtons = toolbar.GetVisualDescendants()
                    .Where(control => control.Name?.StartsWith("ToolbarSlot", StringComparison.Ordinal) == true)
                    .OfType<Button>()
                    .ToArray();
                Assert.NotEmpty(toolbarButtons);
                Assert.All(toolbarButtons, button =>
                {
                    Assert.False(string.IsNullOrWhiteSpace(AutomationProperties.GetName(button)));
                    Assert.False(string.IsNullOrWhiteSpace(AutomationProperties.GetHelpText(button)));
                    Assert.True(KeyboardNavigation.GetTabIndex(button) >= 0);
                });

                var firstToolbarButton = window.FindControl<Button>("ToolbarSlot01")!;
                var secondToolbarButton = window.FindControl<Button>("ToolbarSlot02")!;
                Assert.True(KeyboardNavigation.GetTabIndex(firstToolbarButton) < KeyboardNavigation.GetTabIndex(secondToolbarButton));
                Assert.True(firstToolbarButton.Focus());
                Press(window, Key.Tab, PhysicalKey.Tab);
                Assert.Same(secondToolbarButton, window.FocusManager.GetFocusedElement());

                var skinToggle = window.FindControl<Button>("ToolbarSlot12")!;
                Assert.True(skinToggle.Focus());
                Press(window, Key.Space, PhysicalKey.Space);
                Assert.Contains("OFF", skinToggle.Content?.ToString(), StringComparison.Ordinal);
                Assert.Contains("OFF", AutomationProperties.GetName(skinToggle), StringComparison.Ordinal);

                var offlineOverlay = window.FindControl<Border>("InternetOfflineOverlay")!;
                Assert.True(offlineOverlay.IsVisible);
                Assert.Contains("offline", AutomationProperties.GetName(offlineOverlay), StringComparison.OrdinalIgnoreCase);
                Assert.Contains("disabled", AutomationProperties.GetHelpText(offlineOverlay), StringComparison.OrdinalIgnoreCase);
                Assert.Contains("OFFLINE", offlineOverlay.GetVisualDescendants().OfType<TextBlock>().Single().Text, StringComparison.Ordinal);

                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var frame = Assert.IsAssignableFrom<Bitmap>(window.CaptureRenderedFrame());
                Assert.True(frame.PixelSize.Width >= 1_000);
                Assert.True(frame.PixelSize.Height >= 700);
            }
            finally
            {
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task RobotMenuFindBestDispatchesThroughRenderedSurfaceAndWritesReceipt()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), new AuditEngineClientFactory(), new AuditStorageService())
            {
                WindowState = WindowState.Normal,
                Width = 1280,
                Height = 900,
            };

            try
            {
                window.Show();
                window.UpdateLayout();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var robotMenu = menu.TopLevelItems.Single(item => Equals(item.Header, "Robot"));
                var findBest = menu.FindCommandItem(LegacyCommandId.RobotFindBest)!;
                for (var attempt = 0; attempt < 500 && !findBest.IsEnabled; attempt++)
                    await Task.Delay(10);
                Assert.True(robotMenu.IsVisible);
                Assert.Equal("Find Best", AutomationProperties.GetName(findBest));
                Assert.True(findBest.IsEnabled);

                window.KeyPress(Key.B, RawInputModifiers.Control, PhysicalKey.B, null);
                window.KeyRelease(Key.B, RawInputModifiers.Control, PhysicalKey.B, null);
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                string? status = null;
                for (var attempt = 0; attempt < 50 && status is null; attempt++)
                {
                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    status = window.GetVisualDescendants()
                        .OfType<TextBlock>()
                        .Select(block => block.Text)
                        .FirstOrDefault(text => text is not null &&
                            (text.Contains("BEST FAMILY", StringComparison.Ordinal) ||
                             text.Contains("NO LIVING ROBOTS", StringComparison.Ordinal)));
                    if (status is null) await Task.Delay(10);
                }

                Assert.NotNull(status);

                using var frame = Assert.IsAssignableFrom<Bitmap>(window.CaptureRenderedFrame());
                Assert.True(frame.PixelSize.Width >= 1_000);
                Assert.True(frame.PixelSize.Height >= 700);
                var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR");
                Assert.False(string.IsNullOrWhiteSpace(directory), "DARWINBOTS_GUI_AUDIT_DIR is required.");
                Directory.CreateDirectory(directory!);
                var output = Path.Combine(directory!, "menu-05-robot-find-best.png");
                using (var stream = new FileStream(
                    output,
                    FileMode.Create,
                    FileAccess.Write,
                    FileShare.Read,
                    4096,
                    FileOptions.WriteThrough))
                {
                    frame.Save(stream, new PngBitmapEncoderOptions());
                    stream.Flush(true);
                }
                Assert.True(File.Exists(output), $"Rendered receipt was not written: {output}");
            }
            finally
            {
                if (window.IsVisible) window.Close();
            }
        }, CancellationToken.None);
    }

    private static void Press(Window window, Key key, PhysicalKey physicalKey)
    {
        window.KeyPress(key, RawInputModifiers.None, physicalKey, null);
        window.KeyRelease(key, RawInputModifiers.None, physicalKey, null);
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }
}
