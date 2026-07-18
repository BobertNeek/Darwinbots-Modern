using Avalonia;
using Avalonia.Headless;

namespace Darwinbots.Desktop.Tests;

public static class DesktopTestApplication
{
    public static AppBuilder BuildAvaloniaApp() =>
        AppBuilder.Configure<App>()
            .UseSkia()
            .UseHeadless(new AvaloniaHeadlessPlatformOptions
            {
                UseHeadlessDrawing = false,
            });
}
