using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class GraphAndContestTests
{
    [Fact]
    public void GraphCatalogAndCustomMetricWorkflowPreserveHistoricalChoices()
    {
        (GraphMetric Metric, string Label)[] expectedMetrics =
        [
            (GraphMetric.Population, "Population"),
            (GraphMetric.AverageMutations, "Average mutations"),
            (GraphMetric.AverageMutationsPerDnaLength, "Average mutations/DNA len"),
            (GraphMetric.AverageAge, "Average Age"),
            (GraphMetric.AverageDescendents, "Average Descendents"),
            (GraphMetric.AverageEnergy, "Average Energy"),
            (GraphMetric.AverageDnaLength, "Average DNA length"),
            (GraphMetric.AverageDnaConditions, "Average DNA conditions"),
            (GraphMetric.TotalEnergy, "Total Energy"),
            (GraphMetric.DynamicCostStats, "Dynamic Cost Stats"),
            (GraphMetric.SpeciesDiversity, "Species Diversity"),
            (GraphMetric.AverageChloroplasts, "Average Chloroplasts"),
            (GraphMetric.SelectiveGeneticDistance, "Selective Genetic Distance"),
            (GraphMetric.GenerationalDistance, "Generational Distance"),
            (GraphMetric.SlowGeneticDistance, "Slow Genetic Distance"),
            (GraphMetric.CustomizableGraph1, "Customizable Graph 1"),
            (GraphMetric.CustomizableGraph2, "Customizable Graph 2"),
            (GraphMetric.CustomizableGraph3, "Customizable Graph 3"),
        ];
        GraphMetricSample[] samples =
        [
            new(10, new Dictionary<GraphMetric, double>
            {
                [GraphMetric.Population] = 8,
                [GraphMetric.TotalEnergy] = 80,
            }),
            new(20, new Dictionary<GraphMetric, double>
            {
                [GraphMetric.Population] = 10,
                [GraphMetric.TotalEnergy] = 120,
            }),
        ];
        var suite = new GraphSuiteViewModel(samples);

        Assert.Equal(expectedMetrics, GraphCatalog.Metrics.Select(choice => (choice.Metric!.Value, choice.Label)));
        Assert.Contains(GraphCatalog.Items, choice => choice.Action == GraphMenuAction.ResetAllGraphs);
        Assert.Contains(GraphCatalog.Items, choice => choice.Action == GraphMenuAction.ListAllRunningGraphs);

        suite.ConfigureCustomMetric(1, "Energy / 10", "total_energy / 10");
        suite.SelectedChoice = GraphCatalog.For(GraphMetric.CustomizableGraph1);
        var custom = Assert.IsType<GraphSeriesViewModel>(suite.OpenSelectedGraph());
        Assert.Equal("Energy / 10", custom.Title);
        Assert.Equal([8d, 12d], custom.Samples.Select(sample => sample.Value));

        suite.SelectedChoice = GraphCatalog.For(GraphMetric.Population);
        var population = Assert.IsType<GraphSeriesViewModel>(suite.OpenSelectedGraph());
        var comparison = suite.JoinAndCompare([custom, population]);
        Assert.NotNull(comparison);
        Assert.Equal([0d, 2d], comparison!.Differences.Select(value => value.Difference));
        Assert.Contains("Energy / 10", suite.ExportCsv(), StringComparison.Ordinal);

        suite.ResetAllGraphs();
        Assert.All(suite.RunningGraphs, graph => Assert.Empty(graph.Samples));
    }

    [Fact]
    public void ContestResultsViewModelRecordsManualAndInstantWinsThroughLocalContract()
    {
        var adjudications = new List<ContestAdjudication>();
        var result = new ContestResultSnapshot(
            "F1 Local",
            2,
            5,
            10,
            [
                new ContestCompetitorResult("alpha", "Alpha", 2, 12),
                new ContestCompetitorResult("beta", "Beta", 1, 8),
            ]);
        var viewModel = new ContestResultsViewModel(result, adjudications.Add);

        viewModel.AwardManualWin("alpha");
        viewModel.AwardInstantWin("beta");

        Assert.True(viewModel.IsLocalOnly);
        Assert.Equal(3, viewModel.Competitors.Single(value => value.Id == "alpha").Wins);
        var beta = viewModel.Competitors.Single(value => value.Id == "beta");
        Assert.True(beta.InstantWinner);
        Assert.Equal(
            [ContestAdjudicationKind.ManualWin, ContestAdjudicationKind.InstantWin],
            adjudications.Select(value => value.Kind));
        Assert.Contains("INSTANT WIN", viewModel.Status, StringComparison.Ordinal);
    }
}
