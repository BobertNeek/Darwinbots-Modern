using System.Reflection;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using Avalonia.VisualTree;
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
    private const string Dna = "' first line\r\n// second line\r\n'\r\n\r\n/ after the blank line\r\nstart\r\nstop\r\n";
    private const string LeadingComment = " first line\r\n/ second line\r\n\r\n";

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
                    picker.AssertOptions(expectedFolder, allowMultiple: false);
                    Assert.Equal(startingRows, window.Species.Take(startingRows.Length));

                    WorldSetupOptions? created = null;
                    window.WorldCreated += options => created = options;
                    Click(window, "CreateWorldButton");
                    var imported = Assert.Single(Assert.IsType<WorldSetupOptions>(created).Species, row => row.Name == "Added Species");
                    Assert.Equal(5, imported.InitialCount);
                    Assert.Equal(3_000, imported.InitialEnergy);
                    Assert.Equal(file, imported.DnaPath);
                    Assert.Equal(LeadingComment, imported.Comment);
                    Assert.Equal(Dna, imported.Dna);
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
            var existing = new SpeciesImport("Existing", "start stop", false, 0xff239ac0, 50, 1_000, 0, false, 0)
            {
                Comment = "Already registered",
            };
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
                    var added = await AddSpeciesAsync(editor, native ? 5 : 50, native ? 3_000 : 1_000,
                        native ? "Base.txt" : "Base");
                    Assert.Equal(native ? 0xff0000ffu : 0xffd07a2du, added.Color);
                    picker.AssertOptions(native ? robots.FullName : null, allowMultiple: !native);
                    editor.UpdateLayout();
                    var details = Assert.Single(editor.GetVisualDescendants().OfType<Expander>(), control => control.DataContext == added);
                    details.IsExpanded = true;
                    editor.UpdateLayout();
                    var comment = Assert.Single(DetailsFor(editor, added).GetVisualDescendants().OfType<TextBox>(),
                        control => control.Watermark == "Species comment");
                    Assert.Equal(native ? LeadingComment : string.Empty, comment.Text);
                    Assert.Equal(Dna, added.Dna);
                    var prior = Assert.Single(editor.Species, row => row.Name == existing.Name);
                    Assert.Equal(existing.InitialCount, prior.InitialCount);
                    Assert.Equal(existing.InitialEnergy, prior.InitialEnergy);
                    Assert.Equal(existing.Comment, prior.Comment);

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

    [Theory]
    [InlineData(1440, 1000, false)]
    [InlineData(1080, 820, true)]
    public async Task ExpandedSpeciesDetailsStayInTheVisibleRosterWidth(int width, int height, bool keyboard)
    {
        var temporary = Directory.CreateTempSubdirectory("darwinbots-setup-species-layout-");
        try
        {
            var file = Path.Combine(temporary.FullName, "Added Species.txt");
            await File.WriteAllTextAsync(file, Dna);
            var storage = new AvaloniaDesktopStorageService(Path.Combine(temporary.FullName, "profile"));
            await storage.MarkFirstRunCompleteAsync();

            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var window = new SetupWindow(storage) { WindowState = WindowState.Normal, Width = width, Height = height };
                try
                {
                    _ = new DnaPicker(window, file);
                    window.Show();
                    var added = await AddSpeciesAsync(window, 5, 3_000);
                    window.UpdateLayout();
                    var scroller = window.FindControl<ScrollViewer>("SpeciesRosterScroll")!;
                    var expander = Assert.Single(window.GetVisualDescendants().OfType<Expander>(), row => row.DataContext == added);
                    var header = Assert.Single(expander.GetVisualDescendants().OfType<ToggleButton>());
                    var details = DetailsFor(window, added);
                    Assert.False(details.IsVisible);
                    var compactHeight = expander.Bounds.Height;

                    header.BringIntoView();
                    window.UpdateLayout();
                    Assert.True(scroller.Offset.X > 0);
                    if (keyboard)
                    {
                        Assert.True(header.Focus());
                        window.KeyPress(Key.Space, RawInputModifiers.None, PhysicalKey.Space, null);
                        window.KeyRelease(Key.Space, RawInputModifiers.None, PhysicalKey.Space, null);
                    }
                    else
                    {
                        var center = header.TranslatePoint(new Point(header.Bounds.Width / 2, header.Bounds.Height / 2), window)!.Value;
                        window.MouseDown(center, MouseButton.Left, RawInputModifiers.None);
                        window.MouseUp(center, MouseButton.Left, RawInputModifiers.None);
                    }
                    await WaitForAsync(() => expander.IsExpanded && details.IsVisible, "Species EDIT did not expand its row.");
                    window.UpdateLayout();
                    Assert.InRange(expander.Bounds.Height, compactHeight - 1, compactHeight + 1);

                    var coordinate = details.GetVisualDescendants().OfType<NumericUpDown>().Last(control => control.Maximum == 1_000_000);
                    var skinPoints = details.GetVisualDescendants().OfType<NumericUpDown>()
                        .Where(control => control.Maximum is 0.82m or 1256m).ToArray();
                    Assert.Equal(8, skinPoints.Length);
                    Assert.All(skinPoints, control => Assert.True(control.Bounds.Width >= 100));
                    var restriction = Assert.Single(details.GetVisualDescendants().OfType<CheckBox>(),
                        control => Equals(control.Content, "KILL DISQUALIFIED"));
                    var comment = Assert.Single(details.GetVisualDescendants().OfType<TextBox>(), control => control.Watermark == "Species comment");
                    Assert.Equal(LeadingComment, comment.Text);
                    foreach (var control in new Control[] { coordinate, skinPoints[^1], restriction, comment })
                    {
                        control.BringIntoView();
                        window.UpdateLayout();
                        var relative = control.TranslatePoint(default, scroller)!.Value;
                        var onWindow = control.TranslatePoint(default, window)!.Value;
                        Assert.InRange(relative.X, 0, scroller.Viewport.Width);
                        Assert.InRange(relative.X + control.Bounds.Width, 0, scroller.Viewport.Width + 1);
                        Assert.InRange(onWindow.Y, 64, window.Bounds.Height - 34);
                        Assert.InRange(onWindow.Y + control.Bounds.Height, 64, window.Bounds.Height - 34);
                    }
                    comment.Text = "Updated species detail";
                    Assert.Equal(comment.Text, added.Comment);
                }
                finally { window.Close(); }
            }, CancellationToken.None);
        }
        finally { temporary.Delete(recursive: true); }
    }

    private static Border DetailsFor(SetupWindow window, SetupSpeciesRow row) =>
        Assert.Single(window.GetVisualDescendants().OfType<Border>(), detail => detail.Name == "SpeciesDetails" && detail.DataContext == row);

    private static GlobalSettings Settings(string mainDirectory) => new()
    {
        Main = new GlobalMainSettings { MainDirectory = mainDirectory },
    };

    private static void Click(Window window, string button) =>
        window.FindControl<Button>(button)!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

    private static async Task<SetupSpeciesRow> AddSpeciesAsync(SetupWindow window, int count, float energy)
    {
        Click(window, "AddDnaButton");
        await WaitForAsync(() => window.Species.Any(row => row.Name == "Added Species"),
            $"Add Species did not add the picked DNA. {window.FindControl<TextBlock>("SetupStatus")!.Text}");
        var row = Assert.Single(window.Species, row => row.Name == "Added Species");
        Assert.Equal(count, row.InitialCount);
        Assert.Equal(energy, row.InitialEnergy);
        return row;
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

        public void AssertOptions(string? expectedPath, bool allowMultiple)
        {
            Assert.Equal(expectedPath, _lookup, ignoreCase: true);
            if (_lookup is not null) Assert.True(Directory.Exists(_lookup));
            var options = Assert.IsType<FilePickerOpenOptions>(_open);
            Assert.Same(expectedPath is null ? null : _folder, options.SuggestedStartLocation);
            Assert.Equal(allowMultiple, options.AllowMultiple);
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
