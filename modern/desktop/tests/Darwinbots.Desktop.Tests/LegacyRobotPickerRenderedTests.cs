using System.Reflection;
using Avalonia.Controls;
using Avalonia.Platform.Storage;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class LegacyRobotPickerRenderedTests
{
    [Theory]
    [InlineData("robots")]
    [InlineData("Robots")]
    [InlineData(null)]
    public async Task OriginalRobotMenusUseTheConfiguredRobotFolder(string? folderName)
    {
        var root = Directory.CreateTempSubdirectory("darwinbots-robot-picker-");
        try
        {
            var expected = folderName is null ? null : root.CreateSubdirectory(folderName).FullName;
            var setup = new WorldSetupOptions
            {
                Backend = "Cpu", PopulationCapacity = 64, VegetablePopulationCap = 0,
                GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { MainDirectory = root.FullName, NewSeed = false } },
            };
            await using var session = new SimulationSession(new NativeEngineClient(setup));
            await session.ImportSpeciesAsync(new SpeciesImport("Picker Test", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 0));
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var lookups = new List<Uri>();
                var options = new List<PickerOptions>();
                var folder = PickerProxy.For<IStorageFolder>((method, _) => method.Name == nameof(IDisposable.Dispose)
                    ? null : throw new NotSupportedException(method.Name));
                var provider = PickerProxy.For<IStorageProvider>((method, arguments) => method.Name switch
                {
                    nameof(IStorageProvider.TryGetFolderFromPathAsync) => FindFolder((Uri)arguments[0]!),
                    nameof(IStorageProvider.OpenFilePickerAsync) => Cancel((PickerOptions)arguments[0]!, Task.FromResult<IReadOnlyList<IStorageFile>>([])),
                    nameof(IStorageProvider.SaveFilePickerAsync) => Cancel((PickerOptions)arguments[0]!, Task.FromResult<IStorageFile?>(null)),
                    _ => throw new NotSupportedException(method.Name),
                });
                var window = new MainWindow([], setup, NativeEngineClientFactory.Instance, AvaloniaDesktopStorageService.Instance);
                DesktopTestApplication.UseSession(window, session);
                const BindingFlags fields = BindingFlags.Instance | BindingFlags.NonPublic;
                typeof(TopLevel).GetField("_storageProvider", fields)!.SetValue(window, provider);
                window.Show();
                try
                {
                    var before = session.LatestSnapshot;
                    var robot = Assert.Single(before.Organisms);
                    Assert.IsType<MainWindowViewModel>(window.DataContext).Select(new OrganismKey(robot.Slot, robot.Generation));
                    var commands = (LegacyCommandRouter)typeof(MainWindow).GetField("_legacyCommands", fields)!.GetValue(window)!;
                    foreach (var command in new[] { LegacyCommandId.EditInsertOrganism, LegacyCommandId.RobotSaveEntireOrganism,
                        LegacyCommandId.RobotSaveDna })
                        Assert.True((await commands.ExecuteAsync(command)).Canceled);
                    Assert.Equal(["Import Darwinbots organism", "Save entire organism", "Export Darwinbots robot DNA"],
                        options.Select(picker => picker.Title));
                    Assert.All(options, picker => Assert.Same(expected is null ? null : folder, picker.SuggestedStartLocation));
                    Assert.Equal(Enumerable.Repeat(expected, expected is null ? 0 : 3), lookups.Select(uri => uri.LocalPath));
                    Assert.Equal(before.Tick, session.LatestSnapshot.Tick);
                }
                finally
                {
                    typeof(MainWindow).GetField("_session", fields)!.SetValue(window, null);
                    window.Close();
                }

                Task<IStorageFolder?> FindFolder(Uri uri)
                {
                    lookups.Add(uri);
                    return Task.FromResult<IStorageFolder?>(folder);
                }

                Task<T> Cancel<T>(PickerOptions picker, Task<T> canceled)
                {
                    options.Add(picker);
                    return canceled;
                }
            }, CancellationToken.None);
        }
        finally { root.Delete(recursive: true); }
    }

    public class PickerProxy : DispatchProxy
    {
        private Func<MethodInfo, object?[], object?>? _call;

        internal static T For<T>(Func<MethodInfo, object?[], object?> call) where T : class
        {
            var proxy = Create<T, PickerProxy>();
            ((PickerProxy)(object)proxy)._call = call;
            return proxy;
        }

        protected override object? Invoke(MethodInfo? method, object?[]? args) => _call!(method!, args ?? []);
    }
}
