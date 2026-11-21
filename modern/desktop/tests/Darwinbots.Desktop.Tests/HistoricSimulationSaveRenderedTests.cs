using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using Avalonia.Threading;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class HistoricSimulationSaveRenderedTests
{
    private const BindingFlags WindowFields = BindingFlags.Instance | BindingFlags.NonPublic;

    [Theory]
    [InlineData(LegacyCommandId.FileSaveSimulation)]
    [InlineData(LegacyCommandId.FileSaveWithoutMutations)]
    public async Task OriginalSaveCommandsSuspendTheOwnerDuringThePickerAndWrite(LegacyCommandId command)
    {
        var engine = new RecordingEngineClient();
        await using var session = new SimulationSession(engine);
        await session.StepAsync(12);
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var pickerRequested = new TaskCompletionSource();
            var selection = new TaskCompletionSource<IStorageFile?>();
            var writeRequested = new TaskCompletionSource();
            var writeStream = new TaskCompletionSource<Stream>();
            using var output = new MemoryStream();
            var window = OpenRunningFilePickerWindow(session, StorageProxy.Create<IStorageProvider>((method, _) =>
            {
                Assert.Equal(nameof(IStorageProvider.SaveFilePickerAsync), method.Name);
                pickerRequested.SetResult();
                return selection.Task;
            }));
            try
            {
                var gate = WindowField<SemaphoreSlim>(window, "_advanceGate");
                Task<LegacyCommandResult> saving;
                await gate.WaitAsync();
                try
                {
                    saving = WindowField<LegacyCommandRouter>(window, "_legacyCommands").ExecuteAsync(command);
                    Assert.False(pickerRequested.Task.IsCompleted);
                    Assert.False(WindowField<DispatcherTimer>(window, "_runTimer").IsEnabled);
                }
                finally { gate.Release(); }

                await pickerRequested.Task.WaitAsync(TimeSpan.FromSeconds(5));
                await AssertOwnerIsPausedAsync(window, session);
                selection.SetResult(StorageProxy.Create<IStorageFile>((method, _) =>
                {
                    if (method.Name == "get_Name") return "modal.db3s";
                    Assert.Equal(nameof(IStorageFile.OpenWriteAsync), method.Name);
                    writeRequested.SetResult();
                    return writeStream.Task;
                }));

                await writeRequested.Task.WaitAsync(TimeSpan.FromSeconds(5));
                await AssertOwnerIsPausedAsync(window, session);
                writeStream.SetResult(output);
                Assert.True((await saving.WaitAsync(TimeSpan.FromSeconds(5))).Succeeded);
                Assert.True(WindowField<DispatcherTimer>(window, "_runTimer").IsEnabled);
                if (command == LegacyCommandId.FileSaveWithoutMutations)
                {
                    Assert.Contains("SaveWithoutMutations", engine.Calls);
                    Assert.Equal(new byte[] { 9, 8, 7 }, output.ToArray());
                }
                else
                {
                    Assert.Equal(12ul, BitConverter.ToUInt64(output.ToArray()));
                }
                await AdvanceOwnerAsync(window);
                Assert.Equal(13ul, session.LatestSnapshot.Tick);
            }
            finally
            {
                selection.TrySetResult(null);
                writeStream.TrySetResult(output);
                CloseFilePickerWindow(window);
            }
        }, CancellationToken.None);
    }

    [Theory]
    [InlineData("cancel")]
    [InlineData("reject")]
    [InlineData("accept")]
    public async Task OriginalLoadResumesOnlyWhenThePreviousWorldIsKept(string outcome)
    {
        await using var session = new SimulationSession(new RecordingEngineClient());
        await session.StepAsync(12);
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var pickerRequested = new TaskCompletionSource();
            var selection = new TaskCompletionSource<IReadOnlyList<IStorageFile>>();
            var readRequested = new TaskCompletionSource();
            var readStream = new TaskCompletionSource<Stream>();
            var window = OpenRunningFilePickerWindow(session, StorageProxy.Create<IStorageProvider>((method, _) =>
            {
                Assert.Equal(nameof(IStorageProvider.OpenFilePickerAsync), method.Name);
                pickerRequested.SetResult();
                return selection.Task;
            }));
            try
            {
                var loading = WindowField<LegacyCommandRouter>(window, "_legacyCommands")
                    .ExecuteAsync(LegacyCommandId.FileLoadSimulation);
                await pickerRequested.Task.WaitAsync(TimeSpan.FromSeconds(5));
                await AssertOwnerIsPausedAsync(window, session);
                if (outcome == "cancel")
                {
                    selection.SetResult([]);
                }
                else
                {
                    selection.SetResult([StorageProxy.Create<IStorageFile>((method, _) =>
                    {
                        Assert.Equal(nameof(IStorageFile.OpenReadAsync), method.Name);
                        readRequested.SetResult();
                        return readStream.Task;
                    })]);
                    await readRequested.Task.WaitAsync(TimeSpan.FromSeconds(5));
                    await AssertOwnerIsPausedAsync(window, session);
                    readStream.SetResult(new MemoryStream(outcome == "accept" ? BitConverter.GetBytes(90ul) : [1]));
                }

                var result = await loading.WaitAsync(TimeSpan.FromSeconds(5));
                Assert.Equal(outcome == "accept", result.Succeeded);
                Assert.Equal(outcome == "cancel", result.Canceled);
                if (outcome == "reject") Assert.NotNull(result.Error);
                Assert.Equal(outcome != "accept", WindowField<DispatcherTimer>(window, "_runTimer").IsEnabled);
                Assert.Equal(outcome == "accept" ? 90ul : 12ul, session.LatestSnapshot.Tick);
            }
            finally
            {
                selection.TrySetResult([]);
                readStream.TrySetCanceled();
                CloseFilePickerWindow(window);
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task F3DefaultsToOriginalSimulationAndSavesBothFormatsThroughThePicker()
    {
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu", PopulationCapacity = 64, VegetablePopulationCap = 0,
            GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { NewSeed = false } },
        };
        await using var session = new SimulationSession(new NativeEngineClient(setup));
        await session.ImportSpeciesAsync(new SpeciesImport("Save Test", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 0));

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var storage = new AvaloniaDesktopStorageService(Path.Combine(Path.GetTempPath(), $"darwinbots-save-test-{Guid.NewGuid():N}"));
            var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, storage);
            DesktopTestApplication.UseSession(window, session);

            var pickerRequested = new TaskCompletionSource<FilePickerSaveOptions>();
            var selection = new TaskCompletionSource<IStorageFile?>();
            var provider = StorageProxy.Create<IStorageProvider>((method, arguments) =>
            {
                Assert.Equal(nameof(IStorageProvider.SaveFilePickerAsync), method.Name);
                pickerRequested.SetResult(Assert.IsType<FilePickerSaveOptions>(arguments![0]));
                return selection.Task;
            });
            typeof(TopLevel).GetField("_storageProvider", BindingFlags.Instance | BindingFlags.NonPublic)!
                .SetValue(window, provider);
            window.Show();
            try
            {
                var model = Assert.IsType<MainWindowViewModel>(window.DataContext);
                var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!
                    .FindCommandItem(LegacyCommandId.FileSaveSimulation)!;
                var before = session.LatestSnapshot;

                window.KeyPress(Key.F3, RawInputModifiers.None, PhysicalKey.F3, null);
                window.KeyRelease(Key.F3, RawInputModifiers.None, PhysicalKey.F3, null);
                var options = await pickerRequested.Task.WaitAsync(TimeSpan.FromSeconds(5));
                var types = Assert.IsAssignableFrom<IReadOnlyList<FilePickerFileType>>(options.FileTypeChoices);
                Assert.Equal("simulation.sim", options.SuggestedFileName);
                Assert.Equal("sim", options.DefaultExtension);
                Assert.Equal(["*.sim", "*.db3s"], types.SelectMany(type => type.Patterns!));
                Assert.Same(types[0], options.SuggestedFileType);
                selection.SetResult(null);
                await WaitForStatusAsync(model, "CANCELED");
                Assert.Same(before, session.LatestSnapshot);

                foreach (var (fileName, status) in new[]
                {
                    ("legacy.SIM", "DARWINBOTS 2 SIMULATION SAVED"),
                    ("modern.db3s", "SAVED"),
                })
                {
                    pickerRequested = new TaskCompletionSource<FilePickerSaveOptions>();
                    selection = new TaskCompletionSource<IStorageFile?>();
                    menu.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await pickerRequested.Task.WaitAsync(TimeSpan.FromSeconds(5));

                    // The accepted file must contain the state after the picker opens.
                    await session.StepAsync(1);
                    var toSave = session.LatestSnapshot;
                    using var fileContent = new MemoryStream();
                    selection.SetResult(StorageProxy.Create<IStorageFile>((method, _) => method.Name switch
                    {
                        "get_Name" => fileName,
                        nameof(IStorageFile.OpenWriteAsync) => Task.FromResult<Stream>(fileContent),
                        _ => throw new NotSupportedException(method.Name),
                    }));
                    await WaitForStatusAsync(model, status);
                    var bytes = fileContent.ToArray();
                    Assert.Equal(fileName.EndsWith(".db3s", StringComparison.Ordinal), bytes.AsSpan().StartsWith("DB3S"u8));

                    using var restored = new NativeEngineClient(setup);
                    restored.Load(bytes);
                    var loaded = restored.Snapshot();
                    Assert.Equal(toSave.NativeCycle, loaded.NativeCycle);
                    Assert.Equal(toSave.WorldSize, loaded.WorldSize);
                    Assert.Equal(toSave.Population, loaded.Population);
                    Assert.Equal(Assert.Single(toSave.Organisms).Energy, Assert.Single(loaded.Organisms).Energy);
                }
            }
            finally
            {
                typeof(MainWindow).GetField("_session", BindingFlags.Instance | BindingFlags.NonPublic)!
                    .SetValue(window, null);
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static async Task WaitForStatusAsync(MainWindowViewModel model, string expected)
    {
        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(5));
        while (model.Status != expected && !timeout.IsCancellationRequested)
            await Task.Delay(10);
        Assert.Equal(expected, model.Status);
    }

    private static MainWindow OpenRunningFilePickerWindow(SimulationSession session, IStorageProvider provider)
    {
        var profile = Path.Combine(Path.GetTempPath(), $"darwinbots-file-dialog-{Guid.NewGuid():N}");
        var window = new MainWindow([], new WorldSetupOptions(), new AuditEngineClientFactory(), new AvaloniaDesktopStorageService(profile));
        DesktopTestApplication.UseSession(window, session);
        typeof(TopLevel).GetField("_storageProvider", WindowFields)!.SetValue(window, provider);
        window.Show();
        var timer = WindowField<DispatcherTimer>(window, "_runTimer");
        timer.Interval = TimeSpan.FromHours(1);
        timer.Start();
        return window;
    }

    private static async Task AssertOwnerIsPausedAsync(MainWindow window, SimulationSession session)
    {
        Assert.False(WindowField<DispatcherTimer>(window, "_runTimer").IsEnabled);
        var tick = session.LatestSnapshot.Tick;
        await AdvanceOwnerAsync(window);
        Assert.Equal(tick, session.LatestSnapshot.Tick);
    }

    private static Task AdvanceOwnerAsync(MainWindow window) =>
        (Task)typeof(MainWindow).GetMethod("AdvanceSimulationAsync", WindowFields)!.Invoke(window, [1u])!;

    private static T WindowField<T>(MainWindow window, string field) =>
        (T)typeof(MainWindow).GetField(field, WindowFields)!.GetValue(window)!;

    private static void CloseFilePickerWindow(MainWindow window)
    {
        typeof(MainWindow).GetField("_session", WindowFields)!.SetValue(window, null);
        window.Close();
    }
}

public class StorageProxy : DispatchProxy
{
    private Func<MethodInfo, object?[]?, object?> _invoke = null!;

    public static T Create<T>(Func<MethodInfo, object?[]?, object?> invoke) where T : class
    {
        var proxy = DispatchProxy.Create<T, StorageProxy>();
        ((StorageProxy)(object)proxy)._invoke = invoke;
        return proxy;
    }

    protected override object? Invoke(MethodInfo? targetMethod, object?[]? args) => _invoke(targetMethod!, args);
}
