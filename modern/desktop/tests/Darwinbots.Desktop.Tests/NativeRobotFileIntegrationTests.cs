using System.Reflection;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
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
    [Fact]
    public async Task SavingOriginalRobotDnaAsANewNameRenamesOnlyAfterWritingBothFilesAndConfirming()
    {
        const string fixture = "modern/engine/tests/fixtures/db24832-genuine-movement-cycle1-20261016.sim";
        var root = new DirectoryInfo(AppContext.BaseDirectory);
        while (root is not null && !File.Exists(Path.Combine(root.FullName, fixture))) root = root.Parent;
        Assert.NotNull(root);
        var directory = Directory.CreateTempSubdirectory("darwinbots-saved-robot-rename-");
        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var options = new WorldSetupOptions { Backend = "Cpu", LoadSavePath = Path.Combine(root.FullName, fixture) };
                var storage = new AvaloniaDesktopStorageService(directory.FullName);
                var window = new MainWindow([], options, NativeEngineClientFactory.Instance, storage);
                try
                {
                    window.Show();
                    var model = Field<MainWindowViewModel>(window, "_viewModel");
                    await WaitFor(() => model.Status == "LOADED" && Field<bool>(window, "_startupComplete"));
                    var session = Field<SimulationSession>(window, "_session");
                    Assert.Equal(1, session.LatestSnapshot.NativeCycle);
                    await session.StepAsync(2);
                    await WaitFor(() => model.NativeCycle == 3);
                    var original = Assert.Single(session.LatestSnapshot.Organisms);
                    Assert.Equal(1ul, original.UniqueId);
                    Assert.Equal("Oracle-movement.txt", original.FounderName);
                    model.Select(new OrganismKey(original.Slot, original.Generation));
                    var checkpoint = await session.SaveAsync();
                    var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;

                    string? destination = null;
                    var picker = SaveDnaPickerProxy.Create<IStorageProvider>(method =>
                    {
                        Assert.Equal(nameof(IStorageProvider.SaveFilePickerAsync), method.Name);
                        var path = destination;
                        return Task.FromResult(path is null ? null : SaveDnaPickerProxy.Create<IStorageFile>(fileMethod =>
                            fileMethod.Name == "get_Path" ? new Uri(path) : throw new NotSupportedException(fileMethod.Name)));
                    });
                    typeof(TopLevel).GetField("_storageProvider", BindingFlags.Instance | BindingFlags.NonPublic)!
                        .SetValue(window, picker);

                    void ChooseSave(string? path)
                    {
                        destination = path;
                        model.Status = "WAITING FOR DNA SAVE";
                        menu.FindCommandItem(LegacyCommandId.RobotSaveDna)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    }

                    Window? Prompt() => window.OwnedWindows.SingleOrDefault(child =>
                        child.IsVisible && child.Title == "Robot DNA saved");

                    async Task ChooseRename(string answer)
                    {
                        await WaitFor(() => Prompt() is not null);
                        var button = Assert.Single(Prompt()!.GetVisualDescendants().OfType<Button>(),
                            choice => Equals(choice.Content, answer));
                        button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                        await WaitFor(() => Prompt() is null && model.Status == "ROBOT DNA SAVED");
                    }

                    ChooseSave(null);
                    await WaitFor(() => model.Status == "CANCELED");
                    Assert.Null(Prompt());

                    var failed = Path.Combine(directory.FullName, "failed.txt");
                    Directory.CreateDirectory(Path.ChangeExtension(failed, ".mrate"));
                    ChooseSave(failed);
                    await WaitFor(() => model.Status.StartsWith("RobotSaveDna failed", StringComparison.Ordinal));
                    Assert.False(File.Exists(failed));
                    Assert.Null(Prompt());
                    Assert.Equal(original.FounderName, Assert.Single(session.LatestSnapshot.Organisms).FounderName);

                    var output = Path.Combine(directory.FullName, "Oracle,movement.txt");
                    ChooseSave(output);
                    await WaitFor(() => Prompt() is not null);
                    Assert.Contains(Prompt()!.GetVisualDescendants().OfType<TextBlock>(),
                        text => text.Text == "Do you want to change robot's name to Oracle,movement.txt ?");
                    Assert.Equal(await session.ExportDnaAsync(original.Slot, original.Generation),
                        LegacyDnaText.Decode(File.ReadAllBytes(output)));
                    Assert.Equal(Db2MutationRateCodec.Encode(original.MutationProfile),
                        File.ReadAllBytes(Path.ChangeExtension(output, ".mrate")));
                    await ChooseRename("No");
                    Assert.Equal(original.FounderName, Assert.Single(session.LatestSnapshot.Organisms).FounderName);

                    ChooseSave(Path.Combine(directory.FullName, "Discarded.txt"));
                    await WaitFor(() => Prompt() is not null);
                    var restore = typeof(MainWindow).GetMethod("LoadSimulationAsync", BindingFlags.Instance | BindingFlags.NonPublic)!;
                    await Assert.IsAssignableFrom<Task>(restore.Invoke(window, [checkpoint]));
                    await ChooseRename("Yes");
                    Assert.Equal(original.FounderName, Assert.Single(session.LatestSnapshot.Organisms).FounderName);

                    model.Select(new OrganismKey(original.Slot, original.Generation));
                    ChooseSave(output);
                    await ChooseRename("Yes");
                    var renamed = Assert.Single(session.LatestSnapshot.Organisms);
                    Assert.Equal("Oracle,movement.txt", renamed.FounderName);
                    Assert.Equal(original.UniqueId, renamed.UniqueId);
                    Assert.Equal(original.Species, renamed.Species);
                    Assert.Equal(original.Phenotype.Color, renamed.Phenotype.Color);
                    Assert.Equal(original.MutationProfile, renamed.MutationProfile);
                    Assert.Equal(3, session.LatestSnapshot.NativeCycle);
                    menu.FindCommandItem(LegacyCommandId.RobotShowInfo)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => window.OwnedWindows.OfType<RobotDetailsWindow>().Any());
                    var details = Assert.Single(window.OwnedWindows.OfType<RobotDetailsWindow>());
                    Assert.Contains(details.GetVisualDescendants().OfType<TextBlock>(),
                        text => text.Text?.Contains("Oracle,movement.txt", StringComparison.Ordinal) == true);

                    await session.StepAsync(1);
                    Assert.Equal(4, session.LatestSnapshot.NativeCycle);
                    var observed = Assert.Single(session.LatestSnapshot.Organisms);
                    Assert.Equal("Oracle,movement.txt", observed.FounderName);
                    Assert.Equal(1ul, observed.UniqueId);
                    Assert.Equal(4ul, observed.Age);
                }
                finally
                {
                    foreach (var owned in window.OwnedWindows.ToArray()) owned.Close();
                    SetField(window, "_skipLastExitCheckpoint", true);
                    window.Close();
                }
            }, CancellationToken.None);
        }
        finally { directory.Delete(recursive: true); }
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

    private class SaveDnaPickerProxy : DispatchProxy
    {
        private Func<MethodInfo, object?> _invoke = null!;

        public static T Create<T>(Func<MethodInfo, object?> invoke) where T : class
        {
            var proxy = DispatchProxy.Create<T, SaveDnaPickerProxy>();
            ((SaveDnaPickerProxy)(object)proxy)._invoke = invoke;
            return proxy;
        }

        protected override object? Invoke(MethodInfo? method, object?[]? arguments) =>
            _invoke(method ?? throw new MissingMethodException());
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
