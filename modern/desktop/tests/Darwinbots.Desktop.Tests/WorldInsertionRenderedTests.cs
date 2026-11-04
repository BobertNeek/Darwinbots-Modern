using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Media.Imaging;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class WorldInsertionRenderedTests
{
    [Fact]
    public async Task ActualViewportClickPlacesSelectedSpeciesAndCancelExitsInsertionMode()
    {
        var factory = new AuditEngineClientFactory();
        var setup = new WorldSetupOptions
        {
            Species =
            [
                new SpeciesImport(
                    "Selected Hunter",
                    "start\nstop",
                    false,
                    0xffd07a2d,
                    9,
                    2_500,
                    4,
                    true,
                    0.5f),
            ],
        };
        var storage = new AuditStorageService();
        MainWindow? window = null;

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            window = new MainWindow([], setup, factory, storage)
            {
                WindowState = WindowState.Normal,
                Width = 1_200,
                Height = 800,
            };
            window.Show();
            AvaloniaHeadlessPlatform.ForceRenderTimerTick();
            await WaitFor(() => factory.Clients.TryPeek(out var client)
                && client.Calls.Contains("ImportSpecies:Selected Hunter"));

            var engine = factory.Clients.Single();
            var selector = window.FindControl<ComboBox>("InsertionSpeciesSelector")!;
            Assert.Equal("Selected Hunter", ((SpeciesImport)selector.SelectedItem!).Name);

            var insertButton = window.FindControl<Button>("InsertSpeciesButton")!;
            Click(window, insertButton);
            Assert.Equal("CANCEL INSERTION", insertButton.Content);

            var viewport = window.FindControl<WorldViewport>("Viewport")!;
            var localPoint = new Point(viewport.Bounds.Width * 0.82, viewport.Bounds.Height * 0.78);
            var screenPoint = viewport.TranslatePoint(localPoint, window);
            Assert.True(screenPoint.HasValue);
            window.MouseDown(screenPoint.Value, MouseButton.Left, RawInputModifiers.None);
            window.MouseUp(screenPoint.Value, MouseButton.Left, RawInputModifiers.None);

            await WaitFor(
                () => engine.Calls.Any(call => call.StartsWith("InsertSpeciesAt:Selected Hunter:", StringComparison.Ordinal)),
                () => string.Join(", ", engine.Calls));
            Assert.Contains(
                engine.Calls,
                call => call.StartsWith("InsertSpeciesAt:Selected Hunter:", StringComparison.Ordinal));

            Click(window, insertButton);
            Assert.Equal("INSERT SPECIES", insertButton.Content);
            Capture(window, "world-insertion-exercised.png");
        }, CancellationToken.None);

        if (window is not null)
            await DesktopTestApplication.Session.Dispatch(() =>
            {
                window.Close();
                return Task.CompletedTask;
            }, CancellationToken.None);
    }

    private static void Click(Window window, Control control)
    {
        var point = control.TranslatePoint(
            new Point(Math.Max(1, control.Bounds.Width / 2), Math.Max(1, control.Bounds.Height / 2)),
            window);
        Assert.True(point.HasValue);
        window.MouseDown(point.Value, MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(point.Value, MouseButton.Left, RawInputModifiers.None);
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }

    private static void Capture(Window window, string evidenceName)
    {
        using var frame = window.CaptureRenderedFrame();
        Assert.NotNull(frame);
        var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR");
        if (string.IsNullOrWhiteSpace(directory))
            directory = Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
        Directory.CreateDirectory(directory);
        using var stream = File.Create(Path.Combine(directory, evidenceName));
        frame.Save(stream, new PngBitmapEncoderOptions());
    }

    private static async Task WaitFor(Func<bool> condition, Func<string>? details = null)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(5);
        while (!condition() && DateTime.UtcNow < deadline)
            await Task.Delay(10);
        Assert.True(condition(), $"Timed out waiting for insertion. Calls: {details?.Invoke()}");
    }
}
