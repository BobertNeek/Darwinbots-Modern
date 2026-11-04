using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class World09CommandsRenderedTests
{
    [Fact]
    public async Task World09CommandsUseSelectedPlayerAndStayInertWithoutSelection()
    {
        var engine = new RecordingEngineClient(RecordingEngineClient.CreateSnapshot() with { WorldSize = [640f, 480f] });
        var factory = new World09EngineFactory(engine);

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], new WorldSetupOptions(), factory, new AuditStorageService())
            {
                Width = 1280,
                Height = 900,
            };
            try
            {
                window.Show();
                await WaitFor(() => Session(window) is not null && ViewModel(window).Population == 1);

                var move = window.FindControl<Button>("MoveButton")!;
                var clone = window.FindControl<Button>("CloneButton")!;
                var reproduce = window.FindControl<Button>("ReproduceButton")!;
                var kill = window.FindControl<Button>("KillButton")!;
                ViewModel(window).ClearSelection();

                foreach (var button in new[] { move, clone, reproduce, kill })
                    button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.Empty(engine.Calls);
                Assert.Equal("NONE", ViewModel(window).SelectedId);

                var key = new OrganismKey(3, 2);
                ViewModel(window).Select(key);
                window.FindControl<WorldViewport>("Viewport")!.SelectKey(key);

                move.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                RaiseWorldClick(window.FindControl<WorldViewport>("Viewport")!, [160f, 260f]);
                await WaitFor(() => engine.Calls.Contains("Move:3:2:160:260"));

                ViewModel(window).Select(key);
                clone.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => engine.Calls.Contains("Clone:3:2"));

                ViewModel(window).Select(key);
                kill.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => engine.Calls.Contains("Remove:3:2"));

                ViewModel(window).Select(key);
                reproduce.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(() => engine.Calls.Contains("Reproduce:3:2:none"));

                Assert.Equal(
                    ["Move:3:2:160:260", "Clone:3:2", "Remove:3:2", "Reproduce:3:2:none"],
                    engine.Calls.Where(call => call.StartsWith("Move:", StringComparison.Ordinal)
                        || call.StartsWith("Clone:", StringComparison.Ordinal)
                        || call.StartsWith("Remove:", StringComparison.Ordinal)
                        || call.StartsWith("Reproduce:", StringComparison.Ordinal)).ToArray());

                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var frame = window.CaptureRenderedFrame();
                Assert.True(frame.PixelSize.Width >= 1200 && frame.PixelSize.Height >= 800);
                Assert.All(new[] { move, clone, reproduce, kill }, button => Assert.True(button.IsVisible));
            }
            finally
            {
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static SimulationSession? Session(MainWindow window) =>
        typeof(MainWindow).GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)?.GetValue(window) as SimulationSession;

    private static MainWindowViewModel ViewModel(MainWindow window) =>
        (MainWindowViewModel)typeof(MainWindow).GetField("_viewModel", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(window)!;

    private static void RaiseWorldClick(WorldViewport viewport, float[] position) =>
        ((Action<float[]>)typeof(WorldViewport).GetField("WorldClicked", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(viewport)!).Invoke(position);

    private static async Task WaitFor(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(5);
        while (!condition() && DateTime.UtcNow < deadline) await Task.Delay(10);
        Assert.True(condition());
    }
}

internal sealed class World09EngineFactory(RecordingEngineClient engine) : IEngineClientFactory
{
    public IEngineClient Create(WorldSetupOptions setup) => engine;
}
