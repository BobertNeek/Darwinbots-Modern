using System.Reflection;
using System.Linq;
using System.Runtime.InteropServices;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class CustomSimulationCostsTests
{
    [Fact]
    public async Task CustomSimulationCostsDialogExposesActualSurfaceAndCommitsOnlyOnSave()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var main = new MainWindow([], new WorldSetupOptions());
            main.Show();

            try
            {
                var cancelSettings = new GlobalSettings();
                var originalDnaCopy = Convert.ToDecimal(cancelSettings.Costs.DnaCopy);
                var originalDynamicMultiplier = Convert.ToDecimal(cancelSettings.Costs.DynamicMultiplier);
                var cancelDialog = new CustomSimulationCostsWindow(cancelSettings);
                cancelDialog.Show(main);

                try
                {
                    var numericNames = new[]
                    {
                        "GlobalCostMultiplier", "DnaNumber", "DnaDereference", "DnaBasic", "DnaAdvanced",
                        "DnaBitwise", "DnaCondition", "DnaLogic", "DnaStore", "DnaFlow", "DnaUpkeep", "DnaCopy",
                        "MovementCost", "RotationCost", "TieFormationCost", "ShotFormationCost", "VenomCost",
                        "PoisonCost", "SlimeCost", "ShellCost", "BodyUpkeep", "ChloroplastConstructionCost",
                        "VirusCost", "AgeBase", "AgeStart", "AgeLinear", "AgeLogarithmic", "DynamicMultiplier",
                        "DynamicTargetPopulation", "DynamicPopulationRange", "DynamicSensitivity",
                        "DynamicLowerZeroThreshold", "DynamicUpperZeroThreshold", "DynamicLowerReinstateThreshold",
                        "DynamicUpperReinstateThreshold"
                    };

                    Assert.All(numericNames, name => Assert.NotNull(cancelDialog.FindControl<NumericUpDown>(name)));
                    Assert.NotNull(cancelDialog.FindControl<CheckBox>("DynamicAllRobots"));
                    Assert.NotNull(cancelDialog.FindControl<CheckBox>("DynamicAllowNegative"));

                    var cancelButton = cancelDialog.FindControl<Button>("CancelButton")!;
                    var applyButton = cancelDialog.FindControl<Button>("ApplyButton")!;
                    Assert.Equal("CANCEL", cancelButton.Content?.ToString());
                    Assert.Equal("SAVE COSTS", applyButton.Content?.ToString());

                    Assert.Equal(originalDnaCopy, Convert.ToDecimal(cancelDialog.FindControl<NumericUpDown>("DnaCopy")!.Value));
                    Assert.Equal(originalDynamicMultiplier, Convert.ToDecimal(cancelDialog.FindControl<NumericUpDown>("DynamicMultiplier")!.Value));

                    var surfaceText = string.Join(
                        "\n",
                        cancelDialog.GetVisualDescendants().OfType<TextBlock>().Select(block => block.Text));
                    Assert.Contains("CUSTOM SIMULATION COSTS", surfaceText);
                    Assert.Contains("BASE COSTS", surfaceText);
                    Assert.Contains("DYNAMIC COSTS", surfaceText);
                    Assert.Contains("Changes apply to the next world setup.", surfaceText);

                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    using var frame = Assert.IsAssignableFrom<Bitmap>(cancelDialog.CaptureRenderedFrame());
                    Assert.True(frame.PixelSize.Width >= 620);
                    Assert.True(frame.PixelSize.Height >= 620);

                    var pixelRect = new PixelRect(0, 0, frame.PixelSize.Width, frame.PixelSize.Height);
                    var stride = pixelRect.Width * 4;
                    var pixelBytes = new byte[stride * pixelRect.Height];
                    var pixelBuffer = Marshal.AllocHGlobal(pixelBytes.Length);
                    try
                    {
                        frame.CopyPixels(pixelRect, pixelBuffer, pixelBytes.Length, stride);
                        Marshal.Copy(pixelBuffer, pixelBytes, 0, pixelBytes.Length);
                    }
                    finally
                    {
                        Marshal.FreeHGlobal(pixelBuffer);
                    }

                    Assert.True(ContainsApproximateColor(pixelBytes, 0xF4, 0xF0, 0xE6));
                    Assert.True(ContainsApproximateColor(pixelBytes, 0x41, 0x6E, 0x35));

                    cancelDialog.FindControl<NumericUpDown>("DnaCopy")!.Value = 9.25m;
                    cancelButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    Assert.False(ReadAccepted(cancelDialog));
                    Assert.Equal(originalDnaCopy, Convert.ToDecimal(cancelSettings.Costs.DnaCopy));
                    Assert.Equal(originalDynamicMultiplier, Convert.ToDecimal(cancelSettings.Costs.DynamicMultiplier));
                }
                finally
                {
                    cancelDialog.Close();
                }

                var applySettings = new GlobalSettings();
                var applyDialog = new CustomSimulationCostsWindow(applySettings);
                applyDialog.Show(main);
                try
                {
                    applyDialog.FindControl<NumericUpDown>("DnaCopy")!.Value = 3.75m;
                    applyDialog.FindControl<NumericUpDown>("DynamicMultiplier")!.Value = 1.5m;
                    applyDialog.FindControl<Button>("ApplyButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

                    Assert.True(ReadAccepted(applyDialog));
                    Assert.Equal(3.75m, Convert.ToDecimal(applyDialog.Costs.DnaCopy));
                    Assert.Equal(1.5m, Convert.ToDecimal(applyDialog.Costs.DynamicMultiplier));
                }
                finally
                {
                    applyDialog.Close();
                }
            }
            finally
            {
                main.Close();
            }
        }, CancellationToken.None);

        static bool ReadAccepted(CustomSimulationCostsWindow dialog)
        {
            return (bool)typeof(CustomSimulationCostsWindow)
                .GetProperty("Accepted", BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)!
                .GetValue(dialog)!;
        }

        static bool ContainsApproximateColor(byte[] pixels, byte red, byte green, byte blue)
        {
            for (var index = 0; index + 3 < pixels.Length; index += 4)
            {
                var bgra = Close(pixels[index], blue)
                    && Close(pixels[index + 1], green)
                    && Close(pixels[index + 2], red);
                var rgba = Close(pixels[index], red)
                    && Close(pixels[index + 1], green)
                    && Close(pixels[index + 2], blue);
                if (bgra || rgba) return true;
            }

            return false;

            static bool Close(byte left, byte right) => Math.Abs(left - right) <= 24;
        }
    }

    [Fact]
    public async Task CustomCostEditReachesTheRustEngineConfigPayload()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var dialogType = typeof(GlobalSettingsWindow).Assembly
                .GetType("Darwinbots.Desktop.Views.CustomSimulationCostsWindow");
            Assert.NotNull(dialogType);

            var dialog = Activator.CreateInstance(dialogType!, new GlobalSettings())!;
            var window = (Window)dialog;
            window.Show();
            try
            {
                var dnaCopy = (NumericUpDown)dialogType!.GetField(
                    "DnaCopy", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(dialog)!;
                dnaCopy.Value = 2.75m;
                window.FindControl<Button>("ApplyButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

                var costs = (GlobalSimulationCostSettings)dialogType
                    .GetProperty("Costs", BindingFlags.Instance | BindingFlags.Public)!
                    .GetValue(dialog)!;
                Assert.Equal(2.75, costs.DnaCopy);

                var payload = NativeCommandSerializer.SerializeEngineConfig(new WorldSetupOptions
                {
                    GlobalSettings = new GlobalSettings { Costs = costs },
                });
                Assert.Contains("\"dna_copy\":2.75", payload, StringComparison.Ordinal);
            }
            finally
            {
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task GlobalSettingsRouteCommitsCustomCostsIntoPendingSettings()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new GlobalSettingsWindow(new GlobalSettings());
            window.Show();
            try
            {
                var button = window.FindControl<Button>("CustomSimulationCostsButton");
                Assert.NotNull(button);

                button!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<CustomSimulationCostsWindow>().Any());
                var dialog = Assert.Single(window.OwnedWindows.OfType<CustomSimulationCostsWindow>());
                dialog.FindControl<NumericUpDown>("DnaCopy")!.Value = 3.5m;
                dialog.FindControl<Button>("ApplyButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

                await WaitFor(() => window.Settings.Costs.DnaCopy == 3.5);
                var payload = NativeCommandSerializer.SerializeEngineConfig(new WorldSetupOptions
                {
                    GlobalSettings = window.Settings,
                });
                Assert.Contains("\"dna_copy\":3.5", payload, StringComparison.Ordinal);
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static async Task WaitFor(Func<bool> condition)
    {
        for (var attempt = 0; attempt < 100; attempt++)
        {
            if (condition()) return;
            await Task.Delay(10);
        }

        Assert.True(condition(), "Timed out waiting for the desktop route to commit custom costs.");
    }
}
