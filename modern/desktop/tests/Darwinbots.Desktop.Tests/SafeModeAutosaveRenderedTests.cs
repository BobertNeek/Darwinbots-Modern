using System.Diagnostics;
using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using Avalonia.Threading;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class SafeModeAutosaveRenderedTests
{
    [Fact]
    public async Task FifteenCycleLimitWaitsAfterCheckpointAndCapsFastAndTurbo()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var timeout = TimeSpan.FromSeconds(8);
            var checkpointStarted = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
            var releaseCheckpoint = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
            var storage = new AuditStorageService
            {
                BeforeAutosaveWrite = async () =>
                {
                    checkpointStarted.TrySetResult();
                    await releaseCheckpoint.Task.WaitAsync(timeout);
                },
            };
            var factory = new AuditEngineClientFactory(encodeTickInSave: true);
            var window = new MainWindow([], Setup(true), factory, storage);
            var secondCycle = new TaskCompletionSource<(uint Count, long Started)>(TaskCreationOptions.RunContinuationsAsynchronously);
            var turboCycle = new TaskCompletionSource<uint>(TaskCreationOptions.RunContinuationsAsynchronously);
            var uncheckedCycle = new TaskCompletionSource<uint>(TaskCreationOptions.RunContinuationsAsynchronously);
            var releaseSecond = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
            var releaseTurbo = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
            try
            {
                Show(window);
                await Ready(window);
                var engine = Assert.Single(factory.Clients);
                var session = Field<SimulationSession>(window, "_session");
                var timer = Field<DispatcherTimer>(window, "_runTimer");
                var speed = Named<ComboBox>(window, "RuntimeSpeed");
                void Click(string name) => Named<Button>(window, name).RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

                engine.NativeCycle = 1999;
                await session.RefreshSnapshotAsync();
                engine.AdvanceNativeCycle = (count, cycle) =>
                {
                    switch (cycle)
                    {
                        case 2000:
                            secondCycle.TrySetResult((count, Stopwatch.GetTimestamp()));
                            releaseSecond.Task.WaitAsync(timeout).GetAwaiter().GetResult();
                            break;
                        case 2001:
                            turboCycle.TrySetResult(count);
                            releaseTurbo.Task.WaitAsync(timeout).GetAwaiter().GetResult();
                            break;
                        case 2002:
                            uncheckedCycle.TrySetResult(count);
                            break;
                    }
                    return cycle + checked((int)count);
                };

                speed.SelectedIndex = 1;
                Click("ToolbarSlot10");
                Assert.Equal(3, speed.SelectedIndex);
                var maximumBatch = Field<uint>(window, "_maximumBatch");
                Click("ToolbarSlot09");
                Assert.False(timer.IsEnabled);
                Assert.Equal("15 CPS ON", Named<Button>(window, "ToolbarSlot09").Content);
                Assert.Equal(TimeSpan.FromMilliseconds(67), timer.Interval);
                Click("RunButton");
                await checkpointStarted.Task.WaitAsync(timeout);
                Assert.Equal(2000, session.LatestSnapshot.NativeCycle);
                Assert.Empty(storage.AutosaveWrites);
                await Task.Delay(25);
                var checkpointReleased = Stopwatch.GetTimestamp();
                releaseCheckpoint.SetResult();

                var second = await secondCycle.Task.WaitAsync(timeout);
                Assert.Equal(1u, second.Count);
                Assert.True(Stopwatch.GetElapsedTime(checkpointReleased, second.Started) >= TimeSpan.FromMilliseconds(60));
                Assert.Equal(1ul, BitConverter.ToUInt64(Assert.Single(storage.AutosaveWrites)));
                Click("PauseButton");
                releaseSecond.SetResult();
                await WaitFor(() => !Field<bool>(window, "_tickInFlight"));
                Assert.False(timer.IsEnabled);
                Assert.Equal(2001, session.LatestSnapshot.NativeCycle);

                Click("ToolbarSlot09");
                Assert.Equal("15 CPS OFF", Named<Button>(window, "ToolbarSlot09").Content);
                Assert.Equal(TimeSpan.FromMilliseconds(1), timer.Interval);
                Click("ToolbarSlot09");
                Assert.False(timer.IsEnabled);
                Click("TurboButton");
                Assert.Equal(1u, await turboCycle.Task.WaitAsync(timeout));
                Click("PauseButton");
                releaseTurbo.SetResult();
                await WaitFor(() => !Field<bool>(window, "_tickInFlight"));
                Assert.False(timer.IsEnabled);
                Assert.Equal(3, speed.SelectedIndex);
                Assert.Equal(maximumBatch, Field<uint>(window, "_maximumBatch"));
                Assert.Equal("FAST ON", Named<Button>(window, "ToolbarSlot10").Content);

                Click("ToolbarSlot09");
                Click("RunButton");
                Assert.Equal(maximumBatch, await uncheckedCycle.Task.WaitAsync(timeout));
                Click("PauseButton");
                await WaitFor(() => !Field<bool>(window, "_tickInFlight"));
            }
            finally
            {
                releaseCheckpoint.TrySetResult();
                releaseSecond.TrySetResult();
                releaseTurbo.TrySetResult();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task NativeBoundaryWritesActualRecoveryStateWithoutUsingTheMonotonicTickAsOriginalCycle()
    {
        var path = Path.Combine(Path.GetTempPath(), $"db24832-native-cycle-backup-{Guid.NewGuid():N}.db3s");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var options = Setup(true) with { PopulationCapacity = 16, Species = [], BrownianMotion = 0 };
                var engine = new NativeEngineClient(options);
                // Persist a world just before native cycle2000 while retaining
                // scheduling Tick0, avoiding thousands of fixture-only updates.
                engine.Load(WithNativeCycle(engine.Save(), 1999));
                await using var session = new SimulationSession(engine);
                var storage = new AuditStorageService { LatestAutosave = path, PersistAutosaves = true };
                var window = new MainWindow([], options, new AuditEngineClientFactory(), storage);
                try
                {
                    DesktopTestApplication.UseSession(window, session);
                    Show(window);
                    const string commandStatus = "TURBO · RENDERING SUSPENDED";
                    ViewModel(window).Status = commandStatus;
                    Assert.Equal(0ul, session.LatestSnapshot.Tick);
                    Assert.Equal(1999, session.LatestSnapshot.NativeCycle);

                    await InvokeTask(window, "AdvanceSimulationAsync", 2u);

                    var backup = Assert.Single(storage.AutosaveWrites);
                    Assert.Equal(backup, await File.ReadAllBytesAsync(path));
                    using var saved = System.Text.Json.JsonDocument.Parse(backup.AsMemory(10));
                    Assert.Equal(2000, saved.RootElement.GetProperty("native_cycle").GetInt32());
                    Assert.Equal(1ul, saved.RootElement.GetProperty("tick").GetUInt64());
                    Assert.Equal(2ul, session.LatestSnapshot.Tick);
                    Assert.Equal(2001, session.LatestSnapshot.NativeCycle);
                    Assert.Equal(1ul, Field<OfflineDiagnosticRecovery>(window, "_diagnostics").LastCheckpointTick);
                    Assert.Equal(commandStatus, ViewModel(window).Status);
                }
                finally { window.Close(); }
            }, CancellationToken.None);
        }
        finally { if (File.Exists(path)) File.Delete(path); }
    }

    [Fact]
    public async Task RestartCapableBatchObservesResetAndCanSaveTheSameNativeBoundaryInTwoWorlds()
    {
        var path = Path.Combine(Path.GetTempPath(), $"db24832-reset-cycle-backup-{Guid.NewGuid():N}.db3s");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var factory = new AuditEngineClientFactory(encodeTickInSave: true);
                var storage = new AuditStorageService { LatestAutosave = path, PersistAutosaves = true };
                var window = new MainWindow([], Setup(true), factory, storage);
                try
                {
                    Show(window);
                    await Ready(window);
                    var engine = factory.Clients.Single();
                    engine.NativeCycle = 1999;
                    engine.LocalModeForRead = LocalModeSnapshot.Idle with
                    {
                        Status = LocalModeStatus.Running, Kind = LocalModeKind.SimpleSurvival,
                        Configuration = LocalModeConfig.Default with { Kind = LocalModeKind.SimpleSurvival, RestartOnExtinction = true },
                    };
                    int updates = 0;
                    engine.AdvanceNativeCycle = (count, _) =>
                    {
                        Assert.Equal(1u, count);
                        return ++updates switch { 1 => 2000, 2 => -1, 3 => 0, _ => 2000 };
                    };
                    await Field<SimulationSession>(window, "_session").RefreshSnapshotAsync();

                    await InvokeTask(window, "AdvanceSimulationAsync", 3u);

                    Assert.Equal(new ulong[] { 1 }, storage.AutosaveWrites.Select(bytes => BitConverter.ToUInt64(bytes)));
                    Assert.Equal(0, Field<SimulationSession>(window, "_session").LatestSnapshot.NativeCycle);
                    // The next attempt approaches its own original cycle2000.
                    engine.NativeCycle = 1999;
                    await Field<SimulationSession>(window, "_session").RefreshSnapshotAsync();
                    await InvokeTask(window, "AdvanceSimulationAsync", 1u);

                    Assert.Equal(new ulong[] { 1, 4 }, storage.AutosaveWrites.Select(bytes => BitConverter.ToUInt64(bytes)));
                    Assert.Equal(4ul, BitConverter.ToUInt64(await File.ReadAllBytesAsync(path)));
                    Assert.Equal(4ul, Field<OfflineDiagnosticRecovery>(window, "_diagnostics").LastCheckpointTick);
                    Assert.Equal(new[] { "Tick:1", "Tick:1", "Tick:1", "Tick:1" }, engine.Calls.Where(value => value.StartsWith("Tick:", StringComparison.Ordinal)));
                }
                finally { window.Close(); }
            }, CancellationToken.None);
        }
        finally { if (File.Exists(path)) File.Delete(path); }
    }

    private static byte[] WithNativeCycle(byte[] save, int cycle)
    {
        var payload = System.Text.Json.Nodes.JsonNode.Parse(System.Text.Encoding.UTF8.GetString(save, 10, save.Length - 10))!;
        payload["native_cycle"] = cycle;
        var json = System.Text.Json.JsonSerializer.SerializeToUtf8Bytes(payload);
        var framed = new byte[10 + json.Length];
        save.AsSpan(0, 6).CopyTo(framed);
        System.Buffers.Binary.BinaryPrimitives.WriteUInt32LittleEndian(framed.AsSpan(6, 4), (uint)json.Length);
        json.CopyTo(framed.AsSpan(10));
        return framed;
    }

    [Fact]
    public async Task OrdinaryExtinctionSavesTheCompletedWorldBeforeRestartAndResumesAPendingCheckpoint()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var options = Setup(true) with { PopulationCapacity = 16, BrownianMotion = 0 };
            var storage = new AuditStorageService();
            var window = new MainWindow([], options, NativeEngineClientFactory.Instance, storage);
            try
            {
                Show(window);
                await Ready(window);
                var session = Field<SimulationSession>(window, "_session");
                Activate(Named<LegacyMenuBar>(window, "HistoricMenuBar"), LegacyCommandId.EditRestartAndLeagues);
                var ordinary = Named<CheckBox>(window, "OrdinaryRestartToggle");
                var competitive = Named<CheckBox>(window, "ModeRestartOnExtinction");
                var competitiveSetting = competitive.IsChecked;
                Assert.True(ordinary.IsEnabled);
                Assert.False(ordinary.IsChecked);
                ordinary.IsChecked = true;
                ordinary.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => session.LatestSnapshot.OrdinaryRestartOnExtinction);
                Assert.Equal(competitiveSetting, competitive.IsChecked);

                var oldRobot = Assert.Single(session.LatestSnapshot.Organisms);
                await session.RemoveAsync(oldRobot.Slot, oldRobot.Generation);
                await InvokeTask(window, "LoadSimulationAsync", WithNativeCycle(await session.SaveAsync(), 1999));
                await InvokeTask(window, "AdvanceSimulationAsync", 1u);

                var checkpoint = Assert.Single(storage.AutosaveWrites);
                Assert.Equal(1ul, session.LatestSnapshot.OrdinaryRestartCount);
                Assert.False(session.LatestSnapshot.OrdinaryRestartPending);
                Assert.Single(session.LatestSnapshot.Organisms);

                await InvokeTask(window, "LoadSimulationAsync", checkpoint);
                Assert.Equal(2000, session.LatestSnapshot.NativeCycle);
                Assert.True(session.LatestSnapshot.OrdinaryRestartPending);
                Assert.Equal(0ul, session.LatestSnapshot.OrdinaryRestartCount);
                Assert.Empty(session.LatestSnapshot.Organisms);

                await InvokeTask(window, "AdvanceSimulationAsync", 1u);
                Assert.Equal(2001, session.LatestSnapshot.NativeCycle);
                Assert.Equal(1ul, session.LatestSnapshot.OrdinaryRestartCount);
                Assert.False(session.LatestSnapshot.OrdinaryRestartPending);
                Assert.Single(session.LatestSnapshot.Organisms);
                Assert.Single(storage.AutosaveWrites);
            }
            finally { window.Close(); }
        }, CancellationToken.None);
    }

    [Fact]
    public void OriginalSimulationSourcePathsFollowLegacyRootsAndLocalSaveFallbacks()
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-restart-source-paths-");
        try
        {
            var sim = Path.Combine(profile.CreateSubdirectory("Saves").FullName, "fixture.sim");
            var main = profile.CreateSubdirectory("main");
            var robots = main.CreateSubdirectory("Robots");
            var literal = profile.CreateSubdirectory("custom");
            const string name = "fixture-specific-café.txt";
            const string savedDna = "' literal café\r\nstart stop";
            const string rootDna = "' configured\r\nstart stop";
            const string siblingDna = "' explicit save directory\r\nstart stop";
            File.WriteAllBytes(Path.Combine(literal.FullName, name), LegacyDnaText.Encode(savedDna));
            File.WriteAllBytes(Path.Combine(robots.FullName, name), LegacyDnaText.Encode(rootDna));
            File.WriteAllBytes(Path.Combine(Path.GetDirectoryName(sim)!, name), LegacyDnaText.Encode(siblingDna));

            string? Resolve(string directory) => SetupStorageService.ResolveNativeRestartSource(
                new NativeRestartSource(1, name, directory), sim, main.FullName);
            Assert.Equal(savedDna, Resolve(literal.FullName));
            Assert.Equal(rootDna, Resolve(@"&#\Robots"));
            Assert.Equal(rootDna, Resolve(""));
            Assert.Equal(siblingDna, Resolve("relocated"));
            Assert.Equal(siblingDna, Resolve("https://example.invalid/robots"));
            Assert.Equal(siblingDna, Resolve(@"\\example.invalid\robots"));
            File.WriteAllBytes(Path.Combine(literal.FullName, name), [0xef, 0xbb, 0xbf, 0xff]);
            Assert.Equal(siblingDna, Resolve(literal.FullName));
            Assert.Null(SetupStorageService.ResolveNativeRestartSource(new(2, "../" + name, ""), sim, main.FullName));
        }
        finally { profile.Delete(recursive: true); }
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public async Task ImportedOriginalSimulationRestartsThroughItsPackagedSourceRoster(bool usePlayerPicker)
    {
        const string relative = "modern/engine/tests/fixtures/db24832-original-user.sim";
        var repository = new DirectoryInfo(AppContext.BaseDirectory);
        while (repository is not null && !File.Exists(Path.Combine(repository.FullName, relative)))
            repository = repository.Parent;
        Assert.NotNull(repository);
        var profile = Directory.CreateTempSubdirectory("darwinbots-restart-sim-");
        try
        {
            var path = Path.Combine(profile.FullName, "archived.sim");
            File.Copy(Path.Combine(repository.FullName, relative), path);
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var options = Setup(false);
                if (!usePlayerPicker) options = options with { Species = [], LoadSavePath = path };
                var storage = AvaloniaDesktopStorageService.Instance;
                var window = new MainWindow([], options, NativeEngineClientFactory.Instance, storage);
                try
                {
                    Show(window);
                    if (usePlayerPicker)
                    {
                        await Ready(window);
                        var selected = RestartStorageProxy.Create<IStorageFile>((method, _) => method.Name switch
                        {
                            "get_Path" => new Uri(path),
                            nameof(IStorageFile.OpenReadAsync) => Task.FromResult<Stream>(File.OpenRead(path)),
                            _ => throw new NotSupportedException(method.Name),
                        });
                        var picker = RestartStorageProxy.Create<IStorageProvider>((method, _) => method.Name switch
                        {
                            nameof(IStorageProvider.OpenFilePickerAsync) => Task.FromResult<IReadOnlyList<IStorageFile>>([selected]),
                            _ => throw new NotSupportedException(method.Name),
                        });
                        typeof(TopLevel).GetField("_storageProvider", BindingFlags.Instance | BindingFlags.NonPublic)!
                            .SetValue(window, picker);
                        Activate(Named<LegacyMenuBar>(window, "HistoricMenuBar"), LegacyCommandId.FileLoadSimulation);
                    }
                    await WaitFor(() => ViewModel(window).Status == "LOADED");
                    var session = Field<SimulationSession>(window, "_session");
                    var imported = session.LatestSnapshot;
                    Assert.Equal(281, imported.NativeCycle);
                    Assert.Equal(196, imported.Population);
                    Assert.True(imported.OrdinaryRestartAvailable);
                    Assert.False(imported.OrdinaryRestartOnExtinction);
                    Assert.Contains(imported.Species, species => species.Name == "Alga_Minimalis_Chloroplastus.txt");
                    Assert.Contains(imported.Species, species => species.Name == "The One (1G)(Shen)-23.04.05.txt");

                    Activate(Named<LegacyMenuBar>(window, "HistoricMenuBar"), LegacyCommandId.EditRestartAndLeagues);
                    var restart = Named<CheckBox>(window, "OrdinaryRestartToggle");
                    Assert.True(restart.IsEnabled);
                    restart.IsChecked = true;
                    restart.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    await WaitFor(() => session.LatestSnapshot.OrdinaryRestartOnExtinction);

                    var checkpoint = await session.SaveAsync();
                    File.Delete(path);
                    await InvokeTask(window, "LoadSimulationAsync", checkpoint);
                    var resumed = session.LatestSnapshot;
                    Assert.Equal(196, resumed.Population);
                    Assert.True(resumed.OrdinaryRestartOnExtinction);
                    var original = resumed.Organisms.Select(robot => new OrganismKey(robot.Slot, robot.Generation)).ToArray();
                    await session.RemoveManyAsync(original, suppressDeathPoffs: true);
                    Assert.Empty(session.LatestSnapshot.Organisms);
                    await InvokeTask(window, "AdvanceSimulationAsync", 1u);

                    var restarted = session.LatestSnapshot;
                    Assert.Equal(1ul, restarted.OrdinaryRestartCount);
                    Assert.False(restarted.OrdinaryRestartPending);
                    Assert.Equal(5, restarted.Organisms.Count(robot => !robot.Vegetable));
                    Assert.Equal(5, restarted.Organisms.Count(robot => robot.Vegetable));
                }
                finally { window.Close(); }
            }, CancellationToken.None);
        }
        finally { profile.Delete(recursive: true); }
    }

    [Fact]
    public async Task OrdinaryExtinctionRestartsFromTheOriginalSetPreference()
    {
        var directory = Directory.CreateTempSubdirectory("darwinbots-restart-set-");
        try
        {
            var setupOptions = Setup(false);
            var species = Assert.Single(setupOptions.Species) with { DnaPath = Path.Combine(directory.FullName, "restart-bot.txt") };
            await File.WriteAllBytesAsync(species.DnaPath!, LegacyDnaText.Encode(species.Dna));
            var settings = DesktopSettings.FromWorldSetup(setupOptions with
            {
                OrdinaryRestartOnExtinction = true,
                Species = [species],
            });
            var encoded = LegacyDnaText.Decode(Db2SettingsCodec.Encode(settings));
            const string marker = "#DARWINBOTS_MODERN_SETTINGS_V1#";
            var settingsPath = Path.Combine(directory.FullName, "restart.set");
            var originalSet = LegacyDnaText.Encode(encoded[..encoded.IndexOf(marker, StringComparison.Ordinal)]);
            await File.WriteAllBytesAsync(settingsPath, originalSet);
            var storage = new SettingsWorkflowStorage
            {
                SettingsToOpen = SetupStorageService.DecodeSettings(settingsPath, await File.ReadAllBytesAsync(settingsPath)),
            };

            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var setup = new SetupWindow(storage);
                WorldSetupOptions? created = null;
                setup.WorldCreated += value => created = value;
                try
                {
                    Show(setup);
                    Named<Button>(setup, "LoadSettingsButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    await WaitFor(() => Named<TextBlock>(setup, "SetupStatus").Text == "SETTINGS LOADED");
                    Assert.True(Named<CheckBox>(setup, "OrdinaryRestartToggle").IsChecked);
                    Named<Button>(setup, "SaveSettingsButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    await WaitFor(() => storage.SavedSettings is not null);
                    Assert.True(storage.SavedSettings!.OrdinaryRestartOnExtinction);
                    Named<Button>(setup, "StartNewButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    Assert.NotNull(created);
                    Assert.True(created.EnableOrdinaryRestartOnStart);
                }
                finally { setup.Close(); }

                var window = new MainWindow([], created with { Backend = "Cpu" },
                    NativeEngineClientFactory.Instance, new AuditStorageService());
                try
                {
                    Show(window);
                    await Ready(window);
                    var session = Field<SimulationSession>(window, "_session");
                    var before = session.LatestSnapshot;
                    Assert.True(before.OrdinaryRestartOnExtinction);
                    Assert.Equal(LocalModeStatus.Idle, before.LocalMode.Status);
                    var oldRobot = Assert.Single(before.Organisms);
                    await session.RemoveAsync(oldRobot.Slot, oldRobot.Generation);
                    await InvokeTask(window, "AdvanceSimulationAsync", 1u);

                    var after = session.LatestSnapshot;
                    Assert.Equal(1ul, after.OrdinaryRestartCount);
                    Assert.False(after.OrdinaryRestartPending);
                    Assert.Equal(LocalModeStatus.Idle, after.LocalMode.Status);
                    var replacement = Assert.Single(after.Organisms);
                    Assert.NotEqual(oldRobot.UniqueId, replacement.UniqueId);
                    Assert.Equal(oldRobot.Species, replacement.Species);
                    Assert.Equal(1000, replacement.Energy);
                }
                finally { window.Close(); }
            }, CancellationToken.None);
        }
        finally { directory.Delete(recursive: true); }
    }

    [Fact]
    public async Task SetupGlobalAndRecordingDialogsKeepExplicitDisabledPreferenceThroughWorldCreation()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var setup = new SetupWindow(new AuditStorageService());
            try
            {
                Show(setup);
                var advancedTask = InvokeTask(setup, "EditAdvancedSettingsAsync", 0);
                await WaitFor(() => setup.OwnedWindows.OfType<AdvancedSettingsWindow>().Any());
                var advanced = Assert.Single(setup.OwnedWindows.OfType<AdvancedSettingsWindow>());
                Assert.True(Named<CheckBox>(advanced, "SafeModeBackup").IsChecked);
                Named<CheckBox>(advanced, "SafeModeBackup").IsChecked = false;
                Named<Button>(advanced, "ApplyButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await advancedTask;
                Assert.False(Field<GlobalSettings>(setup, "_globalSettings").Main.SafeMode);

                var globalTask = InvokeTask(setup, "EditGlobalSettingsAsync", 0);
                await WaitFor(() => setup.OwnedWindows.OfType<GlobalSettingsWindow>().Any());
                var globals = Assert.Single(setup.OwnedWindows.OfType<GlobalSettingsWindow>());
                Assert.False(Named<CheckBox>(globals, "SafeMode").IsChecked);
                Named<CheckBox>(globals, "SafeMode").IsChecked = true;
                Named<Button>(globals, "ApplyButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await globalTask;
                Assert.True(Field<EnvironmentUpdate>(setup, "_environment").Recording!.SafeModeBackup);

                globalTask = InvokeTask(setup, "EditGlobalSettingsAsync", 0);
                await WaitFor(() => setup.OwnedWindows.OfType<GlobalSettingsWindow>().Any());
                globals = Assert.Single(setup.OwnedWindows.OfType<GlobalSettingsWindow>());
                Named<CheckBox>(globals, "SafeMode").IsChecked = false;
                Named<Button>(globals, "ApplyButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await globalTask;
                WorldSetupOptions? created = null;
                setup.WorldCreated += value => created = value;
                Named<Button>(setup, "CreateWorldButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.NotNull(created);
                Assert.False(created.GlobalSettings.Main.SafeMode);
                Assert.False(created.ToEnvironmentUpdate().Recording!.SafeModeBackup);
            }
            finally
            {
                foreach (var owned in setup.OwnedWindows.ToArray()) owned.Close();
                setup.Close();
            }
        }, CancellationToken.None);
    }

    [Theory]
    [InlineData(true)]
    [InlineData(false)]
    public async Task PlayerCycleBatchesWriteRecoveryArtifactsAtTheConfiguredBoundaries(bool safeMode)
    {
        var path = Path.Combine(Path.GetTempPath(), $"db24832-cycle-backup-{Guid.NewGuid():N}.db3s");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var factory = new AuditEngineClientFactory(encodeTickInSave: true);
                var storage = new AuditStorageService { LatestAutosave = path, PersistAutosaves = true };
                var window = new MainWindow([], Setup(safeMode) with
                {
                    Recording = new Db2RecordingOptions(AutosaveInterval: safeMode ? 0ul : 7ul),
                }, factory, storage);
                try
                {
                    Show(window);
                    await Ready(window);
                    var menu = Named<LegacyMenuBar>(window, "HistoricMenuBar");
                    Assert.Equal(safeMode, Field<EnvironmentUpdate>(window, "_liveEnvironment").Recording!.SafeModeBackup);
                    Assert.Equal(!safeMode, Field<OfflineDiagnosticRecovery>(window, "_diagnostics").IgnoreErrors);
                    Assert.Empty(storage.AutosaveWrites);
                    const string commandStatus = "TURBO · RENDERING SUSPENDED";
                    ViewModel(window).Status = commandStatus;

                    await InvokeTask(window, "AdvanceSimulationAsync", safeMode ? 4005u : 20u);

                    Assert.Equal(safeMode ? new ulong[] { 2000, 4000 } : new ulong[] { 7, 14 },
                        storage.AutosaveWrites.Select(bytes => BitConverter.ToUInt64(bytes)).ToArray());
                    Assert.Equal(safeMode ? 4000ul : 14ul, BitConverter.ToUInt64(await File.ReadAllBytesAsync(path)));
                    Assert.Equal(safeMode ? 4000ul : 14ul,
                        Field<OfflineDiagnosticRecovery>(window, "_diagnostics").LastCheckpointTick);
                    Assert.Equal(path, Field<OfflineDiagnosticRecovery>(window, "_diagnostics").RecoveryArtifactPath);
                    Assert.Equal(commandStatus, ViewModel(window).Status);

                    if (safeMode)
                    {
                        Activate(menu, LegacyCommandId.EditGeneralSettings);
                        await WaitFor(() => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any());
                        var recording = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                        Named<CheckBox>(recording, "SafeModeBackup").IsChecked = false;
                        Named<Button>(recording, "ApplyButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                        await WaitFor(() => ViewModel(window).Status == "LIVE SETTINGS APPLIED");
                        await InvokeTask(window, "AdvanceSimulationAsync", 2000u);
                        Assert.Equal(2, storage.AutosaveWrites.Count);
                        Assert.False(Field<GlobalSettings>(window, "_globalSettings").Main.SafeMode);
                        Assert.False(factory.Clients.Single().EnvironmentUpdates.Last().Recording!.SafeModeBackup);
                    }
                }
                finally { window.Close(); }
            }, CancellationToken.None);
        }
        finally { if (File.Exists(path)) File.Delete(path); }
    }

    [Fact]
    public async Task ArchiveMenuIsIndependentOfGlobalRecordingAndLoadedCyclePolicy()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var factory = new AuditEngineClientFactory();
            var backupLaunches = 0;
            var window = new MainWindow([], Setup(false), factory, new AuditStorageService(),
                launchSafeModeBackup: () => backupLaunches++);
            try
            {
                Show(window);
                await Ready(window);
                var engine = factory.Clients.Single();
                var menu = Named<LegacyMenuBar>(window, "HistoricMenuBar");
                Activate(menu, LegacyCommandId.RecordingSafemodeBackup);
                await WaitFor(() => backupLaunches == 1);
                Activate(menu, LegacyCommandId.RecordingSafemodeBackup);
                await WaitFor(() => backupLaunches == 2);
                Assert.Empty(window.OwnedWindows.OfType<SafeModeBackupWindow>());
                Assert.False(Field<GlobalSettings>(window, "_globalSettings").Main.SafeMode);

                Activate(menu, LegacyCommandId.EditGlobalSettings);
                await WaitFor(() => window.OwnedWindows.OfType<GlobalSettingsWindow>().Any());
                var globals = Assert.Single(window.OwnedWindows.OfType<GlobalSettingsWindow>());
                Assert.False(Named<CheckBox>(globals, "SafeMode").IsChecked);
                Named<CheckBox>(globals, "SafeMode").IsChecked = true;
                Named<Button>(globals, "ApplyButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => ViewModel(window).Status == "GLOBAL SETTINGS SAVED");
                Assert.True(Field<GlobalSettings>(window, "_globalSettings").Main.SafeMode);
                Assert.True(engine.EnvironmentUpdates.Last().Recording!.SafeModeBackup);

                Activate(menu, LegacyCommandId.EditGeneralSettings);
                await WaitFor(() => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any());
                var recording = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                Assert.True(Named<CheckBox>(recording, "SafeModeBackup").IsChecked);
                Named<CheckBox>(recording, "SafeModeBackup").IsChecked = false;
                Named<Button>(recording, "ApplyButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => ViewModel(window).Status == "LIVE SETTINGS APPLIED");
                Assert.False(Field<GlobalSettings>(window, "_globalSettings").Main.SafeMode);
                Assert.False(engine.EnvironmentUpdates.Last().Recording!.SafeModeBackup);

                engine.ConfigurationForRead = new WorldConfiguration(EnvironmentUpdate.Default with
                {
                    Recording = new Db2RecordingOptions(SafeModeBackup: true),
                }, 1, 25000, 500, []);
                await InvokeTask(window, "SynchronizeLoadedWorldAsync");
                Assert.True(Field<GlobalSettings>(window, "_globalSettings").Main.SafeMode);
                Assert.True(Field<EnvironmentUpdate>(window, "_liveEnvironment").Recording!.SafeModeBackup);
            }
            finally
            {
                foreach (var owned in window.OwnedWindows.ToArray()) owned.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static WorldSetupOptions Setup(bool safeMode) => new()
    {
        Backend = "Cpu",
        GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { SafeMode = safeMode } },
        Species = [new SpeciesImport("Cycle Backup Bot", "start\nstop", false, 0xff239ac0, 1, 1000, 0, false, 1)],
    };

    private static void Show(Window window)
    {
        window.Width = 1200;
        window.Height = 820;
        window.Show();
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }

    private static Task Ready(MainWindow window) =>
        WaitFor(() => ViewModel(window).Status.Contains("WORLD READY", StringComparison.Ordinal));

    private static MainWindowViewModel ViewModel(MainWindow window) => Assert.IsType<MainWindowViewModel>(window.DataContext);
    private static T Named<T>(Control root, string name) where T : Control => Assert.IsType<T>(root.FindControl<T>(name));
    private static T Field<T>(object value, string name) => (T)value.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(value)!;
    private static void Activate(LegacyMenuBar menu, LegacyCommandId id) => menu.FindCommandItem(id)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
    private static Task InvokeTask(object target, string name, params object[] args) =>
        (Task)target.GetType().GetMethod(name, BindingFlags.Instance | BindingFlags.NonPublic)!.Invoke(target, args)!;

    private static async Task WaitFor(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(8);
        while (!condition() && DateTime.UtcNow < deadline) await Task.Delay(10);
        Assert.True(condition(), "Timed out waiting for the safe-mode player route.");
    }
}

public class RestartStorageProxy : DispatchProxy
{
    private Func<MethodInfo, object?[], object?> _invoke = null!;

    public static T Create<T>(Func<MethodInfo, object?[], object?> invoke) where T : class
    {
        var proxy = DispatchProxy.Create<T, RestartStorageProxy>();
        ((RestartStorageProxy)(object)proxy)._invoke = invoke;
        return proxy;
    }

    protected override object? Invoke(MethodInfo? method, object?[]? args) => _invoke(method!, args ?? []);
}
