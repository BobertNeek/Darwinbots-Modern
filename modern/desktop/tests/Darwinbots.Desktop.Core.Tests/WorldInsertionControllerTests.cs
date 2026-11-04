using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class WorldInsertionControllerTests
{
    [Fact]
    public async Task SelectedSpeciesPlacementReachesTheEngineAtTheViewportWorldPosition()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);
        var insertion = new WorldInsertionController(session);
        var selectedSpecies = new SpeciesImport(
            "Selected Hunter",
            "start\nstop",
            false,
            0xffd07a2d,
            7,
            2_500,
            4,
            true,
            0.5f);

        insertion.SelectSpecies(selectedSpecies);
        insertion.Begin();
        await insertion.PlaceAsync([4_321f, 1_234f]);

        Assert.Equal(selectedSpecies, engine.LastInsertedSpecies);
        Assert.Equal([4_321f, 1_234f], engine.LastInsertedPosition!);
        Assert.Equal(1, session.LatestSnapshot.Population);
        Assert.True(insertion.IsActive);

        insertion.Cancel();
        Assert.False(insertion.IsActive);
    }
}
