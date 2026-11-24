using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class NativeGraphFeedTests
{
    [Fact]
    public async Task ExplicitFeedHonorsRequestedMetricsAndNativeBoundariesOnTheSessionWorker()
    {
        await using var session = new SimulationSession(new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu",
            PopulationCapacity = 64,
            Recording = new Db2RecordingOptions(SamplingInterval: 5),
        }));
        await session.ImportDnaAsync("start 1 .up store stop", [500f, 500f]);
        await session.ImportDnaAsync("start 2 .up store stop", [750f, 750f]);

        var initial = session.LatestSnapshot;
        var settings = new GraphSamplingSettings(Slow: true);
        Assert.Equal(-1, initial.NativeCycle);
        Assert.Null(await session.FeedNativeGraphSampleAsync(settings, dueOnly: true));

        var immediate = Assert.IsType<NativeGraphRecordingSample>(await session.FeedNativeGraphSampleAsync(settings));
        Assert.Equal(-1, immediate.NativeCycle);
        Assert.Equal(initial.Tick, immediate.Tick);
        Assert.True(Assert.Single(immediate.SpeciesGraphs).Values["slow_genetic_distance"] > 0);
        Assert.Same(initial, session.LatestSnapshot);

        var read = Assert.IsType<NativeGraphRecordingSample>(await session.ReadNativeGraphSampleAsync());
        Assert.Equal(0.0, Assert.Single(read.SpeciesGraphs).Values["slow_genetic_distance"]);

        await session.StepAsync();
        var due = Assert.IsType<NativeGraphRecordingSample>(await session.FeedNativeGraphSampleAsync(settings, dueOnly: true));
        Assert.Equal(0, due.NativeCycle);
        Assert.Equal(1UL, due.Tick);
    }
}
