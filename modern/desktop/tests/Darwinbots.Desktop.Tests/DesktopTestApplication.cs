using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using System.Reflection;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.ViewModels;

[assembly: AvaloniaTestApplication(typeof(Darwinbots.Desktop.Tests.DesktopTestApplication))]

namespace Darwinbots.Desktop.Tests;

public static class DesktopTestApplication
{
    public static UiSession Session { get; } = new();

    internal static void UseSession(MainWindow window, SimulationSession session)
    {
        // Supplied sessions are owned by the fixture. Do not start a second
        // engine on Opened or race its initialization against the fixture.
        var flags = BindingFlags.Instance | BindingFlags.NonPublic;
        window.Opened -= typeof(MainWindow).GetMethod("Window_Opened", flags)!
            .CreateDelegate<EventHandler>(window);
        typeof(MainWindow).GetField("_session", flags)!.SetValue(window, session);
        typeof(MainWindow).GetField("_insertion", flags)!.SetValue(window, new WorldInsertionController(session));
        session.RefreshSnapshotAsync().GetAwaiter().GetResult();
        ((MainWindowViewModel)window.DataContext!).Update(session.LatestSnapshot);
        window.FindControl<WorldViewport>("Viewport")!.SetSnapshot(session.LatestSnapshot);
        typeof(MainWindow).GetMethod("RefreshLegacyMenuStates", flags)!.Invoke(window, null);
    }

    public sealed class UiSession
    {
        private static HeadlessUnitTestSession Inner =>
            HeadlessUnitTestSession.GetOrStartForAssembly(typeof(DesktopTestApplication).Assembly);

        public Task Dispatch(Action action, CancellationToken cancellationToken) => Inner.Dispatch(action, cancellationToken);
        public Task<T> Dispatch<T>(Func<T> action, CancellationToken cancellationToken) => Inner.Dispatch<T>(action, cancellationToken);
        public Task<T> Dispatch<T>(Func<Task<T>> action, CancellationToken cancellationToken) => Inner.Dispatch<T>(action, cancellationToken);

        // Avalonia 12.1 has no Func<Task> overload. Keep task-only callbacks and
        // their exceptions inside the application/dispatcher isolation scope.
        public Task Dispatch(Func<Task> action, CancellationToken cancellationToken) =>
            Inner.Dispatch<int>(async () => { await action(); return 0; }, cancellationToken);
    }

    public static AppBuilder BuildAvaloniaApp() =>
        AppBuilder.Configure<App>()
            .UseSkia()
            .UseHeadless(new AvaloniaHeadlessPlatformOptions
            {
                UseHeadlessDrawing = false,
            });
}
