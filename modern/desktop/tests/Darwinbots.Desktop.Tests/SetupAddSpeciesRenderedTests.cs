using System.Reflection;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class SetupAddSpeciesRenderedTests
{
    private const BindingFlags WindowFields = BindingFlags.Instance | BindingFlags.NonPublic;
    private const string Dna = "start\r\nstop\r\n";

    [Theory]
    [InlineData("robots")]
    [InlineData("Robots")]
    [InlineData(null)]
    public async Task SetupAddSpeciesUsesTheOriginalFolderCountAndEnergy(string? folderName)
    {
        var temporary = Directory.CreateTempSubdirectory("darwinbots-setup-add-species-");
        try
        {
            var main = temporary.CreateSubdirectory("Original Darwinbots");
            var expectedFolder = folderName is null ? null : main.CreateSubdirectory(folderName).FullName;
            var file = Path.Combine(temporary.FullName, "Added Species.txt");
            await File.WriteAllTextAsync(file, Dna);
            var storage = new AvaloniaDesktopStorageService(Path.Combine(temporary.FullName, "profile"));
            await storage.SaveGlobalSettingsAsync(Settings(main.FullName));

            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var window = new SetupWindow(storage);
                try
                {
                    var picker = new DnaPicker(window, file);
                    window.Show();
                    var startingRows = window.Species.ToArray();
                    Assert.NotEmpty(startingRows);

                    await AddSpeciesAsync(window, 5, 3_000);
                    picker.AssertStartedIn(expectedFolder);
                    Assert.Equal(startingRows, window.Species.Take(startingRows.Length));

                    WorldSetupOptions? created = null;
                    window.WorldCreated += options => created = options;
                    Click(window, "CreateWorldButton");
                    var imported = Assert.Single(Assert.IsType<WorldSetupOptions>(created).Species, row => row.Name == "Added Species");
                    Assert.Equal(5, imported.InitialCount);
                    Assert.Equal(3_000, imported.InitialEnergy);
                    Assert.Equal(file, imported.DnaPath);
                }
                finally { window.Close(); }
            }, CancellationToken.None);
        }
        finally { temporary.Delete(recursive: true); }
    }

    [Theory]
    [InlineData(true)]
    [InlineData(false)]
    public async Task EditSpeciesMenuUsesTheDefaultsForTheActiveWorld(bool native)
    {
        var temporary = Directory.CreateTempSubdirectory("darwinbots-live-add-species-");
        try
        {
            var main = temporary.CreateSubdirectory("Original Darwinbots");
            var robots = main.CreateSubdirectory("robots");
            var file = Path.Combine(temporary.FullName, "Added Species.txt");
            await File.WriteAllTextAsync(file, Dna);
            var storage = new AvaloniaDesktopStorageService(Path.Combine(temporary.FullName, "profile"));
            var existing = new SpeciesImport("Existing", "start stop", false, 0xff239ac0, 50, 1_000, 0, false, 0);
            var engine = new AuditEngineClient
            {
                NativeCycle = native ? 1 : null,
                ConfigurationForRead = new WorldConfiguration(EnvironmentUpdate.Default, 1, 64, 0, [existing]),
            };
            await using var session = new SimulationSession(engine);

            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var world = new MainWindow([], new WorldSetupOptions { GlobalSettings = Settings(main.FullName) },
                    new AuditEngineClientFactory(), storage);
                DesktopTestApplication.UseSession(world, session);
                try
                {
                    world.Show();
                    var command = world.FindControl<LegacyMenuBar>("HistoricMenuBar")!
                        .FindCommandItem(LegacyCommandId.EditSpecies)!;
                    Assert.True(command.IsEnabled);
                    command.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));

                    await WaitForAsync(() => world.OwnedWindows.OfType<SetupWindow>().Any(), "Edit Species did not open.");
                    var editor = Assert.Single(world.OwnedWindows.OfType<SetupWindow>());
                    var picker = new DnaPicker(editor, file);
                    await AddSpeciesAsync(editor, native ? 5 : 50, native ? 3_000 : 1_000);
                    picker.AssertStartedIn(native ? robots.FullName : null);
                    var prior = Assert.Single(editor.Species, row => row.Name == existing.Name);
                    Assert.Equal(existing.InitialCount, prior.InitialCount);
                    Assert.Equal(existing.InitialEnergy, prior.InitialEnergy);

                    Click(editor, "SpeciesCancelButton");
                    await WaitForAsync(() => !editor.IsVisible, "Edit Species did not close after Cancel.");
                    Assert.Same(existing, Assert.Single((await session.ReadConfigurationAsync())!.Species));
                }
                finally
                {
                    foreach (var owned in world.OwnedWindows.ToArray()) owned.Close();
                    typeof(MainWindow).GetField("_session", WindowFields)!.SetValue(world, null);
                    world.Close();
                }
            }, CancellationToken.None);
        }
        finally { temporary.Delete(recursive: true); }
    }

    private static GlobalSettings Settings(string mainDirectory) => new()
    {
        Main = new GlobalMainSettings { MainDirectory = mainDirectory },
    };

    private static void Click(Window window, string button) =>
        window.FindControl<Button>(button)!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

    private static async Task AddSpeciesAsync(SetupWindow window, int count, float energy)
    {
        Click(window, "AddDnaButton");
        await WaitForAsync(() => window.Species.Any(row => row.Name == "Added Species"),
            $"Add Species did not add the picked DNA. {window.FindControl<TextBlock>("SetupStatus")!.Text}");
        var row = Assert.Single(window.Species, row => row.Name == "Added Species");
        Assert.Equal(count, row.InitialCount);
        Assert.Equal(energy, row.InitialEnergy);
    }

    private static async Task WaitForAsync(Func<bool> condition, string message)
    {
        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(5));
        while (!condition() && !timeout.IsCancellationRequested)
            await Task.Delay(10);
        Assert.True(condition(), message);
    }

    private sealed class DnaPicker
    {
        private readonly IStorageFolder _folder;
        private string? _lookup;
        private FilePickerOpenOptions? _open;

        public DnaPicker(Window window, string path)
        {
            _folder = PickerProxy.CreateFor<IStorageFolder>((method, _) =>
                method.Name == nameof(IDisposable.Dispose) ? null : throw new NotSupportedException(method.Name));
            var file = PickerProxy.CreateFor<IStorageFile>((method, _) => method.Name switch
            {
                "get_Name" => Path.GetFileName(path),
                "get_Path" => new Uri(path),
                nameof(IStorageFile.OpenReadAsync) => Task.FromResult<Stream>(File.OpenRead(path)),
                _ => throw new NotSupportedException(method.Name),
            });
            var provider = PickerProxy.CreateFor<IStorageProvider>((method, args) => method.Name switch
            {
                nameof(IStorageProvider.TryGetFolderFromPathAsync) => FindFolder((Uri)args[0]!),
                nameof(IStorageProvider.OpenFilePickerAsync) => ChooseFile((FilePickerOpenOptions)args[0]!),
                _ => throw new NotSupportedException(method.Name),
            });
            typeof(TopLevel).GetField("_storageProvider", WindowFields)!.SetValue(window, provider);

            Task<IStorageFolder?> FindFolder(Uri uri)
            {
                _lookup = uri.LocalPath;
                return Task.FromResult<IStorageFolder?>(_folder);
            }

            Task<IReadOnlyList<IStorageFile>> ChooseFile(FilePickerOpenOptions options)
            {
                _open = options;
                return Task.FromResult<IReadOnlyList<IStorageFile>>([file]);
            }
        }

        public void AssertStartedIn(string? expectedPath)
        {
            Assert.Equal(expectedPath, _lookup, ignoreCase: true);
            if (_lookup is not null) Assert.True(Directory.Exists(_lookup));
            var options = Assert.IsType<FilePickerOpenOptions>(_open);
            Assert.Same(expectedPath is null ? null : _folder, options.SuggestedStartLocation);
        }
    }

    public class PickerProxy : DispatchProxy
    {
        private Func<MethodInfo, object?[], object?>? _call;

        internal static T CreateFor<T>(Func<MethodInfo, object?[], object?> call) where T : class
        {
            var proxy = Create<T, PickerProxy>();
            ((PickerProxy)(object)proxy)._call = call;
            return proxy;
        }

        protected override object? Invoke(MethodInfo? method, object?[]? args) => _call!(method!, args ?? []);
    }
}
