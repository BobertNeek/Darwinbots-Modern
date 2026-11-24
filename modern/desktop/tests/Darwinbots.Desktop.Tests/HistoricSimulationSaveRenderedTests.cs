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
    [InlineData(false)]
    [InlineData(true)]
    public async Task ModernSavePickersPauseAndDrainTheActiveStepBeforeSerializing(bool exportNative)
    {
        var timeout = TimeSpan.FromSeconds(8);
        var tickStarted = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var completeTick = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var engine = new AuditEngineClient
        {
            EncodeTickInSave = true,
            NativeCycle = 0,
            AdvanceNativeCycle = (count, cycle) =>
            {
                tickStarted.TrySetResult();
                completeTick.Task.WaitAsync(timeout).GetAwaiter().GetResult();
                return cycle + checked((int)count);
            },
        };
        await using var session = new SimulationSession(engine);
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var pickerRequested = new TaskCompletionSource<FilePickerSaveOptions>();
            var selection = new TaskCompletionSource<IStorageFile?>();
            using var output = new MemoryStream();
            var window = OpenRunningFilePickerWindow(session, StorageProxy.Create<IStorageProvider>((method, args) =>
            {
                Assert.Equal(nameof(IStorageProvider.SaveFilePickerAsync), method.Name);
                pickerRequested.SetResult((FilePickerSaveOptions)args![0]!);
                return selection.Task;
            }));
            try
            {
                var timer = WindowField<DispatcherTimer>(window, "_runTimer");
                engine.Calls.Clear();
                window.FindControl<Button>("StepButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await tickStarted.Task.WaitAsync(timeout);

                if (exportNative)
                    window.FindControl<LegacyMenuBar>("HistoricMenuBar")!
                        .FindCommandItem(LegacyCommandId.FileSaveNativeSimulation)!
                        .RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                else
                    window.FindControl<Button>("SaveButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

                Assert.False(pickerRequested.Task.IsCompleted);
                Assert.False(timer.IsEnabled);
                completeTick.SetResult();
                var options = await pickerRequested.Task.WaitAsync(timeout);
                var extension = exportNative ? ".sim" : ".db3s";
                Assert.Equal("simulation" + extension, options.SuggestedFileName);
                Assert.Equal(["*" + extension], Assert.Single(options.FileTypeChoices!).Patterns);
                var saveCall = exportNative ? "SaveLegacy" : "Save";
                Assert.Equal(["Tick:1", saveCall],
                    engine.Calls.Where(call => call is "Tick:1" or "Save" or "SaveLegacy"));
                await AssertOwnerIsPausedAsync(window, session);

                selection.SetResult(StorageProxy.Create<IStorageFile>((method, _) =>
                {
                    Assert.Equal(nameof(IStorageFile.OpenWriteAsync), method.Name);
                    return Task.FromResult<Stream>(output);
                }));
                await WaitForStatusAsync(Assert.IsType<MainWindowViewModel>(window.DataContext),
                    exportNative ? "DARWINBOTS 2 SIMULATION EXPORTED" : "SAVED");
                Assert.Equal(1ul, BitConverter.ToUInt64(output.ToArray()));
                Assert.Equal(1ul, session.LatestSnapshot.Tick);
                Assert.True(timer.IsEnabled);
            }
            finally
            {
                completeTick.TrySetResult();
                selection.TrySetResult(null);
                CloseFilePickerWindow(window);
            }
        }, CancellationToken.None);
    }

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
    [InlineData("cancel", true, 1_234)]
    [InlineData("reject", true, 1_234)]
    [InlineData("accept", null, 1_234)]
    [InlineData("accept", false, 6_341_122)]
    [InlineData("accept", false, 0)]
    [InlineData("modern", true, 1_234)]
    public async Task OriginalLoadCapturesTheSeedBeforeChoosingAFileAndResumesOnlyWhenThePreviousWorldIsKept(
        string outcome, bool? generateSeed, int seed)
    {
        var engine = new RecordingEngineClient { NativeLoadSeed = seed };
        await using var session = new SimulationSession(engine);
        await session.StepAsync(12);
        var accepted = outcome is "accept" or "modern";
        var payload = outcome switch
        {
            "accept" => BitConverter.GetBytes(90ul),
            "modern" => "DB3S\0\0\0\0"u8.ToArray(),
            _ => new byte[] { 1 },
        };
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var pickerRequested = new TaskCompletionSource();
            var selection = new TaskCompletionSource<IReadOnlyList<IStorageFile>>();
            var readRequested = new TaskCompletionSource();
            var readStream = new TaskCompletionSource<Stream>();
            var window = OpenRunningFilePickerWindow(session, StorageProxy.Create<IStorageProvider>((method, _) =>
            {
                Assert.Equal(nameof(IStorageProvider.OpenFilePickerAsync), method.Name);
                Assert.Contains($"CaptureNativeLoadSeed:{generateSeed != false}", engine.Calls);
                Assert.Equal(generateSeed != false, engine.Calls.Contains($"SetNativeSeedMetadata:{seed}"));
                pickerRequested.SetResult();
                return selection.Task;
            }), newSeedOnLoad: generateSeed);
            try
            {
                var loading = WindowField<LegacyCommandRouter>(window, "_legacyCommands")
                    .ExecuteAsync(LegacyCommandId.FileLoadSimulation);
                await pickerRequested.Task.WaitAsync(TimeSpan.FromSeconds(5));
                await AssertOwnerIsPausedAsync(window, session);
                engine.NativeLoadSeed = 99;
                if (outcome == "cancel")
                {
                    selection.SetResult([]);
                }
                else
                {
                    selection.SetResult([StorageProxy.Create<IStorageFile>((method, _) =>
                    {
                        if (method.Name == "get_Path") return new Uri("file:///mock-save.sim");
                        Assert.Equal(nameof(IStorageFile.OpenReadAsync), method.Name);
                        readRequested.SetResult();
                        return readStream.Task;
                    })]);
                    await readRequested.Task.WaitAsync(TimeSpan.FromSeconds(5));
                    await AssertOwnerIsPausedAsync(window, session);
                    readStream.SetResult(new MemoryStream(payload));
                }

                var result = await loading.WaitAsync(TimeSpan.FromSeconds(5));
                Assert.Equal(accepted, result.Succeeded);
                Assert.Equal(outcome == "cancel", result.Canceled);
                if (outcome == "reject") Assert.NotNull(result.Error);
                Assert.Equal(!accepted, WindowField<DispatcherTimer>(window, "_runTimer").IsEnabled);
                Assert.Equal(accepted ? BitConverter.ToUInt64(payload) : 12ul, session.LatestSnapshot.Tick);
                Assert.Single(engine.Calls, call => call.StartsWith("CaptureNativeLoadSeed:", StringComparison.Ordinal));
                var reseeds = engine.Calls.Where(call => call.StartsWith("LoadWithNativeSeed:", StringComparison.Ordinal));
                if (outcome == "accept" && seed != 0) Assert.Equal($"LoadWithNativeSeed:{seed}", Assert.Single(reseeds));
                else Assert.Empty(reseeds);
            }
            finally
            {
                selection.TrySetResult([]);
                readStream.TrySetCanceled();
                CloseFilePickerWindow(window);
            }
        }, CancellationToken.None);
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public async Task ToolbarNativeLoadUsesTheLastF2SeedEvenIfTheMenuLoadDidNotComplete(bool reject)
    {
        var engine = new RecordingEngineClient { NativeLoadSeed = 734 };
        await using var session = new SimulationSession(engine);
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            byte[]? selected = null;
            MainWindow? owner = null;
            var provider = StorageProxy.Create<IStorageProvider>((method, _) =>
            {
                Assert.Equal(nameof(IStorageProvider.OpenFilePickerAsync), method.Name);
                Assert.False(WindowField<DispatcherTimer>(owner!, "_runTimer").IsEnabled);
                var files = selected is { } data ? new[]
                {
                    StorageProxy.Create<IStorageFile>((itemMethod, _) => itemMethod.Name switch
                    {
                        nameof(IStorageFile.OpenReadAsync) => Task.FromResult<Stream>(new MemoryStream(data)),
                        "get_Path" => new Uri("file:///mock-save.sim"),
                        _ => throw new NotSupportedException(itemMethod.Name),
                    }),
                } : [];
                return Task.FromResult<IReadOnlyList<IStorageFile>>(files);
            });
            var window = owner = OpenRunningFilePickerWindow(session, provider);
            try
            {
                var commands = WindowField<LegacyCommandRouter>(window, "_legacyCommands");
                var timer = WindowField<DispatcherTimer>(window, "_runTimer");
                var model = Assert.IsType<MainWindowViewModel>(window.DataContext);
                await LoadFromToolbar(BitConverter.GetBytes(20ul));
                Assert.DoesNotContain("LoadWithNativeSeed:734", engine.Calls);

                timer.Start();
                selected = reject ? [1] : null;
                var first = await commands.ExecuteAsync(LegacyCommandId.FileLoadSimulation);
                Assert.False(first.Succeeded);
                Assert.Equal(!reject, first.Canceled);
                Assert.True(timer.IsEnabled);
                Assert.Contains("SetNativeSeedMetadata:734", engine.Calls);

                await LoadFromToolbar("DB3S\0\0\0\0"u8.ToArray());
                Assert.DoesNotContain("LoadWithNativeSeed:734", engine.Calls);
                await LoadFromToolbar(BitConverter.GetBytes(21ul));
                Assert.Single(engine.Calls, call => call == "LoadWithNativeSeed:734");
                Assert.Single(engine.Calls, call => call == "CaptureNativeLoadSeed:True");

                engine.NativeLoadSeed = 0;
                selected = null;
                Assert.True((await commands.ExecuteAsync(LegacyCommandId.FileLoadSimulation)).Canceled);
                Assert.Contains("SetNativeSeedMetadata:0", engine.Calls);
                await LoadFromToolbar(BitConverter.GetBytes(22ul));
                Assert.Single(engine.Calls, call => call.StartsWith("LoadWithNativeSeed:", StringComparison.Ordinal));

                async Task LoadFromToolbar(byte[] data)
                {
                    timer.Start();
                    selected = data;
                    model.Status = "WAITING";
                    window.FindControl<Button>("LoadButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    await WaitForStatusAsync(model, "LOADED");
                    Assert.Equal(BitConverter.ToUInt64(data), session.LatestSnapshot.Tick);
                    Assert.False(timer.IsEnabled);
                }
            }
            finally { CloseFilePickerWindow(window); }
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

    [Theory]
    [InlineData("saves")]
    [InlineData("Saves")]
    [InlineData(null)]
    public async Task OriginalFileCommandsStartInTheActiveMainDirectory(string? folderName)
    {
        var directory = Directory.CreateTempSubdirectory("darwinbots-saves-picker-");
        try
        {
            var expectedPath = folderName is null ? null : directory.CreateSubdirectory(folderName).FullName;
            await using var session = new SimulationSession(new RecordingEngineClient());
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var lookups = new List<Uri>();
                var options = new List<PickerOptions>();
                var folder = StorageProxy.Create<IStorageFolder>((method, _) => method.Name == nameof(IDisposable.Dispose)
                    ? null
                    : throw new NotSupportedException(method.Name));
                var provider = StorageProxy.Create<IStorageProvider>((method, arguments) => method.Name switch
                {
                    nameof(IStorageProvider.TryGetFolderFromPathAsync) => FindFolder((Uri)arguments![0]!),
                    nameof(IStorageProvider.OpenFilePickerAsync) => CancelPicker((PickerOptions)arguments![0]!,
                        Task.FromResult<IReadOnlyList<IStorageFile>>([])),
                    nameof(IStorageProvider.SaveFilePickerAsync) => CancelPicker((PickerOptions)arguments![0]!,
                        Task.FromResult<IStorageFile?>(null)),
                    _ => throw new NotSupportedException(method.Name),
                });
                var window = OpenRunningFilePickerWindow(session, provider, directory.FullName);
                try
                {
                    var commands = WindowField<LegacyCommandRouter>(window, "_legacyCommands");
                    foreach (var command in new[] { LegacyCommandId.FileLoadSimulation,
                        LegacyCommandId.FileSaveSimulation, LegacyCommandId.FileSaveWithoutMutations })
                        Assert.True((await commands.ExecuteAsync(command)).Canceled);

                    Assert.Equal(3, options.Count);
                    Assert.All(options, picker => Assert.Same(expectedPath is null ? null : folder, picker.SuggestedStartLocation));
                    Assert.Equal(Enumerable.Repeat(expectedPath, expectedPath is null ? 0 : 3),
                        lookups.Select(uri => uri.LocalPath),
                        OperatingSystem.IsWindows() ? StringComparer.OrdinalIgnoreCase : StringComparer.Ordinal);
                }
                finally { CloseFilePickerWindow(window); }

                Task<IStorageFolder?> FindFolder(Uri uri)
                {
                    lookups.Add(uri);
                    return Task.FromResult<IStorageFolder?>(folder);
                }

                Task<T> CancelPicker<T>(PickerOptions picker, Task<T> canceled)
                {
                    options.Add(picker);
                    return canceled;
                }
            }, CancellationToken.None);
        }
        finally { directory.Delete(recursive: true); }
    }

    private static async Task WaitForStatusAsync(MainWindowViewModel model, string expected)
    {
        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(5));
        while (model.Status != expected && !timeout.IsCancellationRequested)
            await Task.Delay(10);
        Assert.Equal(expected, model.Status);
    }

    private static MainWindow OpenRunningFilePickerWindow(SimulationSession session, IStorageProvider provider,
        string? mainDirectory = null, bool? newSeedOnLoad = null)
    {
        var profile = Path.Combine(Path.GetTempPath(), $"darwinbots-file-dialog-{Guid.NewGuid():N}");
        var setup = new WorldSetupOptions
        {
            GlobalSettings = new GlobalSettings
            {
                Main = new GlobalMainSettings { MainDirectory = mainDirectory ?? string.Empty, NewSeedOnLoad = newSeedOnLoad },
            },
        };
        var window = new MainWindow([], setup, new AuditEngineClientFactory(), new AvaloniaDesktopStorageService(profile));
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
