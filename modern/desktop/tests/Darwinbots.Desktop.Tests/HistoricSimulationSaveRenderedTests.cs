using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
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
