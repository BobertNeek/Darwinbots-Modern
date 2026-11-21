using System.Reflection;
using System.Linq;
using System.Runtime.InteropServices;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.ViewModels;
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
            // Keep this dialog test on a small CPU world. Physical backends
            // are exercised by the explicit hardware qualification.
            var main = new MainWindow([], new WorldSetupOptions { Backend = "Cpu", PopulationCapacity = 64 });
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

    [Theory]
    [InlineData(true)]
    [InlineData(false)]
    public async Task LiveCustomCostsExposeTheSignedMultiplierOnlyForNativeWorlds(bool native)
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var costs = new GlobalSimulationCostSettings { GlobalMultiplier = native ? -0.5 : 1 };
            var snapshot = EngineSnapshot.Empty with { NativeCycle = native ? 281 : null };
            var window = new AdvancedSettingsWindow(EnvironmentUpdate.Default, snapshot, selectedTab: 2, costs: costs);
            window.Show();
            try
            {
                Click(window, "LiveCustomSimulationCostsButton");
                await WaitFor(() => window.OwnedWindows.OfType<CustomSimulationCostsWindow>().Any());
                var dialog = Assert.Single(window.OwnedWindows.OfType<CustomSimulationCostsWindow>());
                var costX = dialog.FindControl<NumericUpDown>("GlobalCostMultiplier")!;
                Assert.Equal(native ? -100_000m : 0m, costX.Minimum);
                Assert.Equal(native ? -0.5m : 1m, costX.Value);
                Assert.False(dialog.FindControl<CheckBox>("DynamicAllowNegative")!.IsChecked);

                costX.Value = native ? -0.625m : 0.625m;
                Click(dialog, "ApplyButton");
                await WaitFor(() => window.CostUpdate is not null);
                Assert.Equal(native ? -0.625 : 0.625, window.CostUpdate!.GlobalMultiplier);
                Assert.False(window.CostUpdate.DynamicAllowNegative);
            }
            finally
            {
                foreach (var owned in window.OwnedWindows.ToArray()) owned.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task PhysicsAndCostsStagesNativeCostsAndKeepsApplicationDefaults()
    {
        var configuredCosts = new GlobalSimulationCostSettings
        {
            DnaCondition = 1e30,
            DnaCopy = 2.75,
            DynamicUpperRangePercent = 27.5,
            DynamicLowerRangePercent = 11.25,
        };
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            PopulationCapacity = 64,
            GlobalSettings = new GlobalSettings { Costs = configuredCosts },
            Species = [new("Test", "start\nstop", false, 0xff3f8e34u, 1, 1_000, 0, false, 0f)],
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService());
            window.Show();
            try
            {
                var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitForLive(() => viewModel.Population == 1, () => viewModel.Status);
                var session = Field<SimulationSession>(window, "_session");
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var original = await session.SaveAsync();
                var tick = session.LatestSnapshot.Tick;

                async Task<AdvancedSettingsWindow> Open(LegacyCommandId command = LegacyCommandId.EditPhysicsAndCosts)
                {
                    menu.FindCommandItem(command)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitForLive(() => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any(), () => viewModel.Status);
                    var result = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                    var tabs = result.FindControl<TabControl>("SettingsTabs")!;
                    Assert.Equal(command == LegacyCommandId.EditGeneralSettings ? 1 : 2, tabs.SelectedIndex);
                    tabs.SelectedIndex = 2;
                    Assert.True(result.FindControl<Border>("SimulationCostsPanel")!.IsVisible);
                    return result;
                }

                async Task<CustomSimulationCostsWindow> Edit(AdvancedSettingsWindow parent)
                {
                    Click(parent, "LiveCustomSimulationCostsButton");
                    await WaitForLive(() => parent.OwnedWindows.OfType<CustomSimulationCostsWindow>().Any(), () => "The staged cost editor did not open.");
                    var child = Assert.Single(parent.OwnedWindows.OfType<CustomSimulationCostsWindow>());
                    Assert.Equal("STAGE COSTS", child.FindControl<Button>("ApplyButton")!.Content);
                    return child;
                }

                async Task<WorldConfiguration> Apply(AdvancedSettingsWindow parent)
                {
                    viewModel.Status = "WAITING FOR LIVE SETTINGS";
                    Click(parent, "ApplyButton");
                    await WaitForLive(() => viewModel.Status == "LIVE SETTINGS APPLIED", () => viewModel.Status);
                    return Assert.IsType<WorldConfiguration>(await session.ReadConfigurationAsync());
                }

                var settings = await Open();
                Assert.Equal(2, settings.FindControl<ComboBox>("CostPreset")!.SelectedIndex);
                menu.FindCommandItem(LegacyCommandId.EditGeneralSettings)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitForLive(() => viewModel.Status == "LIVE SETTINGS ALREADY OPEN", () => viewModel.Status);
                Assert.Same(settings, Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>()));
                var custom = await Edit(settings);
                Assert.Equal(2.75m, custom.FindControl<NumericUpDown>("DnaCopy")!.Value);
                Assert.Equal(100_000m, custom.FindControl<NumericUpDown>("DnaCondition")!.Value);
                custom.FindControl<NumericUpDown>("DnaCopy")!.Value = 5m;
                Click(custom, "CancelButton");
                await WaitForLive(() => !custom.IsVisible, () => "The cost editor did not cancel.");
                Assert.Null(settings.CostUpdate);
                custom = await Edit(settings);
                custom.FindControl<NumericUpDown>("DnaCopy")!.Value = 3.5m;
                Click(custom, "ApplyButton");
                await WaitForLive(() => settings.CostUpdate?.DnaCopy == 3.5, () => "Custom costs were not staged.");
                Assert.Equal(configuredCosts with { DnaCopy = 3.5 }, settings.CostUpdate);
                Assert.Equal(original, await session.SaveAsync());
                Click(settings, "CancelButton");
                await WaitForLive(() => viewModel.Status == "CANCELED", () => viewModel.Status);
                Assert.False(settings.IsVisible);
                Assert.Equal(original, await session.SaveAsync());

                settings = await Open(LegacyCommandId.EditGeneralSettings);
                custom = await Edit(settings);
                Assert.Equal(2.75m, custom.FindControl<NumericUpDown>("DnaCopy")!.Value);
                custom.FindControl<NumericUpDown>("DnaCopy")!.Value = 3.5m;
                Click(custom, "ApplyButton");
                await WaitForLive(() => settings.CostUpdate?.DnaCopy == 3.5, () => "Custom costs were not staged.");
                Assert.Equal(configuredCosts with { DnaCopy = 3.5 }, (await Apply(settings)).Costs);

                settings = await Open();
                custom = await Edit(settings);
                Assert.Equal(3.5m, custom.FindControl<NumericUpDown>("DnaCopy")!.Value);
                Click(custom, "CancelButton");
                await WaitForLive(() => !custom.IsVisible, () => "The cost editor did not close.");
                settings.FindControl<ComboBox>("CostPreset")!.SelectedIndex = 1;
                settings.FindControl<NumericUpDown>("MaxVelocity")!.Value = 225m;
                var result = await Apply(settings);
                Assert.Equal(GlobalSimulationCostSettings.F1Default, result.Costs);
                Assert.Equal(Db2CostPreset.F1, result.NativeCostPreset);
                Assert.Equal((double)0.004f, result.Costs.DnaCondition);
                Assert.Equal(225f, result.Environment.Physics.MaxVelocity);
                Assert.Equal(tick, session.LatestSnapshot.Tick);

                settings = await Open();
                Assert.Equal(1, settings.FindControl<ComboBox>("CostPreset")!.SelectedIndex);
                settings.FindControl<ComboBox>("CostPreset")!.SelectedIndex = 0;
                Assert.Equal(GlobalSimulationCostSettings.NoCosts, (await Apply(settings)).Costs);
                Assert.Equal(configuredCosts, Field<GlobalSettings>(window, "_globalSettings").Costs);

                Click(window, "LiveAdvancedButton");
                await WaitForLive(() => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any(), () => viewModel.Status);
                var fromPhysicsButton = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                Assert.Equal(2, fromPhysicsButton.FindControl<TabControl>("SettingsTabs")!.SelectedIndex);
                Assert.Equal(0, fromPhysicsButton.FindControl<ComboBox>("CostPreset")!.SelectedIndex);
                Click(fromPhysicsButton, "CancelButton");
            }
            finally
            {
                foreach (var owned in window.OwnedWindows.ToArray()) owned.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task F5StagesAndRestoresCustomPresetWhenItsCostsAreIdenticalToNoCosts()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            PopulationCapacity = 64,
            NativeCostPreset = Db2CostPreset.NoCosts,
            GlobalSettings = new GlobalSettings { Costs = GlobalSimulationCostSettings.NoCosts },
            Species = [new("Test", "start\nstop", false, 0xff3f8e34u, 1, 1_000, 0, false, 0f)],
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, new AuditStorageService());
            window.Show();
            try
            {
                var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitForLive(() => viewModel.Status.StartsWith("WORLD READY", StringComparison.Ordinal), () => viewModel.Status);
                var session = Field<SimulationSession>(window, "_session");
                var original = await session.SaveAsync();

                async Task<AdvancedSettingsWindow> Open(Db2CostPreset expected)
                {
                    window.KeyPress(Key.F5, RawInputModifiers.None, PhysicalKey.F5, null);
                    window.KeyRelease(Key.F5, RawInputModifiers.None, PhysicalKey.F5, null);
                    await WaitForLive(() => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any(), () => viewModel.Status);
                    var dialog = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                    Assert.Equal(2, dialog.FindControl<TabControl>("SettingsTabs")!.SelectedIndex);
                    Assert.Equal((int)expected, dialog.FindControl<ComboBox>("CostPreset")!.SelectedIndex);
                    Assert.Equal(expected == Db2CostPreset.Custom, dialog.FindControl<Button>("LiveCustomSimulationCostsButton")!.IsEnabled);
                    return dialog;
                }

                async Task Complete(AdvancedSettingsWindow dialog, bool apply)
                {
                    viewModel.Status = "WAITING FOR LIVE SETTINGS";
                    Click(dialog, apply ? "ApplyButton" : "CancelButton");
                    await WaitForLive(() => viewModel.Status == (apply ? "LIVE SETTINGS APPLIED" : "CANCELED"), () => viewModel.Status);
                }

                var settings = await Open(Db2CostPreset.NoCosts);
                settings.FindControl<ComboBox>("CostPreset")!.SelectedIndex = (int)Db2CostPreset.Custom;
                Assert.Null(settings.CostUpdate);
                Assert.Equal(Db2CostPreset.Custom, settings.CostPresetUpdate);
                Assert.True(settings.FindControl<Border>("DirtyBadge")!.IsVisible);
                Assert.Equal(original, await session.SaveAsync());
                await Complete(settings, apply: false);
                Assert.Equal(original, await session.SaveAsync());

                settings = await Open(Db2CostPreset.NoCosts);
                settings.FindControl<ComboBox>("CostPreset")!.SelectedIndex = (int)Db2CostPreset.Custom;
                await Complete(settings, apply: true);
                var custom = Assert.IsType<WorldConfiguration>(await session.ReadConfigurationAsync());
                Assert.Equal(Db2CostPreset.Custom, custom.NativeCostPreset);
                Assert.Equal(GlobalSimulationCostSettings.NoCosts, custom.Costs);
                var saved = await session.SaveAsync();

                settings = await Open(Db2CostPreset.Custom);
                settings.FindControl<ComboBox>("CostPreset")!.SelectedIndex = (int)Db2CostPreset.NoCosts;
                Assert.Null(settings.CostUpdate);
                await Complete(settings, apply: true);
                Assert.Equal(Db2CostPreset.NoCosts, (await session.ReadConfigurationAsync())!.NativeCostPreset);
                var restore = typeof(MainWindow).GetMethod("LoadSimulationAsync", BindingFlags.Instance | BindingFlags.NonPublic)!;
                await Assert.IsAssignableFrom<Task>(restore.Invoke(window, [saved]));

                settings = await Open(Db2CostPreset.Custom);
                settings.FindControl<ComboBox>("CostPreset")!.SelectedIndex = (int)Db2CostPreset.F1;
                Assert.Equal(GlobalSimulationCostSettings.F1Default, settings.CostUpdate);
                await Complete(settings, apply: false);
                Assert.Equal(Db2CostPreset.Custom, (await session.ReadConfigurationAsync())!.NativeCostPreset);
            }
            finally
            {
                foreach (var owned in window.OwnedWindows.ToArray()) owned.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public void InvalidCostCommandDoesNotCommitEarlierNativeLiveSettings()
    {
        using var engine = new NativeEngineClient(backend: "Cpu");
        var before = engine.Save();
        Assert.Throws<InvalidOperationException>(() => engine.UpdateLiveSettings(
            EnvironmentUpdate.Default with { MetabolismCost = 5 },
            MutationSettingsUpdate.Default with { Enabled = false },
            new GlobalSimulationCostSettings { GlobalMultiplier = 2 },
            (Db2CostPreset)255));
        Assert.Equal(before, engine.Save());
    }

    private static T Field<T>(object instance, string name) =>
        Assert.IsType<T>(instance.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(instance));

    private static void Click(Window window, string name) =>
        window.FindControl<Button>(name)!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

    private static async Task WaitForLive(Func<bool> condition, Func<string> diagnostic)
    {
        for (var attempt = 0; attempt < 400; attempt++)
        {
            AvaloniaHeadlessPlatform.ForceRenderTimerTick();
            if (condition()) return;
            await Task.Delay(20);
        }
        throw new Xunit.Sdk.XunitException(diagnostic());
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
