using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class LegacyGraphAutoSaveTests
{
    [Theory]
    [InlineData(12, GraphMetric.SelectiveGeneticDistance, "Genetic_Distance_x1000-")]
    [InlineData(13, GraphMetric.GenerationalDistance, "Max_Generational_Distance")]
    [InlineData(14, GraphMetric.SlowGeneticDistance, "Simple_Genetic_Distance_x1000-")]
    public void OriginalGeneticGraphsUseTheirOwnRecordRowsAndSegmentNames(int index, GraphMetric metric, string caption)
    {
        var root = Directory.CreateTempSubdirectory("darwinbots-genetic-graph-save-");
        try
        {
            const string stamp = "original-run";
            const int previousCounter = 4;
            var run = new LegacyGraphRecordingRun(root.FullName, stamp);
            run.RestoreNativeState(new(stamp, [new(index, true, previousCounter)]));
            Assert.Equal(metric, LegacyGraphRecordingRun.NativeMetricAt(index));
            var graph = run.Graph(metric);
            Assert.True(graph.Enabled);
            Assert.Equal(previousCounter, graph.Counter);
            var updates = new List<NativeGraphRecordingRow>();
            run.NativeSettingsChanged += (_, row) => updates.Add(row);

            for (var value = 1; value <= 999; value++)
                graph.AddPoint([new("Alga.txt", 0xff11_22_33, value)], "normal");

            Assert.Equal(new NativeGraphRecordingRow(index, true, 5), Assert.Single(updates));
            Assert.Equal(Path.Combine(root.FullName, stamp, caption + "5.gsave"),
                Assert.Single(root.EnumerateFiles("*.gsave", SearchOption.AllDirectories)).FullName);
        }
        finally { root.Delete(recursive: true); }
    }

    [Fact]
    public void OriginalGraphRetriesTheSameNumberAndKeepsTheSegmentThroughReset()
    {
        var root = Directory.CreateTempSubdirectory("darwinbots-graph-save-");
        try
        {
            // A file temporarily blocks creation of the run directory.
            const string stamp = "run";
            var runFolder = Path.Combine(root.FullName, stamp);
            File.WriteAllText(runFolder, "the original obstacle");
            var run = new LegacyGraphRecordingRun(root.FullName, stamp);
            var graph = run.Graph(GraphMetric.Population);
            var firstPath = Path.Combine(runFolder, "Populations1.gsave");
            var secondPath = Path.Combine(runFolder, "Populations2.gsave");

            for (var value = 1; value <= 998; value++) Add(value);
            graph.SetEnabled(true);
            Add(999);
            Assert.Equal(0, graph.Counter);
            Assert.Contains("WAITING", graph.Status);
            Assert.Equal("the original obstacle", File.ReadAllText(runFolder));

            graph.ResetPoints();
            File.Delete(runFolder);
            Add(10_001); // Retry the immutable first segment despite clearing the graph's history.
            Assert.Equal(1, graph.Counter);
            Assert.True(graph.Enabled);
            var first = Read(firstPath);
            Assert.Equal(Enumerable.Range(1, 999).Select(value => (double)value), first);
            Assert.False(File.Exists(secondPath));

            for (var offset = 2; offset <= 999; offset++) Add(10_000 + offset);
            Assert.Equal(2, graph.Counter);
            Assert.Equal(Enumerable.Range(10_001, 999).Select(value => (double)value), Read(secondPath));
            Assert.Equal(first, Read(firstPath));

            void Add(int value) => graph.AddPoint([new("Alga.txt", 0xff11_22_33, value)], "normal");
        }
        finally { root.Delete(recursive: true); }
    }

    [Fact]
    public void RestoredGraphRecognizesAnIdenticalExistingSegmentWithoutRewritingIt()
    {
        var root = Directory.CreateTempSubdirectory("darwinbots-graph-restore-");
        try
        {
            const string stamp = "original-run";
            var folder = root.CreateSubdirectory(stamp);
            var firstPath = Path.Combine(folder.FullName, "Populations1.gsave");
            var values = Enumerable.Range(1, 999).Select(value => (double)value).ToArray();
            var segment = LegacyDnaText.Encode(LegacyGraphJoin.SerializeNativeSegment("normal",
                [new("Alga.txt", 0xff11_22_33, values)]));
            File.WriteAllBytes(firstPath, segment);
            File.SetLastWriteTimeUtc(firstPath, new DateTime(2001, 1, 1, 0, 0, 0, DateTimeKind.Utc));
            var originalWrite = File.GetLastWriteTimeUtc(firstPath);

            var run = new LegacyGraphRecordingRun(root.FullName, "fresh-run");
            var graph = run.Graph(GraphMetric.Population);
            graph.AddPoint([new("Old graph species", 0xffaa_bb_cc, 70_000)], "normal");
            run.RestoreNativeState(new(stamp, [new(0, true, 0), new(4, false, 17)]));
            Assert.Equal(stamp, run.RunStamp);
            Assert.Same(graph, run.Graph(GraphMetric.Population));
            Assert.True(graph.Enabled);
            Assert.Equal(0, graph.Counter);
            var energy = run.Graph(GraphMetric.AverageEnergy);
            Assert.False(energy.Enabled);
            Assert.Equal(17, energy.Counter);

            AddPopulationPoints(graph, 1, 998);
            Assert.Equal(0, graph.Counter);
            AddPopulationPoints(graph, 999, 1);
            Assert.Equal(1, graph.Counter);
            Assert.Equal(segment, File.ReadAllBytes(firstPath));
            Assert.Equal(originalWrite, File.GetLastWriteTimeUtc(firstPath));
            Assert.Equal(firstPath, Assert.Single(Directory.GetFiles(folder.FullName, "*.gsave")));
        }
        finally { root.Delete(recursive: true); }
    }

    [Fact]
    public void RestoredGraphKeepsADivergentSegmentAndCanRetryItsOriginalNumber()
    {
        var root = Directory.CreateTempSubdirectory("darwinbots-graph-collision-");
        try
        {
            const string stamp = "restored-run";
            var folder = root.CreateSubdirectory(stamp);
            var firstPath = Path.Combine(folder.FullName, "Populations1.gsave");
            var unrelated = LegacyDnaText.Encode("existing data from another graph");
            File.WriteAllBytes(firstPath, unrelated);

            var run = new LegacyGraphRecordingRun(root.FullName, "fresh-run");
            run.RestoreNativeState(new(stamp, [new(0, true, 0)]));
            var graph = run.Graph(GraphMetric.Population);
            AddPopulationPoints(graph, 1, 999);

            Assert.Equal(0, graph.Counter);
            Assert.Contains("COLLISION", graph.Status, StringComparison.Ordinal);
            Assert.Equal(unrelated, File.ReadAllBytes(firstPath));
            Assert.Equal(firstPath, Assert.Single(Directory.GetFiles(folder.FullName, "*.gsave")));

            File.Delete(firstPath);
            AddPopulationPoints(graph, 1_000, 1);
            Assert.Equal(1, graph.Counter);
            Assert.Equal(Enumerable.Range(1, 999).Select(value => (double)value), Read(firstPath));
            Assert.Equal(firstPath, Assert.Single(Directory.GetFiles(folder.FullName, "*.gsave")));
        }
        finally { root.Delete(recursive: true); }
    }

    private static void AddPopulationPoints(LegacyGraphAutoSave graph, int first, int count)
    {
        foreach (var value in Enumerable.Range(first, count))
            graph.AddPoint([new("Alga.txt", 0xff11_22_33, value)], "normal");
    }

    private static IReadOnlyList<double> Read(string path) =>
        Assert.Single(Assert.Single(LegacyGraphJoin.Parse([LegacyDnaText.Decode(File.ReadAllBytes(path))])).Robots).Values;
}
