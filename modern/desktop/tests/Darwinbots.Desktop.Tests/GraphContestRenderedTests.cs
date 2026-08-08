using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Media.Imaging;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class GraphContestRenderedTests
{
    [Fact]
    public async Task GraphSuiteAndContestResultsRenderTogether()
    {
        using var engine = new NativeEngineClient(
            backend: "Cpu",
            capacity: 1_000,
            seed: 24832,
            worldWidth: 2_000,
            worldHeight: 1_500,
            metabolismCost: 0,
            vegetableEnergyPerTick: 0,
            sunlightEnergy: 0);
        var dna = new[] { "start\nstop", "start\n1\nstop", "start\n2\nstop", "start\n3\nstop", "start\n4\nstop" };
        for (var index = 0; index < dna.Length; index++)
        {
            engine.ImportSpecies(new SpeciesImport(
                $"Competitor {index + 1}",
                dna[index],
                false,
                0xff224466u + (uint)(index * 0x001122),
                2,
                1_000,
                0,
                false,
                0f));
        }
        engine.ConfigureLocalMode(new LocalModeConfig(
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
        engine.StartLocalMode();
        engine.Tick(100);
        engine.AdjudicateLocalRound(1, "rendered local adjudication");
        var snapshot = engine.Snapshot();
        Assert.NotEmpty(snapshot.History);
        Assert.All(snapshot.History, sample => Assert.True(sample.AverageDnaLength > 0));

        var graphModel = new GraphSuiteViewModel(GraphMetricSampleProjector.FromSnapshot(snapshot));
        graphModel.ConfigureCustomMetric(1, "Energy / 10", "total_energy / 10");
        graphModel.SelectedChoice = GraphCatalog.For(GraphMetric.AverageDnaLength);
        graphModel.OpenSelectedGraph();
        graphModel.SelectedChoice = GraphCatalog.For(GraphMetric.CustomizableGraph1);
        graphModel.OpenSelectedGraph();

        var result = ContestResultSnapshot.FromSnapshot(snapshot);
        Assert.Equal(5, result.Competitors.Count);
        Assert.Equal(1, result.Competitors.Single(value => value.Name == "Competitor 1").Wins);
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var graphView = new GraphSuiteView(graphModel);
            var contestView = new ContestResultsView(new ContestResultsViewModel(result));
            var host = new Window
            {
                Title = "Graph and contest evidence",
                Width = 1_520,
                Height = 820,
                Content = new Grid
                {
                    ColumnDefinitions = new ColumnDefinitions("1.12*,0.88*"),
                    Children = { graphView, contestView },
                },
            };
            Grid.SetColumn(contestView, 1);
            host.WindowState = WindowState.Normal;
            host.Show();
            AvaloniaHeadlessPlatform.ForceRenderTimerTick();
            Assert.Equal(2, graphModel.RunningGraphs.Count);
            using var frame = host.CaptureRenderedFrame();
            Assert.NotNull(frame);
            Assert.True(frame.PixelSize.Width >= 1000);
            Assert.True(frame.PixelSize.Height >= 600);
            var directory = Environment.GetEnvironmentVariable("DARWINBOTS_GUI_AUDIT_DIR")
                ?? Path.Combine(Path.GetTempPath(), "darwinbots-modern-control-audit");
            Directory.CreateDirectory(directory);
            using var stream = File.Create(Path.Combine(directory, "graph-contest-results.png"));
            frame.Save(stream, new PngBitmapEncoderOptions());
            host.Close();
        }, CancellationToken.None);
    }
}
