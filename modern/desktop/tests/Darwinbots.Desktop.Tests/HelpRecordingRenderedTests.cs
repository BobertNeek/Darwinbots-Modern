using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Input.Raw;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class HelpRecordingRenderedTests
{
    [Theory]
    [InlineData(true)]
    [InlineData(false)]
    public async Task LivingSnapshotOffersMutationHistoryFromTheSameCapture(bool includeHistory)
    {
        var actor = RecordingEngineClient.Organism with { UniqueId = 314, MutationLog = ["Café", "Previous change"] };
        var engine = new RecordingEngineClient(RecordingEngineClient.CreateSnapshot() with { Organisms = [actor] });
        await using var session = new SimulationSession(engine);
        await session.RefreshSnapshotAsync();

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var storage = new AuditStorageService { PromptForLivingMutationHistory = true };
            var window = new MainWindow([], new WorldSetupOptions(), new AuditEngineClientFactory(), storage);
            DesktopTestApplication.UseSession(window, session);
            try
            {
                window.Show();
                if (includeHistory)
                    window.FindControl<LegacyMenuBar>("HistoricMenuBar")!
                        .FindCommandItem(LegacyCommandId.RecordingSnapshotLiving)!
                        .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                else
                    window.FindControl<Button>("ToolbarSlot21")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => window.OwnedWindows.Any(), "Mutation history choice did not open.");
                var prompt = Assert.Single(window.OwnedWindows);
                Assert.Equal("Snapshot of the living", prompt.Title);
                prompt.GetVisualDescendants().OfType<Button>()
                    .Single(button => Equals(button.Content, includeHistory ? "YES" : "NO"))
                    .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => storage.SavedLivingSnapshot is not null, "Snapshot was not saved.");
                Assert.Equal(314UL, Assert.Single(Db2GenomeRecordFile.Decode(LegacyDnaText.Decode(storage.SavedLivingSnapshot!))).Id);
                if (includeHistory)
                {
                    Assert.Contains((byte)0xe9, storage.SavedLivingMutationHistory!);
                    Assert.Equal("Rob id,Mutation History\r\n\r\n314,\r\nCafé\r\nPrevious change\r\n\r\n",
                        LegacyDnaText.Decode(storage.SavedLivingMutationHistory));
                }
                else Assert.Null(storage.SavedLivingMutationHistory);
            }
            finally
            {
                foreach (var owned in window.OwnedWindows.ToArray()) owned.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task FailedHistoryAppendRestoresBothDeadRecordingFiles()
    {
        var directory = Directory.CreateTempSubdirectory("db2-genome-history-");
        try
        {
            var storage = new AvaloniaDesktopStorageService(directory.FullName);
            var records = Path.Combine(directory.FullName, "records", "DeadRobots.snp");
            var history = Path.Combine(directory.FullName, "records", "DeadRobots_Mutations.txt");
            await storage.AppendDeadSnapshotAsync(
                stream => stream.WriteAsync("first record"u8.ToArray()).AsTask(),
                stream => stream.WriteAsync("first history"u8.ToArray()).AsTask());

            await Assert.ThrowsAsync<IOException>(() => storage.AppendDeadSnapshotAsync(
                stream => stream.WriteAsync("second record"u8.ToArray()).AsTask(),
                async stream =>
                {
                    await stream.WriteAsync("second history"u8.ToArray());
                    throw new IOException("Interrupted after appending to both files.");
                }));

            Assert.Equal("first record", File.ReadAllText(records));
            Assert.Equal("first history", File.ReadAllText(history));
        }
        finally { directory.Delete(recursive: true); }
    }

    [Fact]
    public async Task AboutShortcutLeavesSelectAllToLeagueEditors()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), new AuditEngineClientFactory(), new AuditStorageService())
            {
                WindowState = WindowState.Normal,
                Width = 1280,
                Height = 900,
            };

            try
            {
                window.Show();
                window.FindControl<Expander>("LeagueExpander")!.IsExpanded = true;
                window.UpdateLayout();
                var entrants = window.FindControl<TextBox>("ModeEntrantOrder")!;
                entrants.Text = "1,2";
                var cycles = window.FindControl<NumericUpDown>("ModeMaximumCycles")!;
                var cycleText = Assert.Single(cycles.GetVisualDescendants().OfType<TextBox>());

                foreach (var editor in new[] { entrants, cycleText })
                {
                    Assert.False(string.IsNullOrEmpty(editor.Text));
                    Assert.True(editor.Focus());
                    editor.CaretIndex = editor.Text!.Length;
                    PressAboutShortcut(window);
                    Assert.Equal(editor.Text, editor.SelectedText);
                    Assert.Empty(window.OwnedWindows.OfType<LegacyDocumentWindow>());
                }

                Assert.True(window.FindControl<WorldViewport>("Viewport")!.Focus());
                PressAboutShortcut(window);
                await WaitFor(
                    () => window.OwnedWindows.OfType<LegacyDocumentWindow>()
                        .Any(dialog => dialog.Title.Contains("About Darwinbots Modern", StringComparison.Ordinal)),
                    "Ctrl+A on the playfield did not open About.");
            }
            finally
            {
                foreach (var owned in window.OwnedWindows.ToArray()) owned.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static void PressAboutShortcut(Window window)
    {
        window.KeyPress(Key.A, RawInputModifiers.Control, PhysicalKey.A, null);
        window.KeyRelease(Key.A, RawInputModifiers.Control, PhysicalKey.A, null);
    }

    [Fact]
    public async Task MainWindowMenuRoutesRenderRecordingHelpAboutAndSafeError()
    {
        var recoveryPath = Path.Combine(Path.GetTempPath(), $"db24832-help-recording-recovery-{Guid.NewGuid():N}.db3s");
        File.WriteAllBytes(recoveryPath, [1, 2, 3]);

        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var window = new MainWindow(
                    [],
                    new WorldSetupOptions
                    {
                        Backend = "Cpu",
                        GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { SafeMode = true } },
                        Species = [new SpeciesImport("Help Recording Species", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
                    },
                    new AuditEngineClientFactory(),
                    new AuditStorageService { LatestAutosave = recoveryPath })
                {
                    WindowState = WindowState.Normal,
                    Width = 1_280,
                    Height = 820,
                };

                try
                {
                    window.Show();
                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                    await WaitFor(
                        () => viewModel.Status.Contains("WORLD READY", StringComparison.Ordinal),
                        "MainWindow did not start the local player route.");

                    var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                    var recordingMenu = menu.TopLevelItems.Single(item => Equals(item.Header, "Recording"));
                    var recordingCommands = new[]
                    {
                        LegacyCommandId.RecordingSafemodeBackup,
                        LegacyCommandId.RecordingRecoverLatestAutosave,
                        LegacyCommandId.RecordingSnapshotLiving,
                        LegacyCommandId.RecordingDeadEnable,
                        LegacyCommandId.RecordingDeadExcludeRepopulating,
                    };
                    Assert.All(recordingCommands, command => Assert.True(menu.FindCommandItem(command)?.IsVisible, $"{command} is not player-visible."));
                    Assert.Contains(
                        recordingMenu.Items.OfType<MenuItem>(),
                        item => Equals(item.Header, "Snapshot of the dead") && item.IsVisible);

                    var safemode = menu.FindCommandItem(LegacyCommandId.RecordingSafemodeBackup)!;
                    Assert.True(safemode.IsChecked);
                    safemode.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => safemode.IsChecked == false, "Safemode recording command did not turn off.");
                    safemode.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => safemode.IsChecked == true, "Safemode recording command did not turn on.");

                    var deadRecording = menu.FindCommandItem(LegacyCommandId.RecordingDeadEnable)!;
                    var deadFilter = menu.FindCommandItem(LegacyCommandId.RecordingDeadExcludeRepopulating)!;
                    Assert.True(deadRecording.IsEnabled);
                    deadRecording.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    AcknowledgeDeadRecordingNotice(await WaitForDeadRecordingNotice(window));
                    await WaitFor(() => deadRecording.IsChecked == true, "Dead recording command did not turn on.");
                    deadFilter.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => deadFilter.IsChecked == true, "Dead-recording filter command did not turn on.");

                    var helpMenu = menu.TopLevelItems.Single(item => Equals(item.Header, "Help"));
                    Assert.True(helpMenu.IsVisible);
                    Assert.True(menu.FindCommandItem(LegacyCommandId.HelpDna)!.IsVisible);
                    Assert.True(menu.FindCommandItem(LegacyCommandId.HelpRobotTagInformation)!.IsVisible);
                    Assert.True(menu.FindCommandItem(LegacyCommandId.HelpAbout)!.IsVisible);
                    Assert.False(menu.FindCommandItem(LegacyCommandId.HelpSurvivalInfo)!.IsVisible);

                    var dnaHelp = menu.FindCommandItem(LegacyCommandId.HelpDna)!;
                    dnaHelp.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(
                        () => window.OwnedWindows.OfType<LegacyDocumentWindow>().Any(value => value.Title.Contains("DNA Help", StringComparison.Ordinal)),
                        "DNA Help menu command did not open its local document.");
                    var dnaWindow = Assert.Single(window.OwnedWindows.OfType<LegacyDocumentWindow>());
                    Assert.Contains("plain text", Assert.Single(dnaWindow.GetVisualDescendants().OfType<TextBox>()).Text, StringComparison.Ordinal);
                    dnaWindow.Close();
                    await WaitFor(() => !window.OwnedWindows.OfType<LegacyDocumentWindow>().Any(), "DNA Help window did not close.");

                    var about = menu.FindCommandItem(LegacyCommandId.HelpAbout)!;
                    about.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(
                        () => window.OwnedWindows.OfType<LegacyDocumentWindow>().Any(value => value.Title.Contains("About Darwinbots Modern", StringComparison.Ordinal)),
                        "About menu command did not open its local document.");
                    var aboutWindow = Assert.Single(window.OwnedWindows.OfType<LegacyDocumentWindow>());
                    Assert.Contains(
                        "cross-platform continuation",
                        Assert.Single(aboutWindow.GetVisualDescendants().OfType<TextBox>()).Text,
                        StringComparison.Ordinal);
                    aboutWindow.Close();
                    await WaitFor(() => !window.OwnedWindows.OfType<LegacyDocumentWindow>().Any(), "About window did not close.");

                    var handleError = typeof(MainWindow).GetMethod("HandleEngineErrorAsync", BindingFlags.Instance | BindingFlags.NonPublic)!;
                    await (Task)handleError.Invoke(window, [new AccessViolationException("help-recording rendered recovery fault"), true])!;
                    Assert.True(window.FindControl<Border>("RecoveryPanel")!.IsVisible);
                    Assert.Equal("SAFE MODE RECOVERY READY", window.FindControl<TextBlock>("RecoveryTitle")!.Text);
                    Assert.Contains("FAULT WINDOW", window.FindControl<TextBlock>("RecoveryFaultWindowText")!.Text, StringComparison.Ordinal);
                    Assert.Contains("RECOVERY READY", viewModel.Status, StringComparison.Ordinal);

                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    using var frame = Assert.IsAssignableFrom<Bitmap>(window.CaptureRenderedFrame());
                    Assert.True(frame.PixelSize.Width >= 1_000);
                    Assert.True(frame.PixelSize.Height >= 700);
                    var output = Path.Combine(Path.GetTempPath(), "db24832-help-recording.png");
                    using var stream = File.Create(output);
                    frame.Save(stream, new PngBitmapEncoderOptions());
                    Assert.True(File.Exists(output), $"Rendered receipt was not written: {output}");
                }
                finally
                {
                    foreach (var ownedWindow in window.OwnedWindows.ToArray())
                        if (ownedWindow.IsVisible) ownedWindow.Close();
                    if (window.IsVisible) window.Close();
                }
            }, CancellationToken.None);
        }
        finally
        {
            if (File.Exists(recoveryPath)) File.Delete(recoveryPath);
        }
    }

    [Fact]
    public async Task DeadRecordingMenuWaitsForItsNoticeAndDiscardsAcknowledgementAfterLoad()
    {
        var profile = Directory.CreateTempSubdirectory("darwinbots-dead-recording-notice-");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var factory = new AuditEngineClientFactory();
                var storage = new AvaloniaDesktopStorageService(profile.FullName);
                var window = new MainWindow([], new WorldSetupOptions
                {
                    Backend = "Cpu",
                    Species = [new SpeciesImport("Recording Bot", "start\nstop", false, 0xff239ac0, 1, 1000, 0, false, 1)],
                }, factory, storage);
                try
                {
                    window.Show();
                    var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                    await WaitFor(() => viewModel.Status.Contains("WORLD READY", StringComparison.Ordinal), "Recording world did not start.");
                    var engine = factory.Clients.Single();
                    var item = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!.FindCommandItem(LegacyCommandId.RecordingDeadEnable)!;
                    item.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    var notice = await WaitForDeadRecordingNotice(window);
                    var text = Assert.Single(Assert.IsType<StackPanel>(notice.Content).Children.OfType<TextBlock>()).Text;
                    var records = Path.Combine(profile.FullName, "records");
                    Assert.Contains(Path.Combine(records, "DeadRobots.snp"), text, StringComparison.Ordinal);
                    Assert.Contains(Path.Combine(records, "DeadRobots_Mutations.txt"), text, StringComparison.Ordinal);
                    Assert.Contains("consumes disk space quickly", text, StringComparison.Ordinal);
                    Assert.False(item.IsChecked);
                    Assert.False(item.IsEnabled);
                    Assert.Empty(engine.EnvironmentUpdates);
                    item.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    Assert.Same(notice, Assert.Single(window.OwnedWindows.OfType<LegacyConfirmationWindow>()));

                    engine.ConfigurationForRead = new WorldConfiguration(EnvironmentUpdate.Default, 1, 25000, 500, []);
                    await InvokeTask(window, "LoadSimulationAsync", new byte[] { 1, 2, 3 });
                    AcknowledgeDeadRecordingNotice(notice);
                    await WaitFor(() => item.IsEnabled, "The loaded world did not restore the recording command.");
                    Assert.False(item.IsChecked);
                    Assert.Empty(engine.EnvironmentUpdates);

                    item.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    AcknowledgeDeadRecordingNotice(await WaitForDeadRecordingNotice(window));
                    await WaitFor(() => item.IsChecked && item.IsEnabled, "Acknowledgement did not enable recording.");
                    Assert.True(Assert.Single(engine.EnvironmentUpdates).Recording!.DeadSnapshotEnabled);
                    item.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => !item.IsChecked && item.IsEnabled, "Disabling recording did not finish immediately.");
                    Assert.Equal(2, engine.EnvironmentUpdates.Count);
                    Assert.False(engine.EnvironmentUpdates.Last().Recording!.DeadSnapshotEnabled);
                    Assert.Empty(window.OwnedWindows.OfType<LegacyConfirmationWindow>());
                }
                finally
                {
                    foreach (var owned in window.OwnedWindows.ToArray()) owned.Close();
                    window.Close();
                }
            }, CancellationToken.None);
        }
        finally { profile.Delete(recursive: true); }
    }

    [Fact]
    public async Task StartupLoadedAndLiveRecordingPreferencesDriveMenusAndRecordedDeaths()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var factory = new AuditEngineClientFactory();
            var storage = new AuditStorageService();
            var window = new MainWindow([], new WorldSetupOptions
            {
                Backend = "Cpu",
                Recording = new Db2RecordingOptions(DeadSnapshotEnabled: true, ExcludeRepopulatingRobots: true),
                Species = [new SpeciesImport("Recording Policy Bot", "start\nstop", false, 0xff239ac0, 1, 1000, 0, false, 1)],
            }, factory, storage);
            try
            {
                window.Width = 1280;
                window.Height = 820;
                window.Show();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await WaitFor(() => viewModel.Status.Contains("WORLD READY", StringComparison.Ordinal), "Recording world did not start.");
                var engine = factory.Clients.Single();
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var enabled = menu.FindCommandItem(LegacyCommandId.RecordingDeadEnable)!;
                var exclude = menu.FindCommandItem(LegacyCommandId.RecordingDeadExcludeRepopulating)!;
                Assert.True(enabled.IsChecked);
                Assert.True(exclude.IsChecked);
                var deaths = engine.Snapshot() with
                {
                    Organisms = [RecordingEngineClient.Organism with { IsCorpse = true, UniqueId = 42, MutationLog = ["Corpse history"] }],
                    Corpses =
                    [
                        new CorpseSnapshot([10f, 10f], [0f, 0f], 0, 100, 5, 1) { Dna = "ANIMAL-ONLY-DNA", UniqueId = 42 },
                        new CorpseSnapshot([20f, 20f], [0f, 0f], 0, 100, 5, 1, true) { Dna = "REPOPULATING-ONLY-DNA" },
                        new CorpseSnapshot([30f, 30f], [0f, 0f], 0, 100, 5, 1) { Dna = "ARCHIVED-DNA", UniqueId = 45 },
                    ],
                };
                await InvokeTask(window, "RecordDeadSnapshotAsync", deaths);
                var recorded = System.Text.Encoding.UTF8.GetString(Assert.Single(storage.DeadSnapshotWrites));
                Assert.Contains("ANIMAL-ONLY-DNA", recorded, StringComparison.Ordinal);
                Assert.DoesNotContain("REPOPULATING-ONLY-DNA", recorded, StringComparison.Ordinal);
                Assert.Contains("ARCHIVED-DNA", recorded, StringComparison.Ordinal);
                Assert.Equal("Rob id,Mutation History\r\n\r\n42,\r\nCorpse history\r\n\r\n",
                    LegacyDnaText.Decode(Assert.Single(storage.DeadMutationHistoryWrites)));

                menu.FindCommandItem(LegacyCommandId.EditGeneralSettings)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any(), "Recording settings did not open.");
                var settings = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                settings.FindControl<CheckBox>("DeadSnapshotEnabled")!.IsChecked = false;
                settings.FindControl<CheckBox>("ExcludeRepopulatingRobots")!.IsChecked = false;
                settings.FindControl<Button>("ApplyButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => viewModel.Status == "LIVE SETTINGS APPLIED", "Recording settings did not apply.");
                Assert.False(enabled.IsChecked);
                Assert.False(exclude.IsChecked);
                Assert.False(engine.EnvironmentUpdates.Last().Recording!.DeadSnapshotEnabled);
                Assert.False(engine.EnvironmentUpdates.Last().Recording!.ExcludeRepopulatingRobots);
                await InvokeTask(window, "RecordDeadSnapshotAsync", deaths);
                Assert.Single(storage.DeadSnapshotWrites);

                engine.ConfigurationForRead = new WorldConfiguration(EnvironmentUpdate.Default with
                {
                    Recording = new Db2RecordingOptions(DeadSnapshotEnabled: true, ExcludeRepopulatingRobots: false),
                }, 1, 25000, 500, []);
                await InvokeTask(window, "SynchronizeLoadedWorldAsync");
                Assert.True(enabled.IsChecked);
                Assert.False(exclude.IsChecked);
                Assert.Empty(window.OwnedWindows.OfType<LegacyConfirmationWindow>());
                enabled.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => enabled.IsChecked == false, "Recording menu did not disable the loaded preference.");
                exclude.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                await WaitFor(() => exclude.IsChecked == true, "Recording menu did not update the loaded filter.");
                Assert.False(engine.EnvironmentUpdates.Last().Recording!.DeadSnapshotEnabled);
                Assert.True(engine.EnvironmentUpdates.Last().Recording!.ExcludeRepopulatingRobots);
            }
            finally
            {
                foreach (var owned in window.OwnedWindows.ToArray()) owned.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Theory]
    [InlineData(LegacyCommandId.RobotKill, false, false)]
    [InlineData(LegacyCommandId.RobotKillEntireOrganism, false, false)]
    [InlineData(LegacyCommandId.RobotKillEntireOrganism, true, false)]
    [InlineData(LegacyCommandId.RobotKill, false, true)]
    public async Task ManualKillRecordsOnlyTheAffectedRobots(LegacyCommandId command, bool excludeVegetable, bool writeFails)
    {
        var selected = RecordingEngineClient.Organism with { UniqueId = 100, MutationLog = ["Selected change"] };
        var vegetable = selected with { Slot = 4, Generation = 7, UniqueId = 200, Vegetable = true, MutationLog = ["Vegetable change"] };
        var unrelated = selected with { Slot = 5, UniqueId = 300, MutationLog = ["Unrelated change"] };
        var selectedKey = new OrganismKey(selected.Slot, selected.Generation);
        var vegetableKey = new OrganismKey(vegetable.Slot, vegetable.Generation);
        var engine = new RecordingEngineClient(RecordingEngineClient.CreateSnapshot() with
        {
            Population = 3,
            Organisms = [selected, vegetable, unrelated],
            Ties = [new TieSnapshot(selectedKey, vegetableKey, 40f)],
        });
        await using var simulation = new SimulationSession(engine);
        var storage = new AuditStorageService
        {
            DeadSnapshotWriteError = writeFails ? new IOException("Test output is locked") : null,
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions
            {
                Recording = new Db2RecordingOptions(DeadSnapshotEnabled: true, ExcludeRepopulatingRobots: excludeVegetable),
            }, new AuditEngineClientFactory(), storage);
            DesktopTestApplication.UseSession(window, simulation);
            try
            {
                ((MainWindowViewModel)window.DataContext!).Select(selectedKey);
                var router = (LegacyCommandRouter)typeof(MainWindow)
                    .GetField("_legacyCommands", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window)!;
                var result = await router.ExecuteAsync(command);
                Assert.True(result.Succeeded, result.Status);

                var group = command == LegacyCommandId.RobotKillEntireOrganism;
                var removal = Assert.Single(engine.Calls, call => call.StartsWith("Remove", StringComparison.Ordinal));
                Assert.Equal(group ? "RemoveMany:3:2,4:7" : "Remove:3:2", removal);
                if (writeFails)
                {
                    Assert.Contains("DEAD RECORDING FAILED · Test output is locked", result.Status, StringComparison.Ordinal);
                    Assert.Empty(storage.DeadSnapshotWrites);
                    Assert.Empty(storage.DeadMutationHistoryWrites);
                    Assert.False(router.GetState(LegacyCommandId.RecordingDeadEnable).Checked);
                    return;
                }

                var records = Db2GenomeRecordFile.Decode(LegacyDnaText.Decode(Assert.Single(storage.DeadSnapshotWrites)));
                Assert.Equal(group && !excludeVegetable ? new ulong[] { 100, 200 } : [100], records.Select(record => record.Id));
                Assert.All(records, record => Assert.Equal("start\nstop", record.Dna.Trim()));
                Assert.Equal(group && !excludeVegetable ? new[] { (3u, 2u), (4u, 7u) } : [(3u, 2u)], engine.DnaExports);
                Assert.Equal("Rob id,Mutation History\r\n\r\n100,\r\nSelected change\r\n\r\n"
                    + (group && !excludeVegetable ? "\r\n200,\r\nVegetable change\r\n\r\n" : string.Empty),
                    LegacyDnaText.Decode(Assert.Single(storage.DeadMutationHistoryWrites)));
            }
            finally
            {
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static Task InvokeTask(object target, string method, params object[] args) =>
        (Task)target.GetType().GetMethod(method, BindingFlags.Instance | BindingFlags.NonPublic)!.Invoke(target, args)!;

    private static async Task<LegacyConfirmationWindow> WaitForDeadRecordingNotice(MainWindow window)
    {
        await WaitFor(() => window.OwnedWindows.OfType<LegacyConfirmationWindow>().Any(), "Dead recording notice did not open.");
        return Assert.Single(window.OwnedWindows.OfType<LegacyConfirmationWindow>());
    }

    private static void AcknowledgeDeadRecordingNotice(LegacyConfirmationWindow notice)
    {
        var button = Assert.Single(notice.GetVisualDescendants().OfType<Button>());
        Assert.Equal("OK", button.Content);
        button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
    }

    private static async Task WaitFor(Func<bool> condition, string diagnostic)
    {
        for (var attempt = 0; attempt < 500; attempt++)
        {
            AvaloniaHeadlessPlatform.ForceRenderTimerTick();
            if (condition()) return;
            await Task.Delay(20);
        }

        throw new Xunit.Sdk.XunitException(diagnostic);
    }
}
