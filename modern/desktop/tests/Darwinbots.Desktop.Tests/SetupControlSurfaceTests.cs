using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class SetupControlSurfaceTests
{
    [Fact]
    public async Task MainWindowOpensWithDb24832ContestAndPresentationDefaults()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var window = new MainWindow([], new WorldSetupOptions());
            try
            {
                Assert.Equal(5m, window.FindControl<NumericUpDown>("ModeMinimumRounds")!.Value);
                Assert.Equal(1m, window.FindControl<NumericUpDown>("ModeMaximumRounds")!.Value);
                Assert.Equal(15_000m, window.FindControl<NumericUpDown>("ModeMaximumCycles")!.Value);
                Assert.Equal(700m, window.FindControl<NumericUpDown>("ModePopulationCap")!.Value);
                Assert.True(window.FindControl<CheckBox>("ShowVisionToggle")!.IsChecked);

                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                Assert.True(viewport.ShowSelectedVision);
                Assert.True(viewport.ShowMovementVectors);
                Assert.True(viewport.ShowShotImpacts);
                Assert.True(viewport.ShowResourceGauges);
                Assert.True(viewport.HighlightTeleporters);
            }
            finally
            {
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task SetupModesMutationAndAdvancedSettingsReachTheCreatedWorld()
    {
        var session = DesktopTestApplication.Session;
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
        var session = DesktopTestApplication.Session;
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

    [Fact]
    public async Task SetupSettingsButtonsRoundTripTheDesktopSettingsAuthority()
    {
        Assert.NotNull(typeof(IDesktopStorageService).GetMethod("OpenSettingsAsync"));
        Assert.NotNull(typeof(IDesktopStorageService).GetMethod("SaveSettingsAsync"));

        var storage = new SettingsWorkflowStorage
        {
            SettingsToOpen = new DesktopSettings
            {
                Backend = "Gpu",
                TicksPerUpdate = 5,
                OrganismCapacity = 44_000,
                WorldWidth = 91_000,
                WorldHeight = 82_000,
                GlobalSettings = new GlobalSettings
                {
                    Costs = new GlobalSimulationCostSettings { DnaCopy = 4.25 },
                },
            },
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new SetupWindow(storage);
            window.Show();
            try
            {
                typeof(SetupWindow).GetField("_globalSettings", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .SetValue(window, new GlobalSettings
                    {
                        Costs = new GlobalSimulationCostSettings { DnaCopy = 2.5 },
                    });
                window.FindControl<ComboBox>("Backend")!.SelectedIndex = 1;
                window.FindControl<NumericUpDown>("WorldWidth")!.Value = 70_000;
                window.FindControl<NumericUpDown>("WorldHeight")!.Value = 60_000;
                window.FindControl<NumericUpDown>("PopulationCap")!.Value = 55_000;
                window.FindControl<ComboBox>("Speed")!.SelectedIndex = 2;

                var save = window.FindControl<Button>("SaveSettingsButton");
                var load = window.FindControl<Button>("LoadSettingsButton");
                Assert.NotNull(save);
                Assert.NotNull(load);
                save!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => storage.SavedSettings is not null);
                Assert.Equal("Cpu", storage.SavedSettings!.Backend);
                Assert.Equal(20u, storage.SavedSettings.TicksPerUpdate);
                Assert.Equal(2.5, storage.SavedSettings.GlobalSettings.Costs.DnaCopy);
                Assert.Equal(70_000f, storage.SavedSettings.WorldWidth);

                load!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => window.FindControl<NumericUpDown>("WorldWidth")!.Value == 91_000);
                Assert.Equal(2, window.FindControl<ComboBox>("Backend")!.SelectedIndex);
                Assert.Equal(82_000, window.FindControl<NumericUpDown>("WorldHeight")!.Value);
                Assert.Equal(4.25, typeof(SetupWindow).GetField("_globalSettings", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .GetValue(window) is GlobalSettings settings ? settings.Costs.DnaCopy : -1);
            }
            finally
            {
                window.Close();
            }
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

    private static async Task WaitFor(Func<bool> condition)
    {
        for (var attempt = 0; attempt < 100; attempt++)
        {
            if (condition()) return;
            await Task.Delay(10);
        }

        Assert.True(condition(), "Timed out waiting for settings file workflow.");
    }
}

internal sealed class SettingsWorkflowStorage : IDesktopStorageService
{
    public DesktopSettings? SettingsToOpen { get; init; }
    public DesktopSettings? SavedSettings { get; private set; }

    public Task<IReadOnlyList<DesktopTextFile>> OpenDnaFilesAsync(TopLevel owner) => Task.FromResult<IReadOnlyList<DesktopTextFile>>([]);
    public Task<string?> PickSimulationPathAsync(TopLevel owner) => Task.FromResult<string?>(null);
    public Task<byte[]?> OpenSimulationAsync(TopLevel owner) => Task.FromResult<byte[]?>(null);
    public Task<bool> SaveSimulationAsync(TopLevel owner, ReadOnlyMemory<byte> save) => Task.FromResult(true);
    public Task<bool> SaveDnaAsync(TopLevel owner, string suggestedFileName, string dna) => Task.FromResult(true);
    public Task<bool> SaveOrganismAsync(TopLevel owner, string suggestedFileName, Func<Stream, Task> writeAsync) => Task.FromResult(true);
    public string? FindLatestAutosave() => null;
    public Task SaveAutosaveAsync(ReadOnlyMemory<byte> save) => Task.CompletedTask;
    public Task<DesktopSettings?> OpenSettingsAsync(TopLevel owner) => Task.FromResult(SettingsToOpen);

    public Task<bool> SaveSettingsAsync(TopLevel owner, DesktopSettings settings)
    {
        SavedSettings = settings;
        return Task.FromResult(true);
    }
}
