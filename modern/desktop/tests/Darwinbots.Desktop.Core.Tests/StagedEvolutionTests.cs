using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class StagedEvolutionTests
{
    [Fact]
    public void DecimalReductionUnitAndOptInSurviveSettingsRoundtrip()
    {
        var settings = new GlobalSettings { Evolution = new GlobalEvolutionSettings {
            Enabled = true, Mode = EvolutionMode.Eco, StageLength = 0.25, CycleLength = 150,
        }};
        var decoded = GlobalSettings.FromJson(settings.ToJson());
        Assert.True(decoded.Evolution.Enabled);
        Assert.Equal(0.25, decoded.Evolution.StageLength);
        Assert.Equal(EvolutionMode.Eco, decoded.Evolution.Mode);
    }

    [Fact]
    public void NativeStageStateAndHiddenBaselineAreParsed()
    {
        const string json = """
            { "organisms":[{"id":{"slot":0,"generation":1},"position":[500,500],"velocity":[0,0],
                "flags":{"hidden":true}}], "local_mode": {
                "status":"running", "config": {"kind":"eco_survival", "staged_evolution": {
                    "mode":"eco", "source_dna":"start stop", "reduction_unit":0.25, "cycle_length":150,
                    "zerobot_initial_dna_length":32000, "auto_graphs":true }},
                "evolution":{"stage":4,"phase":"testing","hidden":false,"reduction_unit":0.125,
                    "hide_cycles":250,"test_wins":2,"opponent_stage":1,"handicap":-3,"inverse_handicap":-1}
            }}
            """;
        var snapshot = NativeSnapshotParser.Parse(json, "CPU");
        Assert.True(Assert.Single(snapshot.Organisms).IsHidden);
        Assert.Equal(4U, snapshot.LocalMode.Evolution!.Stage);
        Assert.Equal("testing", snapshot.LocalMode.Evolution.Phase);
        Assert.Equal(-2.0, snapshot.LocalMode.Evolution.Handicap);
        Assert.Equal(0.25f, snapshot.LocalMode.Configuration!.StagedEvolution!.ReductionUnit);
        Assert.True(snapshot.LocalMode.Configuration.StagedEvolution.AutoGraphs);
    }
}
