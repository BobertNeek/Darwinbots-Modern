using System.Reflection;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class CustomSimulationCostsTests
{
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
