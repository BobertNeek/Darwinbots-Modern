using System.Reflection;
using Avalonia.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class NativeStartupRobotFileTests
{
    private const string RawDna = "' café € body\r\nstart\nstop\r\n";
    private static readonly byte[] RawDnaBytes =
    [
        0x27, 0x20, 0x63, 0x61, 0x66, 0xe9, 0x20, 0x80, 0x20, 0x62, 0x6f, 0x64, 0x79,
        0x0d, 0x0a, 0x73, 0x74, 0x61, 0x72, 0x74, 0x0a, 0x73, 0x74, 0x6f, 0x70, 0x0d, 0x0a,
    ];
    // Literal original 35-record order; it is independent of the codec encoder.
    private const string MRate = "17\r\n29\r\n111.5\r\n7\r\n0.5\r\n222.25\r\n2\r\n0.25\r\n"
        + "333.5\r\n3\r\n0.75\r\n444.25\r\n4\r\n0.5\r\n555.5\r\n5\r\n0.25\r\n"
        + "666.25\r\n6\r\n0.75\r\n777.5\r\n-3\r\n0.5\r\n888.25\r\n-4\r\n0.25\r\n"
        + "999.5\r\n9\r\n0.75\r\n1010.25\r\n10\r\n0.5\r\n1111.5\r\n11\r\n0.25\r\n";

    [Fact]
    public async Task StartupSidecarReachesItsNativeRobotAtTheExactCenterAndSurvivesExportAndSaveResume()
    {
        var directory = NewDirectory();
        try
        {
            var path = Path.Combine(directory, "Café.txt");
            await File.WriteAllBytesAsync(path, RawDnaBytes);
            var sidecarBytes = System.Text.Encoding.ASCII.GetBytes(MRate);
            await File.WriteAllBytesAsync(Path.ChangeExtension(path, ".mrate"), sidecarBytes);
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var baseline = BaselineSpecies();
                var setup = Setup(baseline);
                var factory = new CapturingNativeFactory();
                var storage = new AuditStorageService();
                var window = new MainWindow(["--bot", path], setup, factory, storage);
                try
                {
                    window.Show();
                    await WaitFor(() => Ready(window, 2));
                    var session = Session(window)!;
                    var snapshot = session.LatestSnapshot;
                    Assert.Equal(3, snapshot.Species.Count);
                    Assert.Equal(2, InsertionSpecies(window).Length);
                    var imported = Assert.Single(snapshot.Organisms.Where(robot =>
                        snapshot.Species[(int)robot.Species].Name == "Café"));
                    Assert.NotEqual(0u, imported.Species);
                    Assert.Equal(new[] { 1_200f, 900f }, imported.Position);
                    Assert.Equal(1_000, imported.Energy);
                    Assert.False(imported.Vegetable);
                    Assert.Equal(0xffd07a2du, snapshot.Species[(int)imported.Species].Color);
                    Assert.Equal(FileProfile(), imported.MutationProfile);
                    Assert.Equal(RawDna, await session.ExportSpeciesTemplateDnaAsync(imported.Species));
                    AssertBaseline(window, baseline, factory);
                    var registered = Assert.Single(InsertionSpecies(window).Where(species => species.Name == "Café"));
                    Assert.Equal(path, registered.DnaPath);
                    Assert.Equal(RawDna, registered.Dna);
                    Assert.Equal(FileProfile(), registered.MutationProfile);
                    Assert.Equal(new SpeciesPlacement(SpeciesPlacementMode.Fixed, 1_200f, 900f), registered.Placement);

                    var pair = await session.ExportRobotFileAsync(imported.Slot, imported.Generation);
                    Assert.Equal(FileProfile(), pair.MutationProfile);
                    Assert.Equal(await session.ExportDnaAsync(imported.Slot, imported.Generation), pair.Dna);
                    var viewModel = ViewModel(window);
                    viewModel.Update(session.LatestSnapshot);
                    viewModel.Select(new OrganismKey(imported.Slot, imported.Generation));
                    await (Task)typeof(MainWindow).GetMethod("SaveSelectedDnaCommandAsync", PrivateInstance)!
                        .Invoke(window, null)!;
                    Assert.Equal(pair.Dna, storage.SavedDna);
                    Assert.Equal(pair.MutationProfile, storage.SavedDnaMutationProfile);

                    var saved = await session.SaveAsync();
                    await session.LoadAsync(saved);
                    var resumed = Assert.Single(session.LatestSnapshot.Organisms.Where(robot =>
                        robot.Slot == imported.Slot && robot.Generation == imported.Generation));
                    Assert.Equal(new[] { 1_200f, 900f }, resumed.Position);
                    Assert.Equal(FileProfile(), resumed.MutationProfile);
                    Assert.Equal(RawDna, await session.ExportSpeciesTemplateDnaAsync(resumed.Species));
                    Assert.Equal(pair, await session.ExportRobotFileAsync(resumed.Slot, resumed.Generation));
                    AssertBaseline(window, baseline, factory);
                    Assert.Equal(0ul, session.LatestSnapshot.Tick);
                }
                finally { await CloseAsync(window); }
            }, CancellationToken.None);
            Assert.Equal(RawDnaBytes, await File.ReadAllBytesAsync(path));
            Assert.Equal(sidecarBytes, await File.ReadAllBytesAsync(Path.ChangeExtension(path, ".mrate")));
        }
        finally { Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task MalformedStartupSidecarFailsBeforeItsDnaImportAndPreservesTheSetupSpecies()
    {
        var directory = NewDirectory();
        try
        {
            var path = Path.Combine(directory, "Rejected.txt");
            await File.WriteAllBytesAsync(path, RawDnaBytes);
            var invalid = System.Text.Encoding.ASCII.GetBytes("17\r\n29\r\ninvalid\r\n");
            await File.WriteAllBytesAsync(Path.ChangeExtension(path, ".mrate"), invalid);
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var baseline = BaselineSpecies();
                var factory = new CapturingNativeFactory();
                var setup = Setup(baseline) with
                {
                    // The default startup policy ignores errors instead of showing the recovery panel.
                    GlobalSettings = new() { Main = new() { SafeMode = true } },
                };
                var window = new MainWindow(["--bot", path], setup, factory, new AuditStorageService());
                try
                {
                    window.Show();
                    await WaitFor(() => Session(window) is not null
                        && window.FindControl<Border>("RecoveryPanel")!.IsVisible);
                    var session = Session(window)!;
                    Assert.Single(session.LatestSnapshot.Organisms);
                    Assert.Equal(2, session.LatestSnapshot.Species.Count);
                    Assert.DoesNotContain(session.LatestSnapshot.Species, species => species.Name == "Rejected");
                    AssertBaseline(window, baseline, factory);
                    Assert.Equal(new[] { baseline }, InsertionSpecies(window));
                    Assert.Equal(0ul, session.LatestSnapshot.Tick);
                }
                finally { await CloseAsync(window); }
            }, CancellationToken.None);
            Assert.Equal(RawDnaBytes, await File.ReadAllBytesAsync(path));
            Assert.Equal(invalid, await File.ReadAllBytesAsync(Path.ChangeExtension(path, ".mrate")));
        }
        finally { Directory.Delete(directory, true); }
    }

    [Fact]
    public async Task StartupWithoutASidecarKeepsTheOriginalDefaultSpeciesAndInsertionRoute()
    {
        var directory = NewDirectory();
        try
        {
            var path = Path.Combine(directory, "Plain.txt");
            await File.WriteAllBytesAsync(path, RawDnaBytes);
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var baseline = BaselineSpecies();
                var factory = new CapturingNativeFactory();
                var window = new MainWindow(["--bot", path], Setup(baseline), factory, new AuditStorageService());
                try
                {
                    window.Show();
                    await WaitFor(() => Ready(window, 2));
                    var session = Session(window)!;
                    var imported = Assert.Single(session.LatestSnapshot.Organisms.Where(robot => robot.Species == 0));
                    Assert.Equal(new[] { 1_200f, 900f }, imported.Position);
                    Assert.Equal(factory.InitialMutationSettings!.Profile, imported.MutationProfile);
                    Assert.Equal(2, session.LatestSnapshot.Species.Count);
                    Assert.DoesNotContain(session.LatestSnapshot.Species, species => species.Name == "Plain");
                    Assert.Equal(new[] { baseline }, InsertionSpecies(window));
                    AssertBaseline(window, baseline, factory);
                    var save = await session.SaveAsync();
                    await session.LoadAsync(save);
                    var resumed = Assert.Single(session.LatestSnapshot.Organisms.Where(robot => robot.Species == 0));
                    Assert.Equal(imported.Slot, resumed.Slot);
                    Assert.Equal(imported.Generation, resumed.Generation);
                    Assert.Equal(new[] { 1_200f, 900f }, resumed.Position);
                    Assert.Equal(imported.MutationProfile, resumed.MutationProfile);
                    Assert.Equal(0ul, session.LatestSnapshot.Tick);
                }
                finally { await CloseAsync(window); }
            }, CancellationToken.None);
            Assert.False(File.Exists(Path.ChangeExtension(path, ".mrate")));
            Assert.Equal(RawDnaBytes, await File.ReadAllBytesAsync(path));
        }
        finally { Directory.Delete(directory, true); }
    }

    private static WorldSetupOptions Setup(SpeciesImport baseline) => new()
    {
        Backend = "Cpu", WorldWidth = 2_400, WorldHeight = 1_800,
        Species = [baseline],
        MutationSettings = new(false, 0.75f, [])
        {
            Profile = MutationProfileUpdate.Default with { PointValueChangePercent = 83, CopyErrorValueChangePercent = 97 },
        },
    };

    private static SpeciesImport BaselineSpecies() => new(
        "Setup baseline", "' authoritative setup source\r\nstart stop\r\n", false, 0xff223344, 1, 1_500, 0, false, 1f)
    {
        MutationProfile = MutationProfileUpdate.Default with
        {
            Point = new(64.5, 9, 1.5), PointValueChangePercent = 61, CopyErrorValueChangePercent = 73,
        },
        Placement = new(SpeciesPlacementMode.Fixed, 300f, 400f),
    };

    private static MutationProfileUpdate FileProfile() => MutationProfileUpdate.Default with
    {
        Point = new(111.5, 7, 0.5), MinorDeletion = new(222.25, 2, 0.25),
        Reversal = new(333.5, 3, 0.75), Insertion = new(444.25, 4, 0.5),
        Amplification = new(555.5, 5, 0.25), MajorDeletion = new(666.25, 6, 0.75),
        CopyError = new(777.5, -3, 0.5), Delta = new(888.25, -4, 0.25),
        Translocation = new(999.5, 9, 0.75), Point2 = new(1010.25, 10, 0.5),
        CopyError2 = new(1111.5, 11, 0.25), PointValueChangePercent = 17, CopyErrorValueChangePercent = 29,
    };

    private static void AssertBaseline(MainWindow window, SpeciesImport baseline, CapturingNativeFactory factory)
    {
        var snapshot = Session(window)!.LatestSnapshot;
        var robot = Assert.Single(snapshot.Organisms.Where(value => snapshot.Species[(int)value.Species].Name == baseline.Name));
        Assert.Equal(baseline.MutationProfile, robot.MutationProfile);
        Assert.Equal(new[] { 300f, 400f }, robot.Position);
        Assert.Equal(factory.InitialMutationSettings, snapshot.MutationSettings);
        Assert.False(snapshot.MutationSettings.Enabled);
        Assert.Equal(0.75f, snapshot.MutationSettings.GlobalMultiplier);
        var definition = Assert.Single(Field<WorldSetupOptions>(window, "_setup").Species);
        Assert.Same(baseline, definition);
        Assert.Equal(baseline.MutationProfile, definition.MutationProfile);
    }

    private static string NewDirectory()
    {
        var path = Path.Combine(Path.GetTempPath(), "db-native-startup-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(path);
        return path;
    }
    private static bool Ready(MainWindow window, int count) => Session(window)?.LatestSnapshot.Organisms.Count == count
        && (ViewModel(window).Status.StartsWith("IMPORTED 1 STARTUP BOT", StringComparison.Ordinal)
            || ViewModel(window).Status.StartsWith("DNA COMPATIBILITY", StringComparison.Ordinal));
    private static readonly BindingFlags PrivateInstance = BindingFlags.Instance | BindingFlags.NonPublic;
    private static SimulationSession? Session(MainWindow window) =>
        typeof(MainWindow).GetField("_session", PrivateInstance)!.GetValue(window) as SimulationSession;
    private static MainWindowViewModel ViewModel(MainWindow window) => Field<MainWindowViewModel>(window, "_viewModel");
    private static T Field<T>(MainWindow window, string name) =>
        (T)typeof(MainWindow).GetField(name, PrivateInstance)!.GetValue(window)!;
    private static SpeciesImport[] InsertionSpecies(MainWindow window) =>
        Field<IEnumerable<SpeciesImport>>(window, "_insertionSpecies").ToArray();
    private static async Task CloseAsync(MainWindow window)
    {
        var session = Session(window);
        typeof(MainWindow).GetField("_session", PrivateInstance)!.SetValue(window, null);
        foreach (var owned in window.OwnedWindows.ToArray()) owned.Close();
        if (window.IsVisible) window.Close();
        if (session is not null) await session.DisposeAsync();
    }
    private static async Task WaitFor(Func<bool> condition)
    {
        for (var attempt = 0; attempt < 200; attempt++)
        {
            if (condition()) return;
            await Task.Delay(25);
        }
        Assert.True(condition(), "Native startup robot-file route did not finish.");
    }
    private sealed class CapturingNativeFactory : IEngineClientFactory
    {
        public MutationSettingsSnapshot? InitialMutationSettings { get; private set; }
        public IEngineClient Create(WorldSetupOptions setup)
        {
            var engine = new NativeEngineClient(setup);
            InitialMutationSettings = engine.Snapshot().MutationSettings;
            return engine;
        }
    }
}
