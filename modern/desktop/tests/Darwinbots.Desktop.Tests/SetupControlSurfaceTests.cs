using System.Reflection;
using System.Text;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
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

    [Fact]
    public async Task SetupWorkingCopyRendersAndRoutesStartNewChangeLoadAndSave()
    {
        var storage = new SettingsWorkflowStorage
        {
            SettingsToOpen = new DesktopSettings
            {
                Backend = "Gpu",
                WorldWidth = 91_000,
                WorldHeight = 82_000,
                OrganismCapacity = 44_000,
            },
        };
        var created = new List<WorldSetupOptions>();

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new SetupWindow(storage);
            window.WorldCreated += created.Add;
            window.Show();
            try
            {
                window.UpdateLayout();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                Assert.Equal(WindowState.Maximized, window.WindowState);
                Assert.Equal("START NEW", window.FindControl<Button>("StartNewButton")!.Content);
                Assert.Equal("CHANGE", window.FindControl<Button>("ChangeButton")!.Content);
                Assert.Equal("LOAD SETTINGS", window.FindControl<Button>("LoadSettingsButton")!.Content);
                Assert.Equal("SAVE SETTINGS", window.FindControl<Button>("SaveSettingsButton")!.Content);
                Assert.True(window.FindControl<Button>("StartNewButton")!.IsVisible);
                Assert.True(window.FindControl<Button>("ChangeButton")!.IsVisible);
                Assert.True(window.FindControl<Button>("LoadSettingsButton")!.IsVisible);
                Assert.True(window.FindControl<Button>("SaveSettingsButton")!.IsVisible);

                window.FindControl<NumericUpDown>("WorldWidth")!.Value = 70_000;
                window.FindControl<Button>("ChangeButton")!
                    .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.Single(created);
                Assert.Equal(70_000f, created[0].WorldWidth);

                window.FindControl<NumericUpDown>("WorldWidth")!.Value = 71_000;
                window.KeyPress(Key.Enter, RawInputModifiers.Control, PhysicalKey.Enter, null);
                window.KeyRelease(Key.Enter, RawInputModifiers.Control, PhysicalKey.Enter, null);
                Assert.Equal(2, created.Count);
                Assert.Equal(71_000f, created[1].WorldWidth);

                window.FindControl<Button>("SaveSettingsButton")!
                    .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => storage.SavedSettings is not null);
                Assert.Equal(71_000f, storage.SavedSettings!.WorldWidth);

                window.FindControl<Button>("LoadSettingsButton")!
                    .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => window.FindControl<NumericUpDown>("WorldWidth")!.Value == 91_000);
                Assert.Equal(82_000, window.FindControl<NumericUpDown>("WorldHeight")!.Value);
                Assert.Equal(2, window.FindControl<ComboBox>("Backend")!.SelectedIndex);

                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var frame = Assert.IsAssignableFrom<Bitmap>(window.CaptureRenderedFrame());
                var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR")
                    ?? Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
                Directory.CreateDirectory(directory);
                using var stream = File.Create(Path.Combine(directory, "set-01-setup-working-copy.png"));
                frame.Save(stream, new PngBitmapEncoderOptions());
            }
            finally
            {
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public void SetupStorageRoutesSetThroughDb2AndJsonThroughTheModernCodec()
    {
        var expected = new DesktopSettings
        {
            Backend = "Gpu",
            TicksPerUpdate = 20,
            OrganismCapacity = 44_000,
            WorldWidth = 91_000,
            WorldHeight = 82_000,
        };

        var legacyBytes = SetupStorageService.EncodeSettings("darwinbots.set", expected);
        var modernBytes = SetupStorageService.EncodeSettings("darwinbots.json", expected);

        Assert.Equal(Db2SettingsCodec.Encode(expected), legacyBytes);
        Assert.Equal(Encoding.UTF8.GetBytes(expected.ToJson()), modernBytes);
        Assert.Equal(expected.ToJson(), SetupStorageService.DecodeSettings("darwinbots.set", legacyBytes).ToJson());
        Assert.Equal(expected.ToJson(), SetupStorageService.DecodeSettings("darwinbots.json", modernBytes).ToJson());
    }

    [Fact]
    public void StandaloneLegacySetImportRoundTripsLegacyPhysicsThroughThePublicSettingsWorkflow()
    {
        var fixture = Path.GetFullPath(Path.Combine(
            AppContext.BaseDirectory,
            "..", "..", "..", "..", "..", "..", "..",
            "Installer", "sets", "Ledge Island.set"));

        var imported = SetupStorageService.DecodeSettings(fixture, File.ReadAllBytes(fixture));
        var roundTripped = SetupStorageService.DecodeSettings(
            "roundtrip.set",
            SetupStorageService.EncodeSettings("roundtrip.set", imported));

        Assert.Equal(new[] { 0f, 2f }, imported.Gravity);
        Assert.Equal(0.5f, imported.BrownianMotion);
        Assert.Equal(0.66f, imported.Physics.MovementEfficiency);
        Assert.Equal(imported.ToJson(), roundTripped.ToJson());
    }

    [Fact]
    public async Task LoadedNativeMutationProfilesSurviveEditingDuplicationSaveAndStart()
    {
        var profile = MutationProfileUpdate.Default with
        {
            Point = new(1e30, 3.123456789, 1.234567891),
            CopyError = new(17, 7.5, 2.25),
            Delta = new(123, 600, 180),
            PointValueChangePercent = 23,
        };
        var imported = new SpeciesImport("Native rates", "start 1 .up store stop", false,
            0xff239ac0, 5, 1_000, 0, false, 7)
        {
            MutationProfile = profile,
            DnaPath = "Bots/native-rates.txt",
        };
        var mutationSettings = new MutationSettingsUpdate(false, 2.5f, []) { Profile = profile };
        var storage = new SettingsWorkflowStorage
        {
            SettingsToOpen = new DesktopSettings
            {
                Species = [imported],
                MutationSettings = mutationSettings,
                LegacySettingsFields = ["retained original field"],
            },
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new SetupWindow(storage);
            window.Show();
            try
            {
                Invoke(window, "LoadSettings_Click");
                await WaitFor(() => window.Species.Count == 1 && window.Species[0].Name == imported.Name);
                var row = Assert.Single(window.Species);
                Assert.Equal(profile, row.ToImport(1).MutationProfile);
                Assert.Equal(imported.DnaPath, row.ToImport(1).DnaPath);
                window.Species.Add(row.Duplicate());
                Assert.Equal(profile, window.Species[1].MutationProfile);
                Invoke(window, "SpeciesMutationRates_Click", new Button { Tag = row });
                await WaitFor(() => window.OwnedWindows.OfType<MutationRatesWindow>().Any());
                var rates = Assert.Single(window.OwnedWindows.OfType<MutationRatesWindow>());
                rates.FindControl<ComboBox>("ClassSelector")!.SelectedIndex = 3;
                rates.FindControl<NumericUpDown>("OneIn")!.Value = 29;
                Invoke(rates, "Apply_Click");
                await WaitFor(() => row.MutationProfile!.CopyError.OneIn == 29);
                Assert.Equal(profile.CopyError with { OneIn = 29 }, row.MutationProfile!.CopyError);
                Assert.Equal(profile with { CopyError = profile.CopyError with { OneIn = 29 } }, row.MutationProfile);
                Assert.Equal(profile, window.Species[1].MutationProfile);
                Invoke(window, "Advanced_Click");
                await WaitFor(() => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any());
                var advanced = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                Assert.Null(advanced.MutationUpdate.Profile);
                Invoke(advanced, "Apply_Click");
                await WaitFor(() => !window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any());
                Invoke(window, "SaveSettings_Click");
                await WaitFor(() => storage.SavedSettings is not null);
                Assert.Equal(mutationSettings, storage.SavedSettings!.MutationSettings);
                Assert.Equal(storage.SettingsToOpen!.LegacySettingsFields, storage.SavedSettings.LegacySettingsFields);
                Assert.Equal(row.MutationProfile, storage.SavedSettings.Species[0].MutationProfile);
                WorldSetupOptions? created = null;
                window.WorldCreated += setup => created = setup;
                Invoke(window, "StartNew_Click");
                Assert.NotNull(created);
                Assert.Equal(mutationSettings, created!.MutationSettings);
                Assert.Equal(row.MutationProfile, created.Species[0].MutationProfile);
                Assert.Equal(imported.DnaPath, created.Species[0].DnaPath);
                using var engine = new NativeEngineClient(created with { Backend = "Cpu" });
                foreach (var species in created.Species) engine.ImportSpecies(species);
                var native = engine.Snapshot();
                Assert.False(native.MutationSettings.Enabled);
                Assert.Equal(2.5f, native.MutationSettings.GlobalMultiplier);
                Assert.Equal(profile, native.MutationSettings.Profile);
                Assert.Equal(10, native.Population);
                Assert.All(native.Organisms, robot => Assert.Equal(
                    created.Species[(int)robot.Species - 1].MutationProfile, robot.MutationProfile));
            }
            finally
            {
                foreach (var child in window.OwnedWindows.ToArray()) child.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task LiveMutationRateEditsPreserveLoadedDistributionsAndPreferences()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var profile = MutationProfileUpdate.Default with
            {
                CopyError = new(17, 7.5, 2.25),
                Delta = new(123, 600, 180),
                PointValueChangePercent = 23,
                Point2 = new(0, 9, 2),
                Amplification = new(1e50, 250, 75),
            };
            var snapshot = EngineSnapshot.Empty with
            {
                MutationSettings = new(true, 50f) { Profile = profile },
            };
            var window = new AdvancedSettingsWindow(EnvironmentUpdate.Default, snapshot);
            try
            {
                // Environment-only edits must not replace evolved per-robot rates.
                Assert.Null(window.MutationUpdate.Profile);
                Assert.Equal(50f, window.MutationUpdate.GlobalMultiplier);
                window.FindControl<NumericUpDown>("CopyErrorOneIn")!.Value = 29;
                Assert.Equal(profile with { CopyError = profile.CopyError with { OneIn = 29 } },
                    window.MutationUpdate.Profile);
            }
            finally { window.Close(); }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task SetupCategoriesOpenRealEditorsAndCommitTheirValuesToTheNewWorld()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new SetupWindow(new SettingsWorkflowStorage());
            window.Show();
            try
            {
                var categories = window.FindControl<TabControl>("SimulationSettingsTabs")!;
                categories.SelectedIndex = 3;
                await WaitFor(() => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any());
                var mutation = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                Assert.Equal(0, mutation.FindControl<TabControl>("SettingsTabs")!.SelectedIndex);
                mutation.FindControl<NumericUpDown>("GlobalMutationRate")!.Value = 2.75m;
                mutation.FindControl<CheckBox>("MutationsEnabled")!.IsChecked = false;
                Invoke(mutation, "Apply_Click");
                await WaitFor(() => categories.SelectedIndex == 0);
                categories.SelectedIndex = 2;
                await WaitFor(() => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any());
                var physics = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                Assert.Equal(2, physics.FindControl<TabControl>("SettingsTabs")!.SelectedIndex);
                physics.FindControl<NumericUpDown>("GravityY")!.Value = 0.125m;
                Invoke(physics, "Apply_Click");
                await WaitFor(() => categories.SelectedIndex == 0);
                categories.SelectedIndex = 4;
                await WaitFor(() => window.OwnedWindows.OfType<GlobalSettingsWindow>().Any());
                var evolution = Assert.Single(window.OwnedWindows.OfType<GlobalSettingsWindow>());
                Assert.Equal(2, evolution.FindControl<TabControl>("Tabs")!.SelectedIndex);
                evolution.FindControl<TabControl>("Tabs")!.SelectedIndex = 0;
                evolution.FindControl<NumericUpDown>("BodyCutoff")!.Value = 12345;
                Invoke(evolution, "Apply_Click");
                await WaitFor(() => categories.SelectedIndex == 0);
                WorldSetupOptions? created = null;
                window.WorldCreated += setup => created = setup;
                Invoke(window, "StartNew_Click");
                Assert.NotNull(created);
                Assert.False(created!.MutationSettings.Enabled);
                Assert.Equal(2.75f, created.MutationSettings.GlobalMultiplier);
                Assert.Equal(0.125f, created.Gravity[1]);
                Assert.Equal(12345, created.GlobalSettings.Main.BodyCutoff);
            }
            finally
            {
                foreach (var child in window.OwnedWindows.ToArray()) child.Close();
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
