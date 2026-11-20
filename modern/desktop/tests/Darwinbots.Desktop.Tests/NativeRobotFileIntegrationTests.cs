using System.Reflection;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class NativeRobotFileIntegrationTests
{
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
