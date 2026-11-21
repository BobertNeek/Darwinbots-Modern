using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;
using Avalonia.Media;
using Avalonia.Styling;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.Views;

namespace Darwinbots.Desktop;

public sealed partial class App : Application
{
    public App() => DesktopUiPreferences.Changed += ApplyUiPreferences;

    public override void Initialize()
    {
        AvaloniaXamlLoader.Load(this);
        Resources["ContentControlThemeFontFamily"] = DesktopFonts.Body;
        ApplyUiPreferences();
    }

    public override void OnFrameworkInitializationCompleted()
    {
        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
            desktop.MainWindow = CreateStartupWindow(desktop.Args ?? [], AvaloniaDesktopStorageService.Instance,
                window => desktop.MainWindow = window);
        base.OnFrameworkInitializationCompleted();
    }

    private static SafeModeBackupWindow? CreateSafeModeBackupWindow(string[]? arguments, IDesktopStorageService storage) =>
        arguments is [SafeModeBackupProcess.StartupSwitch]
            ? new SafeModeBackupWindow(storage) { WindowStartupLocation = WindowStartupLocation.CenterScreen }
            : null;

    private static Window CreateStartupWindow(string[] arguments, IDesktopStorageService storage, Action<Window> setMainWindow)
    {
        if (CreateSafeModeBackupWindow(arguments, storage) is { } viewer) return viewer;

        MainWindow OpenWorld(WorldSetupOptions options, bool? keepGraphsUpdatingOnClose = null, bool? newSeedOnLoad = null) =>
            new(arguments, options, NativeEngineClientFactory.Instance, storage, null, keepGraphsUpdatingOnClose, newSeedOnLoad);
        if (arguments.Length == 0 && storage.FindRestartContinuation() is { } continuation)
        {
            var globals = SetupWindow.ReadProfileSettings(storage, out _);
            var simulation = OpenWorld(new WorldSetupOptions
            {
                LoadSavePath = continuation,
                ResumeRestartMode = true,
                GlobalSettings = globals,
            });
            simulation.RestartContinuationFailed += message =>
            {
                var fallback = OpenSetup(message);
                setMainWindow(fallback);
                fallback.Show();
                simulation.Close();
            };
            return simulation;
        }

        var (requestedSave, error) = ReadStartupSaveArgument(arguments);
        if (requestedSave is not null)
        {
            var globals = SetupWindow.ReadProfileSettings(storage, out _);
            if (FindStartupSave(requestedSave, globals.Main.MainDirectory) is { } path)
                return OpenWorld(new WorldSetupOptions
                {
                    LoadSavePath = path, GlobalSettings = globals,
                    Vegetation = Db2VegetationOptions.Default with
                    {
                        StartChloroplasts = globals.Main.StartChloroplasts ?? Db2VegetationOptions.Default.StartChloroplasts,
                    },
                });
            error = $"Simulation file not found: {requestedSave}";
        }

        return OpenSetup(error);

        SetupWindow OpenSetup(string? startupError)
        {
            var setup = new SetupWindow(storage, allowLastExitResume: arguments.Length == 0,
                loadLastExitSettings: arguments.Length == 0, stageNativeAppearance: SetupWindow.CreateNativeAppearanceProvider());
            if (startupError is not null) setup.ShowStartupLoadError(startupError);
            setup.WorldCreated += options =>
            {
                var simulation = OpenWorld(options, setup.InitialKeepGraphsUpdatingOnClose, setup.InitialNewSeedOnLoad);
                setMainWindow(simulation);
                simulation.Show();
                setup.Close();
            };
            return setup;
        }
    }

    private static (string? RequestedPath, string? Error) ReadStartupSaveArgument(string[] arguments)
    {
        string? requested = null;
        for (var index = 0; index < arguments.Length; index++)
        {
            var argument = arguments[index];
            if (argument.Equals("--bot", StringComparison.OrdinalIgnoreCase) && index + 1 < arguments.Length
                && !string.IsNullOrWhiteSpace(arguments[index + 1]))
            {
                index++;
                continue;
            }

            var extension = Path.GetExtension(argument);
            if (argument.StartsWith('-') || (!extension.Equals(".sim", StringComparison.OrdinalIgnoreCase)
                && !extension.Equals(".db3s", StringComparison.OrdinalIgnoreCase)))
                continue;

            if (requested is not null) return (null, "Pass one simulation file to open.");
            requested = argument;
        }
        return (requested, null);
    }

    private static string? FindStartupSave(string requested, string legacyDirectory)
    {
        if (File.Exists(requested)) return Path.GetFullPath(requested);

        if (requested.IndexOfAny(['/', '\\', ':']) < 0 && !string.IsNullOrWhiteSpace(legacyDirectory))
        {
            var legacySave = Path.Combine(legacyDirectory, "Saves", requested);
            if (File.Exists(legacySave)) return Path.GetFullPath(legacySave);
        }
        return null;
    }

    private void ApplyUiPreferences()
    {
        RequestedThemeVariant = DesktopUiPreferences.Theme switch
        {
            DesktopUiTheme.Dark => ThemeVariant.Dark,
            DesktopUiTheme.Light => ThemeVariant.Light,
            _ => ThemeVariant.Default,
        };

        var dark = DesktopUiPreferences.Theme == DesktopUiTheme.Dark;
        SetBrush("WindowBrush", dark ? "#1B2020" : "#F3F0E8");
        SetBrush("HeaderBrush", dark ? "#252C2B" : "#FBF9F2");
        SetBrush("ToolbarBrush", dark ? "#303735" : "#E9E7DE");
        SetBrush("PanelBrush", dark ? "#252C2B" : "#FBF9F2");
        SetBrush("BorderBrush", dark ? "#56605B" : "#C8C5BB");
        SetBrush("TextBrush", dark ? "#F0F3EE" : "#202725");
        SetBrush("MutedBrush", dark ? "#B0B9B2" : "#69716D");
        SetBrush("FieldBrush", dark ? "#151A1A" : "#F9F8F2");
    }

    private void SetBrush(string key, string color) =>
        Resources[key] = new SolidColorBrush(Color.Parse(color));
}
