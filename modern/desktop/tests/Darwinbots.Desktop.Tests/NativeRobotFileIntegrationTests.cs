using System.Reflection;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Input.Raw;
using Avalonia.Interactivity;
using Avalonia.Threading;
using Darwinbots.Desktop.Controls;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class NativeRobotFileIntegrationTests
{
    [Theory]
    [InlineData(true)]
    [InlineData(false)]
    public async Task NativeRobotMutationRatesFreezeAndRestoreRunAndApplyToTheOpeningRobot(bool running)
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var setup = new WorldSetupOptions
            {
                Backend = "Cpu",
                MutationSettings = MutationSettingsUpdate.Default with { Enabled = false },
            };
            using var engine = new NativeEngineClient(setup);
            engine.ImportSpecies(new("Pair", "start stop", false, 0xff112233, 2, 1000, 0, false, 1)
                { MutationProfile = FileProfile() });
            await using var session = new SimulationSession(engine);
            var window = new MainWindow([], setup, new FixedFactory(engine), new AuditStorageService());
            try
            {
                DesktopTestApplication.UseSession(window, session);
                window.Show();
                Assert.NotNull(session.LatestSnapshot.NativeCycle);
                var robots = session.LatestSnapshot.Organisms.ToArray();
                Assert.Equal(2, robots.Length);
                var selected = robots[0];
                var other = robots[1];
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                var timer = Field<DispatcherTimer>(window, "_runTimer");
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var command = menu.FindCommandItem(LegacyCommandId.RobotMutationRates)!;

                if (running)
                {
                    Invoke(window, "Run_Click");
                    await WaitFor(() => session.LatestSnapshot.Tick > 0);
                }
                viewModel.Select(selected.Slot);
                await WaitFor(() => command.IsEnabled);
                command.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<MutationRatesWindow>().Any(tool => tool.IsVisible));
                var dialog = Assert.Single(window.OwnedWindows.OfType<MutationRatesWindow>());
                Assert.False(timer.IsEnabled);
                Assert.False(dialog.FindControl<Button>("CancelButton")!.IsVisible);
                var tick = session.LatestSnapshot.Tick;
                await Task.Delay(TimeSpan.FromMilliseconds(100));
                Assert.Equal(tick, session.LatestSnapshot.Tick);

                dialog.FindControl<NumericUpDown>("Mean")!.Value = 19m;
                viewModel.Select(other.Slot);
                if (running)
                    dialog.Close();
                else
                    dialog.FindControl<Button>("ApplyButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

                await WaitFor(() => session.LatestSnapshot.Organisms.Any(robot =>
                    robot.Slot == selected.Slot && robot.Generation == selected.Generation
                    && robot.MutationProfile.Point.Mean == 19) && timer.IsEnabled == running);
                var unchanged = Assert.Single(session.LatestSnapshot.Organisms,
                    robot => robot.Slot == other.Slot && robot.Generation == other.Generation);
                Assert.Equal(other.MutationProfile, unchanged.MutationProfile);
                if (running)
                    await WaitFor(() => session.LatestSnapshot.Tick > tick);
                else
                    Assert.Equal(tick, session.LatestSnapshot.Tick);
            }
            finally
            {
                SetField(window, "_session", null);
                if (window.IsVisible) window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task NativeGenomeExportsScoreLivingFamiliesAndRecordEachManualRemovalInOrder()
    {
        await using var session = new SimulationSession(new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu", PopulationCapacity = 16,
        }));
        await session.ImportDnaAsync("start\nstop", [500f, 500f]);
        var parent = Assert.Single(session.LatestSnapshot.Organisms);
        var parentKey = new OrganismKey(parent.Slot, parent.Generation);
        var childKey = await session.CloneAsync(parent.Slot, parent.Generation, [900f, 900f]);
        await session.SetOrganismEnergyAsync(parent.Slot, parent.Generation, 125);
        await session.SetOrganismEnergyAsync(childKey.Slot, childKey.Generation, 250);
        var save = await session.SaveAsync();

        var living = await session.CaptureLivingSnpAsync(50);
        var rows = Db2GenomeRecordFile.Decode(Db2GenomeRecordFile.EncodeGenomeRecords(living.Records, includeHeader: true));
        var parentRow = Assert.Single(rows, row => row.Id == parent.UniqueId);
        var childRow = Assert.Single(rows, row => row.ParentId == parent.UniqueId);
        Assert.Equal(2.0 * Math.Sqrt(parentRow.Energy + childRow.Energy), parentRow.Fitness, 10);
        Assert.Equal(Math.Sqrt(childRow.Energy), childRow.Fitness, 10);
        Assert.Equal(save, await session.SaveAsync());

        var removed = await session.RemoveRecordingDeathsAsync([childKey, parentKey],
            suppressDeathPoffs: true, excludeVegetables: false, weightPercent: 50);
        var deadRows = Db2GenomeRecordFile.Decode(Db2GenomeRecordFile.EncodeGenomeRecords(removed, includeHeader: true));
        Assert.Equal([childRow.Id, parentRow.Id], deadRows.Select(row => row.Id));
        Assert.Equal(childRow, deadRows[0]);
        Assert.Equal(parentRow with { Fitness = deadRows[1].Fitness }, deadRows[1]);
        Assert.Equal(Math.Sqrt(parentRow.Energy), deadRows[1].Fitness, 10);
        Assert.Empty(session.LatestSnapshot.Organisms);
    }

    [Fact]
    public async Task GenomeRowsKeepAbsoluteParentsAndBiologicalGenerationAfterSlotReuse()
    {
        await using var session = new SimulationSession(new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu", PopulationCapacity = 16,
        }));
        const string dna = "start\nstop";
        await session.ImportDnaAsync(dna, [500f, 500f]);
        var filler = Assert.Single(session.LatestSnapshot.Organisms);
        await session.RemoveAsync(filler.Slot, filler.Generation);
        await session.ImportDnaAsync(dna, [500f, 500f]);
        var parent = Assert.Single(session.LatestSnapshot.Organisms);
        var childKey = await session.CloneAsync(parent.Slot, parent.Generation, [900f, 900f]);
        var child = Assert.Single(session.LatestSnapshot.Organisms,
            robot => robot.Slot == childKey.Slot && robot.Generation == childKey.Generation);
        Assert.NotEqual((ulong)parent.Slot, parent.UniqueId);
        Assert.NotEqual(1u, childKey.Generation);

        await session.RemoveAsync(parent.Slot, parent.Generation);
        await session.ImportDnaAsync(dna, [1300f, 1300f]);
        var replacement = Assert.Single(session.LatestSnapshot.Organisms, robot => robot.Slot == parent.Slot);
        Assert.NotEqual(parent.Generation, replacement.Generation);
        Assert.NotEqual(parent.UniqueId, replacement.UniqueId);

        var living = await session.CaptureLivingGenomeRecordsAsync();
        var rows = Db2GenomeRecordFile.Decode(Db2GenomeRecordFile.EncodeLiving(living.Snapshot, living.Dna));
        var livingChild = Assert.Single(rows, row => row.Id == child.UniqueId);
        Assert.Equal(parent.UniqueId, livingChild.ParentId);
        Assert.Equal(1u, livingChild.Generation);
        var livingReplacement = Assert.Single(rows, row => row.Id == replacement.UniqueId);
        Assert.Equal(0UL, livingReplacement.ParentId);
        Assert.Equal(0u, livingReplacement.Generation);

        var removed = await session.RemoveWithDnaAsync([childKey], suppressDeathPoffs: false, excludeVegetables: false);
        var manualChild = Assert.Single(Db2GenomeRecordFile.Decode(LegacyRecordingSerializer.SerializeRemoved(
            removed.Snapshot, removed.Dna, includeHeader: true, format: LegacyRecordingFormat.Db2GenomeRecords)));
        Assert.Equal(livingChild, manualChild);
    }

    [Fact]
    public async Task NativeRobotInfoShowsWatchedPostDnaMemoryAndClearsItOnFocusLoadAndHide()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            const ushort address = 772;
            var setup = new WorldSetupOptions
            {
                Backend = "Cpu",
                MutationSettings = MutationSettingsUpdate.Default with { Enabled = false },
            };
            using var engine = new NativeEngineClient(setup);
            engine.ImportDna($"start 12345 {address} store 100 .strbody store stop", [500f, 500f]);
            engine.ImportDna($"start 23456 {address} store stop", [900f, 500f]);
            await using var session = new SimulationSession(engine);
            var window = new MainWindow([], setup, new FixedFactory(engine), new AuditStorageService());
            try
            {
                DesktopTestApplication.UseSession(window, session);
                window.Show();
                var robots = session.LatestSnapshot.Organisms;
                Assert.Equal(2, robots.Count);
                var first = new OrganismKey(robots[0].Slot, robots[0].Generation);
                var second = new OrganismKey(robots[1].Slot, robots[1].Generation);
                await session.ConfigurePlayerBotAsync(new(true, first, [new(65, address, -3000, Invert: true)]));
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                viewModel.Select(first);
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                menu.FindCommandItem(LegacyCommandId.RobotShowInfo)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<RobotDetailsWindow>().Any());
                var info = Assert.Single(window.OwnedWindows.OfType<RobotDetailsWindow>());
                info.SelectTab(RobotDetailsTab.Memory);
                var memory = Field<TextBox>(info, "_memory");
                var facts = Field<TextBlock>(info, "_facts");
                var watch = Field<CheckBox>(info, "_postDnaMemory");
                Assert.True(watch.IsVisible);
                Assert.False(watch.IsChecked);
                Assert.EndsWith(" 991:\t 0\t 0\t 0\t 0\t 0\t 0\t 0\t 0\t 0\t 0\r\n", memory.Text);

                await session.StepAsync();
                await WaitFor(() => memory.Text?.Contains("\t-3000\t", StringComparison.Ordinal) == true);
                Assert.Null(session.LatestSnapshot.NativePostDnaMemory);
                await session.SetOrganismMemoryAsync(second.Slot, second.Generation, address, -4321);
                var save = await session.SaveAsync();
                watch.IsChecked = true;
                Assert.Contains("\t-3000\t", memory.Text);
                await session.StepAsync();
                await WaitFor(() => memory.Text?.Contains("\t 12345\t", StringComparison.Ordinal) == true);
                Assert.Equal(-3000, session.LatestSnapshot.Organisms[0].Memory.Single(cell => cell.Address == address).Value);
                Assert.Equal(first, session.LatestSnapshot.NativePostDnaMemory!.Focus);

                info.SelectTab(RobotDetailsTab.Dna);
                await session.StepAsync();
                var final = session.LatestSnapshot.Organisms[0];
                var sample = Assert.IsType<NativePostDnaMemorySnapshot>(session.LatestSnapshot.NativePostDnaMemory);
                var early = Assert.IsType<OrganismSnapshot>(sample.Facts);
                Assert.Equal(first, sample.Focus);
                Assert.Null(sample.Memory);
                Assert.Equal(final.Age - 1, early.Age);
                Assert.Equal(final.Body - 10, early.Body);
                Assert.True(final.Energy <= early.Energy - 100);
                await WaitFor(() => facts.Text?.Contains($"AGE / GENER.   {early.Age:N0} /", StringComparison.Ordinal) == true);
                Assert.Contains($"ENERGY         {early.Energy:N0}", facts.Text);
                info.SelectTab(RobotDetailsTab.Memory);
                Assert.True(watch.IsChecked);
                Assert.Contains("\t-3000\t", memory.Text);
                Assert.Contains($"AGE / GENER.   {early.Age:N0} /", facts.Text);
                await session.StepAsync();
                await WaitFor(() => memory.Text?.Contains("\t 12345\t", StringComparison.Ordinal) == true);

                viewModel.Select(second);
                Assert.DoesNotContain("\t 12345\t", memory.Text);
                await session.StepAsync();
                await WaitFor(() => memory.Text?.Contains("\t 23456\t", StringComparison.Ordinal) == true);
                Assert.Equal(second, session.LatestSnapshot.NativePostDnaMemory!.Focus);

                var load = (Task)typeof(MainWindow).GetMethod("LoadSimulationAsync", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .Invoke(window, [save, false])!;
                await load;
                Assert.Null(session.LatestSnapshot.NativePostDnaMemory);
                Assert.Contains("\t-4321\t", memory.Text);
                await session.StepAsync();
                await WaitFor(() => memory.Text?.Contains("\t 23456\t", StringComparison.Ordinal) == true);

                info.Hide();
                await session.StepAsync();
                Assert.Null(session.LatestSnapshot.NativePostDnaMemory);
            }
            finally
            {
                SetField(window, "_session", null);
                if (window.IsVisible) window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task NativeRobotInfoShortDescriptionAppliesAndClearsOnlyTheStillFocusedRobot()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var setup = new WorldSetupOptions { Backend = "Cpu" };
            using var engine = new NativeEngineClient(setup);
            engine.ImportDna("start stop", [500f, 500f]);
            engine.ImportDna("start stop", [900f, 500f]);
            await using var session = new SimulationSession(engine);
            var window = new MainWindow([], setup, new FixedFactory(engine), new AuditStorageService());
            try
            {
                DesktopTestApplication.UseSession(window, session);
                window.Show();
                var robots = session.LatestSnapshot.Organisms;
                Assert.Equal(2, robots.Count);
                var first = new OrganismKey(robots[0].Slot, robots[0].Generation);
                var second = new OrganismKey(robots[1].Slot, robots[1].Generation);
                await session.SetRobotTagAsync(first, "Original");
                await session.SetRobotTagAsync(second, "Peer");
                var published = session.LatestSnapshot;
                Assert.Equal("Original", await session.ReadRobotTagAsync(first));
                Assert.Same(published, session.LatestSnapshot);
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                viewModel.Select(first);
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                menu.FindCommandItem(LegacyCommandId.RobotShowInfo)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<RobotDetailsWindow>().Any());
                var info = Assert.Single(window.OwnedWindows.OfType<RobotDetailsWindow>());
                var strip = Field<Border>(info, "_shortDescription");
                var label = Field<TextBlock>(info, "_shortDescriptionValue");
                Assert.False(strip.IsVisible);
                info.SelectTab(RobotDetailsTab.Dna);
                await WaitFor(() => label.Text == "Original");
                Assert.True(strip.IsVisible);

                var edit = await OpenDescriptionAsync();
                Assert.Equal("Original", Input(edit).Text);
                Input(edit).Text = "Café description";
                await FinishAsync(edit, "APPLY");
                Assert.Equal("Caf? description", label.Text);
                Assert.Equal("Peer", await session.ReadRobotTagAsync(second));
                info.SelectTab(RobotDetailsTab.Memory);
                Assert.True(strip.IsVisible);
                Assert.Equal("Caf? description", label.Text);

                edit = await OpenDescriptionAsync();
                Assert.Equal("Caf? description", Input(edit).Text);
                edit.KeyPress(Key.Escape, RawInputModifiers.None, PhysicalKey.Escape, null);
                await WaitFor(() => !Field<bool>(info, "_editingShortDescription"));
                Assert.True(info.Topmost);
                Assert.Equal(string.Empty, label.Text);
                Assert.Equal(string.Empty, await session.ReadRobotTagAsync(first));
                Assert.Equal("Peer", await session.ReadRobotTagAsync(second));

                edit = await OpenDescriptionAsync();
                Input(edit).Text = "Stale edit";
                viewModel.Select(second);
                await FinishAsync(edit, "APPLY");
                Assert.Equal(string.Empty, await session.ReadRobotTagAsync(first));
                Assert.Equal("Peer", await session.ReadRobotTagAsync(second));

                // Loading the same world can reuse every numeric identity.
                // The stale dialog must still belong to the old world.
                viewModel.Select(first);
                menu.FindCommandItem(LegacyCommandId.RobotShowInfo)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                edit = await OpenDescriptionAsync();
                Input(edit).Text = "Stale world edit";
                var saved = await session.SaveAsync();
                var load = (Task)typeof(MainWindow).GetMethod("LoadSimulationAsync", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .Invoke(window, [saved, false])!;
                await load;
                viewModel.Select(first);
                await FinishAsync(edit, "APPLY");
                Assert.Equal(string.Empty, await session.ReadRobotTagAsync(first));
                Assert.Equal("Peer", await session.ReadRobotTagAsync(second));
                Assert.Equal(0UL, session.LatestSnapshot.Tick);

                async Task<LegacyValueWindow> OpenDescriptionAsync()
                {
                    info.UpdateLayout();
                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    var center = strip.TranslatePoint(new Point(strip.Bounds.Width / 2, strip.Bounds.Height / 2), info);
                    Assert.True(center.HasValue);
                    for (var click = 0; click < 2; click++)
                    {
                        info.MouseDown(center.Value, MouseButton.Left, RawInputModifiers.None);
                        info.MouseUp(center.Value, MouseButton.Left, RawInputModifiers.None);
                    }
                    await WaitFor(() => info.OwnedWindows.OfType<LegacyValueWindow>().Any());
                    Assert.False(info.Topmost);
                    return Assert.Single(info.OwnedWindows.OfType<LegacyValueWindow>());
                }

                async Task FinishAsync(LegacyValueWindow dialog, string action)
                {
                    Assert.Single(dialog.GetVisualDescendants().OfType<Button>(), button => Equals(button.Content, action))
                        .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    await WaitFor(() => !Field<bool>(info, "_editingShortDescription"));
                    Assert.True(info.Topmost);
                }

                static TextBox Input(LegacyValueWindow dialog) =>
                    Assert.Single(dialog.GetVisualDescendants().OfType<TextBox>());
            }
            finally { window.Close(); }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task PickedSidecarProfileReachesOnlyItsActualNativeSpeciesAndSelectedExport()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var setup = new WorldSetupOptions { Backend = "Cpu" };
            using var engine = new NativeEngineClient(setup);
            var unrelated = MutationProfileUpdate.Default with { PointValueChangePercent = 91 };
            engine.ImportSpecies(new("Unrelated", "start stop", false, 0xff112233, 1, 1000, 0, false, 1)
                { MutationProfile = unrelated });
            var global = engine.Snapshot().MutationSettings.Profile;
            await using var session = new SimulationSession(engine);
            await session.RefreshSnapshotAsync();
            var profile = FileProfile();
            const string raw = "' é source body\nstart stop\r\n";
            var storage = new AuditStorageService
            {
                DnaFiles = [new("Café.txt", raw, Path.Combine(Path.GetTempPath(), "Café.txt"), profile)],
            };
            var window = new MainWindow([], setup, new FixedFactory(engine), storage);
            try
            {
                DesktopTestApplication.UseSession(window, session);
                window.Show();
                Invoke(window, "Import_Click");
                await WaitFor(() => session.LatestSnapshot.Organisms.Count == 2);
                var snapshot = session.LatestSnapshot;
                var imported = Assert.Single(snapshot.Organisms.Where(value => snapshot.Species[(int)value.Species].Name == "Café"));
                var other = Assert.Single(snapshot.Organisms.Where(value => value.Slot != imported.Slot));
                Assert.Equal(profile, imported.MutationProfile);
                Assert.Equal(unrelated, other.MutationProfile);
                Assert.Equal(global, snapshot.MutationSettings.Profile);
                Assert.Equal(raw, engine.ExportSpeciesTemplateDna(imported.Species));
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                viewModel.Update(snapshot);
                viewModel.Select(imported.Slot);
                var saved = (Task)typeof(MainWindow).GetMethod("SaveSelectedDnaCommandAsync", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .Invoke(window, null)!;
                await saved;
                Assert.Equal(profile, storage.SavedDnaMutationProfile);
                Assert.Contains("start", storage.SavedDna);
                Assert.DoesNotContain('\n', storage.SavedDna!.Replace("\r\n", "", StringComparison.Ordinal));
                Assert.Equal(0ul, session.LatestSnapshot.Tick);
            }
            finally
            {
                SetField(window, "_session", null);
                if (window.IsVisible) window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task SetupKeepsPickedProfileAndOriginalSourcePathOnTheSpeciesDefinition()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var profile = FileProfile();
            var path = Path.Combine(Path.GetTempPath(), "picked-native.txt");
            var storage = new AuditStorageService { DnaFiles = [new("picked-native.txt", "start stop", path, profile)] };
            var window = new SetupWindow(storage);
            try
            {
                window.Show();
                window.Species.Clear();
                Invoke(window, "AddDna_Click");
                await WaitFor(() => window.Species.Count == 1);
                var row = Assert.Single(window.Species);
                var species = row.ToImport(0);
                Assert.Equal(profile, species.MutationProfile);
                Assert.Equal(path, species.DnaPath);
            }
            finally { if (window.IsVisible) window.Close(); }
        }, CancellationToken.None);
    }

    [Fact]
    public void OriginalSettingsProfileRemainsAuthoritativeOverAnAdjacentSidecar()
    {
        var directory = Path.Combine(Path.GetTempPath(), "db-mrate-set-authority-" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(directory);
            var path = Path.Combine(directory, "original.set");
            var expected = FileProfile();
            var settings = new DesktopSettings
            {
                Species = [new("Source.txt", "' raw\nstart stop\r\n", false, 0xff112233, 1, 1000, 0, false, 1)
                    { MutationProfile = expected }],
            };
            var bytes = SetupStorageService.EncodeSettings(path, settings);
            var decoded = SetupStorageService.DecodeSettings(path, bytes);
            var dnaPath = Assert.Single(decoded.Species).DnaPath!;
            File.WriteAllBytes(Path.ChangeExtension(dnaPath, ".mrate"), Db2MutationRateCodec.Encode(
                expected with { PointValueChangePercent = 99 }));
            var text = LegacyDnaText.Decode(bytes);
            var nativeOnly = LegacyDnaText.Encode(text[..text.IndexOf("#DARWINBOTS_MODERN_SETTINGS_V1#", StringComparison.Ordinal)]);
            var native = Assert.Single(SetupStorageService.DecodeSettings(path, nativeOnly).Species);
            Assert.Equal(expected, native.MutationProfile);
            Assert.Equal("' raw\nstart stop\r\n", native.Dna);
        }
        finally { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task ExplicitEvolutionFileCarriesItsSidecarThroughTheNativeConfigAndSavedSnapshot()
    {
        var directory = Path.Combine(Path.GetTempPath(), "db-mrate-evolution-" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(directory);
            var path = Path.Combine(directory, "Source.txt");
            const string raw = "' source kept intact\nstart stop\r\n";
            var profile = FileProfile();
            await Db2RobotFileStorage.WriteAsync(path, raw, profile);
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var setup = new WorldSetupOptions { Backend = "Cpu" };
                using var engine = new NativeEngineClient(setup);
                var global = engine.Snapshot().MutationSettings.Profile;
                var window = new MainWindow([], setup, new FixedFactory(engine), new AuditStorageService());
                try
                {
                    SetField(window, "_globalSettings", new GlobalSettings
                    {
                        Evolution = new GlobalEvolutionSettings { Enabled = true, Mode = EvolutionMode.SimpleSurvival, SourceBot = path },
                    });
                    var config = LocalModeConfig.Default with { Kind = LocalModeKind.SimpleSurvival };
                    var resolve = (Task<LocalModeConfig>)typeof(MainWindow)
                        .GetMethod("ResolveStagedEvolutionAsync", BindingFlags.Instance | BindingFlags.NonPublic)!.Invoke(window, [config])!;
                    var resolved = await resolve;
                    Assert.Equal(raw, resolved.StagedEvolution!.SourceDna);
                    Assert.Equal(profile, resolved.StagedEvolution.SourceMutationProfile);
                    engine.ConfigureLocalMode(resolved);
                    Assert.Equal(profile, engine.Snapshot().LocalMode.Configuration!.StagedEvolution!.SourceMutationProfile);
                    var saved = engine.Save();
                    engine.Load(saved);
                    Assert.Equal(profile, engine.Snapshot().LocalMode.Configuration!.StagedEvolution!.SourceMutationProfile);
                    Assert.Equal(global, engine.Snapshot().MutationSettings.Profile);
                    Assert.Equal(0ul, engine.Snapshot().Tick);
                }
                finally { if (window.IsVisible) window.Close(); }
            }, CancellationToken.None);
        }
        finally { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task InvalidNativeSnapshotTextIsRejectedBeforeTheStorageDestinationIsOpened()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var snapshot = RecordingEngineClient.CreateSnapshot() with
            {
                Species = [new("Unassigned", false, 0xff112233, 0, false), new("🙂", false, 0xff445566, 0, false)],
            };
            var engine = new RecordingEngineClient(snapshot);
            await using var session = new SimulationSession(engine);
            await session.RefreshSnapshotAsync();
            var storage = new AuditStorageService();
            var window = new MainWindow([], new WorldSetupOptions(), new FixedFactory(engine), storage);
            try
            {
                DesktopTestApplication.UseSession(window, session);
                var save = (Task)typeof(MainWindow).GetMethod("SaveLivingSnapshotAsync", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .Invoke(window, null)!;
                await Assert.ThrowsAsync<InvalidDataException>(() => save);
                Assert.Null(storage.SavedLivingSnapshot);
            }
            finally { SetField(window, "_session", null); if (window.IsVisible) window.Close(); }
        }, CancellationToken.None);
    }

    private static MutationProfileUpdate FileProfile() => MutationProfileUpdate.Default with
    {
        Point = new(123.5, 7, 0.5), CopyError = new(765.25, -3, 0.25),
        PointValueChangePercent = 13, CopyErrorValueChangePercent = 27,
    };
    private static void Invoke(object target, string method) => target.GetType()
        .GetMethod(method, BindingFlags.Instance | BindingFlags.NonPublic)!.Invoke(target, [null, new RoutedEventArgs()]);
    private static T Field<T>(object target, string name) => (T)target.GetType()
        .GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(target)!;
    private static void SetField(object target, string name, object? value) => target.GetType()
        .GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)!.SetValue(target, value);
    private static async Task WaitFor(Func<bool> condition)
    {
        for (var attempt = 0; attempt < 200; attempt++)
        {
            if (condition()) return;
            await Task.Delay(10);
        }
        Assert.True(condition(), "Native robot-file integration did not complete.");
    }
    private sealed class FixedFactory(IEngineClient engine) : IEngineClientFactory
    {
        public IEngineClient Create(WorldSetupOptions setup) => engine;
    }
}
