using Avalonia;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;
using Avalonia.Media;
using Avalonia.Styling;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;

namespace Darwinbots.Desktop;

public sealed partial class App : Application
{
    public App() => DesktopUiPreferences.Changed += ApplyUiPreferences;

    public override void Initialize()
    {
        AvaloniaXamlLoader.Load(this);
        ApplyUiPreferences();
    }

    public override void OnFrameworkInitializationCompleted()
    {
        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
        {
            var setup = new SetupWindow();
            setup.WorldCreated += options =>
            {
                var simulation = new MainWindow(desktop.Args ?? [], options);
                desktop.MainWindow = simulation;
                simulation.Show();
                setup.Close();
            };
            desktop.MainWindow = setup;
        }
        base.OnFrameworkInitializationCompleted();
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
