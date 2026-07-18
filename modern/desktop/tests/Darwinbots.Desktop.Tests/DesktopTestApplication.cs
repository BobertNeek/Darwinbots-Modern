using Avalonia;
using Avalonia.Headless;

[assembly: AvaloniaTestApplication(typeof(Darwinbots.Desktop.Tests.DesktopTestApplication))]

namespace Darwinbots.Desktop.Tests;

public static class DesktopTestApplication
{
    public static HeadlessUnitTestSession Session =>
        HeadlessUnitTestSession.GetOrStartForAssembly(typeof(DesktopTestApplication).Assembly);

    public static AppBuilder BuildAvaloniaApp() =>
        AppBuilder.Configure<App>()
            .UseSkia()
            .UseHeadless(new AvaloniaHeadlessPlatformOptions
            {
                UseHeadlessDrawing = false,
            });
}
