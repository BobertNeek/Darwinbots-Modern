using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class SetupControlSurfaceTests
{
    [Fact]
    public async Task SetupModesMutationAndAdvancedSettingsReachTheCreatedWorld()
    {
        using var session = HeadlessUnitTestSession.StartNew(typeof(DesktopTestApplication));
        await session.Dispatch(() =>
        {
            var window = new SetupWindow();
            window.Show();

            Assert.Equal(WindowState.Maximized, window.WindowState);
            Assert.Equal(2, window.Species.Count);
            Select(window, window.FindControl<RadioButton>("ZerobotMode")!, "Zerobots");
            Assert.Single(window.Species);
            Assert.Equal("Zerobot", window.Species[0].Name);
            Select(window, window.FindControl<RadioButton>("ZerobotVegetableMode")!, "ZerobotsAndVegetables");
            Assert.Equal(2, window.Species.Count);

            Select(window, window.FindControl<RadioButton>("StarterMode")!, "StarterBotsAndVegetables");
            window.Species[0].MutationRate = 37.5;
            var custom = EnvironmentUpdate.Default with
            {
                BrownianMotion = 3f,
                AutoSpeciation = true,
                SpeciationGeneticDistancePercent = 12.5f,
                ToroidalWorld = false,
            };
            SetEnvironment(window, custom);
            WorldSetupOptions? created = null;
            window.WorldCreated += options => created = options;

            Invoke(window, "CreateWorld_Click");

            Assert.NotNull(created);
            Assert.Equal(37.5f, created.Species[0].MutationRate);
            Assert.Equal(3f, created.BrownianMotion);
            Assert.True(created.AutoSpeciation);
            Assert.Equal(12.5f, created.SpeciationGeneticDistancePercent);
            Assert.False(created.ToroidalWorld);
            window.Close();
        }, CancellationToken.None);
    }

    [Fact]
    public async Task NewWorldRestoresEverySetupDefaultAndAdvancedDialogRoundTripsAllValues()
    {
        using var session = HeadlessUnitTestSession.StartNew(typeof(DesktopTestApplication));
        await session.Dispatch(() =>
        {
            var window = new SetupWindow();
            window.Show();
            SetEnvironment(window, EnvironmentUpdate.Default with { BrownianMotion = 9f, ToroidalWorld = false });
            window.FindControl<ComboBox>("Backend")!.SelectedIndex = 2;
            window.FindControl<NumericUpDown>("WorldWidth")!.Value = 99_000;
            window.FindControl<NumericUpDown>("WorldHeight")!.Value = 88_000;
            window.FindControl<NumericUpDown>("PopulationCap")!.Value = 77_000;
            window.FindControl<NumericUpDown>("VegetablePopulationCap")!.Value = 66_000;
            window.FindControl<ComboBox>("Speed")!.SelectedIndex = 0;

            Invoke(window, "NewWorld_Click");

            AssertEnvironmentEqual(EnvironmentUpdate.Default, GetEnvironment(window));
            Assert.Equal(0, window.FindControl<ComboBox>("Backend")!.SelectedIndex);
            Assert.Equal(16_000, window.FindControl<NumericUpDown>("WorldWidth")!.Value);
            Assert.Equal(12_000, window.FindControl<NumericUpDown>("WorldHeight")!.Value);
            Assert.Equal(25_000, window.FindControl<NumericUpDown>("PopulationCap")!.Value);
            Assert.Equal(500, window.FindControl<NumericUpDown>("VegetablePopulationCap")!.Value);
            Assert.Equal(3, window.FindControl<ComboBox>("Speed")!.SelectedIndex);

            var custom = EnvironmentUpdate.Default with
            {
                MetabolismCost = 4,
                SunlightEnergy = 65,
                Gravity = [1f, 2f],
                AutoSpeciation = true,
                SpeciationGeneticDistancePercent = 8f,
                ToroidalWorld = false,
            };
            var advanced = new AdvancedSettingsWindow(custom);
            AssertEnvironmentEqual(custom, advanced.Update);
            window.Close();
        }, CancellationToken.None);
    }

    private static void Select(SetupWindow window, RadioButton radio, string modeName)
    {
        radio.IsChecked = true;
        Invoke(window, "Mode_Checked", radio);
    }

    private static void AssertEnvironmentEqual(EnvironmentUpdate expected, EnvironmentUpdate actual)
    {
        Assert.Equal(expected.Gravity, actual.Gravity);
        Assert.Equal(expected with { Gravity = actual.Gravity }, actual);
    }

    private static void Invoke(object target, string method, object? sender = null) =>
        target.GetType().GetMethod(method, BindingFlags.Instance | BindingFlags.NonPublic)!
            .Invoke(target, [sender, new RoutedEventArgs()]);

    private static void SetEnvironment(SetupWindow window, EnvironmentUpdate update) =>
        typeof(SetupWindow).GetField("_environment", BindingFlags.Instance | BindingFlags.NonPublic)!
            .SetValue(window, update);

    private static EnvironmentUpdate GetEnvironment(SetupWindow window) =>
        (EnvironmentUpdate)typeof(SetupWindow).GetField("_environment", BindingFlags.Instance | BindingFlags.NonPublic)!
            .GetValue(window)!;
}
