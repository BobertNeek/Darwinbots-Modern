using System.Reflection;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Avalonia.Media.Imaging;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class GraphContestRenderedTests
{
    [Fact]
    public async Task MainWindowRoutesToRustBackedGraphAndContestResultsWithOnlineCommandsDisabled()
    {
        var dna = new[] { "start\nstop", "start\n1\nstop", "start\n2\nstop", "start\n3\nstop", "start\n4\nstop" };
        var setup = new WorldSetupOptions
        {
            Backend = "Cpu",
            Seed = 24832,
            PopulationCapacity = 1_000,
            WorldWidth = 2_000,
            WorldHeight = 1_500,
            MetabolismCost = 0,
            VegetableEnergyPerTick = 0,
            SunlightEnergy = 0,
            Species = dna.Select((value, index) => new SpeciesImport(
                $"Competitor {index + 1}",
                value,
                false,
                0xff224466u + (uint)(index * 0x001122),
                2,
                1_000,
                0,
                false,
                0f)).ToArray(),
        };

        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var window = new MainWindow([], setup);
            try
            {
                window.WindowState = WindowState.Normal;
                window.Width = 940;
                window.Height = 650;
                window.Show();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();

                await WaitFor(
                    () => TryField<SimulationSession>(window, "_session")?.LatestSnapshot.Species.Count == 5,
                    () => "MainWindow did not publish all Rust-backed imported species.");

                var session = TryField<SimulationSession>(window, "_session")!;
                await session.ConfigureLocalModeAsync(new LocalModeConfig(
                    LocalModeKind.Contest,
                    LeaguePreset.F1,
                    1,
                    1,
                    200,
                    1_000,
                    true,
                    null,
                    0.5f,
                    0.5f,
                    new()));
                await session.StartLocalModeAsync();
                await session.StepAsync(100);
                await session.AdjudicateLocalRoundAsync(1, "rendered route adjudication");
                await WaitFor(
                    () => session.LatestSnapshot.History.Count > 0 && session.LatestSnapshot.LocalMode.Results.Count > 0,
                    () => "Rust-backed history or local contest result was not published.");

                var graphsButton = window.FindControl<Button>("GraphsButton")!;
                graphsButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(
                    () => window.OwnedWindows.OfType<GraphSuiteWindow>().Any(),
                    () => "MainWindow did not open Graph Suite.");
                var graphWindow = Assert.Single(window.OwnedWindows.OfType<GraphSuiteWindow>());
                graphWindow.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.AverageDnaLength);
                var graphView = graphWindow.FindControl<GraphSuiteView>("GraphView")!;
                var openGraphButton = graphView.FindControl<Button>("OpenGraphButton")!;
                openGraphButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(
                    () => graphWindow.ViewModel.RunningGraphs.Count > 0,
                    () => "Graph Suite did not open a graph from the routed window.");

                graphWindow.ViewModel.SelectedChoice = GraphCatalog.For(GraphMetric.Population);
                openGraphButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(
                    () => graphWindow.OwnedWindows.OfType<GraphSeriesWindow>().Count() == 2,
                    () => "Graph Suite did not open two live graph windows.");
                Assert.All(graphWindow.ViewModel.RunningGraphs, graph => Assert.NotEmpty(graph.Samples));

                var preUpdateTick = graphWindow.ViewModel.RunningGraphs
                    .SelectMany(graph => graph.Samples)
                    .Max(sample => sample.Tick);
                await session.StepAsync(100);
                await WaitFor(
                    () => graphWindow.ViewModel.RunningGraphs.All(graph => graph.Samples.Any(sample => sample.Tick > preUpdateTick)),
                    () => "Live graph windows did not receive the next Rust history sample.");

                var export = graphWindow.ViewModel.ExportCsv();
                Assert.StartsWith("series_id,metric,title,expression,tick,value\n", export, StringComparison.Ordinal);
                Assert.Contains("Population", export, StringComparison.Ordinal);
                Assert.Contains("Average DNA length", export, StringComparison.Ordinal);

                foreach (var graph in graphWindow.ViewModel.RunningGraphs) graph.IsSelected = true;
                graphView.FindControl<Button>("JoinCompareButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.Contains("JOINED", graphWindow.ViewModel.Status, StringComparison.Ordinal);

                var resetTick = graphWindow.ViewModel.RunningGraphs
                    .SelectMany(graph => graph.Samples)
                    .Max(sample => sample.Tick);
                graphView.FindControl<Button>("ResetGraphsButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Assert.All(graphWindow.ViewModel.RunningGraphs, graph => Assert.Empty(graph.Samples));
                await session.StepAsync(100);
                await WaitFor(
                    () => graphWindow.ViewModel.RunningGraphs.All(graph => graph.Samples.Count > 0 && graph.Samples.Min(sample => sample.Tick) > resetTick),
                    () => "Reset graphs did not resume from a new live history segment.");

                var contestButton = window.FindControl<Button>("ContestResultsButton")!;
                contestButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await WaitFor(
                    () => window.OwnedWindows.OfType<ContestResultsWindow>().Any(),
                    () => "MainWindow did not open Contest Results.");
                var contestWindow = Assert.Single(window.OwnedWindows.OfType<ContestResultsWindow>());
                Assert.Equal(5, contestWindow.ViewModel.Competitors.Count);
                Assert.Contains(contestWindow.ViewModel.Competitors, competitor => competitor.Wins > 0);

                var historicMenu = window.FindControl<LegacyMenuBar>("HistoricMenuBar")!;
                var onlineCommands = new[]
                {
                    LegacyCommandId.InternetEnabled,
                    LegacyCommandId.InternetTeleporter,
                    LegacyCommandId.EditInternetOptions,
                };
                var ownedWindowCount = window.OwnedWindows.Count;
                foreach (var command in onlineCommands)
                {
                    var item = historicMenu.FindCommandItem(command)!;
                    Assert.True(item.IsVisible);
                    Assert.False(item.IsEnabled);
                    Assert.Contains("online", ToolTip.GetTip(item)?.ToString() ?? string.Empty, StringComparison.OrdinalIgnoreCase);
                }
                Assert.Equal(ownedWindowCount, window.OwnedWindows.Count);

                var evidenceGraphView = new GraphSuiteView(graphWindow.ViewModel);
                var evidenceContestView = new ContestResultsView(contestWindow.ViewModel);
                var host = new Window
                {
                    Title = "Graph and contest route evidence",
                    Width = 1_520,
                    Height = 820,
                    Content = new Grid
                    {
                        ColumnDefinitions = new ColumnDefinitions("1.12*,0.88*"),
                        Children = { evidenceGraphView, evidenceContestView },
                    },
                };
                Grid.SetColumn(evidenceContestView, 1);
                host.WindowState = WindowState.Normal;
                host.Show();
                AvaloniaHeadlessPlatform.ForceRenderTimerTick();
                using var frame = host.CaptureRenderedFrame();
                Assert.NotNull(frame);
                Assert.True(frame.PixelSize.Width >= 1_000);
                Assert.True(frame.PixelSize.Height >= 600);
                var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR")
                    ?? Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
                Directory.CreateDirectory(directory);
                using var stream = File.Create(Path.Combine(directory, "graph-contest-results.png"));
                frame.Save(stream, new PngBitmapEncoderOptions());
                host.Close();
            }
            finally
            {
                foreach (var ownedWindow in window.OwnedWindows.ToArray()) ownedWindow.Close();
                window.Close();
            }
        }, CancellationToken.None);
    }

    private static T? TryField<T>(object target, string name) where T : class =>
        target.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic)?.GetValue(target) as T;

    private static async Task WaitFor(Func<bool> condition, Func<string> diagnostic)
    {
        for (var attempt = 0; attempt < 500; attempt++)
        {
            AvaloniaHeadlessPlatform.ForceRenderTimerTick();
            if (condition()) return;
            await Task.Delay(20);
        }

        throw new Xunit.Sdk.XunitException(diagnostic());
    }
}
