using System.Reflection;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class DnaExportRenderedTests
{
    [Fact]
    public async Task MainWindowSelectedOrganismExportsExactDnaAndKeepsCanceledRouteInert()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var engine = new RecordingEngineClient();
            await using var simulation = new SimulationSession(engine);
            await simulation.StepAsync(0);
            var storage = new DnaExportStorage { AcceptDnaSave = true };
            var window = new MainWindow(
                [],
                new WorldSetupOptions(),
                new FixedEngineFactory(engine),
                storage);
            DnaEditorWindow? editor = null;
            try
            {
                DesktopTestApplication.UseSession(window, simulation);
                var viewModel = Field<MainWindowViewModel>(window, "_viewModel");
                window.Show();
                viewModel.Status = "READY";
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                using var frame = window.CaptureRenderedFrame();
                Assert.True(frame.PixelSize.Width >= 900, $"Rendered width was {frame.PixelSize.Width}.");
                Assert.True(window.FindControl<Button>("EditDnaButton")!.Bounds.Width > 0);

                viewModel.ClearSelection();
                Invoke(window, "EditDna_Click");
                Assert.Equal(0, storage.DnaSaveRequests);
                Assert.Empty(engine.DnaExports);

                viewModel.Update(RecordingEngineClient.CreateSnapshot());
                viewModel.Select(RecordingEngineClient.Organism.Slot);
                Assert.True(window.FindControl<Button>("EditDnaButton")!.IsEffectivelyEnabled);

                Invoke(window, "EditDna_Click");
                await WaitForAsync(() =>
                {
                    editor = window.OwnedWindows.OfType<DnaEditorWindow>().SingleOrDefault();
                    // Keep Opened and its awaited export inside this application scope.
                    return editor is not null && engine.DnaExports.Count == 1
                        && editor.FindControl<TextBox>("Editor")!.Text == "start\nstop";
                }, () => $"Owned windows: {string.Join(", ", window.OwnedWindows.Select(owned => owned.GetType().Name))}; "
                    + $"exports: {engine.DnaExports.Count}; editor visible: {editor?.IsVisible}; "
                    + $"text: {editor?.FindControl<TextBox>("Editor")?.Text ?? "<null>"}; "
                    + $"status: {editor?.FindControl<TextBlock>("Status")?.Text ?? "<null>"}.");

                var loadedEditor = Assert.IsType<DnaEditorWindow>(editor);
                Assert.Equal("start\nstop", loadedEditor.FindControl<TextBox>("Editor")!.Text);
                Invoke(loadedEditor, "Save_Click");
                await WaitForAsync(() => storage.DnaSaveRequests == 1
                    && storage.SuccessfulDnaSaves == 1
                    && loadedEditor.FindControl<TextBlock>("Status")!.Text == "BOT EXPORTED",
                    () => $"Save requests: {storage.DnaSaveRequests}; successful writes: {storage.SuccessfulDnaSaves}; "
                        + $"editor status: {loadedEditor.FindControl<TextBlock>("Status")!.Text}.");

                Assert.Equal((typeof(DnaEditorWindow), "bot-3-2.txt", "start\nstop"),
                    (storage.LastOwnerType, storage.LastSuggestedFileName, storage.LastDna));
                Assert.Equal(1, storage.SuccessfulDnaSaves);
                Assert.Equal("BOT EXPORTED", loadedEditor.FindControl<TextBlock>("Status")!.Text);
                Assert.Equal([(3u, 2u)], engine.DnaExports.ToArray());

                storage.AcceptDnaSave = false;
                Invoke(loadedEditor, "Save_Click");
                await WaitForAsync(() => storage.DnaSaveRequests == 2,
                    () => $"Cancelled save requests: {storage.DnaSaveRequests}; successful writes: {storage.SuccessfulDnaSaves}; "
                        + $"editor status: {loadedEditor.FindControl<TextBlock>("Status")!.Text}.");
                Assert.Equal(1, storage.SuccessfulDnaSaves);
                Assert.Equal("BOT EXPORTED", loadedEditor.FindControl<TextBlock>("Status")!.Text);
            }
            finally
            {
                foreach (var owned in window.OwnedWindows.ToArray())
                    if (owned.IsVisible) owned.Close();
                SetField(window, "_session", null);
                if (window.IsVisible) window.Close();
            }
        }, CancellationToken.None);
    }

    private static async Task WaitForAsync(Func<bool> predicate, Func<string> observedState)
    {
        for (var attempt = 0; attempt < 100; attempt++)
        {
            if (predicate()) return;
            await Task.Delay(10);
        }

        Assert.True(predicate(), $"The DNA export route did not reach the expected state. {observedState()}");
    }

    private static void Invoke(object target, string methodName) =>
        target.GetType().GetMethod(methodName, BindingFlags.Instance | BindingFlags.NonPublic)!
            .Invoke(target, [null, new RoutedEventArgs()]);

    private static T Field<T>(object target, string name) =>
        (T)target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(target)!;

    private static void SetField(object target, string name, object? value) =>
        target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)!.SetValue(target, value);

    private sealed class FixedEngineFactory(IEngineClient engine) : IEngineClientFactory
    {
        public IEngineClient Create(WorldSetupOptions setup) => engine;
    }

    private sealed class DnaExportStorage : IDesktopStorageService
    {
        public bool AcceptDnaSave { get; set; }
        public int DnaSaveRequests { get; private set; }
        public int SuccessfulDnaSaves { get; private set; }
        public Type? LastOwnerType { get; private set; }
        public string? LastSuggestedFileName { get; private set; }
        public string? LastDna { get; private set; }

        public Task<IReadOnlyList<DesktopTextFile>> OpenDnaFilesAsync(TopLevel owner) =>
            Task.FromResult<IReadOnlyList<DesktopTextFile>>([]);

        public Task<DesktopSettings?> OpenSettingsAsync(TopLevel owner) =>
            Task.FromResult<DesktopSettings?>(null);

        public Task<string?> PickSimulationPathAsync(TopLevel owner) =>
            Task.FromResult<string?>(null);

        public Task<byte[]?> OpenSimulationAsync(TopLevel owner) =>
            Task.FromResult<byte[]?>(null);

        public Task<bool> SaveSettingsAsync(TopLevel owner, DesktopSettings settings) =>
            Task.FromResult(false);

        public Task<bool> SaveSimulationAsync(TopLevel owner, ReadOnlyMemory<byte> save) =>
            Task.FromResult(false);

        public Task<bool> SaveDnaAsync(TopLevel owner, string suggestedFileName, string dna)
        {
            DnaSaveRequests++;
            LastOwnerType = owner.GetType();
            LastSuggestedFileName = suggestedFileName;
            LastDna = dna;
            if (AcceptDnaSave) SuccessfulDnaSaves++;
            return Task.FromResult(AcceptDnaSave);
        }

        public Task<bool> SaveOrganismAsync(
            TopLevel owner,
            string suggestedFileName,
            Func<Stream, Task> writeAsync) => Task.FromResult(false);

        public string? FindLatestAutosave() => null;

        public Task SaveAutosaveAsync(ReadOnlyMemory<byte> save) => Task.CompletedTask;
    }
}
