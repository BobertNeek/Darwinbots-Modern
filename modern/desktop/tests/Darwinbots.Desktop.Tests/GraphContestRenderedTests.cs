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
        GraphMetricSample[] graphSamples =
        [
            new(100, new Dictionary<GraphMetric, double>
            {
                [GraphMetric.Population] = 12,
                [GraphMetric.TotalEnergy] = 1_200,
            }),
            new(200, new Dictionary<GraphMetric, double>
            {
                [GraphMetric.Population] = 18,
                [GraphMetric.TotalEnergy] = 1_950,
            }),
            new(300, new Dictionary<GraphMetric, double>
            {
                [GraphMetric.Population] = 15,
                [GraphMetric.TotalEnergy] = 1_710,
            }),
        ];
        var graphModel = new GraphSuiteViewModel(graphSamples);
        graphModel.ConfigureCustomMetric(1, "Energy / 10", "total_energy / 10");
        graphModel.SelectedChoice = GraphCatalog.For(GraphMetric.Population);
        graphModel.OpenSelectedGraph();
        graphModel.SelectedChoice = GraphCatalog.For(GraphMetric.CustomizableGraph1);
        graphModel.OpenSelectedGraph();

        var result = new ContestResultSnapshot(
            "F1 Local Practice",
            3,
            5,
            10,
            [
                new ContestCompetitorResult("alpha", "Alpha", 2, 18),
                new ContestCompetitorResult("beta", "Beta", 1, 15),
                new ContestCompetitorResult("gamma", "Gamma", 0, 9),
                new ContestCompetitorResult("delta", "Delta", 0, 7),
                new ContestCompetitorResult("epsilon", "Epsilon", 0, 4),
            ]);
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
