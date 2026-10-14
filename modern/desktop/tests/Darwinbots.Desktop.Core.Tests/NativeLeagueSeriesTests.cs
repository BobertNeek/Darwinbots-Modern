using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class NativeLeagueSeriesTests
{
    [Fact]
    public void NativeScheduleKeepsPairRankingsAndConfiguration()
    {
        const string json = """
            { "tick": 40, "organisms": [], "species": [], "local_mode": {
              "status": "running", "current_round": 4, "cycle_in_round": 7,
              "config": { "kind": "league", "workflow": "ladder", "preset": "f1",
                "entrant_order": [4,2,1], "initial_ladder": [4], "seeding_opponent": 3,
                "minimum_rounds": 5, "maximum_rounds": 1, "maximum_cycles": 15000 },
              "series": { "workflow": "ladder", "active_match": [4,1], "match_number": 3,
                "ladder": [4,2], "seeded": [], "matches": [
                  { "number": 1, "entrants": [4,2], "winner": 4, "first_result": 0, "result_count": 2 } ] }
            } }
            """;
        var mode = NativeSnapshotParser.Parse(json, "CPU").LocalMode;
        Assert.Equal(LeagueWorkflow.Ladder, mode.Configuration!.Workflow);
        Assert.Equal(new uint[] { 4, 2, 1 }, mode.Configuration.EntrantOrder);
        Assert.Equal(new uint[] { 4 }, mode.Configuration.InitialLadder);
        Assert.Equal(3U, mode.Configuration.SeedingOpponent);
        Assert.Equal(new uint[] { 4, 1 }, mode.Series!.ActiveMatch);
        Assert.Equal(new uint[] { 4, 2 }, mode.Series.Ladder);
        Assert.Equal(2, Assert.Single(mode.Series.Matches).ResultCount);
    }

    [Fact]
    public void NativeZerobotModeDoesNotFallBackToLeague()
    {
        const string json = """{ "local_mode": { "config": { "kind": "zerobot" } } }""";
        Assert.Equal(LocalModeKind.Zerobot, NativeSnapshotParser.Parse(json, "CPU").LocalMode.Kind);
    }

    [Fact]
    public void ContestResultsUseCurrentPairAndExcludePreviousMatchWins()
    {
        const string json = """
            { "organisms": [], "species": [{"name":"a"},{"name":"b"},{"name":"c"}],
              "local_mode": { "status":"running", "current_round":3,
                "config": { "kind":"league", "workflow":"tournament" },
                "series": { "workflow":"tournament", "active_match":[1,2], "match_start_result":1 },
                "results": [ {"round":1,"winner":0,"reason":"old"}, {"round":2,"winner":1,"reason":"current"} ]
              } }
            """;
        var results = ContestResultSnapshot.FromSnapshot(NativeSnapshotParser.Parse(json, "CPU"));
        var competitor = Assert.Single(results.Competitors);
        Assert.Equal("b", competitor.Name);
        Assert.Equal(1, competitor.Wins);
        Assert.True(competitor.IsActive);
    }
}
