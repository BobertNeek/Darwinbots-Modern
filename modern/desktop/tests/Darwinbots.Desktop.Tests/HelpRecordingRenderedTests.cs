using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class HelpRecordingRenderedTests
{
    [Fact]
    public async Task MainWindowMenuRoutesRenderRecordingHelpAboutAndSafeError()
    {
        var recoveryPath = Path.Combine(Path.GetTempPath(), $"db24832-help-recording-recovery-{Guid.NewGuid():N}.db3s");
        File.WriteAllBytes(recoveryPath, [1, 2, 3]);

        try
        {
            await DesktopTestApplication.Session.Dispatch(async () =>
            {
                var window = new MainWindow(
                    [],
                    new WorldSetupOptions
                    {
                        Backend = "Cpu",
                        Species = [new SpeciesImport("Help Recording Species", "start\nstop", false, 0xff239ac0, 1, 1_000, 0, false, 1)],
                    },
                    new AuditEngineClientFactory(),
                    new AuditStorageService { LatestAutosave = recoveryPath })
                {
                    WindowState = WindowState.Normal,
                    Width = 1_280,
                    Height = 820,
                };

                try
                {
                    window.Show();
                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                    await WaitFor(
                        () => viewModel.Status.Contains("WORLD READY", StringComparison.Ordinal),
                        "MainWindow did not start the local player route.");

                    var menu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                    var recordingMenu = menu.TopLevelItems.Single(item => Equals(item.Header, "Recording"));
                    var recordingCommands = new[]
                    {
                        LegacyCommandId.RecordingSafemodeBackup,
                        LegacyCommandId.RecordingRecoverLatestAutosave,
                        LegacyCommandId.RecordingSnapshotLiving,
                        LegacyCommandId.RecordingDeadEnable,
                        LegacyCommandId.RecordingDeadExcludeRepopulating,
                    };
                    Assert.All(recordingCommands, command => Assert.True(menu.FindCommandItem(command)?.IsVisible, $"{command} is not player-visible."));
                    Assert.Contains(
                        recordingMenu.Items.OfType<MenuItem>(),
                        item => Equals(item.Header, "Snapshot of the dead") && item.IsVisible);

                    var safemode = menu.FindCommandItem(LegacyCommandId.RecordingSafemodeBackup)!;
                    Assert.True(safemode.IsChecked);
                    safemode.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => safemode.IsChecked == false, "Safemode recording command did not turn off.");
                    safemode.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => safemode.IsChecked == true, "Safemode recording command did not turn on.");

                    var deadRecording = menu.FindCommandItem(LegacyCommandId.RecordingDeadEnable)!;
                    var deadFilter = menu.FindCommandItem(LegacyCommandId.RecordingDeadExcludeRepopulating)!;
                    Assert.True(deadRecording.IsEnabled);
                    deadRecording.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => deadRecording.IsChecked == true, "Dead recording command did not turn on.");
                    deadFilter.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(() => deadFilter.IsChecked == true, "Dead-recording filter command did not turn on.");

                    var helpMenu = menu.TopLevelItems.Single(item => Equals(item.Header, "Help"));
                    Assert.True(helpMenu.IsVisible);
                    Assert.True(menu.FindCommandItem(LegacyCommandId.HelpDna)!.IsVisible);
                    Assert.True(menu.FindCommandItem(LegacyCommandId.HelpRobotTagInformation)!.IsVisible);
                    Assert.True(menu.FindCommandItem(LegacyCommandId.HelpAbout)!.IsVisible);
                    Assert.False(menu.FindCommandItem(LegacyCommandId.HelpSurvivalInfo)!.IsVisible);

                    var dnaHelp = menu.FindCommandItem(LegacyCommandId.HelpDna)!;
                    dnaHelp.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(
                        () => window.OwnedWindows.OfType<LegacyDocumentWindow>().Any(value => value.Title.Contains("DNA Help", StringComparison.Ordinal)),
                        "DNA Help menu command did not open its local document.");
                    var dnaWindow = Assert.Single(window.OwnedWindows.OfType<LegacyDocumentWindow>());
                    Assert.Contains("plain text", Assert.Single(dnaWindow.GetVisualDescendants().OfType<TextBox>()).Text, StringComparison.Ordinal);
                    dnaWindow.Close();
                    await WaitFor(() => !window.OwnedWindows.OfType<LegacyDocumentWindow>().Any(), "DNA Help window did not close.");

                    var about = menu.FindCommandItem(LegacyCommandId.HelpAbout)!;
                    about.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
                    await WaitFor(
                        () => window.OwnedWindows.OfType<LegacyDocumentWindow>().Any(value => value.Title.Contains("About Darwinbots Modern", StringComparison.Ordinal)),
                        "About menu command did not open its local document.");
                    var aboutWindow = Assert.Single(window.OwnedWindows.OfType<LegacyDocumentWindow>());
                    Assert.Contains(
                        "cross-platform continuation",
                        Assert.Single(aboutWindow.GetVisualDescendants().OfType<TextBox>()).Text,
                        StringComparison.Ordinal);
                    aboutWindow.Close();
                    await WaitFor(() => !window.OwnedWindows.OfType<LegacyDocumentWindow>().Any(), "About window did not close.");

                    var handleError = typeof(MainWindow).GetMethod("HandleEngineErrorAsync", BindingFlags.Instance | BindingFlags.NonPublic)!;
                    await (Task)handleError.Invoke(window, [new AccessViolationException("help-recording rendered recovery fault"), true])!;
                    Assert.True(window.FindControl<Border>("RecoveryPanel")!.IsVisible);
                    Assert.Equal("SAFE MODE RECOVERY READY", window.FindControl<TextBlock>("RecoveryTitle")!.Text);
                    Assert.Contains("FAULT WINDOW", window.FindControl<TextBlock>("RecoveryFaultWindowText")!.Text, StringComparison.Ordinal);
                    Assert.Contains("RECOVERY READY", viewModel.Status, StringComparison.Ordinal);

                    AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                    using var frame = Assert.IsAssignableFrom<Bitmap>(window.CaptureRenderedFrame());
                    Assert.True(frame.PixelSize.Width >= 1_000);
                    Assert.True(frame.PixelSize.Height >= 700);
                    var output = Path.Combine(Path.GetTempPath(), "db24832-help-recording.png");
                    using var stream = File.Create(output);
                    frame.Save(stream, new PngBitmapEncoderOptions());
                    Assert.True(File.Exists(output), $"Rendered receipt was not written: {output}");
                }
                finally
                {
                    foreach (var ownedWindow in window.OwnedWindows.ToArray())
                        if (ownedWindow.IsVisible) ownedWindow.Close();
                    if (window.IsVisible) window.Close();
                }
            }, CancellationToken.None);
        }
        finally
        {
            if (File.Exists(recoveryPath)) File.Delete(recoveryPath);
        }
    }

    private static async Task WaitFor(Func<bool> condition, string diagnostic)
    {
        for (var attempt = 0; attempt < 500; attempt++)
        {
            AvaloniaHeadlessPlatform.ForceRenderTimerTick();
            if (condition()) return;
            await Task.Delay(20);
        }

        throw new Xunit.Sdk.XunitException(diagnostic);
    }
}
