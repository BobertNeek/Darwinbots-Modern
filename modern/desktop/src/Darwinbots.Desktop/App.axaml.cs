using Avalonia;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;
using Darwinbots.Desktop.Views;

namespace Darwinbots.Desktop;

public sealed partial class App : Application
{
    public override void Initialize() => AvaloniaXamlLoader.Load(this);

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
}
