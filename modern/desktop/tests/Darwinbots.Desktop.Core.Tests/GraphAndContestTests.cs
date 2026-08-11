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
            (GraphMetric.AverageAge, "Average Age"),
            (GraphMetric.AverageDescendents, "Average Descendents"),
            (GraphMetric.AverageEnergy, "Average Energy"),
            (GraphMetric.AverageDnaLength, "Average DNA length"),
            (GraphMetric.AverageDnaConditions, "Average DNA conditions"),
            (GraphMetric.AverageMutationsPerDnaLength, "Average mutations/DNA len"),
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

        var left = new GraphSeriesViewModel(
            "left",
            GraphMetric.Population,
            "Left",
            [new GraphSample(10, 1), new GraphSample(20, 2)]);
        var right = new GraphSeriesViewModel(
            "right",
            GraphMetric.TotalEnergy,
            "Right",
            [new GraphSample(20, 10), new GraphSample(30, 20)]);
        var overlapOnly = suite.JoinAndCompare([left, right]);
        Assert.NotNull(overlapOnly);
        Assert.Equal([20UL], overlapOnly!.Differences.Select(value => value.Tick));
        Assert.Equal([-8d], overlapOnly.Differences.Select(value => value.Difference));

        suite.ResetAllGraphs();
        Assert.All(suite.RunningGraphs, graph => Assert.Empty(graph.Samples));
    }

    [Fact]
    public void GraphProjectorUsesRustHistoryMetricsForEveryHistoricalTick()
    {
        var snapshot = new EngineSnapshot(
            30,
            2,
            "CPU",
            [
                new OrganismSnapshot(0, 0, [0, 0], [0, 0], 100, 90, Chloroplasts: 90),
                new OrganismSnapshot(1, 0, [1, 1], [0, 0], 100, 90, Chloroplasts: 90),
            ])
        {
            History =
            [
                new HistorySampleSnapshot(10, 2, 20, 0, 0, 0, 0)
                {
                    AverageMutations = 2,
                    AverageMutationsPerDnaLength = 0.5,
                    AverageAge = 1.5,
                    AverageDescendents = 3,
                    AverageEnergy = 10,
                    AverageDnaLength = 4,
                    AverageDnaConditions = 5,
                    DynamicCostStats = 6,
                    SpeciesDiversity = 2,
                    AverageChloroplasts = 2.5,
                    SelectiveGeneticDistance = 7,
                    GenerationalDistance = 8,
                    SlowGeneticDistance = 9,
                },
                new HistorySampleSnapshot(20, 2, 40, 0, 0, 0, 0)
                {
                    AverageMutations = 4,
                    AverageMutationsPerDnaLength = 1,
                    AverageAge = 2.5,
                    AverageDescendents = 6,
                    AverageEnergy = 20,
                    AverageDnaLength = 8,
                    AverageDnaConditions = 10,
                    DynamicCostStats = 12,
                    SpeciesDiversity = 3,
                    AverageChloroplasts = 4.5,
                    SelectiveGeneticDistance = 14,
                    GenerationalDistance = 16,
                    SlowGeneticDistance = 18,
                },
            ],
        };

        var projected = GraphMetricSampleProjector.FromSnapshot(snapshot);

        Assert.Equal([2d, 4d], projected.Select(value => value.Values[GraphMetric.AverageMutations]));
        Assert.Equal([0.5d, 1d], projected.Select(value => value.Values[GraphMetric.AverageMutationsPerDnaLength]));
        Assert.Equal([1.5d, 2.5d], projected.Select(value => value.Values[GraphMetric.AverageAge]));
        Assert.Equal([4d, 8d], projected.Select(value => value.Values[GraphMetric.AverageDnaLength]));
        Assert.Equal([5d, 10d], projected.Select(value => value.Values[GraphMetric.AverageDnaConditions]));
        Assert.Equal([6d, 12d], projected.Select(value => value.Values[GraphMetric.DynamicCostStats]));
        Assert.Equal([2d, 3d], projected.Select(value => value.Values[GraphMetric.SpeciesDiversity]));
        Assert.Equal([2.5d, 4.5d], projected.Select(value => value.Values[GraphMetric.AverageChloroplasts]));
        Assert.Equal([7d, 14d], projected.Select(value => value.Values[GraphMetric.SelectiveGeneticDistance]));
        Assert.Equal([8d, 16d], projected.Select(value => value.Values[GraphMetric.GenerationalDistance]));
        Assert.Equal([9d, 18d], projected.Select(value => value.Values[GraphMetric.SlowGeneticDistance]));
    }

    [Fact]
    public void HistoricalGraphCatalogOrderMapsEveryExposedHistoryMetric()
    {
        (GraphMetric Metric, string Label, double[] Values)[] expectedMetrics =
        [
            (GraphMetric.Population, "Population", [4d, 6d]),
            (GraphMetric.AverageMutations, "Average mutations", [1d, 2d]),
            (GraphMetric.AverageAge, "Average Age", [3d, 4d]),
            (GraphMetric.AverageDescendents, "Average Descendents", [5d, 6d]),
            (GraphMetric.AverageEnergy, "Average Energy", [7d, 8d]),
            (GraphMetric.AverageDnaLength, "Average DNA length", [9d, 10d]),
            (GraphMetric.AverageDnaConditions, "Average DNA conditions", [11d, 12d]),
            (GraphMetric.AverageMutationsPerDnaLength, "Average mutations/DNA len", [13d, 14d]),
            (GraphMetric.TotalEnergy, "Total Energy", [150d, 250d]),
            (GraphMetric.DynamicCostStats, "Dynamic Cost Stats", [15d, 16d]),
            (GraphMetric.SpeciesDiversity, "Species Diversity", [2d, 3d]),
            (GraphMetric.AverageChloroplasts, "Average Chloroplasts", [17d, 18d]),
            (GraphMetric.SelectiveGeneticDistance, "Selective Genetic Distance", [19d, 20d]),
            (GraphMetric.GenerationalDistance, "Generational Distance", [21d, 22d]),
            (GraphMetric.SlowGeneticDistance, "Slow Genetic Distance", [23d, 24d]),
            (GraphMetric.CustomizableGraph1, "Customizable Graph 1", [7d, 8d]),
            (GraphMetric.CustomizableGraph2, "Customizable Graph 2", [150d, 250d]),
            (GraphMetric.CustomizableGraph3, "Customizable Graph 3", [17d, 18d]),
        ];
        var snapshot = new EngineSnapshot(30, 6, "CPU", [])
        {
            History =
            [
                new HistorySampleSnapshot(10, 4, 150, 0, 0, 0, 0)
                {
                    AverageMutations = 1,
                    AverageAge = 3,
                    AverageDescendents = 5,
                    AverageEnergy = 7,
                    AverageDnaLength = 9,
                    AverageDnaConditions = 11,
                    AverageMutationsPerDnaLength = 13,
                    DynamicCostStats = 15,
                    SpeciesDiversity = 2,
                    AverageChloroplasts = 17,
                    SelectiveGeneticDistance = 19,
                    GenerationalDistance = 21,
                    SlowGeneticDistance = 23,
                },
                new HistorySampleSnapshot(20, 6, 250, 0, 0, 0, 0)
                {
                    AverageMutations = 2,
                    AverageAge = 4,
                    AverageDescendents = 6,
                    AverageEnergy = 8,
                    AverageDnaLength = 10,
                    AverageDnaConditions = 12,
                    AverageMutationsPerDnaLength = 14,
                    DynamicCostStats = 16,
                    SpeciesDiversity = 3,
                    AverageChloroplasts = 18,
                    SelectiveGeneticDistance = 20,
                    GenerationalDistance = 22,
                    SlowGeneticDistance = 24,
                },
            ],
        };

        var projected = GraphMetricSampleProjector.FromSnapshot(snapshot);

        Assert.Equal(
            expectedMetrics.Select(value => (value.Metric, value.Label)),
            GraphCatalog.Metrics.Select(value => (value.Metric!.Value, value.Label)));
        foreach (var expected in expectedMetrics)
            Assert.Equal(expected.Values, projected.Select(value => value.Values[expected.Metric]));
    }

    [Fact]
    public void ResetAllGraphsStartsANewLiveHistorySegment()
    {
        GraphMetricSample[] beforeReset =
        [
            new(10, new Dictionary<GraphMetric, double> { [GraphMetric.Population] = 4 }),
            new(20, new Dictionary<GraphMetric, double> { [GraphMetric.Population] = 5 }),
        ];
        var suite = new GraphSuiteViewModel(beforeReset)
        {
            SelectedChoice = GraphCatalog.For(GraphMetric.Population),
        };
        var graph = Assert.IsType<GraphSeriesViewModel>(suite.OpenSelectedGraph());

        suite.ResetAllGraphs();
        suite.SetSamples(
        [
            .. beforeReset,
            new(30, new Dictionary<GraphMetric, double> { [GraphMetric.Population] = 6 }),
        ]);

        Assert.Equal([30UL], graph.Samples.Select(sample => sample.Tick));
        Assert.Equal([6d], graph.Samples.Select(sample => sample.Value));
    }

    [Fact]
    public void CustomGraphDefinitionUpdatesRunningSeriesAndExportRetainsItsContract()
    {
        var suite = new GraphSuiteViewModel(
        [
            new(10, new Dictionary<GraphMetric, double> { [GraphMetric.TotalEnergy] = 80 }),
            new(20, new Dictionary<GraphMetric, double> { [GraphMetric.TotalEnergy] = 120 }),
        ]);
        suite.ConfigureCustomMetric(1, "Energy / 10", "total_energy / 10");
        suite.SelectedChoice = GraphCatalog.For(GraphMetric.CustomizableGraph1);
        var graph = Assert.IsType<GraphSeriesViewModel>(suite.OpenSelectedGraph());

        suite.ConfigureCustomMetric(1, "Energy / 5", "total_energy / 5");

        Assert.Equal("Energy / 5", graph.Title);
        Assert.Equal("total_energy / 5", graph.CustomExpression);
        Assert.Equal([16d, 24d], graph.Samples.Select(sample => sample.Value));

        var export = suite.ExportCsv();
        Assert.StartsWith("series_id,metric,title,expression,tick,value\n", export, StringComparison.Ordinal);
        Assert.Contains("CustomizableGraph1", export, StringComparison.Ordinal);
        Assert.Contains("Energy / 5", export, StringComparison.Ordinal);
        Assert.Contains("total_energy / 5", export, StringComparison.Ordinal);
    }

    [Fact]
    public void LegacyGraphJoinImportsSegmentedSamplesAndDatabaseExportUsesLegacyHeader()
    {
        var segmentText = string.Join('\n',
            [
                "trial one",
                "16711680:Alpha,255:Beta",
                .. Enumerable.Range(0, 999).Select(index => $"{index}\t{index * 2}"),
            ]);

        var segments = LegacyGraphJoin.Parse([segmentText]);
        var imported = LegacyGraphJoin.ToSeries(segments);

        Assert.Equal("trial one · Alpha", imported[0].Title);
        Assert.Equal([new GraphSample(0, 0), new GraphSample(998, 998)],
            imported[0].Samples.Where(sample => sample.Tick is 0 or 998));
        Assert.Equal("trial one · Beta", imported[1].Title);
        Assert.Equal(1996d, imported[1].Samples[^1].Value);

        var organism = new OrganismSnapshot(7, 3, [10, 20], [0, 0], 100, 4, Species: 0)
        {
            Parents = [new OrganismKey(2, 1), null],
            UniqueId = 7003,
            TotalMutations = 12,
            NewMutations = 3,
            DnaLength = 18,
            Offspring = 5,
            Chloroplasts = 9,
        };
        var snapshot = new EngineSnapshot(100, 1, "CPU", [organism])
        {
            Species = [new SpeciesSnapshot("Alpha", false, 0xff0000, 0, false)],
        };

        var database = DatabaseExport.Serialize(snapshot);
        Assert.Equal(
            "Rob id,Parent id,Founder name,Generation,Birth cycle,Age,Mutations,New mutations,Dna length,Offspring number,kills,Fitness,Energy,Chloroplasts\r\n"
                + "7003,2,Alpha,3,0,4,12,3,18,5,0,1100,1100,9\r\n",
            database);

        var history = new GraphSuiteViewModel
        (
            [
                new(10, new Dictionary<GraphMetric, double>
                {
                    [GraphMetric.Population] = 2,
                    [GraphMetric.AverageEnergy] = 12.5,
                }),
                new(20, new Dictionary<GraphMetric, double>
                {
                    [GraphMetric.Population] = 4,
                    [GraphMetric.AverageEnergy] = 18.25,
                }),
            ]);
        history.SelectedChoice = GraphCatalog.For(GraphMetric.Population);
        Assert.NotNull(history.OpenSelectedGraph());
        history.SelectedChoice = GraphCatalog.For(GraphMetric.AverageEnergy);
        Assert.NotNull(history.OpenSelectedGraph());

        Assert.Equal("RUNNING · AVERAGE ENERGY", history.Status);
        Assert.Equal(
            "series_id,metric,title,expression,tick,value\r\n"
                + "graph-1,Population,Population,,10,2\r\n"
                + "graph-1,Population,Population,,20,4\r\n"
                + "graph-2,AverageEnergy,Average Energy,,10,12.5\r\n"
                + "graph-2,AverageEnergy,Average Energy,,20,18.25\r\n",
            history.ExportCsv());
    }

    [Fact]
    public void GraphHistoryExportUsesLegacyCrLfRecordBoundaries()
    {
        var suite = new GraphSuiteViewModel(
        [
            new GraphMetricSample(10, new Dictionary<GraphMetric, double>
            {
                [GraphMetric.Population] = 2,
            }),
            new GraphMetricSample(20, new Dictionary<GraphMetric, double>
            {
                [GraphMetric.Population] = 4,
            }),
        ]);

        suite.SelectedChoice = GraphCatalog.For(GraphMetric.Population);
        Assert.NotNull(suite.OpenSelectedGraph());

        Assert.Equal(
            "series_id,metric,title,expression,tick,value\r\n"
                + "graph-1,Population,Population,,10,2\r\n"
                + "graph-1,Population,Population,,20,4\r\n",
            suite.ExportCsv());
    }

    [Fact]
    public void LegacyGraphExportWritesJoinCompatibleNineHundredNinetyNineRowSegments()
    {
        var graph = new GraphSeriesViewModel(
            "population",
            GraphMetric.Population,
            "Population",
            Enumerable.Range(0, 1_000)
                .Select(index => new GraphSample((ulong)index, index))
                .ToArray());

        var files = LegacyGraphJoin.Serialize([graph], "population");

        Assert.Equal(2, files.Count);
        var segments = LegacyGraphJoin.Parse(files);
        Assert.Equal("population", segments[0].Title);
        Assert.Equal(999, segments[0].Robots[0].Values.Count);
        Assert.Equal(998d, segments[0].Robots[0].Values[^1]);
        Assert.Equal(999d, segments[1].Robots[0].Values[0]);
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

    [Fact]
    public void ContestResultsFromSnapshotUsesAuthoritativeRustLocalModeState()
    {
        var snapshot = new EngineSnapshot(
            12,
            3,
            "CPU",
            [
                new OrganismSnapshot(0, 0, [0, 0], [0, 0], 100, 1, Species: 0),
                new OrganismSnapshot(1, 0, [1, 1], [0, 0], 100, 1, Species: 0),
                new OrganismSnapshot(2, 0, [2, 2], [0, 0], 100, 1, Species: 1),
            ])
        {
            Species =
            [
                new SpeciesSnapshot("Alpha", false, 0xff0000, 0, false),
                new SpeciesSnapshot("Beta", false, 0x00ff00, 0, false),
            ],
            LocalMode = new LocalModeSnapshot(
                LocalModeStatus.Completed,
                LocalModeKind.Contest,
                LeaguePreset.F1,
                2,
                3,
                12,
                100,
                0,
                [
                    new LocalModeResult(1, 0, 2, 200, "manual", false),
                    new LocalModeResult(2, 0, 2, 200, "manual", false),
                ],
                new LocalModeAdjudication(0, "manual", false),
                new LocalModeRestrictions()),
        };

        var result = ContestResultSnapshot.FromSnapshot(snapshot);

        Assert.Equal(2, result.Round);
        Assert.Equal(1, result.MinimumRounds);
        Assert.Equal(3, result.MaximumRounds);
        Assert.Equal(
            [("Alpha", 2, 2), ("Beta", 0, 1)],
            result.Competitors.Select(value => (value.Name, value.Wins, value.Population)));

        var idle = snapshot with { LocalMode = LocalModeSnapshot.Idle };
        Assert.Equal(ContestResultSnapshot.Empty, ContestResultSnapshot.FromSnapshot(idle));
    }
}
