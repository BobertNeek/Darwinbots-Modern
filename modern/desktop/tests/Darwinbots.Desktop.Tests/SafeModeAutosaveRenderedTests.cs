using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class SafeModeAutosaveRenderedTests
{
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
                    Assert.Equal(safeMode, menu.FindCommandItem(LegacyCommandId.RecordingSafemodeBackup)!.IsChecked);
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
                        Activate(menu, LegacyCommandId.RecordingSafemodeBackup);
                        await WaitFor(() => menu.FindCommandItem(LegacyCommandId.RecordingSafemodeBackup)!.IsChecked == false);
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
    public async Task GlobalRecordingAndLoadedPreferencesSynchronizeTheSameMenuAndEnginePolicy()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var factory = new AuditEngineClientFactory();
            var window = new MainWindow([], Setup(false), factory, new AuditStorageService());
            try
            {
                Show(window);
                await Ready(window);
                var engine = factory.Clients.Single();
                var menu = Named<LegacyMenuBar>(window, "HistoricMenuBar");
                Activate(menu, LegacyCommandId.RecordingSafemodeBackup);
                await WaitFor(() => menu.FindCommandItem(LegacyCommandId.RecordingSafemodeBackup)!.IsChecked == true);
                Assert.True(Field<GlobalSettings>(window, "_globalSettings").Main.SafeMode);
                Assert.True(engine.EnvironmentUpdates.Last().Recording!.SafeModeBackup);

                Activate(menu, LegacyCommandId.EditGlobalSettings);
                await WaitFor(() => window.OwnedWindows.OfType<GlobalSettingsWindow>().Any());
                var globals = Assert.Single(window.OwnedWindows.OfType<GlobalSettingsWindow>());
                Assert.True(Named<CheckBox>(globals, "SafeMode").IsChecked);
                Named<CheckBox>(globals, "SafeMode").IsChecked = false;
                Named<Button>(globals, "ApplyButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => ViewModel(window).Status == "GLOBAL SETTINGS SAVED");
                Assert.False(menu.FindCommandItem(LegacyCommandId.RecordingSafemodeBackup)!.IsChecked);
                Assert.False(engine.EnvironmentUpdates.Last().Recording!.SafeModeBackup);

                Activate(menu, LegacyCommandId.EditGeneralSettings);
                await WaitFor(() => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any());
                var recording = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                Named<CheckBox>(recording, "SafeModeBackup").IsChecked = true;
                Named<Button>(recording, "ApplyButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => ViewModel(window).Status == "LIVE SETTINGS APPLIED");
                Assert.True(menu.FindCommandItem(LegacyCommandId.RecordingSafemodeBackup)!.IsChecked);
                Assert.True(Field<GlobalSettings>(window, "_globalSettings").Main.SafeMode);

                engine.ConfigurationForRead = new WorldConfiguration(EnvironmentUpdate.Default with
                {
                    Recording = new Db2RecordingOptions(SafeModeBackup: false),
                }, 1, 25000, 500, []);
                await InvokeTask(window, "SynchronizeLoadedWorldAsync");
                Assert.False(menu.FindCommandItem(LegacyCommandId.RecordingSafemodeBackup)!.IsChecked);
                Assert.False(Field<GlobalSettings>(window, "_globalSettings").Main.SafeMode);
                Assert.False(Field<EnvironmentUpdate>(window, "_liveEnvironment").Recording!.SafeModeBackup);
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
