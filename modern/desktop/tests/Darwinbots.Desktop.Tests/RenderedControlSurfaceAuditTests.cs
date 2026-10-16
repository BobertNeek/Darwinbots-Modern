using System.Collections.Concurrent;
using System.Reflection;
using Avalonia;
using Avalonia.Automation;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Controls.Primitives;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class RenderedControlSurfaceAuditTests
{
    [Fact]
    public async Task PopulationSeedingDropdownShowsZeroScoresAndExportsTemplatesThroughTheFolderPicker()
    {
        var directory = Path.Combine(Path.GetTempPath(), "db-rendered-seeds-" + Guid.NewGuid().ToString("N"));
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var factory = new AuditEngineClientFactory();
                var storage = new AuditStorageService();
                var window = new MainWindow([], new WorldSetupOptions
                {
                    Species = [new("Audit Species", "start stop", false, 0xff123456, 1, 1000, 0, false, 0)],
                }, factory, storage);
                SimulationSession? session = null;
                try
                {
                    ShowForInteraction(window, 1500, 900);
                    await WaitFor(() => factory.Clients.TryPeek(out var client) && client.Calls.Contains("ImportSpecies:Audit Species"));
                    var engine = factory.Clients.First();
                    session = Field<SimulationSession>(window, "_session");
                    var workflow = Named<ComboBox>(window, "LocalModeWorkflow");
                    Assert.Equal(5, workflow.ItemCount);
                    Assert.Equal("SEED FILTER", ((ComboBoxItem)workflow.Items[3]!).Content);
                    Assert.Equal("MODERN KNOCKOUT BRACKET", ((ComboBoxItem)workflow.Items[2]!).Content);
                    Assert.Equal("CLASSIC TOURNAMENT", ((ComboBoxItem)workflow.Items[4]!).Content);
                    AssertComboSelection(window, workflow, 4);
                    Assert.True(Named<TextBlock>(window, "PopulationSeedHelp").IsVisible);
                    Assert.Contains("2,000", Named<TextBlock>(window, "PopulationSeedHelp").Text);
                    Assert.True(Named<NumericUpDown>(window, "ModeMinimumRounds").IsEffectivelyEnabled);
                    Assert.True(Named<NumericUpDown>(window, "ModeMaximumRounds").IsEffectivelyEnabled);
                    Assert.True(Named<NumericUpDown>(window, "ModeMaximumCycles").IsEffectivelyEnabled);
                    Assert.True(Named<NumericUpDown>(window, "ModePopulationCap").IsEffectivelyEnabled);
                    Assert.False(Named<NumericUpDown>(window, "ModeSeedingOpponent").IsEffectivelyEnabled);
                    Assert.False(Named<Button>(window, "AdjudicateModeButton").IsEnabled);
                    ToggleAndAssert(window, Named<CheckBox>(window, "ModeFinishWithLadder"));
                    Named<TextBox>(window, "ModeEntrantOrder").Text = "1";
                    Click(window, Named<Button>(window, "ConfigureModeButton"));
                    await WaitFor(() => engine.Calls.Contains("ConfigureLocalMode"));
                    Assert.Equal(LeagueWorkflow.PopulationSeeding, engine.LastLocalModeConfiguration.Workflow);
                    Assert.Null(engine.LastLocalModeConfiguration.SeedingOpponent);
                    Assert.True(engine.LastLocalModeConfiguration.FinishWithLadder);
                    Assert.Equal(new uint[] { 1 }, engine.LastLocalModeConfiguration.EntrantOrder);

                    engine.LocalModeForRead = new(LocalModeStatus.Running, LocalModeKind.League, LeaguePreset.F1,
                        0, 1, 2000, 15000, 0, [new(1, 0, 4, 1000, "previous match")], null, new())
                    {
                        Configuration = engine.LastLocalModeConfiguration,
                        Series = new(LeagueWorkflow.PopulationSeeding, [1, 0], 1, [], [], [])
                        {
                            PopulationSeeds = [new(1, 0, 2000) { FileName = "0.txt" }],
                            PopulationSeedingPhase = ClassicTournamentPhase.Tournament,
                            SeedFileOrder = [1],
                            MatchStartResult = 1,
                        },
                    };
                    engine.NativeCycle = 2000;
                    const string template = "start\n1 .up store\nstop";
                    engine.SpeciesTemplateDna[1] = template;
                    await session.RefreshSnapshotAsync();
                    await WaitFor(() => Named<Button>(window, "ExportPopulationSeedsButton").IsEnabled);
                    Assert.Equal("POPULATION SCORES · Audit Species: 0", Named<TextBlock>(window, "PopulationSeedScores").Text);
                    var model = Field<MainWindowViewModel>(window, "_viewModel");
                    Assert.Contains("CLASSIC TOURNAMENT", model.LocalMode.SeriesLabel);
                    Assert.NotEmpty(ContestResultSnapshot.FromSnapshot(session.LatestSnapshot).Competitors);
                    Assert.True(Named<Button>(window, "AdjudicateModeButton").IsEnabled);
                    Click(window, Named<Button>(window, "AdjudicateModeButton"));
                    await WaitFor(() => engine.Calls.Contains("AdjudicateLocalRound:1"));
                    Click(window, Named<Button>(window, "ExportPopulationSeedsButton"));
                    await WaitFor(() => model.Status == "POPULATION SEED EXPORT CANCELLED");
                    Assert.False(Directory.Exists(directory));
                    Assert.DoesNotContain(engine.Calls, call => call.StartsWith("ExportSpeciesTemplateDna:", StringComparison.Ordinal));

                    Directory.CreateDirectory(Path.Combine(directory, "seeded"));
                    var existing = Path.Combine(directory, "seeded", "0.txt");
                    await File.WriteAllTextAsync(existing, "existing seed");
                    storage.PickedPopulationSeedDirectory = directory;
                    Click(window, Named<Button>(window, "ExportPopulationSeedsButton"));
                    await WaitFor(() => model.Status.StartsWith("POPULATION SEED EXPORT FAILED", StringComparison.Ordinal));
                    Assert.Equal("existing seed", await File.ReadAllTextAsync(existing));
                    Assert.Single(Directory.GetFiles(Path.Combine(directory, "seeded")));
                    storage.PickedPopulationSeedDirectory = Path.Combine(directory, "exact");
                    Click(window, Named<Button>(window, "ExportPopulationSeedsButton"));
                    await WaitFor(() => model.Status == "EXPORTED 1 POPULATION SEEDS", () => model.Status);
                    Assert.Equal(template + Environment.NewLine + "'#tag:Audit Species" + Environment.NewLine,
                        await File.ReadAllTextAsync(Path.Combine(directory, "exact", "seeded", "0.txt")));
                    Assert.Equal("existing seed", await File.ReadAllTextAsync(existing));
                    Assert.Equal(3, storage.PopulationSeedPickerOwners.Count);
                    Assert.All(storage.PopulationSeedPickerOwners, owner => Assert.Same(window, owner));
                    Assert.Contains("ExportSpeciesTemplateDna:1", engine.Calls);
                    Assert.DoesNotContain("ExportDna", engine.Calls);
                    Assert.False(Field<DispatcherTimer>(window, "_runTimer").IsEnabled);
                    AssertComboSelection(window, workflow, 3);
                    Assert.True(Named<NumericUpDown>(window, "ModeSeedingOpponent").IsEffectivelyEnabled);
                    Assert.True(Named<NumericUpDown>(window, "ModeMaximumCycles").IsEffectivelyEnabled);
                }
                finally
                {
                    foreach (var owned in window.OwnedWindows.ToArray()) owned.Close();
                    window.Close();
                    if (session is not null) await session.DisposeAsync();
                }
            }, CancellationToken.None);
        }
        finally { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task CompletedClassicDnaExportUsesTheFolderPickerAndPreservesOriginalFiles()
    {
        var directory = Path.Combine(Path.GetTempPath(), "db-rendered-classic-dna-" + Guid.NewGuid().ToString("N"));
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var factory = new AuditEngineClientFactory();
                var storage = new AuditStorageService();
                var window = new MainWindow([], new WorldSetupOptions {
                    Species = [new("Audit Species", "start stop", false, 0xff123456, 1, 1000, 0, false, 0)],
                }, factory, storage);
                SimulationSession? session = null;
                try
                {
                    ShowForInteraction(window, 1500, 900);
                    await WaitFor(() => factory.Clients.TryPeek(out var client) && client.Calls.Contains("ImportSpecies:Audit Species"));
                    var engine = factory.Clients.First();
                    session = Field<SimulationSession>(window, "_session");
                    var model = Field<MainWindowViewModel>(window, "_viewModel");
                    var config = LocalModeConfig.Default with { Workflow = LeagueWorkflow.PopulationSeeding, FinishWithLadder = true };
                    var complete = new LocalModeSnapshot(LocalModeStatus.Completed, LocalModeKind.League, LeaguePreset.F1,
                        1, 1, 0, 15000, 0, [], null, new()) {
                        Configuration = config,
                        Series = new(LeagueWorkflow.PopulationSeeding, [], 1, [1], [], []) {
                            PopulationSeedingPhase = ClassicTournamentPhase.Complete,
                            PopulationSeeds = [new(1, 0, 2000) { FileName = "0.txt" }],
                        },
                    };
                    var button = Named<Button>(window, "ExportClassicTournamentButton");
                    foreach (var phase in new ClassicTournamentPhase?[] { null, ClassicTournamentPhase.Seeding,
                        ClassicTournamentPhase.Tournament, ClassicTournamentPhase.Ladder })
                    {
                        engine.LocalModeForRead = complete with { Series = complete.Series! with { PopulationSeedingPhase = phase } };
                        await session.RefreshSnapshotAsync();
                        await WaitFor(() => model.LocalMode.Series?.PopulationSeedingPhase == phase);
                        Assert.False(button.IsEnabled);
                    }
                    engine.LocalModeForRead = complete with { Configuration = config with { Workflow = LeagueWorkflow.Tournament },
                        Series = complete.Series! with { Workflow = LeagueWorkflow.Tournament } };
                    await session.RefreshSnapshotAsync();
                    await WaitFor(() => model.LocalMode.Workflow == LeagueWorkflow.Tournament);
                    Assert.False(button.IsEnabled);

                    engine.LocalModeForRead = complete;
                    engine.SpeciesTemplateDna[1] = "' raw original\r\n'#tag:PreviousName.txt\r\nstart 7 .up store stop\r\n";
                    engine.SpeciesTemplateDna[0] = "' raw second\nstart stop\n";
                    await session.RefreshSnapshotAsync();
                    await WaitFor(() => button.IsEnabled);
                    Assert.True(Named<CheckBox>(window, "ModeFinishWithLadder").IsChecked);
                    Click(window, button);
                    await WaitFor(() => model.Status == "CLASSIC TOURNAMENT DNA EXPORT CANCELLED");
                    Assert.False(Directory.Exists(directory));
                    Assert.DoesNotContain(engine.Calls, call => call.StartsWith("ExportSpeciesTemplateDna:", StringComparison.Ordinal));

                    var preserved = Path.Combine(directory, "Tournament_Results", "Audit Species.txt");
                    Directory.CreateDirectory(Path.GetDirectoryName(preserved)!);
                    await File.WriteAllTextAsync(preserved, "existing result");
                    storage.PickedPopulationSeedDirectory = directory;
                    Click(window, button);
                    await WaitFor(() => model.Status.StartsWith("CLASSIC TOURNAMENT DNA EXPORT FAILED", StringComparison.Ordinal));
                    Assert.Equal("existing result", await File.ReadAllTextAsync(preserved));

                    var clean = Path.Combine(directory, "clean");
                    storage.PickedPopulationSeedDirectory = clean;
                    Click(window, button);
                    await WaitFor(() => model.Status == "EXPORTED 1 CLASSIC TOURNAMENT DNA FILE(S)");
                    Assert.Equal(engine.SpeciesTemplateDna[1] + Environment.NewLine + "'#tag:Audit Species" + Environment.NewLine,
                        await File.ReadAllTextAsync(Path.Combine(clean, "Tournament_Results", "Audit Species.txt")));
                    Assert.False(Directory.Exists(Path.Combine(clean, "stepladder")));

                    engine.LocalModeForRead = complete with { Series = complete.Series! with { EnteredLadder = true, Ladder = [1, 0] } };
                    await session.RefreshSnapshotAsync();
                    await WaitFor(() => model.LocalMode.Series?.EnteredLadder == true);
                    Click(window, button);
                    await WaitFor(() => model.Status == "EXPORTED 2 CLASSIC TOURNAMENT DNA FILE(S)");
                    Assert.Equal(engine.SpeciesTemplateDna[1] + Environment.NewLine + "'#tag:Audit Species" + Environment.NewLine,
                        await File.ReadAllTextAsync(Path.Combine(clean, "stepladder", "1-Audit Species.txt")));
                    Assert.Equal(engine.SpeciesTemplateDna[0] + Environment.NewLine + "'#tag:Unassigned" + Environment.NewLine,
                        await File.ReadAllTextAsync(Path.Combine(clean, "stepladder", "2-Unassigned.txt")));
                    Assert.All(storage.PopulationSeedPickerOwners, owner => Assert.Same(window, owner));
                    Assert.DoesNotContain("ExportDna", engine.Calls);
                }
                finally
                {
                    foreach (var owned in window.OwnedWindows.ToArray()) owned.Close();
                    window.Close();
                    if (session is not null) await session.DisposeAsync();
                }
            }, CancellationToken.None);
        }
        finally { if (Directory.Exists(directory)) Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task RealNativeNewAndResetRenderOriginalCyclesWhileKeepingSchedulingTicks()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var windows = new List<Window>();
            using var opened = Window.WindowOpenedEvent.AddClassHandler<Window>((window, _) => windows.Add(window));
            var initial = new MainWindow([], new WorldSetupOptions
            {
                Backend = "Cpu", Seed = 42, PopulationCapacity = 16,
                BrownianMotion = 0, WorldWidth = 3000, WorldHeight = 2000,
                Species = [new("Cycle Display", "start stop", false, 0xff123456, 1, 1000, 0, false, 0)],
            }, NativeEngineClientFactory.Instance, new AuditStorageService());
            try
            {
                ShowForInteraction(initial, 1280, 900);
                await WaitFor(() => Field<MainWindowViewModel>(initial, "_viewModel").Status.Contains("WORLD READY", StringComparison.Ordinal));
                AssertOriginalCycleDisplay(initial, -1, 0);
                await AdvanceForCycleDisplayAsync(initial);
                await WaitFor(() => Field<MainWindowViewModel>(initial, "_viewModel").NativeCycle == 0);
                AssertOriginalCycleDisplay(initial, 0, 1);

                Named<Button>(initial, "ResetButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => windows.OfType<MainWindow>().Count() == 2);
                var reset = windows.OfType<MainWindow>().Single(value => value != initial);
                await WaitFor(() => Field<MainWindowViewModel>(reset, "_viewModel").Status.Contains("WORLD READY", StringComparison.Ordinal));
                AssertOriginalCycleDisplay(reset, -1, 0);
                await AdvanceForCycleDisplayAsync(reset);
                await WaitFor(() => Field<MainWindowViewModel>(reset, "_viewModel").NativeCycle == 0);
                AssertOriginalCycleDisplay(reset, 0, 1);

                Named<Button>(reset, "ToolbarSlot01").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => windows.OfType<SetupWindow>().Any());
                var setup = windows.OfType<SetupWindow>().Single();
                Named<ComboBox>(setup, "Backend").SelectedIndex = 1;
                Named<NumericUpDown>(setup, "PopulationCap").Value = 16;
                Named<NumericUpDown>(setup, "WorldWidth").Value = 3000;
                Named<NumericUpDown>(setup, "WorldHeight").Value = 2000;
                setup.Species.Clear();
                setup.Species.Add(new("New Cycle Display", "start stop", false, 0xff123456, 1, 1000, 0, false));
                Named<Button>(setup, "StartNewButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => windows.OfType<MainWindow>().Count() == 3);
                var created = windows.OfType<MainWindow>().Single(value => value != initial && value != reset);
                await WaitFor(() => Field<MainWindowViewModel>(created, "_viewModel").Status.Contains("WORLD READY", StringComparison.Ordinal));
                AssertOriginalCycleDisplay(created, -1, 0);
                await AdvanceForCycleDisplayAsync(created);
                await WaitFor(() => Field<MainWindowViewModel>(created, "_viewModel").NativeCycle == 0);
                AssertOriginalCycleDisplay(created, 0, 1);
            }
            finally { await CloseNavigationWindowsAsync(windows); }
        }, CancellationToken.None);
    }

    private static Task AdvanceForCycleDisplayAsync(MainWindow window) =>
        (Task)typeof(MainWindow).GetMethod("AdvanceSimulationAsync", BindingFlags.Instance | BindingFlags.NonPublic)!.Invoke(window, [1u])!;

    private static void AssertOriginalCycleDisplay(MainWindow window, int nativeCycle, ulong tick)
    {
        window.UpdateLayout();
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        var model = Field<MainWindowViewModel>(window, "_viewModel");
        Assert.Equal(nativeCycle, model.NativeCycle);
        Assert.Equal(tick, model.Tick);
        Assert.Equal(nativeCycle.ToString(System.Globalization.CultureInfo.InvariantCulture), Named<TextBlock>(window, "CycleValue").Text);
        Assert.Equal($"Cycles {model.CycleLabel}", model.StatusPanels[5].Display);
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public async Task NewSimulationRoutesCaptureCurrentSpareAndWireTheReplacementWithTheInjectedServices(bool useMenu)
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var factory = new AuditEngineClientFactory();
            var storage = new AuditStorageService();
            var windows = new List<Window>();
            using var opened = Window.WindowOpenedEvent.AddClassHandler<Window>((window, _) => windows.Add(window));
            var original = new MainWindow([], new WorldSetupOptions
            {
                NativeGaussianSpare = 1.25f, NativeSunChange = 7,
                Species = [new("Handoff", "start stop", false, 0xff123456, 1, 1000, 0, false, 0)],
            }, factory, storage);
            try
            {
                ShowForInteraction(original, 1280, 900);
                await WaitFor(() => factory.Clients.TryPeek(out var client) && client.Calls.Contains("ImportSpecies:Handoff"));
                var oldClient = factory.Clients.Single();
                Assert.Equal(1.25f, factory.CreatedSetups.Single().NativeGaussianSpare);
                oldClient.NativeGaussianSpare = 2.5f;
                oldClient.NativeSunChange = 12;
                Field<DispatcherTimer>(original, "_runTimer").Start();
                if (useMenu)
                    ActivateMenu(Named<LegacyMenuBar>(original, "HistoricMenuBar")
                        .FindCommandItem(Core.LegacyMenus.LegacyCommandId.FileNewSimulation)!);
                else
                    Named<Button>(original, "ToolbarSlot01").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => windows.OfType<SetupWindow>().Any() && oldClient.Calls.Contains("Dispose"));
                var setup = Assert.Single(windows.OfType<SetupWindow>());
                Assert.False(original.IsVisible);
                Assert.False(Field<DispatcherTimer>(original, "_runTimer").IsEnabled);
                AssertReadBeforeDisposeWithoutTicks(oldClient, 2.5f);
                Assert.Equal((byte)12, Assert.Single(oldClient.SunChangeReads));
                if (Application.Current!.ApplicationLifetime is IClassicDesktopStyleApplicationLifetime lifetime)
                    Assert.Same(setup, lifetime.MainWindow);

                Named<Button>(setup, "StartNewButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => factory.Clients.Count == 2 && windows.OfType<MainWindow>().Count() == 2);
                var replacement = windows.OfType<MainWindow>().Single(value => value != original);
                await WaitFor(() => factory.Clients.Last().Calls.Any(value => value.StartsWith("ImportSpecies:", StringComparison.Ordinal)));
                Assert.Equal(2.5f, factory.CreatedSetups.Last().NativeGaussianSpare);
                Assert.Equal(2.5f, factory.Clients.Last().NativeGaussianSpare);
                Assert.Equal((byte)12, factory.CreatedSetups.Last().NativeSunChange);
                Assert.Equal((byte)12, factory.Clients.Last().NativeSunChange);
                Assert.Same(factory, Field<IEngineClientFactory>(replacement, "_engineFactory"));
                Assert.Same(storage, Field<IDesktopStorageService>(replacement, "_storage"));
                Assert.Null(Field<WorldSetupOptions>(replacement, "_setup").NativeGaussianSpare);
                Assert.Null(Field<WorldSetupOptions>(replacement, "_setup").NativeSunChange);
                Assert.Null(typeof(MainWindow).GetField("_initialNativeGaussianSpare", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(replacement));
                Assert.Null(typeof(MainWindow).GetField("_initialNativeSunChange", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(replacement));
                Assert.False(setup.IsVisible);
                Assert.True(replacement.IsVisible);
                if (Application.Current.ApplicationLifetime is IClassicDesktopStyleApplicationLifetime replacedLifetime)
                    Assert.Same(replacement, replacedLifetime.MainWindow);
                Assert.Equal(2, factory.CreatedSetups.Count);
                Assert.All(storage.LastStartWrites, bytes =>
                {
                    Assert.DoesNotContain("Gaussian", System.Text.Encoding.UTF8.GetString(bytes), StringComparison.OrdinalIgnoreCase);
                    Assert.DoesNotContain("NativeSunChange", System.Text.Encoding.UTF8.GetString(bytes), StringComparison.OrdinalIgnoreCase);
                });
            }
            finally { await CloseNavigationWindowsAsync(windows); }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task ResetCapturesTheCurrentSpareOnceAndDoesNotReplayTheOriginalHandoffAfterItClears()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var factory = new AuditEngineClientFactory();
            var storage = new AuditStorageService();
            var windows = new List<Window>();
            using var opened = Window.WindowOpenedEvent.AddClassHandler<Window>((window, _) => windows.Add(window));
            var original = new MainWindow([], new WorldSetupOptions
            {
                NativeGaussianSpare = 1.25f, NativeSunChange = 7,
                Species = [new("Reset Handoff", "start stop", false, 0xff123456, 1, 1000, 0, false, 0)],
            }, factory, storage);
            try
            {
                ShowForInteraction(original, 1280, 900);
                await WaitFor(() => factory.Clients.TryPeek(out var client) && client.Calls.Contains("ImportSpecies:Reset Handoff"));
                var oldClient = factory.Clients.Single();
                oldClient.NativeGaussianSpare = 2.5f;
                oldClient.NativeSunChange = 12;
                Field<DispatcherTimer>(original, "_runTimer").Start();
                Named<Button>(original, "ResetButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => factory.Clients.Count == 2 && oldClient.Calls.Contains("Dispose"));
                var replacement = windows.OfType<MainWindow>().Single(value => value != original);
                await WaitFor(() => factory.Clients.Last().Calls.Contains("ImportSpecies:Reset Handoff"));
                var middleClient = factory.Clients.Last();
                Assert.Equal(2.5f, factory.CreatedSetups.Last().NativeGaussianSpare);
                Assert.Equal((byte)12, factory.CreatedSetups.Last().NativeSunChange);
                AssertReadBeforeDisposeWithoutTicks(oldClient, 2.5f);
                Assert.Null(Field<WorldSetupOptions>(replacement, "_setup").NativeGaussianSpare);
                middleClient.NativeGaussianSpare = null;
                middleClient.NativeSunChange = 2;

                Named<Button>(replacement, "ResetButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

                await WaitFor(() => factory.Clients.Count == 3 && middleClient.Calls.Contains("Dispose"));
                var finalWindow = windows.OfType<MainWindow>().Single(value => value != original && value != replacement);
                await WaitFor(() => factory.Clients.Last().Calls.Contains("ImportSpecies:Reset Handoff"));
                Assert.Null(factory.CreatedSetups.Last().NativeGaussianSpare);
                Assert.Null(factory.Clients.Last().NativeGaussianSpare);
                Assert.Equal((byte)2, factory.CreatedSetups.Last().NativeSunChange);
                Assert.Equal((byte)2, factory.Clients.Last().NativeSunChange);
                AssertReadBeforeDisposeWithoutTicks(middleClient, null);
                Assert.Same(factory, Field<IEngineClientFactory>(finalWindow, "_engineFactory"));
                Assert.Same(storage, Field<IDesktopStorageService>(finalWindow, "_storage"));
                Assert.True(finalWindow.IsVisible);
                if (Application.Current!.ApplicationLifetime is IClassicDesktopStyleApplicationLifetime lifetime)
                    Assert.Same(finalWindow, lifetime.MainWindow);
                Assert.Equal(new float?[] { 1.25f, 2.5f, null }, factory.CreatedSetups.Select(value => value.NativeGaussianSpare));
                Assert.All(storage.LastStartWrites, bytes =>
                {
                    Assert.DoesNotContain("Gaussian", System.Text.Encoding.UTF8.GetString(bytes), StringComparison.OrdinalIgnoreCase);
                    Assert.DoesNotContain("NativeSunChange", System.Text.Encoding.UTF8.GetString(bytes), StringComparison.OrdinalIgnoreCase);
                });
            }
            finally { await CloseNavigationWindowsAsync(windows); }
        }, CancellationToken.None);
    }

    private static void AssertReadBeforeDisposeWithoutTicks(AuditEngineClient client, float? expected)
    {
        Assert.Equal(expected, Assert.Single(client.GaussianSpareReads));
        var calls = client.Calls.ToArray();
        int read = Array.IndexOf(calls, "ReadNativeGaussianSpare");
        Assert.True(read >= 0 && read < Array.IndexOf(calls, "Dispose"), string.Join(", ", calls));
        int sun = Array.IndexOf(calls, "ReadNativeSunChange");
        Assert.True(sun > read && sun < Array.IndexOf(calls, "Dispose"), string.Join(", ", calls));
        Assert.DoesNotContain(calls, value => value.StartsWith("Tick:", StringComparison.Ordinal));
    }

    [Fact]
    public async Task FailedInitialCreationRetainsPendingContextForRetryThenResetReadsCurrentState()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var factory = new AuditEngineClientFactory { FailedCreatesRemaining = 1 };
            var storage = new AuditStorageService();
            var windows = new List<Window>();
            using var opened = Window.WindowOpenedEvent.AddClassHandler<Window>((window, _) => windows.Add(window));
            var original = new MainWindow([], new WorldSetupOptions
            {
                NativeGaussianSpare = 1.25f, NativeSunChange = 12,
                Species = [new("Creation Retry", "start stop", false, 0xff123456, 1, 1000, 0, false, 0)],
            }, factory, storage);
            try
            {
                ShowForInteraction(original, 1280, 900);
                await WaitFor(() => Field<MainWindowViewModel>(original, "_viewModel").Status.Contains("Controlled initial creation failure", StringComparison.Ordinal));
                Assert.Empty(factory.Clients);
                Assert.Equal(1.25f, typeof(MainWindow).GetField("_initialNativeGaussianSpare", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(original));
                Assert.Equal((byte)12, typeof(MainWindow).GetField("_initialNativeSunChange", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(original));
                Named<Button>(original, "ResetButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => factory.Clients.TryPeek(out var client) && client.Calls.Contains("ImportSpecies:Creation Retry"));
                var retry = windows.OfType<MainWindow>().Single(value => value != original);
                var retryClient = Assert.Single(factory.Clients);
                Assert.Equal(2, factory.CreatedSetups.Count);
                Assert.All(factory.CreatedSetups, options =>
                {
                    Assert.Equal(1.25f, options.NativeGaussianSpare);
                    Assert.Equal((byte)12, options.NativeSunChange);
                });
                Assert.Null(typeof(MainWindow).GetField("_initialNativeGaussianSpare", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(retry));
                Assert.Null(typeof(MainWindow).GetField("_initialNativeSunChange", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(retry));
                retryClient.NativeGaussianSpare = 3.5f;
                retryClient.NativeSunChange = 2;

                Named<Button>(retry, "ResetButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

                await WaitFor(() => factory.Clients.Count == 2 && retryClient.Calls.Contains("Dispose"));
                await WaitFor(() => factory.Clients.Last().Calls.Contains("ImportSpecies:Creation Retry"));
                Assert.Equal(3.5f, factory.CreatedSetups.Last().NativeGaussianSpare);
                Assert.Equal((byte)2, factory.CreatedSetups.Last().NativeSunChange);
                AssertReadBeforeDisposeWithoutTicks(retryClient, 3.5f);
                Assert.Equal((byte)2, Assert.Single(retryClient.SunChangeReads));
            }
            finally { await CloseNavigationWindowsAsync(windows); }
        }, CancellationToken.None);
    }

    private static async Task CloseNavigationWindowsAsync(IEnumerable<Window> windows)
    {
        foreach (var window in windows.Reverse().ToArray())
        {
            if (window is MainWindow main) Field<DispatcherTimer>(main, "_runTimer").Stop();
            window.Close();
            if (window is MainWindow && typeof(MainWindow).GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window) is SimulationSession session)
                await session.DisposeAsync();
        }
    }

    private static readonly IReadOnlyDictionary<string, string[]> Coverage =
        new Dictionary<string, string[]>(StringComparer.Ordinal)
        {
            [nameof(SetupRenderedControlsDispatchThroughVisualTree)] =
            [
                "setup.file", "setup.file.new", "setup.file.open", "setup.file.recover", "setup.file.exit",
                "setup.view", "setup.view.advanced", "setup.help", "setup.help.about",
                "setup.mode.starter", "setup.mode.zerobot", "setup.mode.zerobot_veg",
                "setup.species.name", "setup.species.vegetable", "setup.species.count", "setup.species.energy",
                "setup.species.mutation", "setup.species.mutation_label", "setup.species.reseed",
                "setup.species.remove", "setup.species.add", "setup.backend", "setup.world.width",
                "setup.world.height", "setup.population_cap", "setup.vegetable_cap", "setup.speed",
                "setup.sustenance", "setup.auto_progression", "setup.advanced_button", "setup.create",
                "setup.recover_button",
            ],
            [nameof(RuntimeRenderedControlsDispatchThroughVisualTree)] =
            [
                "live.run", "live.pause", "live.step", "live.turbo", "live.import", "live.reset",
                "live.speed", "live.backend", "live.save", "live.load", "live.toroidal", "live.render_waste",
                "live.add_obstacle", "live.add_teleporter", "live.remove_feature", "live.live_physics",
                "live.show_vision", "live.metabolism", "live.plant_energy", "live.feeding_body",
                "live.apply_energy", "live.advance_zerobot", "live.viewport.select", "live.viewport.drag",
                "live.dna.edit", "live.move", "live.clone", "live.reproduce", "live.kill", "live.follow",
                "live.expander.world", "live.expander.parameters", "live.expander.environment",
                "live.expander.physics", "live.expander.senses", "live.expander.energy",
                "live.expander.history", "live.expander.statistics", "live.expander.experiments",
                "live.expander.league", "live.local_mode.kind", "live.local_mode.preset",
                "live.local_mode.minimum_rounds", "live.local_mode.maximum_rounds",
                "live.local_mode.maximum_cycles", "live.local_mode.population_cap",
                "live.local_mode.restart", "live.local_mode.info", "live.local_mode.viruses",
                "live.local_mode.sexual", "live.local_mode.configure", "live.local_mode.start",
                "live.local_mode.restart_button", "live.local_mode.adjudicate", "live.local_mode.reason",
            ],
            [nameof(PopulationSeedingDropdownShowsZeroScoresAndExportsTemplatesThroughTheFolderPicker)] =
            ["live.local_mode.workflow", "live.local_mode.finish_with_ladder", "live.local_mode.export_population_seeds"],
            [nameof(CompletedClassicDnaExportUsesTheFolderPickerAndPreservesOriginalFiles)] =
            ["live.local_mode.export_classic_dna"],
            [nameof(AdvancedSettingsRenderedControlsRoundTripEveryField)] =
            [
                "advanced.metabolism", "advanced.vegetableenergy", "advanced.sunlight",
                "advanced.autospeciation", "advanced.speciationdistance", "advanced.toroidalworld",
                "advanced.maxvelocity", "advanced.movementefficiency", "advanced.gravityx", "advanced.gravityy",
                "advanced.surfacegravity", "advanced.staticfriction", "advanced.kineticfriction",
                "advanced.density", "advanced.viscosity", "advanced.elasticity", "advanced.drag",
                "advanced.brownian", "advanced.shotspeed", "advanced.shotrangemultiplier",
                "advanced.shotdecay", "advanced.energyshotsnodecay", "advanced.wasteshotsnodecay",
                "advanced.feedingproportional", "advanced.feedingfixedpower", "advanced.feedingproportion",
                "advanced.startchloroplasts", "advanced.maxplantenergy",
                "advanced.minimumchloroplastequivalents", "advanced.repopulationamount",
                "advanced.repopulationcooldown", "advanced.feedingtobody", "advanced.daytime",
                "advanced.daynightenabled", "advanced.cyclelength", "advanced.cancel", "advanced.apply",
            ],
            [nameof(GlobalSettingsRenderedSurfaceShowsFourTabs)] =
            [
                "global.main", "global.advanced_mutation", "global.league", "global.evolution",
                "global.recognition_fudging", "global.cancel", "global.apply",
            ],
            [nameof(DnaEditorRenderedControlsDispatchThroughVisualTree)] =
            ["dna.editor", "dna.save", "dna.clone", "dna.apply"],
        };

    [Fact]
    public void InventoryDefinesEveryCurrentInteractiveSurfaceExactlyOnce()
    {
        var ids = Coverage.Values.SelectMany(value => value).ToArray();

        Assert.Equal(139, ids.Length);
        Assert.Equal(ids.Length, ids.Distinct(StringComparer.Ordinal).Count());
        Assert.All(ids, id => Assert.False(string.IsNullOrWhiteSpace(id)));
    }

    [Fact]
    public async Task ClassicResetControlRendersItsPlayerFacingPresentation()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), new AuditEngineClientFactory(), new AuditStorageService());
            try
            {
                ShowForInteraction(window, 1280, 900);
                var reset = Named<Button>(window, "ResetButton");

                Assert.Equal("RESET", reset.Content?.ToString());
                Assert.Equal("RESET SIMULATION", AutomationProperties.GetName(reset));
                Assert.Equal(
                    "Stop the current simulation and start a fresh local world.",
                    AutomationProperties.GetHelpText(reset));
                Capture(window, "run-01-classic-reset.png");
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }

            return 0;
        }, CancellationToken.None);
    }

    [Fact]
    public async Task SetupRenderedControlsDispatchThroughVisualTree()
    {
        var storage = new AuditStorageService
        {
            DnaFiles = [new DesktopTextFile("Audit_Bot.txt", "start\n10 .up store\nstop")],
            PickedSimulationPath = Path.Combine(Path.GetTempPath(), "audit-open.db3s"),
            LatestAutosave = Path.Combine(Path.GetTempPath(), "audit-autosave.db3s"),
        };
        var headless = DesktopTestApplication.Session;
        SetupWindow? window = null;
        var created = new List<WorldSetupOptions>();

        await headless.Dispatch(() =>
        {
            window = new SetupWindow(storage);
            window.WorldCreated += created.Add;
            ShowAndCapture(window, 1280, 820, "setup-initial.png");

            OpenAndCloseMenu(window, "FileMenu");
            OpenAndCloseMenu(window, "ViewMenu");
            OpenAndCloseMenu(window, "HelpMenu");

            Click(window, Named<RadioButton>(window, "ZerobotMode"));
            Assert.Single(window.Species);
            Click(window, Named<RadioButton>(window, "ZerobotVegetableMode"));
            Assert.Equal(2, window.Species.Count);
            Click(window, Named<RadioButton>(window, "StarterMode"));
            Assert.Equal(2, window.Species.Count);

            var firstRow = window.Species[0];
            var rowControls = Named<ItemsControl>(window, "SpeciesList").GetVisualDescendants().ToArray();
            var name = rowControls.OfType<TextBox>().First(control => ReferenceEquals(control.DataContext, firstRow));
            name.Focus();
            name.SelectAll();
            window.KeyTextInput("Audit Alga");
            Assert.Equal("Audit Alga", firstRow.Name);

            var rowChecks = rowControls.OfType<CheckBox>().Where(control => ReferenceEquals(control.DataContext, firstRow)).ToArray();
            var vegetable = rowChecks.Single(control => Equals(control.Content, "VEG"));
            var vegetableBefore = firstRow.Vegetable;
            Click(window, vegetable);
            Assert.Equal(!vegetableBefore, firstRow.Vegetable);
            var reseed = rowChecks.Single(control => control.Content is null);
            var reseedBefore = firstRow.Reseed;
            Click(window, reseed);
            Assert.Equal(!reseedBefore, firstRow.Reseed);

            var rowNumbers = rowControls.OfType<NumericUpDown>().Where(control => ReferenceEquals(control.DataContext, firstRow)).ToArray();
            Assert.Equal(2, rowNumbers.Length);
            AssertKeyboardIncrement(window, rowNumbers[0]);
            AssertKeyboardIncrement(window, rowNumbers[1]);

            var slider = rowControls.OfType<Slider>().First(control => ReferenceEquals(control.DataContext, firstRow));
            DragSlider(window, slider, 0.20);
            var lowMutationRate = firstRow.MutationRate;
            DragSlider(window, slider, 0.80);
            var highMutationRate = firstRow.MutationRate;
            Assert.True(
                highMutationRate > lowMutationRate + 40,
                $"Mutation slider did not traverse its range: low={lowMutationRate:0.##}, high={highMutationRate:0.##}.");
            var mutationLabel = rowControls.OfType<TextBlock>()
                .First(control => ReferenceEquals(control.DataContext, firstRow)
                    && control.Text?.Contains('%', StringComparison.Ordinal) == true);
            Assert.Contains('%', mutationLabel.Text!);

            var remove = rowControls.OfType<Button>().Last(control => control.Tag is SetupSpeciesRow);
            Click(window, remove);
            Assert.Single(window.Species);
            Click(window, Named<Button>(window, "AddDnaButton"));
            Assert.Equal(2, window.Species.Count);
            Assert.Equal("Audit_Bot", window.Species[^1].Name);

            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "WorldWidth"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "WorldHeight"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "PopulationCap"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "VegetablePopulationCap"));
            AssertComboSelection(window, Named<ComboBox>(window, "Backend"), 1);
            AssertComboSelection(window, Named<ComboBox>(window, "Speed"), 1);

            Click(window, Named<RadioButton>(window, "ZerobotMode"));
            var sustenance = Named<ComboBox>(window, "Sustenance");
            Assert.True(sustenance.IsEnabled);
            AssertComboSelection(window, sustenance, 1);
            var progression = Named<CheckBox>(window, "AutomaticProgression");
            var progressionBefore = progression.IsChecked;
            Click(window, progression);
            Assert.NotEqual(progressionBefore, progression.IsChecked);

            ActivateMenu(Named<MenuItem>(window, "OpenSaveMenuItem"));
            Assert.Equal(storage.PickedSimulationPath, created[^1].LoadSavePath);
            ActivateMenu(Named<MenuItem>(window, "RecoverMenuItem"));
            Assert.Equal(storage.LatestAutosave, created[^1].LoadSavePath);

            Click(window, Named<Button>(window, "AdvancedSettingsButton"));
            var advanced = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
            Click(advanced, Named<Button>(advanced, "ApplyButton"));
            Assert.Equal("ADVANCED SETTINGS APPLIED", Named<TextBlock>(window, "SetupStatus").Text);

            ActivateMenu(Named<MenuItem>(window, "AdvancedMenuItem"));
            advanced = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
            Click(advanced, Named<Button>(advanced, "CancelButton"));

            ActivateMenu(Named<MenuItem>(window, "AboutMenuItem"));
            var about = Assert.Single(window.OwnedWindows, value => value.Title == "About Darwinbots Modern");
            Click(about, about.GetVisualDescendants().OfType<Button>().Single());

            Click(window, Named<Button>(window, "CreateWorldButton"));
            Assert.Equal(StartingMode.Zerobots, created[^1].StartingMode);
            Click(window, Named<Button>(window, "RecoverButton"));
            Assert.Equal(storage.LatestAutosave, created[^1].LoadSavePath);

            ActivateMenu(Named<MenuItem>(window, "NewWorldMenuItem"));
            Assert.True(Named<RadioButton>(window, "StarterMode").IsChecked);
            Assert.Equal(3, Named<ComboBox>(window, "Speed").SelectedIndex);
            Capture(window, "setup-exercised.png");
            window.Close();

            var exitWindow = new SetupWindow(storage);
            ShowForInteraction(exitWindow, 1280, 820);
            ActivateMenu(Named<MenuItem>(exitWindow, "ExitMenuItem"));
            Assert.False(exitWindow.IsVisible);
        }, CancellationToken.None);
    }

    [Fact]
    public async Task MainWindowRendersSafemodeRecoveryAndRecoversAutosave()
    {
        var artifactPath = Path.Combine(Path.GetTempPath(), $"db24832-io05-recovery-{Guid.NewGuid():N}.db3s");
        File.WriteAllBytes(artifactPath, [9, 8, 7]);
        var storage = new AuditStorageService { LatestAutosave = artifactPath };
        var factory = new AuditEngineClientFactory();

        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var window = new MainWindow(
                    [],
                    new WorldSetupOptions
                    {
                        Species = [new SpeciesImport("IO-05 Recovery Species", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
                        GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { SafeMode = true } },
                    },
                    factory,
                    storage);
                try
                {
                    ShowForInteraction(window, 1_200, 820);
                    var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                    await WaitFor(() => viewModel.Status.Contains("WORLD READY", StringComparison.Ordinal));

                    var menu = Named<LegacyMenuBar>(window, "HistoricMenuBar");
                    var safemode = menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.RecordingSafemodeBackup)!;
                    Assert.True(safemode.IsChecked);
                    safemode.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => safemode.IsChecked == false);
                    Assert.Equal("SAFEMODE BACKUP OFF", viewModel.Status);
                    safemode.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => safemode.IsChecked == true);
                    Assert.Equal("SAFEMODE BACKUP ON", viewModel.Status);

                    var handleError = typeof(MainWindow).GetMethod("HandleEngineErrorAsync", BindingFlags.Instance | BindingFlags.NonPublic)!;
                    await (Task)handleError.Invoke(window, [new AccessViolationException("IO-05 rendered recovery fault"), true])!;

                    var panel = Named<Border>(window, "RecoveryPanel");
                    var title = Named<TextBlock>(window, "RecoveryTitle");
                    var faultWindow = Named<TextBlock>(window, "RecoveryFaultWindowText");
                    var artifact = Named<TextBlock>(window, "RecoveryArtifactText");
                    var recover = Named<Button>(window, "RecoveryRecoverButton");
                    var ignore = Named<Button>(window, "RecoveryIgnoreButton");
                    Assert.True(panel.IsVisible);
                    Assert.Equal("SAFE MODE RECOVERY READY", title.Text);
                    Assert.Contains("FAULT WINDOW", faultWindow.Text, StringComparison.Ordinal);
                    Assert.Equal($"RECOVERY ARTIFACT · {Path.GetFileName(artifactPath)}", artifact.Text);
                    Assert.True(recover.IsVisible);
                    Assert.True(ignore.IsVisible);
                    Assert.Contains("RECOVERY READY", viewModel.Status, StringComparison.Ordinal);
                    Assert.Contains(
                        viewModel.Status,
                        window.GetVisualDescendants().OfType<TextBlock>().Select(value => value.Text));

                    Capture(window, "io05-autosave-safemode-recovery.png");
                    using var frame = window.CaptureRenderedFrame();
                    using var png = new MemoryStream();
                    frame.Save(png, new PngBitmapEncoderOptions());
                    var bytes = png.ToArray();
                    Assert.True(bytes.Length > 1_000, $"Rendered PNG was only {bytes.Length} bytes.");
                    Assert.Equal(new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 }, bytes.Take(8));

                    Click(window, recover);
                    await WaitFor(() => viewModel.Status.StartsWith("RECOVERED ·", StringComparison.Ordinal));
                    Assert.False(panel.IsVisible);
                    Assert.Equal($"RECOVERED · {Path.GetFileName(artifactPath)}", viewModel.Status);
                }
                finally
                {
                    window.Close();
                }
            }, CancellationToken.None);
        }
        finally
        {
            if (File.Exists(artifactPath)) File.Delete(artifactPath);
        }
    }

    [Fact]
    public async Task AdvancedSettingsRenderedControlsRoundTripEveryField()
    {
        var headless = DesktopTestApplication.Session;
        await headless.Dispatch(() =>
        {
            var snapshot = EngineSnapshot.Empty with
            {
                Species =
                [
                    new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false),
                    new SpeciesSnapshot("Animal Minimalis", false, 0xff239ac0, 0, false, 1f),
                    new SpeciesSnapshot("Alga Minimalis", true, 0xff4f9f45, 0, false, 0.5f),
                ],
            };
            var window = new AdvancedSettingsWindow(EnvironmentUpdate.Default, snapshot);
            ShowAndCapture(window, 860, 760, "advanced-initial.png");
            var tabs = window.GetVisualDescendants().OfType<TabControl>().Single();
            (int Tab, string[] Numeric, string[] Checks)[] groups =
            [
                (0, ["GlobalMutationRate"], ["MutationsEnabled"]),
                (1, ["Metabolism", "VegetableEnergy", "Sunlight", "StartChloroplasts", "MaxPlantEnergy", "MinimumChloroplastEquivalents", "RepopulationAmount", "RepopulationCooldown", "FeedingToBody", "CycleLength"], ["Daytime", "DayNightEnabled"]),
                (2, ["MaxVelocity", "MovementEfficiency", "GravityX", "GravityY", "SurfaceGravity", "StaticFriction", "KineticFriction", "Density", "Viscosity", "Elasticity", "Drag", "Brownian", "ShotSpeed", "ShotRangeMultiplier", "ShotDecay", "FeedingFixedPower", "FeedingProportion"], ["EnergyShotsNoDecay", "WasteShotsNoDecay", "FeedingProportional"]),
                (3, ["SpeciationDistance"], ["AutoSpeciation", "ToroidalWorld"]),
            ];
            foreach (var group in groups)
            {
                tabs.SelectedIndex = group.Tab;
                window.UpdateLayout();
                foreach (var name in group.Numeric) AssertKeyboardIncrement(window, Named<NumericUpDown>(window, name), resetToMinimum: true);
                foreach (var name in group.Checks)
                {
                    var check = Named<CheckBox>(window, name);
                    var before = check.IsChecked;
                    Click(window, check);
                    Assert.True(before != check.IsChecked, $"{name} did not toggle from {before}.");
                }
                if (group.Tab == 2) Capture(window, "feeding-settings-controls.png");
            }

            var update = window.Update;
            Assert.False(update.Shots.FeedingProportional);
            Assert.Equal((int)Named<NumericUpDown>(window, "FeedingFixedPower").Value!, update.Shots.FeedingFixedPower);
            Assert.Equal((float)Named<NumericUpDown>(window, "FeedingProportion").Value!, update.Shots.FeedingProportion);
            Assert.NotEqual(EnvironmentUpdate.Default, update);
            Assert.NotEqual(MutationSettingsUpdate.Default, window.MutationUpdate);
            Assert.Equal(2, window.MutationUpdate.SpeciesRates.Count);
            Assert.True(Named<Border>(window, "DirtyBadge").IsVisible);
            Click(window, Named<Button>(window, "ApplyButton"));
            Assert.True(window.Accepted);
            Assert.False(window.IsVisible);

            var cancel = new AdvancedSettingsWindow(EnvironmentUpdate.Default);
            ShowForInteraction(cancel, 860, 760);
            Click(cancel, Named<Button>(cancel, "CancelButton"));
            Assert.False(cancel.Accepted);
            Assert.False(cancel.IsVisible);
        }, CancellationToken.None);
    }

    [Fact]
    public async Task AdvancedWeatherCheckboxRetainsNativeLoadedSunOnRndUntilThePlayerTurnsItOff()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            using var source = new NativeEngineClient(new WorldSetupOptions
            {
                Backend = "Cpu", Seed = 42,
                Vegetation = Db2VegetationOptions.Default with { WeatherEnabled = false, SunOnRandom = true },
            });
            using var loaded = new NativeEngineClient(new WorldSetupOptions { Backend = "Cpu" });
            loaded.Load(source.SaveLegacy());
            var environment = loaded.ReadConfiguration().Environment;
            Assert.False(environment.Vegetation.WeatherEnabled);
            Assert.True(environment.Vegetation.SunOnRandom);
            var windows = new List<AdvancedSettingsWindow>();
            try
            {
                var unrelated = new AdvancedSettingsWindow(environment, loaded.Snapshot(), selectedTab: 1);
                windows.Add(unrelated);
                ShowForInteraction(unrelated, 860, 760);
                Assert.True(Named<CheckBox>(unrelated, "WeatherEnabled").IsChecked);
                Named<NumericUpDown>(unrelated, "Metabolism").Value = environment.MetabolismCost + 1;
                Click(unrelated, Named<Button>(unrelated, "ApplyButton"));
                Assert.True(unrelated.Accepted);
                loaded.UpdateEnvironment(unrelated.Update);
                environment = loaded.ReadConfiguration().Environment;
                Assert.Equal(unrelated.Update.MetabolismCost, environment.MetabolismCost);
                Assert.True(environment.Vegetation.WeatherEnabled || environment.Vegetation.SunOnRandom);

                var turnOff = new AdvancedSettingsWindow(environment, loaded.Snapshot(), selectedTab: 1);
                windows.Add(turnOff);
                ShowForInteraction(turnOff, 860, 760);
                Assert.True(Named<CheckBox>(turnOff, "WeatherEnabled").IsChecked);
                Click(turnOff, Named<CheckBox>(turnOff, "WeatherEnabled"));
                Click(turnOff, Named<Button>(turnOff, "ApplyButton"));
                Assert.True(turnOff.Accepted);
                loaded.UpdateEnvironment(turnOff.Update);
                environment = loaded.ReadConfiguration().Environment;
                Assert.False(environment.Vegetation.WeatherEnabled || environment.Vegetation.SunOnRandom);
                var savedOff = loaded.Save();

                var cancel = new AdvancedSettingsWindow(environment, loaded.Snapshot(), selectedTab: 1);
                windows.Add(cancel);
                ShowForInteraction(cancel, 860, 760);
                Click(cancel, Named<CheckBox>(cancel, "WeatherEnabled"));
                Click(cancel, Named<Button>(cancel, "CancelButton"));
                Assert.False(cancel.Accepted);
                Assert.Equal(savedOff, loaded.Save());
            }
            finally
            {
                foreach (var window in windows) window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task GlobalSettingsRenderedSurfaceShowsFourTabs()
    {
        var headless = DesktopTestApplication.Session;
        await headless.Dispatch(() =>
        {
            var window = new GlobalSettingsWindow(new GlobalSettings
            {
                Main = new GlobalMainSettings { NewSeed = true },
                AdvancedMutation = new GlobalAdvancedMutationSettings { Delta2Enabled = true },
                League = new GlobalLeagueSettings { RecognitionFudging = 1 },
                Evolution = new GlobalEvolutionSettings { Mode = EvolutionMode.Zerobot },
            });
            ShowAndCapture(window, 1050, 780, "global-settings-four-tabs.png");
            var tabs = window.FindControl<TabControl>("Tabs")!;
            Assert.Equal(
                ["MAIN", "ADVANCED MUTATION", "LEAGUE", "EVOLUTION"],
                tabs.Items.Cast<TabItem>().Select(item => item.Header?.ToString() ?? string.Empty).ToArray());
            Assert.NotNull(window.FindControl<CheckBox>("NewSeed"));
            Assert.NotNull(window.FindControl<CheckBox>("Delta2Enabled"));
            Assert.NotNull(window.FindControl<CheckBox>("LeagueOfflineOwned"));
            Assert.NotNull(window.FindControl<ComboBox>("EvolutionMode"));
            var weight = Named<NumericUpDown>(window, "FindBestWeight");
            Assert.Equal(0m, weight.Minimum);
            Assert.Equal(200m, weight.Maximum);
            weight.Value = 150m;
            Capture(window, "global-settings-find-best-weight.png");
            tabs.SelectedIndex = 2;
            var fudging = Named<ComboBox>(window, "RecognitionFudging");
            StabilizeForInput(window, fudging);
            Assert.Equal(1, fudging.SelectedIndex);
            Assert.Equal(
                ["NONE", "EYES ONLY", "ALL RECOGNITION METHODS"],
                fudging.Items.Cast<ComboBoxItem>().Select(item => item.Content?.ToString()).ToArray());
            AssertComboSelection(window, fudging, 2);
            Capture(window, "global-settings-recognition-fudging.png");
            Click(window, Named<Button>(window, "ApplyButton"));
            Assert.True(window.Accepted);
            Assert.Equal(150, window.Settings.Main.EffectiveFindBestWeight);
            Assert.Equal(2, window.Settings.League.RecognitionFudging);
            window.Close();
        }, CancellationToken.None);
    }

    [Fact]
    public async Task MainWindowAppliesRecognitionFudgingAndKeepsCancelledChangesOutOfTheEngine()
    {
        var factory = new AuditEngineClientFactory();
        var setup = new WorldSetupOptions
        {
            TicksPerUpdate = 1,
            Species = [new SpeciesImport("Recognition Rival", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, factory, new AuditStorageService());
            try
            {
                ShowForInteraction(window, 1_200, 820);
                await WaitFor(() => factory.Clients.TryPeek(out var client)
                    && client.Calls.Contains("ImportSpecies:Recognition Rival"));
                var engine = factory.Clients.Single();
                var menu = Named<LegacyMenuBar>(window, "HistoricMenuBar");
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                await WaitFor(() => menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditGlobalSettings)!.IsEnabled);

                ActivateMenu(menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditGlobalSettings)!);
                await WaitFor(() => window.OwnedWindows.OfType<GlobalSettingsWindow>().Any());
                var settings = Assert.Single(window.OwnedWindows.OfType<GlobalSettingsWindow>());
                Named<TabControl>(settings, "Tabs").SelectedIndex = 2;
                AssertComboSelection(settings, Named<ComboBox>(settings, "RecognitionFudging"), 2);
                Click(settings, Named<Button>(settings, "ApplyButton"));
                await WaitFor(() => engine.Calls.Contains("RecognitionFudging:2")
                    && viewModel.Status == "GLOBAL SETTINGS SAVED");
                Assert.Equal(2, Field<GlobalSettings>(window, "_globalSettings").League.RecognitionFudging);

                ActivateMenu(menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditGlobalSettings)!);
                await WaitFor(() => window.OwnedWindows.OfType<GlobalSettingsWindow>().Any());
                var cancelled = Assert.Single(window.OwnedWindows.OfType<GlobalSettingsWindow>());
                Named<TabControl>(cancelled, "Tabs").SelectedIndex = 2;
                var fudging = Named<ComboBox>(cancelled, "RecognitionFudging");
                StabilizeForInput(cancelled, fudging);
                Assert.Equal(2, fudging.SelectedIndex);
                AssertComboSelection(cancelled, fudging, 1);
                Click(cancelled, Named<Button>(cancelled, "CancelButton"));
                await WaitFor(() => !cancelled.IsVisible);

                Assert.Equal(new[] { "RecognitionFudging:2" }, engine.Calls
                    .Where(call => call.StartsWith("RecognitionFudging:", StringComparison.Ordinal)).ToArray());
                Assert.Equal(2, Field<GlobalSettings>(window, "_globalSettings").League.RecognitionFudging);
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task DnaEditorRenderedControlsDispatchThroughVisualTree()
    {
        var engine = new AuditEngineClient();
        await using var simulation = new SimulationSession(engine);
        var storage = new AuditStorageService();
        var headless = DesktopTestApplication.Session;
        await headless.Dispatch(async () =>
        {
            var window = new DnaEditorWindow(simulation, engine.Organism, storage);
            ShowAndCapture(window, 760, 620, "dna-initial.png");
            var editor = Named<TextBox>(window, "Editor");
            await WaitFor(() =>
                engine.Calls.Contains("ExportDna")
                && editor.Text == "start\nstop");
            editor.Focus();
            editor.SelectAll();
            window.KeyTextInput("start\n20 .up store\nstop");
            await WaitFor(() => editor.Text?.Contains("20 .up store", StringComparison.Ordinal) == true);
            Click(window, Named<Button>(window, "ApplyDnaButton"));
            await WaitFor(() => engine.Calls.Contains("ReplaceDna:3:2"));
            Click(window, Named<Button>(window, "ApplyCloneButton"));
            await WaitFor(() =>
                engine.Calls.Contains("Clone:3:2")
                && engine.Calls.Contains("ReplaceDna:40:0"));
            Click(window, Named<Button>(window, "SaveBotButton"));
            await WaitFor(() => storage.SavedDna is not null);
            Capture(window, "dna-exercised.png");

            Assert.Contains("20 .up store", storage.SavedDna!, StringComparison.Ordinal);
            window.Close();
            return 0;
        }, CancellationToken.None);

    }

    [Theory]
    [InlineData(1280)]
    [InlineData(1500)]
    public async Task RuntimeHeaderRemainsClickableWithWiderFontMetricsAndBackendLabels(int width)
    {
        var factory = new AuditEngineClientFactory();
        var storage = new AuditStorageService { OpenedSimulation = [9, 8, 7, 6] };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), factory, storage);
            try
            {
                ShowForInteraction(window, width, 900);
                await WaitFor(() => !factory.Clients.IsEmpty);
                var load = Named<Button>(window, "LoadButton");
                var header = load.GetVisualAncestors().OfType<Viewbox>().First();
                foreach (var button in header.GetVisualDescendants().OfType<Button>())
                    button.FontSize = 16;
                Named<Button>(window, "BackendButton").Content = "GPU · INTEGRATED / METAL";
                Capture(window, $"runtime-header-wide-{width}.png");
                Click(window, load);
                await WaitFor(() => factory.Clients.Any(client => client.Calls.Contains("Load")));
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
            return 0;
        }, CancellationToken.None);
    }

    [Fact]
    public async Task RuntimeRenderedControlsDispatchThroughVisualTree()
    {
        var factory = new AuditEngineClientFactory();
        var storage = new AuditStorageService
        {
            DnaFiles = [new DesktopTextFile("Imported_Hunter.txt", "start\n30 .up store\nstop")],
            OpenedSimulation = [9, 8, 7, 6],
        };
        var setup = new WorldSetupOptions
        {
            TicksPerUpdate = 1,
            Species = [new SpeciesImport("Audit Species", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
        };
        var headless = DesktopTestApplication.Session;
        await headless.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, factory, storage);
            ShowAndCapture(window, 1500, 900, "runtime-initial.png");
            var historicMenu = Named<LegacyMenuBar>(window, "HistoricMenuBar");
            foreach (var topLevel in historicMenu.TopLevelItems)
            {
                topLevel.IsSubMenuOpen = true;
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                Assert.True(topLevel.IsSubMenuOpen, $"Historic menu {topLevel.Header} did not open.");
                topLevel.IsSubMenuOpen = false;
            }
            await WaitFor(
                () => factory.Clients.TryPeek(out var client) && client.Calls.Contains("ImportSpecies:Audit Species"),
                () => factory.Clients.TryPeek(out var client) ? string.Join(", ", client.Calls) : "Engine client was not created.");
            var engine = factory.Clients.First();

            var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
            Assert.Equal(3U, viewModel.SelectedSlot);

            Click(window, Named<Button>(window, "RunButton"));
            Assert.True(Field<DispatcherTimer>(window, "_runTimer").IsEnabled);
            Click(window, Named<Button>(window, "PauseButton"));
            Assert.False(Field<DispatcherTimer>(window, "_runTimer").IsEnabled);
            Click(window, Named<Button>(window, "StepButton"));

            Click(window, Named<Button>(window, "TurboButton"));
            Assert.False(Named<WorldViewport>(window, "Viewport").IsVisible);
            Click(window, Named<Button>(window, "TurboButton"));
            Assert.True(Named<WorldViewport>(window, "Viewport").IsVisible);
            Click(window, Named<Button>(window, "PauseButton"));

            AssertComboSelection(window, Named<ComboBox>(window, "RuntimeSpeed"), 2);
            Click(window, Named<Button>(window, "BackendButton"));
            Click(window, Named<Button>(window, "ImportButton"));
            Click(window, Named<Button>(window, "SaveButton"));
            Click(window, Named<Button>(window, "LoadButton"));

            ToggleAndAssert(window, Named<CheckBox>(window, "ToroidalWorldToggle"));
            ToggleAndAssert(window, Named<CheckBox>(window, "RenderWasteToggle"));

            ClickExpander(window, Named<Expander>(window, "ParametersExpander"));
            ClickExpander(window, Named<Expander>(window, "PhysicsExpander"));
            Click(window, Named<Button>(window, "LiveAdvancedButton"));
            await WaitFor(() => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any(),
                () => "The live physics button did not open its dialog.");
            var advanced = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
            Named<NumericUpDown>(advanced, "Brownian").Value = 4;
            Click(advanced, Named<Button>(advanced, "ApplyButton"));

            ClickExpander(window, Named<Expander>(window, "SensesExpander"));
            ToggleAndAssert(window, Named<CheckBox>(window, "ShowVisionToggle"));
            ClickExpander(window, Named<Expander>(window, "EnergyExpander"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "LiveMetabolism"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "LivePlantEnergy"));
            AssertKeyboardIncrement(window, Named<NumericUpDown>(window, "LiveFeedingToBody"));
            Click(window, Named<Button>(window, "ApplyEnergyButton"));

            ClickExpander(window, Named<Expander>(window, "HistoryExpander"));
            ClickExpander(window, Named<Expander>(window, "StatisticsExpander"));
            ClickExpander(window, Named<Expander>(window, "ExperimentsExpander"));
            Click(window, Named<Button>(window, "AdvanceZerobotButton"));

            AssertComboSelection(window, Named<ComboBox>(window, "LocalModeKindSelector"), 1);
            AssertComboSelection(window, Named<ComboBox>(window, "LocalModePreset"), 1);
            Named<NumericUpDown>(window, "ModeMinimumRounds").Value = 1;
            Named<NumericUpDown>(window, "ModeMaximumRounds").Value = 2;
            Named<NumericUpDown>(window, "ModeMaximumCycles").Value = 120;
            Named<NumericUpDown>(window, "ModePopulationCap").Value = 16;
            ToggleAndAssert(window, Named<CheckBox>(window, "ModeRestartOnExtinction"));
            ToggleAndAssert(window, Named<CheckBox>(window, "ModeProhibitInformation"));
            ToggleAndAssert(window, Named<CheckBox>(window, "ModeProhibitViruses"));
            ToggleAndAssert(window, Named<CheckBox>(window, "ModeProhibitSexual"));
            Named<TextBox>(window, "LocalModeReason").Text = "rendered adjudication";
            Click(window, Named<Button>(window, "ConfigureModeButton"));
            Click(window, Named<Button>(window, "StartModeButton"));
            Click(window, Named<Button>(window, "RestartModeButton"));
            Click(window, Named<Button>(window, "AdjudicateModeButton"));

            var viewport = Named<WorldViewport>(window, "Viewport");
            ClickAt(window, viewport, 0.5, 0.5);
            Assert.Equal(3U, viewModel.SelectedSlot);

            Click(window, Named<Button>(window, "MoveButton"));
            ClickAt(window, viewport, 0.82, 0.78);
            DragAt(window, viewport, 0.5, 0.5, 0.57, 0.58);

            Click(window, Named<Button>(window, "AddObstacleButton"));
            ClickAt(window, viewport, 0.30, 0.30);
            ClickAt(window, viewport, 0.40, 0.40);
            Click(window, Named<Button>(window, "AddTeleporterButton"));
            ClickAt(window, viewport, 0.60, 0.60);
            ClickAt(window, viewport, 0.70, 0.70);

            ClickAt(window, viewport, 0.09375, 0.125);
            Click(window, Named<Button>(window, "RemoveFeatureButton"));
            await WaitFor(() => window.OwnedWindows.OfType<DeleteConfirmationWindow>().Any());
            var confirmation = window.OwnedWindows.OfType<DeleteConfirmationWindow>().Single();
            Click(confirmation, Named<Button>(confirmation, "ConfirmButton"));

            Click(window, Named<Button>(window, "CloneButton"));
            Click(window, Named<Button>(window, "ReproduceButton"));
            Click(window, Named<Button>(window, "FollowButton"));
            Click(window, Named<Button>(window, "FollowButton"));

            // The fixture does not create the child returned by its reproduction stub.
            // Select the original robot through the viewport before testing its editor.
            ClickAt(window, viewport, 0.5, 0.5);
            Assert.Equal(3U, viewModel.SelectedSlot);
            Click(window, Named<Button>(window, "EditDnaButton"));
            await WaitFor(() => window.OwnedWindows.OfType<DnaEditorWindow>().Any());
            var editor = Assert.Single(window.OwnedWindows.OfType<DnaEditorWindow>());
            editor.Close();
            await WaitFor(() => !window.OwnedWindows.OfType<DnaEditorWindow>().Any());
            Capture(window, "runtime-before-kill.png");

            Click(window, Named<Button>(window, "KillButton"));
            ClickExpander(window, Named<Expander>(window, "EnvironmentExpander"));
            ClickExpander(window, Named<Expander>(window, "WorldExpander"));

            await WaitFor(() =>
                engine.Calls.Any(call => call.StartsWith("Tick:", StringComparison.Ordinal))
                && engine.Calls.Contains("Switch:Gpu")
                && engine.Calls.Contains("ImportSpecies:Imported_Hunter")
                && engine.Calls.Contains("Save")
                && engine.Calls.Contains("Load")
                && engine.Calls.Any(call => call.StartsWith("Move:3:2", StringComparison.Ordinal))
                && engine.Calls.Contains("AddObstacle")
                && engine.Calls.Contains("AddTeleporter")
                && engine.Calls.Contains("RemoveObstacle:7")
                && engine.Calls.Contains("Clone:3:2")
                && engine.Calls.Contains("Reproduce:3:2")
                && engine.Calls.Contains("Remove:3:2")
                && engine.Calls.Contains("ConfigureLocalMode")
                && engine.Calls.Contains("StartLocalMode")
                && engine.Calls.Contains("RestartLocalMode")
                && engine.Calls.Any(call => call.StartsWith("AdjudicateLocalRound:", StringComparison.Ordinal))
                && engine.EnvironmentUpdates.Count >= 3
                && storage.SavedSimulation is not null,
                () => string.Join(", ", engine.Calls));
            window.Height = 1200;
            AvaloniaHeadlessPlatform.ForceRenderTimerTick();
            Capture(window, "runtime-exercised.png");

            Click(window, Named<Button>(window, "ResetButton"));
            await WaitFor(() => factory.Clients.Count >= 2);
            return 0;
        }, CancellationToken.None);
    }

    [Fact]
    public async Task LegacyToolbarRoutesRemainingSlotsAndExposesAccessibleState()
    {
        var factory = new AuditEngineClientFactory();
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            Species = [new SpeciesImport("Audit Species", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
            GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { SafeMode = true } },
        };
        var callbackResult = new TaskCompletionSource<Exception?>(TaskCreationOptions.RunContinuationsAsynchronously);

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            try
            {
                var window = new MainWindow([], setup, factory, new AuditStorageService());
                try
                {
                    ShowForInteraction(window, 1280, 900);
                    await WaitFor(
                        () => factory.Clients.TryPeek(out var client) && client.Calls.Contains("ImportSpecies:Audit Species"),
                        () => "Audit engine did not import the toolbar test species.");

                    var toolbar = Named<UniformGrid>(window, "LegacyToolbar");
                    var slots = toolbar.GetVisualDescendants()
                        .Where(control => control.Name?.StartsWith("ToolbarSlot", StringComparison.Ordinal) == true)
                        .OrderBy(control => control.Name)
                        .ToArray();
                    Assert.Equal(24, slots.Length);
                    Assert.Equal(5, slots.OfType<Border>().Count());
                    Assert.Equal(18, slots.OfType<Button>().Count());
                    Assert.Single(slots.OfType<ComboBox>());
                    Assert.All(slots, slot => Assert.False(string.IsNullOrWhiteSpace(AutomationProperties.GetName(slot))));

                    var buttons = slots.OfType<Button>().ToArray();
                    var tabIndexes = buttons.Select(KeyboardNavigation.GetTabIndex).ToArray();
                    Assert.Equal(tabIndexes.OrderBy(index => index).ToArray(), tabIndexes);
                    Assert.Equal("NEW SIMULATION", AutomationProperties.GetName(buttons[0]));

                    var viewport = Named<WorldViewport>(window, "Viewport");
                    Assert.True(viewport.Focusable);
                    Assert.Equal("Simulation playfield", AutomationProperties.GetName(viewport));
                    Assert.Contains("click", AutomationProperties.GetHelpText(viewport), StringComparison.OrdinalIgnoreCase);
                    Assert.True(Named<Border>(window, "InternetOfflineOverlay").IsVisible);

                    var engine = factory.Clients.First();
                    Capture(window, "toolbar-1280x900.png");
                    Click(window, Named<Button>(window, "SunButton"));
                    await WaitFor(
                        () => engine.EnvironmentUpdates.Any(),
                        () => "Sun toolbar route did not publish an environment update.");
                    Assert.False(engine.EnvironmentUpdates.Last().Vegetation.Daytime);

                    Click(window, Named<Button>(window, "ToolbarSlot16"));
                    await WaitFor(
                        () => Named<Button>(window, "InsertSpeciesButton").Content?.ToString() == "CANCEL INSERTION",
                        () => $"Insertion toolbar slot did not enter insertion mode: {Field<MainWindowViewModel>(window, "_viewModel").Status}");
                    Assert.True(Named<Border>(window, "RuntimeOverlay").IsVisible);
                    Assert.Contains("INSERTION", Named<TextBlock>(window, "OverlayText").Text, StringComparison.OrdinalIgnoreCase);

                    Click(window, Named<Button>(window, "ToolbarSlot19"));
                    await WaitFor(
                        () => window.OwnedWindows.OfType<GraphSuiteWindow>().Any(),
                        () => "Graph toolbar slot did not open the graph suite.");
                    window.OwnedWindows.OfType<GraphSuiteWindow>().Single().Close();

                    Click(window, Named<Button>(window, "ContestIndicator"));
                    await WaitFor(
                        () => window.OwnedWindows.OfType<ContestResultsWindow>().Any(),
                        () => "Contest indicator did not open contest results.");
                    window.OwnedWindows.OfType<ContestResultsWindow>().Single().Close();

                    Click(window, Named<Button>(window, "ToolbarSlot24"));
                    Assert.True(Field<OfflineDiagnosticRecovery>(window, "_diagnostics").IgnoreErrors);
                    Assert.Contains("ON", Named<Button>(window, "ToolbarSlot24").Content?.ToString(), StringComparison.Ordinal);
                }
                finally
                {
                    foreach (var child in window.OwnedWindows.ToArray()) child.Close();
                    window.Close();
                }
            }
            catch (Exception exception)
            {
                callbackResult.TrySetResult(exception);
                return false;
            }
            callbackResult.TrySetResult(null);
            return true;
        }, CancellationToken.None);

        var callbackFailure = await callbackResult.Task.WaitAsync(TimeSpan.FromSeconds(30));
        Assert.Null(callbackFailure);
    }

    [Fact]
    public async Task Online01RealPlayerSetupRouteKeepsHistoricalControlsVisibleDisabledAndInert()
    {
        var factory = new AuditEngineClientFactory();
        var setup = new WorldSetupOptions
        {
            Species = [new SpeciesImport("Luna", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, factory, new AuditStorageService());
            SetupWindow? setupWindow = null;
            try
            {
                ShowForInteraction(window, 1_280, 900);
                await WaitFor(
                    () => factory.Clients.TryPeek(out var client) && client.Calls.Contains("ImportSpecies:Luna"),
                    () => factory.Clients.TryPeek(out var client) ? string.Join(", ", client.Calls) : "Audit engine was not created.");

                var engine = factory.Clients.First();
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                var session = Field<SimulationSession>(window, "_session");
                // Import dispatch precedes the final startup UI presentation.
                // Capture the inert-control baseline only after both are ready.
                await WaitFor(() => viewModel.Status == "WORLD READY · 1 SPECIES"
                    && viewModel.Tick == session.LatestSnapshot.Tick
                    && viewModel.Population == session.LatestSnapshot.Population,
                    () => viewModel.Status);
                var menu = Named<LegacyMenuBar>(window, "HistoricMenuBar");
                var onlineButton = Named<Button>(window, "InternetButton");
                var offlineOverlay = Named<Border>(window, "InternetOfflineOverlay");
                var onlineCommands = new[]
                {
                    Core.LegacyMenus.LegacyCommandId.InternetEnabled,
                    Core.LegacyMenus.LegacyCommandId.InternetTeleporter,
                    Core.LegacyMenus.LegacyCommandId.EditInternetOptions,
                };

                Assert.True(offlineOverlay.IsVisible);
                Assert.Contains("disabled", AutomationProperties.GetHelpText(offlineOverlay), StringComparison.OrdinalIgnoreCase);
                Assert.False(onlineButton.IsEnabled);
                Assert.Equal("INTERNET OFFLINE", onlineButton.Content?.ToString());
                Assert.Contains("disabled", AutomationProperties.GetHelpText(onlineButton), StringComparison.OrdinalIgnoreCase);

                var statusBefore = viewModel.Status;
                var callsBefore = engine.Calls.ToArray();
                onlineButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                ClickAt(window, onlineButton, 0.5, 0.5);
                onlineButton.Focus();
                window.KeyPress(Key.Enter, RawInputModifiers.None, PhysicalKey.Enter, null);
                window.KeyRelease(Key.Enter, RawInputModifiers.None, PhysicalKey.Enter, null);
                Assert.Equal(statusBefore, viewModel.Status);
                Assert.Equal(callsBefore, engine.Calls.ToArray());

                foreach (var command in onlineCommands)
                {
                    var item = menu.FindCommandItem(command)!;
                    Assert.True(item.IsVisible, $"Historical command {command} disappeared from the player menu.");
                    Assert.False(item.IsEnabled, $"Historical command {command} must remain disabled in offline mode.");
                    Assert.Contains("online", ToolTip.GetTip(item)?.ToString() ?? string.Empty, StringComparison.OrdinalIgnoreCase);
                    item.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    Assert.Equal(statusBefore, viewModel.Status);
                    Assert.Equal(callsBefore, engine.Calls.ToArray());
                }

                setupWindow = new SetupWindow(new AuditStorageService());
                ShowForInteraction(setupWindow, 1_280, 820);
                var tabs = Named<TabControl>(setupWindow, "SimulationSettingsTabs");
                var internetTab = tabs.Items.OfType<TabItem>().Single(tab => Equals(tab.Header?.ToString(), "INTERNET"));
                Assert.True(internetTab.IsVisible);
                Assert.Contains("disabled", string.Join(" ", setupWindow.GetVisualDescendants().OfType<TextBlock>().Select(block => block.Text)), StringComparison.OrdinalIgnoreCase);
                var setupStateBefore = setupWindow.Species.Select(species => species.Name).ToArray();
                Click(setupWindow, internetTab);
                internetTab.Focus();
                setupWindow.KeyPress(Key.Enter, RawInputModifiers.None, PhysicalKey.Enter, null);
                setupWindow.KeyRelease(Key.Enter, RawInputModifiers.None, PhysicalKey.Enter, null);
                Assert.Equal(setupStateBefore, setupWindow.Species.Select(species => species.Name).ToArray());

                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var frame = setupWindow.CaptureRenderedFrame();
                Assert.True(frame.PixelSize.Width >= 1_000);
                Assert.True(frame.PixelSize.Height >= 600);
                Assert.True(offlineOverlay.Bounds.Width > 0);
                Assert.True(offlineOverlay.Bounds.Bottom <= window.Bounds.Height);
                Assert.True(internetTab.Bounds.Width > 0);
                Assert.True(internetTab.Bounds.Height > 0);
            }
            finally
            {
                setupWindow?.Close();
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task MainWindowKeepsBottomDashboardVisibleAndUsableAt1500x900()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var window = new MainWindow([], new WorldSetupOptions());
            ShowForInteraction(window, 1500, 900);
            try
            {
                Capture(window, "shell-dashboard-1500x900.png");
                var surface = Named<Grid>(window, "MainSurfaceGrid");
                var dashboard = Named<ItemsControl>(window, "StatusInstrument");
                var theme = Named<ComboBox>(window, "ThemeSelector");
                var culture = Named<ComboBox>(window, "CultureSelector");

                Assert.True(dashboard.IsVisible);
                Assert.True(dashboard.Bounds.Height >= 52, $"Dashboard height was {dashboard.Bounds.Height}.");
                Assert.InRange(dashboard.Bounds.Bottom, 0, surface.Bounds.Height);
                Assert.InRange(theme.Bounds.Bottom, 0, surface.Bounds.Height);
                Assert.InRange(culture.Bounds.Bottom, 0, surface.Bounds.Height);
                Assert.True(theme.Bounds.Width >= 90);
                Assert.True(culture.Bounds.Width >= 120);
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task MainWindowRenderedSettingsWalkthroughCoversShellMenuRunEnergyAndMutation()
    {
        var factory = new AuditEngineClientFactory();
        var setup = new WorldSetupOptions
        {
            TicksPerUpdate = 1,
            Species = [new SpeciesImport("Luna", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, factory, new AuditStorageService());
            try
            {
                ShowForInteraction(window, 1_500, 900);
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                var surface = Named<Grid>(window, "MainSurfaceGrid");
                var dashboard = Named<ItemsControl>(window, "StatusInstrument");
                var menu = Named<LegacyMenuBar>(window, "HistoricMenuBar");

                await WaitFor(
                    () => factory.Clients.TryPeek(out var client) && client.Calls.Contains("ImportSpecies:Luna"),
                    () => factory.Clients.TryPeek(out var client) ? string.Join(", ", client.Calls) : "Engine client was not created.");
                var engine = factory.Clients.First();

                Assert.True(surface.IsVisible);
                Assert.True(dashboard.IsVisible);
                await WaitFor(() => viewModel.Status.Contains("WORLD READY", StringComparison.Ordinal), () => viewModel.Status);
                Assert.True(Named<Border>(window, "InternetOfflineOverlay").IsVisible);

                var onlineButton = Named<Button>(window, "InternetButton");
                var statusBeforeOnlineInput = viewModel.Status;
                var callsBeforeOnlineInput = engine.Calls.ToArray();
                Assert.False(onlineButton.IsEnabled);
                ClickAt(window, onlineButton, 0.5, 0.5);
                Assert.Equal(statusBeforeOnlineInput, viewModel.Status);
                Assert.Equal(callsBeforeOnlineInput, engine.Calls.ToArray());
                var onlineCommand = menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.InternetEnabled)!;
                Assert.False(onlineCommand.IsEnabled);

                var generalSettingsCommand = menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditGeneralSettings)!;
                ActivateMenu(generalSettingsCommand);
                var generalSettings = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                Assert.True(generalSettings.IsVisible);
                // Inflate templates/bindings before editing numeric values; a
                // first layout must not commit stale initial editor text.
                StabilizeForInput(generalSettings, Named<TabControl>(generalSettings, "SettingsTabs"));
                Named<NumericUpDown>(generalSettings, "Metabolism").Value = 7;
                Named<NumericUpDown>(generalSettings, "VegetableEnergy").Value = 33;
                Named<NumericUpDown>(generalSettings, "Sunlight").Value = 222;
                Named<CheckBox>(generalSettings, "Daytime").IsChecked = false;
                Named<CheckBox>(generalSettings, "DayNightEnabled").IsChecked = true;
                Named<NumericUpDown>(generalSettings, "CycleLength").Value = 240;
                var appliedEnvironment = generalSettings.Update;
                Click(generalSettings, Named<Button>(generalSettings, "ApplyButton"));
                await WaitFor(() => engine.EnvironmentUpdates.Count >= 1);
                Assert.Equal(System.Text.Json.JsonSerializer.Serialize(appliedEnvironment), System.Text.Json.JsonSerializer.Serialize(engine.EnvironmentUpdates.Single()));
                Assert.Equal(System.Text.Json.JsonSerializer.Serialize(appliedEnvironment), System.Text.Json.JsonSerializer.Serialize(Field<EnvironmentUpdate>(window, "_liveEnvironment")));
                Assert.Contains("LIVE SETTINGS APPLIED", viewModel.Status, StringComparison.Ordinal);
                Assert.Equal("SUN OFF", Named<Button>(window, "SunButton").Content);
                Assert.Equal(7m, Named<NumericUpDown>(window, "LiveMetabolism").Value);
                Assert.Equal(100m, Named<NumericUpDown>(window, "LivePlantEnergy").Value);

                var mutationSettingsCommand = menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditGeneralSettings)!;
                ActivateMenu(mutationSettingsCommand);
                var mutationSettings = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                Assert.True(mutationSettings.IsVisible);
                StabilizeForInput(mutationSettings, Named<TabControl>(mutationSettings, "SettingsTabs"));
                Named<TabControl>(mutationSettings, "SettingsTabs").SelectedIndex = 0;
                mutationSettings.UpdateLayout();
                Named<CheckBox>(mutationSettings, "MutationsEnabled").IsChecked = true;
                Named<NumericUpDown>(mutationSettings, "GlobalMutationRate").Value = 1.75m;
                var appliedMutation = mutationSettings.MutationUpdate;
                var previousMutationUpdates = engine.MutationUpdates.Count;
                Click(mutationSettings, Named<Button>(mutationSettings, "ApplyButton"));
                await WaitFor(() => engine.MutationUpdates.Count == previousMutationUpdates + 1);
                Assert.Equal(System.Text.Json.JsonSerializer.Serialize(appliedMutation),
                    System.Text.Json.JsonSerializer.Serialize(engine.MutationUpdates.Last()));
                Assert.Contains("LIVE SETTINGS APPLIED", viewModel.Status, StringComparison.Ordinal);

                AssertComboSelection(window, Named<ComboBox>(window, "RuntimeSpeed"), 1);
                Assert.Contains("5 TICKS/FRAME", viewModel.Status, StringComparison.Ordinal);
                AssertComboSelection(window, Named<ComboBox>(window, "RuntimeSpeed"), 3);
                Assert.Contains("MAXIMUM", viewModel.Status, StringComparison.Ordinal);
                Click(window, Named<Button>(window, "RunButton"));
                Assert.True(Field<DispatcherTimer>(window, "_runTimer").IsEnabled);
                Click(window, Named<Button>(window, "PauseButton"));
                Assert.False(Field<DispatcherTimer>(window, "_runTimer").IsEnabled);
                Click(window, Named<Button>(window, "StepButton"));

                var turbo = Named<Button>(window, "TurboButton");
                Click(window, turbo);
                Assert.False(Named<WorldViewport>(window, "Viewport").IsVisible);
                Assert.Contains("TURBO", viewModel.Status, StringComparison.Ordinal);
                Click(window, turbo);
                Assert.True(Named<WorldViewport>(window, "Viewport").IsVisible);

                ClickExpander(window, Named<Expander>(window, "EnergyExpander"));
                Named<NumericUpDown>(window, "LiveMetabolism").Value = 6;
                Named<NumericUpDown>(window, "LivePlantEnergy").Value = 250;
                Named<NumericUpDown>(window, "LiveFeedingToBody").Value = 0.75m;
                var previousEnergyUpdates = engine.EnvironmentUpdates.Count;
                Click(window, Named<Button>(window, "ApplyEnergyButton"));
                await WaitFor(() => engine.EnvironmentUpdates.Count == previousEnergyUpdates + 1
                    && viewModel.Status.Contains("ENERGY APPLIED", StringComparison.Ordinal));
                Assert.Contains("ENERGY APPLIED", viewModel.Status, StringComparison.Ordinal);

                var sun = Named<Button>(window, "SunButton");
                var sunBefore = sun.Content?.ToString();
                Click(window, sun);
                await WaitFor(() => !string.Equals(sun.Content?.ToString(), sunBefore, StringComparison.Ordinal));
                Assert.Contains("SUN", sun.Content?.ToString(), StringComparison.Ordinal);

                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var frame = window.CaptureRenderedFrame();
                Assert.NotNull(frame);
                Assert.True(frame.PixelSize.Width >= 400);
                Assert.True(frame.PixelSize.Height >= 300);
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public async Task AdvancedMutationRowsPreserveNativeSpeciesZeroAndSkipOnlyAnEmptySentinel(bool nativeFirstSpecies)
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var snapshot = RecordingEngineClient.CreateSnapshot();
            if (nativeFirstSpecies)
                snapshot = snapshot with
                {
                    Species = [snapshot.Species[0] with { Name = "Native first species" }, snapshot.Species[1]],
                    Organisms = [snapshot.Organisms[0] with { Species = 0 }],
                };
            var settings = new AdvancedSettingsWindow(EnvironmentUpdate.Default, snapshot);
            var speciesIds = settings.MutationUpdate.SpeciesRates.Select(rate => rate.Species).ToArray();
            Assert.Equal(nativeFirstSpecies ? new uint[] { 0, 1 } : new uint[] { 1 }, speciesIds);
        }, CancellationToken.None);
    }

    [Fact]
    public async Task MainWindowMutationSurfaceRendersDefaultsAndPublishesGlobalAndSpeciesPressure()
    {
        var factory = new AuditEngineClientFactory();
        var setup = new WorldSetupOptions
        {
            TicksPerUpdate = 1,
            Species = [new SpeciesImport("Luna", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup, factory, new AuditStorageService());
            try
            {
                ShowForInteraction(window, 1_200, 820);
                await WaitFor(
                    () => factory.Clients.TryPeek(out var client) && client.Calls.Contains("ImportSpecies:Luna"),
                    () => "The real MainWindow route did not create the Luna simulation.");

                var engine = factory.Clients.Single();
                var menu = Named<LegacyMenuBar>(window, "HistoricMenuBar");
                await WaitFor(() => Field<MainWindowViewModel>(window, "_viewModel").Population == 1
                    && menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditGeneralSettings)!.IsEnabled,
                    () => Field<MainWindowViewModel>(window, "_viewModel").Status);
                ActivateMenu(menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditGeneralSettings)!);
                await WaitFor(
                    () => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any(),
                    () => "MainWindow did not open the live settings surface. " + Field<MainWindowViewModel>(window, "_viewModel").Status);

                var settings = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                Named<TabControl>(settings, "SettingsTabs").SelectedIndex = 0;
                settings.UpdateLayout();
                await Dispatcher.UIThread.InvokeAsync(() => { }, DispatcherPriority.Loaded);
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                Assert.True(settings.IsVisible);
                Assert.True(Named<CheckBox>(settings, "MutationsEnabled").IsChecked);
                Assert.Equal(1m, Named<NumericUpDown>(settings, "GlobalMutationRate").Value);
                var speciesRows = Named<StackPanel>(settings, "SpeciesMutationRows");
                var speciesRate = Assert.Single(speciesRows.GetVisualDescendants().OfType<NumericUpDown>());
                Assert.Equal(0m, speciesRate.Value);
                Assert.Contains("GLOBAL MUTATION PRESSURE", settings.GetVisualDescendants().OfType<TextBlock>().Select(text => text.Text));
                Assert.Equal(
                    Avalonia.Media.Color.Parse("#F4F0E6"),
                    Assert.IsAssignableFrom<Avalonia.Media.ISolidColorBrush>(settings.Background).Color);

                using var frame = settings.CaptureRenderedFrame();
                Assert.True(frame.PixelSize.Width >= 760);
                Assert.True(frame.PixelSize.Height >= 620);

                Named<NumericUpDown>(settings, "GlobalMutationRate").Value = 2.25m;
                speciesRate.Value = 12.5m;
                var staged = settings.MutationUpdate;
                Assert.Equal(2.25f, staged.GlobalMultiplier);
                Assert.Equal(12.5f, Assert.Single(staged.SpeciesRates).Rate);
                Click(settings, Named<Button>(settings, "ApplyButton"));
                await WaitFor(() => engine.MutationUpdates.Count == 1);
                var applied = engine.MutationUpdates.Single();
                Assert.Equal(2.25f, applied.GlobalMultiplier);
                Assert.Equal(12.5f, Assert.Single(applied.SpeciesRates).Rate);

                ActivateMenu(menu.FindCommandItem(Core.LegacyMenus.LegacyCommandId.EditGeneralSettings)!);
                await WaitFor(() => window.OwnedWindows.OfType<AdvancedSettingsWindow>().Any());
                var canceled = Assert.Single(window.OwnedWindows.OfType<AdvancedSettingsWindow>());
                Named<TabControl>(canceled, "SettingsTabs").SelectedIndex = 0;
                Named<NumericUpDown>(canceled, "GlobalMutationRate").Value = 4m;
                Click(canceled, Named<Button>(canceled, "CancelButton"));
                Assert.Equal(1, engine.MutationUpdates.Count);
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static void ShowAndCapture(Window window, double width, double height, string evidenceName)
    {
        ShowForInteraction(window, width, height);
        Capture(window, evidenceName);
    }

    private static void ShowForInteraction(Window window, double width, double height)
    {
        window.WindowState = WindowState.Normal;
        window.Width = width;
        window.Height = height;
        window.Show();
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }

    private static void Capture(Window window, string evidenceName)
    {
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        using var frame = window.CaptureRenderedFrame();
        Assert.NotNull(frame);
        Assert.True(frame.PixelSize.Width >= 400, $"Rendered width was {frame.PixelSize.Width}.");
        Assert.True(frame.PixelSize.Height >= 300, $"Rendered height was {frame.PixelSize.Height}.");
        var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR");
        if (string.IsNullOrWhiteSpace(directory))
            directory = Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
        Directory.CreateDirectory(directory);
        using var stream = File.Create(Path.Combine(directory, evidenceName));
        frame.Save(stream, new PngBitmapEncoderOptions());
    }

    private static T Named<T>(Control root, string name) where T : Control =>
        root.FindControl<T>(name) ?? throw new Xunit.Sdk.XunitException($"Missing {typeof(T).Name} named {name}.");

    private static void Click(Window window, Control control)
    {
        StabilizeForInput(window, control);
        // A stretched checkbox can have empty, non-hit-testable space beyond
        // its label under a platform's fallback font. Target its square.
        var local = new Point(control is CheckBox ? Math.Min(10, control.Bounds.Width / 2) : Math.Max(1, control.Bounds.Width / 2), Math.Max(1, control.Bounds.Height / 2));
        var point = control.TranslatePoint(local, window);
        Assert.True(point.HasValue, $"{control.Name ?? control.GetType().Name} was not attached to the rendered window.");
        AssertPointInsideWindow(window, point.Value, control);
        var hit = window.InputHitTest(point.Value) as Control;
        if (hit is null || !(ReferenceEquals(hit, control) || hit.GetVisualAncestors().Contains(control)))
            Capture(window, $"input-obscured-{control.Name ?? control.GetType().Name}.png");
        Assert.True(hit is not null && (ReferenceEquals(hit, control) || hit.GetVisualAncestors().Contains(control)),
            $"{control.Name} was obscured at {point.Value}; input hit {hit?.Name ?? hit?.GetType().Name ?? "nothing"}.");
        window.MouseDown(point.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(point.Value, MouseButton.Left, RawInputModifiers.None);
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }

    private static void ClickAt(Window window, Control control, double fractionX, double fractionY)
    {
        StabilizeForInput(window, control);
        var local = new Point(control.Bounds.Width * fractionX, control.Bounds.Height * fractionY);
        var point = control.TranslatePoint(local, window);
        Assert.True(point.HasValue);
        AssertPointInsideWindow(window, point.Value, control);
        window.MouseDown(point.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(point.Value, MouseButton.Left, RawInputModifiers.None);
    }

    private static void DragAt(
        Window window,
        Control control,
        double startFractionX,
        double startFractionY,
        double endFractionX,
        double endFractionY)
    {
        StabilizeForInput(window, control);
        var start = control.TranslatePoint(
            new Point(control.Bounds.Width * startFractionX, control.Bounds.Height * startFractionY), window);
        var end = control.TranslatePoint(
            new Point(control.Bounds.Width * endFractionX, control.Bounds.Height * endFractionY), window);
        Assert.True(start.HasValue && end.HasValue);
        AssertPointInsideWindow(window, start.Value, control);
        AssertPointInsideWindow(window, end.Value, control);
        window.MouseDown(start.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseMove(end.Value, RawInputModifiers.LeftMouseButton);
        window.MouseUp(end.Value, MouseButton.Left, RawInputModifiers.None);
    }

    private static void DragSlider(Window window, Slider slider, double fraction)
    {
        StabilizeForInput(window, slider);
        var start = slider.TranslatePoint(new Point(2, slider.Bounds.Height / 2), window);
        var end = slider.TranslatePoint(new Point(slider.Bounds.Width * fraction, slider.Bounds.Height / 2), window);
        Assert.True(start.HasValue && end.HasValue);
        AssertPointInsideWindow(window, start.Value, slider);
        AssertPointInsideWindow(window, end.Value, slider);
        window.MouseDown(start.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseMove(end.Value, RawInputModifiers.LeftMouseButton);
        window.MouseUp(end.Value, MouseButton.Left, RawInputModifiers.None);
    }

    private static void ClickExpander(Window window, Expander expander)
    {
        var header = expander.GetVisualDescendants().OfType<ToggleButton>().First();
        StabilizeForInput(window, header);
        var before = expander.IsExpanded;
        var point = header.TranslatePoint(
            new Point(Math.Max(1, header.Bounds.Width / 2), Math.Max(1, header.Bounds.Height / 2)),
            window);
        Assert.True(point.HasValue);
        AssertPointInsideWindow(window, point.Value, header);
        window.MouseDown(point.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(point.Value, MouseButton.Left, RawInputModifiers.None);
        Assert.NotEqual(before, expander.IsExpanded);
    }

    private static void OpenAndCloseMenu(Window window, string name)
    {
        var menu = Named<MenuItem>(window, name);
        Click(window, menu);
        Assert.True(menu.IsSubMenuOpen);
        menu.IsSubMenuOpen = false;
    }

    private static void ActivateMenu(MenuItem item) =>
        item.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));

    private static void ToggleAndAssert(Window window, CheckBox check)
    {
        var before = check.IsChecked;
        Click(window, check);
        Assert.NotEqual(before, check.IsChecked);
    }

    private static void AssertKeyboardIncrement(Window window, NumericUpDown numeric, bool resetToMinimum = false)
    {
        StabilizeForInput(window, numeric);
        if (resetToMinimum) numeric.Value = numeric.Minimum;
        var before = numeric.Value;
        numeric.Focus();
        window.KeyPress(Key.Up, RawInputModifiers.None, PhysicalKey.ArrowUp, null);
        window.KeyRelease(Key.Up, RawInputModifiers.None, PhysicalKey.ArrowUp, null);
        Assert.True(numeric.Value > before, $"{numeric.Name} did not increment from {before}.");
    }

    private static void AssertComboSelection(Window window, ComboBox combo, int targetIndex)
    {
        StabilizeForInput(window, combo);
        combo.SelectedIndex = Math.Max(0, targetIndex - 1);
        combo.Focus();
        window.KeyPress(Key.Down, RawInputModifiers.None, PhysicalKey.ArrowDown, null);
        window.KeyRelease(Key.Down, RawInputModifiers.None, PhysicalKey.ArrowDown, null);
        Assert.Equal(targetIndex, combo.SelectedIndex);
    }

    private static void StabilizeForInput(Window window, Control control)
    {
        control.BringIntoView();
        Dispatcher.UIThread.RunJobs();
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        Dispatcher.UIThread.RunJobs();
        // BringIntoView can queue another scroll/layout update. Synchronize
        // the rendered hit-test tree after those jobs, before choosing a point.
        window.UpdateLayout();
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
        Assert.True(window.IsVisible, $"{window.Title} was not visible while targeting {control.Name ?? control.GetType().Name}.");
    }

    private static void AssertPointInsideWindow(Window window, Point point, Control control)
    {
        Assert.InRange(point.X, 0, window.ClientSize.Width);
        Assert.InRange(point.Y, 0, window.ClientSize.Height);
    }

    private static T Field<T>(object target, string name) =>
        (T)target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(target)!;

    private static async Task WaitFor(Func<bool> condition, Func<string>? details = null)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(8);
        while (!condition() && DateTime.UtcNow < deadline) await Task.Delay(10);
        Assert.True(condition(), $"Timed out waiting for rendered control dispatch. {details?.Invoke()}");
    }
}

internal sealed class AuditStorageService : IDesktopStorageService
{
    public IReadOnlyList<DesktopTextFile> DnaFiles { get; init; } = [];
    public string? PickedSimulationPath { get; init; }
    public byte[]? OpenedSimulation { get; init; }
    public DesktopSettings? SettingsToOpen { get; init; }
    public string? LatestAutosave { get; init; }
    public byte[]? SavedSimulation { get; private set; }
    public string? SavedDna { get; private set; }
    public string? PickedPopulationSeedDirectory { get; set; }
    public ConcurrentQueue<TopLevel> PopulationSeedPickerOwners { get; } = [];
    public Task<string?> PickPopulationSeedDirectoryAsync(TopLevel owner)
    {
        PopulationSeedPickerOwners.Enqueue(owner);
        return Task.FromResult(PickedPopulationSeedDirectory);
    }
    public byte[]? SavedLivingSnapshot { get; private set; }
    public List<byte[]> DeadSnapshotWrites { get; } = [];
    public bool AcceptOrganismSave { get; init; } = true;
    public string? SavedOrganismSuggestedFileName { get; private set; }
    public byte[]? SavedOrganism { get; private set; }
    public byte[]? Autosave { get; private set; }
    public List<byte[]> AutosaveWrites { get; } = [];
    public bool PersistAutosaves { get; init; }
    public DesktopSettings? SavedSettings { get; private set; }
    public ConcurrentQueue<byte[]> LastStartWrites { get; } = [];

    public Task SaveLastStartAsync(ReadOnlyMemory<byte> setup)
    {
        LastStartWrites.Enqueue(setup.ToArray());
        return Task.CompletedTask;
    }

    public Task<IReadOnlyList<DesktopTextFile>> OpenDnaFilesAsync(TopLevel owner) => Task.FromResult(DnaFiles);
    public Task<DesktopSettings?> OpenSettingsAsync(TopLevel owner) => Task.FromResult(SettingsToOpen);
    public Task<string?> PickSimulationPathAsync(TopLevel owner) => Task.FromResult(PickedSimulationPath);
    public Task<byte[]?> OpenSimulationAsync(TopLevel owner) => Task.FromResult(OpenedSimulation);

    public Task<bool> SaveSettingsAsync(TopLevel owner, DesktopSettings settings)
    {
        SavedSettings = settings;
        return Task.FromResult(true);
    }

    public Task<bool> SaveSimulationAsync(TopLevel owner, ReadOnlyMemory<byte> save)
    {
        SavedSimulation = save.ToArray();
        return Task.FromResult(true);
    }

    public Task<bool> SaveDnaAsync(TopLevel owner, string suggestedFileName, string dna)
    {
        SavedDna = dna;
        return Task.FromResult(true);
    }

    public async Task<bool> SaveLivingSnapshotAsync(TopLevel owner, string suggestedFileName, Func<Stream, Task> writeAsync)
    {
        using var stream = new MemoryStream();
        await writeAsync(stream);
        SavedLivingSnapshot = stream.ToArray();
        return true;
    }

    public async Task<bool> AppendDeadSnapshotAsync(Func<Stream, Task> writeAsync)
    {
        using var stream = new MemoryStream();
        await writeAsync(stream);
        DeadSnapshotWrites.Add(stream.ToArray());
        return true;
    }

    public async Task<bool> SaveOrganismAsync(TopLevel owner, string suggestedFileName, Func<Stream, Task> writeAsync)
    {
        SavedOrganismSuggestedFileName = suggestedFileName;
        if (!AcceptOrganismSave) return false;
        using var stream = new MemoryStream();
        await writeAsync(stream);
        SavedOrganism = stream.ToArray();
        return true;
    }

    public string? FindLatestAutosave() => LatestAutosave;

    public async Task SaveAutosaveAsync(ReadOnlyMemory<byte> save)
    {
        Autosave = save.ToArray();
        AutosaveWrites.Add(Autosave);
        if (PersistAutosaves)
            await File.WriteAllBytesAsync(LatestAutosave ?? throw new InvalidOperationException("Missing test autosave path."), Autosave);
    }
}

internal sealed class AuditEngineClientFactory(bool includeWorldObjects = true, bool encodeTickInSave = false) : IEngineClientFactory
{
    public ConcurrentQueue<AuditEngineClient> Clients { get; } = [];
    public ConcurrentQueue<WorldSetupOptions> CreatedSetups { get; } = [];
    public int FailedCreatesRemaining { get; set; }

    public IEngineClient Create(WorldSetupOptions setup)
    {
        CreatedSetups.Enqueue(setup);
        if (FailedCreatesRemaining > 0)
        {
            FailedCreatesRemaining--;
            throw new InvalidOperationException("Controlled initial creation failure.");
        }
        var client = new AuditEngineClient(includeWorldObjects)
        {
            EncodeTickInSave = encodeTickInSave,
            NativeGaussianSpare = setup.NativeGaussianSpare,
            NativeSunChange = setup.NativeSunChange ?? 0,
        };
        Clients.Enqueue(client);
        return client;
    }
}

internal sealed class AuditEngineClient : IEngineClient
{
    private ulong _tick;
    private string _backend = "CPU";
    private EngineSnapshot _snapshot;
    private LocalModeConfig _localModeConfig = LocalModeConfig.Default;

    public AuditEngineClient(bool includeWorldObjects = true)
    {
        Organism = new OrganismSnapshot(3, 2, [8_000f, 6_000f], [0f, 0f], 1_000, 10, Species: 1);
        _snapshot = new EngineSnapshot(0, 1, _backend, [Organism])
        {
            Species =
            [
                new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false),
                new SpeciesSnapshot("Audit Species", false, 0xff239ac0, 0, false),
            ],
            RenderInstances =
            [
                new RenderInstanceSnapshot(3, [8_000f, 6_000f], 12f, 0xff239ac0) { Generation = 2 },
            ],
            Obstacles = [new ObstacleSnapshot(7, [1_000f, 1_000f], [2_000f, 2_000f])],
            Teleporters = [new TeleporterSnapshot(8, [14_000f, 10_000f], 180f, [2_000f, 2_000f])],
        };
        if (!includeWorldObjects) _snapshot = _snapshot with { Obstacles = [], Teleporters = [] };
    }

    public OrganismSnapshot Organism { get; }
    public ConcurrentQueue<string> Calls { get; } = [];
    public ConcurrentQueue<EnvironmentUpdate> EnvironmentUpdates { get; } = [];
    public ConcurrentQueue<MutationSettingsUpdate> MutationUpdates { get; } = [];
    public Dictionary<uint, string> SpeciesTemplateDna { get; } = [];
    public LocalModeConfig LastLocalModeConfiguration => _localModeConfig;
    public string Backend => _backend;
    public bool EncodeTickInSave { get; init; }
    public WorldConfiguration? ConfigurationForRead { get; set; }
    public WorldConfiguration? ReadConfiguration() => ConfigurationForRead;
    public int? NativeCycle { get; set; }
    public Func<uint, int?, int?>? AdvanceNativeCycle { get; set; }
    public LocalModeSnapshot? LocalModeForRead { get; set; }
    public float? NativeGaussianSpare { get; set; }
    public byte NativeSunChange { get; set; }
    public ConcurrentQueue<byte> SunChangeReads { get; } = [];
    public byte ReadNativeSunChange()
    {
        var sun = NativeSunChange;
        SunChangeReads.Enqueue(sun);
        Calls.Enqueue("ReadNativeSunChange");
        return sun;
    }
    public ConcurrentQueue<float?> GaussianSpareReads { get; } = [];
    public float? ReadNativeGaussianSpare()
    {
        var spare = NativeGaussianSpare;
        GaussianSpareReads.Enqueue(spare);
        Calls.Enqueue("ReadNativeGaussianSpare");
        return spare;
    }

    public EngineCapabilities Capabilities() => new(1, _backend, true, null);
    public void Tick(uint count = 1)
    {
        _tick += count;
        if (AdvanceNativeCycle is { } advance) NativeCycle = advance(count, NativeCycle);
        else if (NativeCycle is { } cycle) NativeCycle = checked(cycle + checked((int)count));
        Calls.Enqueue($"Tick:{count}");
    }
    public DnaImportReport ImportDna(string dna, float[]? position = null) { Calls.Enqueue("ImportDna"); return DnaImportReport.Compatible; }
    public DnaImportReport ImportSpecies(SpeciesImport species) { Calls.Enqueue($"ImportSpecies:{species.Name}"); return DnaImportReport.Compatible; }
    public DnaImportReport InsertSpeciesAt(SpeciesImport species, float[] position)
    {
        Calls.Enqueue($"InsertSpeciesAt:{species.Name}:{position[0]:0}:{position[1]:0}");
        return DnaImportReport.Compatible;
    }
    public void Remove(uint slot, uint generation) => Calls.Enqueue($"Remove:{slot}:{generation}");
    public void RemoveMany(IReadOnlyList<OrganismKey> organisms) => Calls.Enqueue($"RemoveMany:{string.Join(',', organisms.Select(value => $"{value.Slot}:{value.Generation}"))}");
    public void Move(uint slot, uint generation, float[] position) => Calls.Enqueue($"Move:{slot}:{generation}:{position[0]:0}:{position[1]:0}");
    public OrganismKey CloneOrganism(uint slot, uint generation, float[] position) { Calls.Enqueue($"Clone:{slot}:{generation}"); return new OrganismKey(40, 0); }
    public void ReplaceDna(uint slot, uint generation, string dna) => Calls.Enqueue($"ReplaceDna:{slot}:{generation}");
    public string ExportDna(uint slot, uint generation) { Calls.Enqueue("ExportDna"); return "start\nstop"; }
    public string ExportSpeciesTemplateDna(uint species)
    {
        Calls.Enqueue($"ExportSpeciesTemplateDna:{species}");
        return SpeciesTemplateDna[species];
    }
    public OrganismKey Reproduce(uint firstSlot, uint firstGeneration, uint? secondSlot, uint? secondGeneration, float[] position)
    {
        Calls.Enqueue($"Reproduce:{firstSlot}:{firstGeneration}");
        return new OrganismKey(41, 0);
    }
    public void SwitchBackend(string backend) { _backend = backend.ToUpperInvariant(); Calls.Enqueue($"Switch:{backend}"); }
    public void AddObstacle(ObstacleSnapshot obstacle) { Calls.Enqueue("AddObstacle"); _snapshot = _snapshot with { Obstacles = [.. _snapshot.Obstacles, obstacle] }; }
    public void UpdateObstacle(ObstacleSnapshot obstacle) { Calls.Enqueue("UpdateObstacle"); _snapshot = _snapshot with { Obstacles = _snapshot.Obstacles.Select(value => value.Id == obstacle.Id ? obstacle : value).ToArray() }; }
    public void RemoveObstacle(uint id) { Calls.Enqueue($"RemoveObstacle:{id}"); _snapshot = _snapshot with { Obstacles = _snapshot.Obstacles.Where(value => value.Id != id).ToArray() }; }
    public void AddTeleporter(TeleporterSnapshot teleporter) { Calls.Enqueue("AddTeleporter"); _snapshot = _snapshot with { Teleporters = [.. _snapshot.Teleporters, teleporter] }; }
    public void UpdateTeleporter(TeleporterSnapshot teleporter) { Calls.Enqueue("UpdateTeleporter"); _snapshot = _snapshot with { Teleporters = _snapshot.Teleporters.Select(value => value.Id == teleporter.Id ? teleporter : value).ToArray() }; }
    public void RemoveTeleporter(uint id) { Calls.Enqueue($"RemoveTeleporter:{id}"); _snapshot = _snapshot with { Teleporters = _snapshot.Teleporters.Where(value => value.Id != id).ToArray() }; }
    public void AddFileTeleporter(FileTeleporterSnapshot teleporter) { Calls.Enqueue("AddFileTeleporter"); _snapshot = _snapshot with { FileTeleporters = [.. _snapshot.FileTeleporters, teleporter] }; }
    public void UpdateFileTeleporter(FileTeleporterSnapshot teleporter) { Calls.Enqueue("UpdateFileTeleporter"); _snapshot = _snapshot with { FileTeleporters = _snapshot.FileTeleporters.Select(value => value.Id == teleporter.Id ? teleporter : value).ToArray() }; }
    public void RemoveFileTeleporter(uint id) { Calls.Enqueue($"RemoveFileTeleporter:{id}"); _snapshot = _snapshot with { FileTeleporters = _snapshot.FileTeleporters.Where(value => value.Id != id).ToArray() }; }
    public void SetBrownianMotion(float value) => Calls.Enqueue($"Brownian:{value}");
    public void UpdateEnvironment(EnvironmentUpdate update) { EnvironmentUpdates.Enqueue(update); Calls.Enqueue("Environment"); }
    public void UpdateMutationSettings(MutationSettingsUpdate update) { MutationUpdates.Enqueue(update); Calls.Enqueue("Mutations"); }
    public void SetTiesEnabled(bool enabled) => Calls.Enqueue($"Ties:{enabled}");
    public void SetSpeciesColor(uint species, uint color) => Calls.Enqueue($"Color:{species}:{color}");
    public void MakeNewSpecies(uint slot, uint generation, string name, uint color) => Calls.Enqueue($"NewSpecies:{slot}:{generation}:{name}");
    public void UpdateLegacyRestrictions(bool robotFixingAllowed, bool asexualReproductionAllowed) => Calls.Enqueue($"Restrictions:{robotFixingAllowed}:{asexualReproductionAllowed}");
    public void ConfigurePlayerBot(PlayerBotConfig config) => Calls.Enqueue("ConfigurePlayerBot");
    public void SetPlayerBotKey(uint slot, uint generation, int keyCode, bool pressed) => Calls.Enqueue($"PlayerBotKey:{slot}:{generation}:{keyCode}:{pressed}");
    public void SetOrganismMemory(uint slot, uint generation, ushort address, int value) => Calls.Enqueue($"Memory:{slot}:{generation}:{address}:{value}");
    public void UpdateRecognitionFudging(int mode) => Calls.Enqueue($"RecognitionFudging:{mode}");
    public void SetOrganismEnergy(uint slot, uint generation, int energy) => Calls.Enqueue($"Energy:{slot}:{generation}:{energy}");
    public OrganismKey? FindBestRobot(bool energyWeighting) { Calls.Enqueue("FindBestRobot"); return new(3, 2); }
    public void UpdateRestrictionOptions(RestrictionOptions options) => Calls.Enqueue("RestrictionOptions");
    public void ConfigureLocalMode(LocalModeConfig config)
    {
        Calls.Enqueue("ConfigureLocalMode");
        _localModeConfig = config;
        _snapshot = _snapshot with { LocalMode = LocalModeSnapshotFor(config, LocalModeStatus.Configured, 0, 0, 0, []) };
    }
    public void StartLocalMode()
    {
        Calls.Enqueue("StartLocalMode");
        _snapshot = _snapshot with { LocalMode = LocalModeSnapshotFor(_localModeConfig, LocalModeStatus.Running, 1, 0, 0, []) };
    }
    public void RestartLocalMode()
    {
        Calls.Enqueue("RestartLocalMode");
        _snapshot = _snapshot with { LocalMode = LocalModeSnapshotFor(_localModeConfig, LocalModeStatus.Running, 1, 0, 1, []) };
    }
    public void AdjudicateLocalRound(uint? winner, string reason)
    {
        Calls.Enqueue($"AdjudicateLocalRound:{winner?.ToString() ?? "none"}");
        var result = new LocalModeResult(1, winner, 1, 1_000, reason);
        _snapshot = _snapshot with
        {
            LocalMode = LocalModeSnapshotFor(_localModeConfig, LocalModeStatus.Completed, 1, 0, 1, [result], new LocalModeAdjudication(winner, reason)),
        };
    }

    private static LocalModeSnapshot LocalModeSnapshotFor(
        LocalModeConfig config,
        LocalModeStatus status,
        uint round,
        ulong cycle,
        uint restarts,
        IReadOnlyList<LocalModeResult> results,
        LocalModeAdjudication? adjudication = null) => new(
        status,
        config.Kind,
        config.Preset,
        round,
        config.MaximumRounds,
        cycle,
        config.MaximumCycles,
        restarts,
        results,
        adjudication,
        config.Restrictions);
    public void SetOrganismAim(uint slot, uint generation, int aim) => Calls.Enqueue($"Aim:{slot}:{generation}:{aim}");
    public byte[] Save() { Calls.Enqueue("Save"); return EncodeTickInSave ? BitConverter.GetBytes(_tick) : [1, 2, 3, 4]; }
    public byte[] SaveWithoutMutations() { Calls.Enqueue("SaveWithoutMutations"); return [4, 3, 2, 1]; }
    public void Load(byte[] save) => Calls.Enqueue("Load");
    public EngineSnapshot Snapshot() => _snapshot with
    {
        Tick = _tick, NativeCycle = NativeCycle, Backend = _backend,
        LocalMode = LocalModeForRead ?? _snapshot.LocalMode,
    };
    public void Dispose() => Calls.Enqueue("Dispose");
}
