using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class NativeDisqualificationTests
{
    [Fact]
    public void NativeReasonKeepsEliminatedCompetitorInResultsAndExcludesPlants()
    {
        const string json = """
            { "tick": 20, "organisms": [],
              "species": [{ "name": "a.txt" }, { "name": "b.txt" }, { "name": "Alga.txt", "vegetable": true }],
              "local_mode": {
                "status": "Running", "current_round": 1, "cycle_in_round": 20,
                "config": { "kind": "League", "preset": "F3", "minimum_rounds": 5,
                  "maximum_rounds": 0, "maximum_cycles": 15000, "maximum_population": 700 },
                "results": [{ "round": 1, "winner": 1, "winning_population": 5, "reason": "winner" }],
                "disqualifications": [{ "tick": 19, "round": 1, "species": 0,
                  "file_name": "a.txt", "tag": "entrant a", "reason": "making shell" }]
              }
            }
            """;
        var snapshot = NativeSnapshotParser.Parse(json, "CPU");
        var results = ContestResultSnapshot.FromSnapshot(snapshot);
        Assert.Equal(2, results.Competitors.Count);
        var eliminated = Assert.Single(results.Competitors.Where(value => value.IsDisqualified));
        Assert.Equal("a.txt", eliminated.Name);
        Assert.Equal("making shell", eliminated.DisqualificationReason);
        var model = new ContestCompetitorViewModel(eliminated);
        Assert.False(model.CanAdjudicate);
        Assert.Contains("making shell", model.Status);
    }

    [Fact]
    public void CorpseFlagSurvivesNativeSnapshotParsing()
    {
        const string json = """
            { "tick": 1, "organisms": [{ "id": { "slot": 0, "generation": 1 },
              "position": [500,500], "velocity": [0,0], "flags": { "corpse": true } }] }
            """;
        Assert.True(Assert.Single(NativeSnapshotParser.Parse(json, "CPU").Organisms).IsCorpse);
    }
}
