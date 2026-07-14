using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class NativeSnapshotParserTests
{
    [Fact]
    public void ParsesStableIdsVectorsAndPopulationFromRustSnapshot()
    {
        const string json = """
            {
              "tick": 91,
              "organisms": [
                {
                  "id": { "slot": 7, "generation": 3 },
                  "position": [12.5, 44.25],
                  "velocity": [1.0, -2.0],
                  "energy": 875,
                  "age": 19
                }
              ]
            }
            """;

        var snapshot = NativeSnapshotParser.Parse(json, "GPU");

        Assert.Equal(91UL, snapshot.Tick);
        Assert.Equal(1, snapshot.Population);
        Assert.Equal("GPU", snapshot.Backend);
        Assert.Equal(7U, snapshot.Organisms[0].Slot);
        Assert.Equal(3U, snapshot.Organisms[0].Generation);
        Assert.Equal(12.5f, snapshot.Organisms[0].Position[0]);
    }

    [Fact]
    public void ParsesEngineGeneratedRenderInstances()
    {
        const string json = """
            {
              "tick": 5,
              "organisms": [],
              "render_instances": [
                { "slot": 9, "position": [100.0, 200.0], "radius": 4.5, "color": 4281339936 }
              ]
            }
            """;

        var snapshot = NativeSnapshotParser.Parse(json, "GPU");

        Assert.Single(snapshot.RenderInstances);
        Assert.Equal(9U, snapshot.RenderInstances[0].Slot);
        Assert.Equal(4.5f, snapshot.RenderInstances[0].Radius);
        Assert.Equal(200f, snapshot.RenderInstances[0].Position[1]);
    }

    [Fact]
    public void ParsesSpeciesEcologyAndLiveStatistics()
    {
        const string json = """
            {
              "tick": 12,
              "organisms": [
                {
                  "id": { "slot": 1, "generation": 0 },
                  "position": [10.0, 20.0],
                  "velocity": [0.0, 0.0],
                  "energy": 1300,
                  "age": 12,
                  "species": 2,
                  "vegetable": true,
                  "body": 140,
                  "waste": 6,
                  "shell": 20,
                  "slime": 3,
                  "venom": 5,
                  "poison": 7,
                  "chloroplasts": 160,
                  "aim": 314,
                  "parents": [null, null]
                }
              ],
              "species": [
                { "name": "Unassigned", "vegetable": false, "color": 4284653636, "minimum_population": 0, "reseed": false },
                { "name": "Animal", "vegetable": false, "color": 4280494784, "minimum_population": 0, "reseed": false },
                { "name": "Alga", "vegetable": true, "color": 4283213371, "minimum_population": 20, "reseed": true }
              ],
              "stats": {
                "population": 1, "births": 4, "deaths": 2, "shots_fired": 8,
                "energy_harvested": 75, "mutations": 3, "ties_created": 1, "total_energy": 1300
              }
            }
            """;

        var snapshot = NativeSnapshotParser.Parse(json, "CPU");

        Assert.Equal(2U, snapshot.Organisms[0].Species);
        Assert.True(snapshot.Organisms[0].Vegetable);
        Assert.Equal(140, snapshot.Organisms[0].Body);
        Assert.Equal(160, snapshot.Organisms[0].Chloroplasts);
        Assert.Equal(314, snapshot.Organisms[0].Aim);
        Assert.Equal("Alga", snapshot.Species[2].Name);
        Assert.Equal(4UL, snapshot.Stats.Births);
        Assert.Equal(75UL, snapshot.Stats.EnergyHarvested);
        Assert.Equal(1300L, snapshot.Stats.TotalEnergy);
    }

    [Fact]
    public void ParsesCorpsesForWorldRendering()
    {
        const string json = """
            {
              "tick": 22,
              "organisms": [],
              "corpses": [
                { "position": [120.0, 240.0], "velocity": [1.0, 2.0], "energy": 80, "body": 70, "age": 15 }
              ]
            }
            """;

        var snapshot = NativeSnapshotParser.Parse(json, "CPU");

        Assert.Single(snapshot.Corpses);
        Assert.Equal(80, snapshot.Corpses[0].Energy);
        Assert.Equal(240f, snapshot.Corpses[0].Position[1]);
        Assert.Equal(15UL, snapshot.Corpses[0].Age);
    }

    [Fact]
    public void ParsesShotTrailsForWorldRendering()
    {
        const string json = """
            {
              "tick": 23,
              "organisms": [],
              "shots": [
                { "owner": { "slot": 4, "generation": 2 }, "start": [10.0, 20.0], "end": [30.0, 40.0], "kind": -1, "value": 25 }
              ]
            }
            """;

        var snapshot = NativeSnapshotParser.Parse(json, "CPU");

        Assert.Single(snapshot.Shots);
        Assert.Equal(new OrganismKey(4, 2), snapshot.Shots[0].Owner);
        Assert.Equal(-1, snapshot.Shots[0].Kind);
        Assert.Equal(40f, snapshot.Shots[0].End[1]);
    }

    [Fact]
    public void ParsesProjectileVelocityAgeRangeImpactAndDb2Telemetry()
    {
        const string json = """
            {
              "tick": 24,
              "organisms": [],
              "shots": [
                {
                  "owner": { "slot": 4, "generation": 2 },
                  "start": [10.0, 20.0],
                  "end": [30.0, 20.0],
                  "velocity": [40.0, 0.0],
                  "age": 2,
                  "range": 7,
                  "energy": 80.5,
                  "kind": -1,
                  "value": 25,
                  "impact_flash": false
                }
              ],
              "stats": {
                "births": 0,
                "deaths": 0,
                "shots_fired": 8,
                "projectile_impacts": 3,
                "energy_harvested": 0,
                "mutations": 0,
                "ties_created": 0,
                "total_energy": 0,
                "plant_energy_generated": 99
              }
            }
            """;

        var snapshot = NativeSnapshotParser.Parse(json, "CPU");
        var shot = Assert.Single(snapshot.Shots);

        Assert.Equal([40f, 0f], shot.Velocity);
        Assert.Equal(2U, shot.Age);
        Assert.Equal(7U, shot.Range);
        Assert.Equal(80.5f, shot.Energy);
        Assert.False(shot.ImpactFlash);
        Assert.Equal(3UL, snapshot.Stats.ProjectileImpacts);
        Assert.Equal(99UL, snapshot.Stats.PlantEnergyGenerated);
    }

    [Fact]
    public void ParsesHistoricalPopulationAndEnergyRecords()
    {
        const string json = """
            {
              "tick": 200,
              "organisms": [],
              "history": [
                { "tick": 100, "population": 42, "total_energy": 9000, "births": 5, "deaths": 2, "mutations": 3, "shots_fired": 7 },
                { "tick": 200, "population": 48, "total_energy": 9700, "births": 9, "deaths": 3, "mutations": 4, "shots_fired": 11 }
              ]
            }
            """;

        var snapshot = NativeSnapshotParser.Parse(json, "CPU");

        Assert.Equal(2, snapshot.History.Count);
        Assert.Equal(48, snapshot.History[1].Population);
        Assert.Equal(9700L, snapshot.History[1].TotalEnergy);
        Assert.Equal(11UL, snapshot.History[1].ShotsFired);
    }

    [Fact]
    public void ParsesStableTieLinksForWorldRendering()
    {
        const string json = """
            {
              "tick": 10,
              "organisms": [],
              "ties": [
                { "first": { "slot": 2, "generation": 1 }, "second": { "slot": 5, "generation": 3 }, "rest_length": 42.5 }
              ]
            }
            """;

        var snapshot = NativeSnapshotParser.Parse(json, "CPU");

        Assert.Single(snapshot.Ties);
        Assert.Equal(new OrganismKey(2, 1), snapshot.Ties[0].First);
        Assert.Equal(new OrganismKey(5, 3), snapshot.Ties[0].Second);
        Assert.Equal(42.5f, snapshot.Ties[0].RestLength);
    }
}
