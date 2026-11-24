using System.Reflection;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class SetupSettingsPickerRenderedTests
{
    [Theory]
    [InlineData("settings", true)]
    [InlineData("Settings", true)]
    [InlineData("settings", false)]
    [InlineData(null, false)]
    public async Task SetupSettingsButtonsUseTheOriginalSettingsFolder(string? folderName, bool providerCanResolveFolder)
    {
        var temporary = Directory.CreateTempSubdirectory("darwinbots-settings-picker-");
        try
        {
            var main = temporary.CreateSubdirectory("Original Darwinbots");
            var expectedPath = folderName is null ? null : main.CreateSubdirectory(folderName).FullName;
            var storage = new AvaloniaDesktopStorageService(Path.Combine(temporary.FullName, "profile"));
            await storage.SaveGlobalSettingsAsync(new GlobalSettings
            {
                Main = new GlobalMainSettings { MainDirectory = main.FullName },
            });

            await DesktopTestApplication.Session.Dispatch(() =>
            {
                var folder = PickerProxy.CreateFor<IStorageFolder>((method, _) =>
                    method.Name == nameof(IDisposable.Dispose) ? null : throw new NotSupportedException(method.Name));
                var requestedFolders = new List<string>();
                var opens = new List<FilePickerOpenOptions>();
                var saves = new List<FilePickerSaveOptions>();
                var provider = PickerProxy.CreateFor<IStorageProvider>((method, args) => method.Name switch
                {
                    nameof(IStorageProvider.TryGetFolderFromPathAsync) => FindFolder((Uri)args[0]!),
                    nameof(IStorageProvider.OpenFilePickerAsync) => CancelOpen((FilePickerOpenOptions)args[0]!),
                    nameof(IStorageProvider.SaveFilePickerAsync) => CancelSave((FilePickerSaveOptions)args[0]!),
                    _ => throw new NotSupportedException(method.Name),
                });

                var window = new SetupWindow(storage);
                try
                {
                    typeof(TopLevel).GetField("_storageProvider", BindingFlags.Instance | BindingFlags.NonPublic)!
                        .SetValue(window, provider);
                    window.Show();
                    var status = window.FindControl<TextBlock>("SetupStatus")!;

                    window.FindControl<Button>("LoadSettingsButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    Assert.Equal("SETTINGS LOAD CANCELLED", status.Text);
                    window.FindControl<Button>("SaveSettingsButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    Assert.Equal("SETTINGS SAVE CANCELLED", status.Text);

                    IReadOnlyList<string> expectedLookups = expectedPath is null ? [] : [expectedPath, expectedPath];
                    Assert.Equal(expectedLookups, requestedFolders, StringComparer.OrdinalIgnoreCase);
                    Assert.All(requestedFolders, path => Assert.True(Directory.Exists(path)));
                    var expectedFolder = providerCanResolveFolder ? folder : null;
                    var open = Assert.Single(opens);
                    Assert.Same(expectedFolder, open.SuggestedStartLocation);
                    Assert.False(open.AllowMultiple);
                    Assert.Equal(["*.set", "*.json"], Assert.Single(open.FileTypeFilter!).Patterns);
                    var save = Assert.Single(saves);
                    Assert.Same(expectedFolder, save.SuggestedStartLocation);
                    Assert.Equal("set", save.DefaultExtension);
                    Assert.Equal(["*.set", "*.json"], Assert.Single(save.FileTypeChoices!).Patterns);
                }
                finally { window.Close(); }

                Task<IStorageFolder?> FindFolder(Uri uri)
                {
                    requestedFolders.Add(uri.LocalPath);
                    return Task.FromResult(providerCanResolveFolder ? folder : null);
                }

                Task<IReadOnlyList<IStorageFile>> CancelOpen(FilePickerOpenOptions options)
                {
                    opens.Add(options);
                    return Task.FromResult<IReadOnlyList<IStorageFile>>([]);
                }

                Task<IStorageFile?> CancelSave(FilePickerSaveOptions options)
                {
                    saves.Add(options);
                    return Task.FromResult<IStorageFile?>(null);
                }
            }, CancellationToken.None);
        }
        finally { temporary.Delete(recursive: true); }
    }

    public class PickerProxy : DispatchProxy
    {
        private Func<MethodInfo, object?[], object?>? _call;

        internal static T CreateFor<T>(Func<MethodInfo, object?[], object?> call) where T : class
        {
            var proxy = Create<T, PickerProxy>();
            ((PickerProxy)(object)proxy)._call = call;
            return proxy;
        }

        protected override object? Invoke(MethodInfo? method, object?[]? args) => _call!(method!, args ?? []);
    }
}
