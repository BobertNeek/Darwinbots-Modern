using System.Reflection;
using System.Text;
using System.Text.Json;
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
    [Theory]
    [InlineData(null)]
    [InlineData(0f)]
    [InlineData(1.25f)]
    public async Task SetupCarriesProcessSpareThroughEveryLaunchRouteWithoutSavingIt(float? spare)
    {
        var storage = new SettingsWorkflowStorage
        {
            SettingsToOpen = new DesktopSettings { WorldWidth = 21_000 },
            SimulationPathToOpen = "chosen-original.sim",
            LatestAutosave = "chosen-modern.db3s",
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new SetupWindow(storage, spare, 12);
            var created = new List<WorldSetupOptions>();
            window.WorldCreated += created.Add;
            var fresh = new SetupWindow(storage);
            WorldSetupOptions? independentlyCreated = null;
            fresh.WorldCreated += value => independentlyCreated = value;
            try
            {
                window.Show();
                Invoke(window, "StartNew_Click");
                Invoke(window, "Change_Click");
                Invoke(window, "NewWorld_Click");
                Invoke(window, "StartNew_Click");
                Invoke(window, "LoadSettings_Click");
                await WaitFor(() => window.FindControl<NumericUpDown>("WorldWidth")!.Value == 21_000);
                Invoke(window, "Change_Click");
                Invoke(window, "SaveSettings_Click");
                await WaitFor(() => storage.SavedSettings is not null);
                Assert.DoesNotContain("Gaussian", storage.SavedSettings!.ToJson(), StringComparison.OrdinalIgnoreCase);
                SetEnvironment(window, EnvironmentUpdate.Default with
                {
                    Vegetation = Db2VegetationOptions.Default with { StartChloroplasts = 7321, MaxEnergyPerTick = 70 },
                });
                Invoke(window, "OpenSave_Click");
                await WaitFor(() => created.Count == 5);
                Invoke(window, "Recover_Click");

                Assert.Equal(6, created.Count);
                Assert.All(created, value => Assert.Equal(spare, value.NativeGaussianSpare));
                Assert.All(created, value => Assert.Equal((byte)12, value.NativeSunChange));
                Assert.Equal("chosen-original.sim", created[4].LoadSavePath);
                Assert.False(created[4].IsAutosaveRecovery);
                Assert.Equal(Db2VegetationOptions.Default with { StartChloroplasts = 7321 }, created[4].Vegetation);
                Assert.Equal("chosen-modern.db3s", created[5].LoadSavePath);
                Assert.True(created[5].IsAutosaveRecovery);
                Invoke(fresh, "StartNew_Click");
                Assert.NotNull(independentlyCreated);
                Assert.Null(independentlyCreated.NativeGaussianSpare);
                Assert.Null(independentlyCreated.NativeSunChange);
            }
            finally
            {
                fresh.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

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
            var window = new SetupWindow(new AuditStorageService());
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
    public async Task NewWorldRestoresSimulationDefaultsAndKeepsStartingChloroplasts()
    {
        var session = DesktopTestApplication.Session;
        await session.Dispatch(() =>
        {
            var window = new SetupWindow(new AuditStorageService());
            window.Show();
            var vegetation = Db2VegetationOptions.Default with { StartChloroplasts = 7321 };
            SetEnvironment(window, EnvironmentUpdate.Default with
            {
                BrownianMotion = 9f, ToroidalWorld = false, Vegetation = vegetation,
            });
            window.FindControl<ComboBox>("Backend")!.SelectedIndex = 2;
            window.FindControl<NumericUpDown>("WorldWidth")!.Value = 99_000;
            window.FindControl<NumericUpDown>("WorldHeight")!.Value = 88_000;
            window.FindControl<NumericUpDown>("PopulationCap")!.Value = 77_000;
            window.FindControl<NumericUpDown>("VegetablePopulationCap")!.Value = 66_000;
            window.FindControl<ComboBox>("Speed")!.SelectedIndex = 0;

            Invoke(window, "NewWorld_Click");

            AssertEnvironmentEqual(EnvironmentUpdate.Default with { Vegetation = vegetation }, GetEnvironment(window));
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
                Assert.True(window.FindControl<Button>("LoadSettingsButton")!.Focus());
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
        var fixture = OriginalSettingsFixture("Ledge Island.set");

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

    [Fact]
    public async Task OriginalWorldFlagsAndEmptyRosterReachTheNewWorldAndSaveFromSetup()
    {
        var expected = new Db2ShapeCreationOptions(Black: true, Transparent: true, SeeThrough: false, AbsorbShots: false);
        var legacy = LegacyDnaText.Decode(Db2SettingsCodec.Encode(new DesktopSettings
        {
            ShapeCreation = expected,
            ShapesVisible = true,
            FixedBotRadii = true,
            TiesEnabled = false,
            RobotFixingAllowed = false,
            AsexualReproductionAllowed = false,
        }));
        var marker = legacy.IndexOf("#DARWINBOTS_MODERN_SETTINGS_V1#", StringComparison.Ordinal);
        var storage = new SettingsWorkflowStorage
        {
            SettingsToOpen = Db2SettingsCodec.Decode(LegacyDnaText.Encode(legacy[..marker])),
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new SetupWindow(storage);
            WorldSetupOptions? created = null;
            window.WorldCreated += options => created = options;
            window.Show();
            try
            {
                Assert.Equal(2, window.Species.Count);
                window.FindControl<Button>("LoadSettingsButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => window.FindControl<TextBlock>("SetupStatus")!.Text == "SETTINGS LOADED");
                Assert.Empty(window.Species);
                window.FindControl<Button>("SaveSettingsButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => storage.SavedSettings is not null);
                var saved = storage.SavedSettings!;
                Assert.Equal(expected, saved.ShapeCreation);
                Assert.True(saved.ShapesVisible);
                Assert.True(saved.FixedBotRadii);
                Assert.False(saved.TiesEnabled);
                Assert.False(saved.RobotFixingAllowed);
                Assert.False(saved.AsexualReproductionAllowed);
                Assert.Empty(Db2SettingsCodec.Decode(Db2SettingsCodec.Encode(saved)).Species);

                window.FindControl<Button>("StartNewButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.NotNull(created);
                Assert.Empty(created.Species);
                Assert.Equal(expected, created.ShapeCreation);
                using var engine = new NativeEngineClient(created with { Backend = "Cpu" });
                var initial = engine.Snapshot();
                Assert.Equal(0, initial.Population);
                Assert.Empty(initial.Obstacles);
                Assert.False(initial.TiesEnabled);
                Assert.False(initial.RobotFixingAllowed);
                Assert.False(initial.AsexualReproductionAllowed);
                engine.ImportDna("start stop", [1_000, 1_000]);
                Assert.Equal(60f, Assert.Single(engine.Snapshot().RenderInstances).Radius);
                Assert.Equal(1, engine.AddNativeRandomObstacles(1));
                var shape = Assert.Single(engine.Snapshot().Obstacles);
                Assert.Equal(0xff000000u, shape.Color);
                Assert.Equal(0, shape.Opacity);
                Assert.Equal("opaque", shape.VisionPolicy);
                Assert.Equal("reflect", shape.ShotPolicy);

                window.FindControl<MenuItem>("NewWorldMenuItem")!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                window.FindControl<Button>("StartNewButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.Equal(2, created!.Species.Count);
                Assert.False(created.FixedBotRadii);
                Assert.True(created.TiesEnabled);
                Assert.True(created.RobotFixingAllowed);
                Assert.True(created.AsexualReproductionAllowed);
            }
            finally
            {
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task OriginalWallRowsFollowTheCurrentSetupFieldThroughSaveStartAndResume()
    {
        var fixture = OriginalSettingsFixture("Walls and Waste.set");
        var imported = SetupStorageService.DecodeSettings(fixture, File.ReadAllBytes(fixture));
        var storage = new SettingsWorkflowStorage { SettingsToOpen = imported };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new SetupWindow(storage);
            WorldSetupOptions? created = null;
            window.WorldCreated += options => created = options;
            window.Show();
            try
            {
                window.FindControl<Button>("LoadSettingsButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => window.FindControl<TextBlock>("SetupStatus")!.Text == "SETTINGS LOADED");
                var width = window.FindControl<NumericUpDown>("WorldWidth")!;
                var height = window.FindControl<NumericUpDown>("WorldHeight")!;
                var save = window.FindControl<Button>("SaveSettingsButton")!;
                async Task<DesktopSettings> SaveSettingsAsync()
                {
                    var previous = storage.SavedSettings;
                    save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    await WaitFor(() => !ReferenceEquals(previous, storage.SavedSettings));
                    return storage.SavedSettings!;
                }

                width.Value = 48_000;
                height.Value = 12_000;
                var resized = await SaveSettingsAsync();
                Assert.Equal(48_000f, resized.WorldWidth);
                Assert.Equal(12_000f, resized.WorldHeight);
                Assert.Equal(18, resized.Obstacles.Count);
                var wall = resized.Obstacles[0];
                Assert.Equal(6007.50, wall.Minimum[0], 2);
                Assert.Equal(1512.59, wall.Minimum[1], 2);
                Assert.Equal(750.00, wall.Maximum[0] - wall.Minimum[0], 2);
                Assert.Equal(5890.50, wall.Maximum[1] - wall.Minimum[1], 2);
                Assert.Equal(imported.Obstacles[0].Color, wall.Color);
                Assert.Equal(imported.Obstacles[0].Drift, wall.Drift);
                Assert.Equal(resized.ToJson(), (await SaveSettingsAsync()).ToJson());

                var encoded = LegacyDnaText.Decode(SetupStorageService.EncodeSettings("resized.set", resized));
                var marker = encoded.IndexOf("#DARWINBOTS_MODERN_SETTINGS_V1#", StringComparison.Ordinal);
                var originalFormat = Db2SettingsCodec.Decode(LegacyDnaText.Encode(encoded[..marker]));
                Assert.Equal(resized.WorldWidth, originalFormat.WorldWidth);
                Assert.Equal(resized.WorldHeight, originalFormat.WorldHeight);
                Assert.Equal(18, originalFormat.Obstacles.Count);
                Assert.Equal(6007.50, originalFormat.Obstacles[0].Minimum[0], 2);
                Assert.Equal(7403.09, originalFormat.Obstacles[0].Maximum[1], 2);

                width.Value = 32_000;
                height.Value = 24_000;
                var originalSize = await SaveSettingsAsync();
                Assert.Equal(imported.Obstacles[0].Minimum, originalSize.Obstacles[0].Minimum);
                Assert.Equal(imported.Obstacles[0].Maximum, originalSize.Obstacles[0].Maximum);

                width.Value = 16_000;
                height.Value = 36_000;
                window.FindControl<Button>("StartNewButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.NotNull(created);
                Assert.Equal(16_000f, created.WorldWidth);
                Assert.Equal(36_000f, created.WorldHeight);
                Assert.Equal(18, created.Obstacles.Count);
                Assert.Equal(2002.50, created.Obstacles[0].Minimum[0], 2);
                Assert.Equal(4537.78, created.Obstacles[0].Minimum[1], 2);
                using var engine = new NativeEngineClient(created with { Backend = "Cpu" });
                var active = engine.Snapshot().Obstacles;
                Assert.Equal(18, active.Count);
                Assert.Equal(created.Obstacles[0].Minimum, active[0].Minimum);
                Assert.Equal(created.Obstacles[0].Maximum, active[0].Maximum);
                Assert.Equal([0.5954616f, 0.6160923f], active[0].Drift);
                engine.Load(engine.Save());
                var restored = engine.Snapshot().Obstacles;
                Assert.Equal(18, restored.Count);
                Assert.Equal(active[0].Minimum, restored[0].Minimum);
                Assert.Equal(active[0].Maximum, restored[0].Maximum);
                Assert.Equal(active[0].Drift, restored[0].Drift);

                window.FindControl<MenuItem>("NewWorldMenuItem")!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                window.FindControl<Button>("StartNewButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.Empty(created!.Obstacles);
            }
            finally
            {
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Theory]
    [InlineData("modern.json")]
    [InlineData("modern.set")]
    public async Task ModernWallRowsKeepAbsoluteBoundsWhenTheSetupFieldChanges(string fileName)
    {
        var fixture = OriginalSettingsFixture("Walls and Waste.set");
        var original = SetupStorageService.DecodeSettings(fixture, File.ReadAllBytes(fixture));
        var reopened = SetupStorageService.DecodeSettings(fileName, SetupStorageService.EncodeSettings(fileName, original));
        Assert.NotNull(reopened.LegacySettingsFields);
        var storage = new SettingsWorkflowStorage { SettingsToOpen = reopened };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new SetupWindow(storage);
            WorldSetupOptions? created = null;
            window.WorldCreated += options => created = options;
            window.Show();
            try
            {
                window.FindControl<Button>("LoadSettingsButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => window.FindControl<TextBlock>("SetupStatus")!.Text == "SETTINGS LOADED");
                window.FindControl<NumericUpDown>("WorldWidth")!.Value = 48_000;
                window.FindControl<NumericUpDown>("WorldHeight")!.Value = 12_000;
                window.FindControl<Button>("SaveSettingsButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => storage.SavedSettings is not null);
                var saved = storage.SavedSettings!;
                Assert.Equal(48_000f, saved.WorldWidth);
                Assert.Equal(12_000f, saved.WorldHeight);
                Assert.Equal(18, saved.Obstacles.Count);
                Assert.Equal(original.Obstacles[0].Minimum, saved.Obstacles[0].Minimum);
                Assert.Equal(original.Obstacles[0].Maximum, saved.Obstacles[0].Maximum);

                window.FindControl<Button>("StartNewButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.NotNull(created);
                Assert.Equal(18, created.Obstacles.Count);
                Assert.Equal(original.Obstacles[0].Minimum, created.Obstacles[0].Minimum);
                Assert.Equal(original.Obstacles[0].Maximum, created.Obstacles[0].Maximum);
            }
            finally
            {
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task OriginalShapeDriftCanBeSavedInSetupBeforeStartingTheNativeWorld()
    {
        var fixture = OriginalSettingsFixture("Walls and Waste.set");
        var records = File.ReadAllLines(fixture);
        records[393] = "-37";
        records[394] = "#FALSE#";
        records[395] = "#TRUE#";
        var imported = SetupStorageService.DecodeSettings(fixture, LegacyDnaText.Encode(string.Join("\r\n", records)));
        var drift = new Db2ShapeDriftOptions(Vertical: true, Rate: -37);
        var storage = new SettingsWorkflowStorage { SettingsToOpen = imported };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new SetupWindow(storage);
            WorldSetupOptions? created = null;
            window.WorldCreated += options => created = options;
            window.Show();
            try
            {
                window.FindControl<Button>("LoadSettingsButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => window.FindControl<TextBlock>("SetupStatus")!.Text == "SETTINGS LOADED");
                window.FindControl<Button>("SaveSettingsButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => storage.SavedSettings is not null);
                Assert.Equal(drift, storage.SavedSettings!.ShapeDrift);
                Assert.Null(created);

                window.FindControl<Button>("StartNewButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.NotNull(created);
                Assert.Equal(drift, created.ShapeDrift);
                using var engine = new NativeEngineClient(created with { Backend = "Cpu" });
                using var save = JsonDocument.Parse(engine.Save().AsMemory(10));
                var nativeDrift = save.RootElement.GetProperty("config").GetProperty("shape_drift");
                Assert.False(nativeDrift.GetProperty("horizontal").GetBoolean());
                Assert.True(nativeDrift.GetProperty("vertical").GetBoolean());
                Assert.Equal(-37, nativeDrift.GetProperty("rate").GetInt32());
                Assert.Equal([0.5954616f, 0.6160923f], engine.Snapshot().Obstacles[0].Drift);
            }
            finally
            {
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static string OriginalSettingsFixture(string name)
    {
        for (DirectoryInfo? directory = new(AppContext.BaseDirectory); directory is not null; directory = directory.Parent)
        {
            var path = Path.Combine(directory.FullName, "Installer", "sets", name);
            if (File.Exists(path)) return path;
        }
        throw new FileNotFoundException($"Original settings fixture '{name}' was not found.");
    }

    [Fact]
    public async Task SetupLoadsSavesAndStartsWithTheOrdinaryRestartPreference()
    {
        var encoded = LegacyDnaText.Decode(Db2SettingsCodec.Encode(new DesktopSettings { OrdinaryRestartOnExtinction = true }));
        int marker = encoded.IndexOf("#DARWINBOTS_MODERN_SETTINGS_V1#", StringComparison.Ordinal);
        var storage = new SettingsWorkflowStorage
        {
            SettingsToOpen = Db2SettingsCodec.Decode(LegacyDnaText.Encode(encoded[..marker])),
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new SetupWindow(storage);
            WorldSetupOptions? created = null;
            window.WorldCreated += options => created = options;
            try
            {
                window.FindControl<Button>("LoadSettingsButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                var restart = window.FindControl<CheckBox>("OrdinaryRestartToggle")!;
                await WaitFor(() => restart.IsChecked == true);
                window.FindControl<Button>("SaveSettingsButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => storage.SavedSettings is not null);
                Assert.True(storage.SavedSettings!.OrdinaryRestartOnExtinction);
                Assert.Null(created);

                window.FindControl<Button>("StartNewButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.NotNull(created);
                Assert.True(created.EnableOrdinaryRestartOnStart);
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
    public string? SimulationPathToOpen { get; init; }
    public string? LatestAutosave { get; init; }

    public Task<IReadOnlyList<DesktopTextFile>> OpenDnaFilesAsync(TopLevel owner, string? mainDirectory = null, bool allowMultiple = true) => Task.FromResult<IReadOnlyList<DesktopTextFile>>([]);
    public Task<string?> PickSimulationPathAsync(TopLevel owner) => Task.FromResult(SimulationPathToOpen);
    public Task<byte[]?> OpenSimulationAsync(TopLevel owner) => Task.FromResult<byte[]?>(null);
    public Task<bool> SaveSimulationAsync(TopLevel owner, ReadOnlyMemory<byte> save) => Task.FromResult(true);
    public MutationProfileUpdate? SavedDnaMutationProfile { get; private set; }
    public Task<bool> SaveDnaAsync(TopLevel owner, string suggestedFileName, string dna, MutationProfileUpdate? mutationProfile,
            string? mainDirectory = null)
    {
        SavedDnaMutationProfile = mutationProfile;
        return Task.FromResult(true);
    }
    public Task<bool> SaveOrganismAsync(TopLevel owner, string suggestedFileName, Func<Stream, Task> writeAsync,
            string? mainDirectory = null) => Task.FromResult(true);
    public string? FindLatestAutosave() => LatestAutosave;
    public Task SaveAutosaveAsync(ReadOnlyMemory<byte> save) => Task.CompletedTask;
    public Task<DesktopSettings?> OpenSettingsAsync(TopLevel owner, string? mainDirectory = null) => Task.FromResult(SettingsToOpen);

    public Task<bool> SaveSettingsAsync(TopLevel owner, DesktopSettings settings, string? mainDirectory = null)
    {
        SavedSettings = settings;
        return Task.FromResult(true);
    }
}
