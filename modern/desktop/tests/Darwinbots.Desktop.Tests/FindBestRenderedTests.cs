using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Avalonia.Threading;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.ViewModels;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class FindBestRenderedTests
{
    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public async Task ManualZerobotSelectionUsesTheSourceActionAndClearsRestartedSelection(bool menu)
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var factory = new AuditEngineClientFactory();
            var setup = new WorldSetupOptions { Species = [
                new("Audit Species", "start stop", false, 0xff239ac0, 1, 1000, 0, false, 0),
            ] };
            var window = new MainWindow([], setup, factory, new AuditStorageService());
            try
            {
                window.Show();
                await WaitFor(() => !factory.Clients.IsEmpty && Field<SimulationSession>(window, "_session") is { LatestSnapshot.Population: 1 }
                    && Field<MainWindowViewModel>(window, "_viewModel")!.Status.StartsWith("WORLD READY", StringComparison.Ordinal),
                    () => DescribeReadiness(window));
                var engine = factory.Clients.First();
                var session = Field<SimulationSession>(window, "_session")!;
                var model = Field<MainWindowViewModel>(window, "_viewModel")!;
                var evolving = LocalModeSnapshot.Idle with {
                    Kind = LocalModeKind.Zerobot, Status = LocalModeStatus.Running,
                    Configuration = LocalModeConfig.Default with {
                        Kind = LocalModeKind.Zerobot,
                        StagedEvolution = new(EvolutionMode.Zerobot, "", 10, 150, false, 32, false),
                    },
                    Evolution = new(0, "evolving", false, 10, 0, 0, 0, 0) { FitnessWeight = 29 },
                };
                var menus = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                // Staging a mode has not entered original 7/8 and must keep pending settings.
                engine.LocalModeForRead = evolving with { Status = LocalModeStatus.Configured };
                await session.RefreshSnapshotAsync();
                await Flush();
                await CheckWeightDialogAndCancel(window, menus, 100m);
                InvokeFindBest(window, menus, menu);
                await WaitFor(() => model.Status.Contains("BEST FAMILY", StringComparison.Ordinal),
                    () => DescribeReadiness(window));
                Assert.Equal(LocalModeStatus.Configured, session.LatestSnapshot.LocalMode.Status);
                Assert.Equal(new OrganismKey(3, 2), model.SelectedKey);
                engine.LocalModeForRead = evolving;
                await session.RefreshSnapshotAsync();
                await Flush();
                // Pausing does not change Running: show the active search weight while paused.
                await CheckWeightDialogAndCancel(window, menus, 29m);
                model.Select(new OrganismKey(3, 2));
                var viewport = window.FindControl<WorldViewport>("Viewport")!;
                viewport.SelectKey(new(3, 2));
                Assert.Equal(new OrganismKey(3, 2), model.SelectedKey);
                Assert.Equal(new OrganismKey(3, 2), Assert.IsType<OrganismKey>(Field<object>(viewport, "_selectedKey")));
                int? fallback = null;
                engine.SelectBestOverride = weight => {
                    fallback = weight;
                    engine.LocalModeForRead = evolving with {
                        RestartCount = 1,
                        Evolution = evolving.Evolution! with { Stage = 1, Phase = "zerobot_testing" },
                    };
                    return new(null, true) { EffectiveWeight = 29 };
                };
                InvokeFindBest(window, menus, menu);
                await WaitFor(() => model.Status.Contains("ZEROBOT ADVANCED TO TESTING", StringComparison.Ordinal),
                    () => DescribeReadiness(window));
                Assert.Equal(100, fallback);
                Assert.Null(model.SelectedKey);
                Assert.Equal("zerobot_testing", session.LatestSnapshot.LocalMode.Evolution!.Phase);
                Assert.DoesNotContain("NO LIVING ROBOTS", model.Status);
                Assert.Null(Field<object>(viewport, "_selectedKey"));
                await session.RefreshSnapshotAsync();
                await Flush();
                Assert.Null(model.SelectedKey);
            }
            finally { window.Close(); }
        }, CancellationToken.None);
    }

    private static void InvokeFindBest(MainWindow window, LegacyMenuBar menus, bool menu)
    {
        if (menu)
            menus.FindCommandItem(LegacyCommandId.RobotFindBest)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
        else
            window.FindControl<Button>("ToolbarSlot18")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
    }

    private static async Task CheckWeightDialogAndCancel(MainWindow window, LegacyMenuBar menus, decimal expected)
    {
        menus.FindCommandItem(LegacyCommandId.EditGlobalSettings)!.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
        await WaitFor(() => window.OwnedWindows.OfType<GlobalSettingsWindow>().Any(),
            () => DescribeReadiness(window));
        var settings = Assert.Single(window.OwnedWindows.OfType<GlobalSettingsWindow>());
        Assert.Equal(expected, settings.FindControl<NumericUpDown>("FindBestWeight")!.Value);
        var model = Field<MainWindowViewModel>(window, "_viewModel")!;
        Assert.NotEqual("CANCELED", model.Status);
        settings.FindControl<Button>("CancelButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        // Closing the dialog precedes the outer async menu handler publishing cancellation.
        // Require a fresh terminal status before the next command can publish its result.
        await WaitFor(() => !window.OwnedWindows.OfType<GlobalSettingsWindow>().Any()
            && model.Status == "CANCELED", () => DescribeReadiness(window));
    }

    private static string DescribeReadiness(MainWindow window)
    {
        var model = Field<MainWindowViewModel>(window, "_viewModel");
        var session = Field<SimulationSession>(window, "_session");
        return $"Status={model?.Status}; SelectedKey={model?.SelectedKey}; "
            + $"Tick={session?.LatestSnapshot.Tick}; Population={session?.LatestSnapshot.Population}; "
            + $"GlobalSettingsWindows={window.OwnedWindows.OfType<GlobalSettingsWindow>().Count()}";
    }

    private static T? Field<T>(object target, string name) where T : class =>
        target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)?.GetValue(target) as T;

    private static async Task Flush()
    {
        await Dispatcher.UIThread.InvokeAsync(() => { }, DispatcherPriority.Background);
        AvaloniaHeadlessPlatform.ForceRenderTimerTick();
    }

    private static async Task WaitFor(Func<bool> ready, Func<string>? details = null)
    {
        for (var attempt = 0; attempt < 500; attempt++)
        {
            await Flush();
            if (ready()) return;
            await Task.Delay(20);
        }
        Assert.True(ready(), $"Find Best route did not publish its expected UI state. {details?.Invoke()}");
    }
}
