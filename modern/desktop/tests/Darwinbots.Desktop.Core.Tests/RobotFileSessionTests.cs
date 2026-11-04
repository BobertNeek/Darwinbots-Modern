using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class RobotFileSessionTests
{
    [Fact]
    public async Task DnaAndProfileAreCapturedBeforeALaterQueuedProfileEdit()
    {
        var before = MutationProfileUpdate.Default with { Point = new(123, 7, 2) };
        var after = before with { CopyErrorValueChangePercent = 17 };
        using var exporting = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        var engine = new FakeEngineClient
        {
            SnapshotOrganisms = [new(3, 2, [0, 0], [0, 0], 1000, 0) { MutationProfile = before }],
            BeforeDnaExport = () =>
            {
                exporting.Set();
                if (!release.Wait(TimeSpan.FromSeconds(5))) throw new TimeoutException("Test did not release DNA export.");
            },
        };
        await using var session = new SimulationSession(engine);
        var capture = session.ExportRobotFileAsync(3, 2);
        try
        {
            Assert.True(await Task.Run(() => exporting.Wait(TimeSpan.FromSeconds(5))));
            var edit = session.UpdateOrganismMutationProfileAsync(3, 2, after);
            release.Set();
            var file = await capture;
            await edit;
            Assert.Equal("start\nstop", file.Dna);
            Assert.Equal(before, file.MutationProfile);
            Assert.Equal(after, Assert.Single(session.LatestSnapshot.Organisms).MutationProfile);
            Assert.Equal(0ul, session.LatestSnapshot.Tick);
            Assert.Single(engine.ExecutionThreads);
        }
        finally { release.Set(); }
    }

    [Fact]
    public async Task MissingSelectedIdentityDoesNotReturnDnaWithAnotherRobotsProfile()
    {
        var engine = new FakeEngineClient
        {
            SnapshotOrganisms = [new(3, 3, [0, 0], [0, 0], 1000, 0)],
        };
        await using var session = new SimulationSession(engine);
        await Assert.ThrowsAsync<InvalidDataException>(() => session.ExportRobotFileAsync(3, 2));
    }
}
