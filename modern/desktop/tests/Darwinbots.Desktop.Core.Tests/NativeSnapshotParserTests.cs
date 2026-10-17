using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class NativeSnapshotParserTests
{
    [Fact]
    public void NativeEnergyStatusDistinguishesMissingNullZeroAndSignedValues()
    {
        foreach (var json in new[]
        {
            "{\"tick\":1,\"organisms\":[]}",
            "{\"tick\":1,\"organisms\":[],\"stats\":{\"total_energy\":99}}",
            "{\"tick\":1,\"organisms\":[],\"stats\":{\"native_energy_displayed\":null,\"native_energy_delta\":null}}",
        })
        {
            var stats = NativeSnapshotParser.Parse(json, "CPU").Stats;
            Assert.Null(stats.NativeEnergyDisplayed);
            Assert.Null(stats.NativeEnergyDelta);
        }
        foreach (var value in new[] { 0, -123, 456 })
        {
            var json = System.Text.Json.JsonSerializer.Serialize(new
            {
                tick = 1, organisms = Array.Empty<object>(),
                stats = new { total_energy = 99, native_energy_displayed = value, native_energy_delta = (long)value },
            });
            var stats = NativeSnapshotParser.Parse(json, "CPU").Stats;
            Assert.Equal<int?>(value, stats.NativeEnergyDisplayed);
            Assert.Equal<long?>(value, stats.NativeEnergyDelta);
            Assert.Equal(99L, stats.TotalEnergy);
        }
    }

    [Fact]
    public void ProcessCachesRoundTripTheirSourceShapeWithoutEnteringSavedSetup()
    {
        var ring = Enumerable.Range(0, 101).Select(value => value * 7).ToArray();
        var caches = new NativeProcessCachesSnapshot(0.125, new(ring, 99, -123), 17, false);
        var json = System.Text.Json.JsonSerializer.Serialize(caches);
        Assert.Contains("\"light_availability\":0.125", json);
        Assert.Contains("\"current_index\":99", json);
        Assert.Contains("\"shots_this_cycle\":17", json);
        Assert.Contains("\"death_poffs_enabled\":false", json);
        var restored = System.Text.Json.JsonSerializer.Deserialize<NativeProcessCachesSnapshot>(json)!;
        Assert.Equal(caches.LightAvailability, restored.LightAvailability);
        Assert.Equal(ring, restored.Energy.Ring);
        Assert.Equal((byte)99, restored.Energy.CurrentIndex);
        Assert.Equal(-123, restored.Energy.Displayed);
        Assert.Equal(17UL, restored.ShotsThisCycle);
        Assert.False(restored.DeathPoffsEnabled);
        var oldJson = System.Text.Json.JsonSerializer.Serialize(new { light_availability = 0.125, energy = caches.Energy });
        var oldCaches = System.Text.Json.JsonSerializer.Deserialize<NativeProcessCachesSnapshot>(oldJson)!;
        Assert.Equal(0UL, oldCaches.ShotsThisCycle);
        Assert.True(oldCaches.DeathPoffsEnabled);
        Assert.True(NativeProcessCachesSnapshot.Empty.DeathPoffsEnabled);

        var setup = new WorldSetupOptions { Seed = 42, NativeProcessCaches = caches };
        var savedSetup = System.Text.Json.JsonSerializer.Serialize(setup);
        Assert.DoesNotContain("NativeProcessCaches", savedSetup, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("light_availability", savedSetup, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("death_poffs_enabled", savedSetup, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("native_energy", NativeCommandSerializer.SerializeEngineConfig(setup), StringComparison.OrdinalIgnoreCase);
        var loadedSetup = System.Text.Json.JsonSerializer.Deserialize<WorldSetupOptions>(savedSetup)!;
        Assert.Null(loadedSetup.NativeProcessCaches);
        Assert.Equal(42UL, loadedSetup.Seed);
    }

    [Fact]
    public void DeathParticlePolicyAndShotArgbPreserveOldPayloadsAndExplicitZero()
    {
        Assert.True(EngineSnapshot.Empty.DeathPoffsEnabled);
        foreach (var policy in new[] { "", ",\"death_poffs_enabled\":null", ",\"death_poffs_enabled\":true" })
            Assert.True(NativeSnapshotParser.Parse("{\"tick\":1,\"organisms\":[]" + policy + "}", "CPU").DeathPoffsEnabled);
        Assert.False(NativeSnapshotParser.Parse("{\"tick\":1,\"organisms\":[],\"death_poffs_enabled\":false}", "CPU").DeathPoffsEnabled);

        // Native SIM import converts original COLORREF red255/blue16711680/black0
        // to these existing opaque ARGB snapshot values before managed transport.
        foreach (var (member, expected) in new (string, uint?)[]
        {
            ("", null), (",\"color\":null", null), (",\"color\":0", 0),
            (",\"color\":4294901760", 0xffff0000),
            (",\"color\":4278190335", 0xff0000ff),
            (",\"color\":4278190080", 0xff000000),
        })
        {
            var json = "{\"tick\":1,\"organisms\":[],\"shots\":[{\"owner\":{\"slot\":4,\"generation\":2},\"start\":[1,2],\"end\":[3,4],\"kind\":-100" + member + "}]}";
            Assert.Equal(expected, Assert.Single(NativeSnapshotParser.Parse(json, "CPU").Shots).Color);
        }
        Assert.Null(new ShotSnapshot(new(4, 2), [1, 2], [3, 4], [0, 0], 42, 0, 0, 10, 1, false).Color);
    }

    [Fact]
    public void MissingAndNullNativeShotCountsRemainAbsentForOlderSnapshots()
    {
        string[] payloads =
        [
            "{\"tick\":1,\"organisms\":[]}",
            "{\"tick\":1,\"organisms\":[],\"stats\":null}",
            "{\"tick\":1,\"organisms\":[],\"stats\":{\"shots_fired\":9}}",
            "{\"tick\":1,\"organisms\":[],\"stats\":{\"shots_fired\":9,\"native_shots_visited\":null}}",
        ];
        foreach (var json in payloads)
            Assert.Null(NativeSnapshotParser.Parse(json, "CPU").Stats.NativeShotsVisited);

        Assert.Null(SimulationStatsSnapshot.Empty.NativeShotsVisited);
        Assert.Null(new SimulationStatsSnapshot(0, 0, 9, 0, 0, 0, 0).NativeShotsVisited);
    }

    [Fact]
    public void NativeShotVisitsIncludingZeroRemainDistinctFromFiringAndVisibleTrails()
    {
        foreach (var visited in new[] { 0UL, 17UL })
        {
            var json = System.Text.Json.JsonSerializer.Serialize(new
            {
                tick = 24,
                organisms = Array.Empty<object>(),
                stats = new { shots_fired = 99, native_shots_visited = visited },
                shots = new[]
                {
                    new { owner = new { slot = 4, generation = 2 }, start = new[] { 10f, 20f },
                        end = new[] { 30f, 20f }, kind = -1, value = 25 },
                    new { owner = new { slot = 5, generation = 2 }, start = new[] { 40f, 20f },
                        end = new[] { 60f, 20f }, kind = -2, value = 80 },
                },
            });

            var snapshot = NativeSnapshotParser.Parse(json, "CPU");

            Assert.Equal<ulong?>(visited, snapshot.Stats.NativeShotsVisited);
            Assert.Equal(99UL, snapshot.Stats.ShotsFired);
            Assert.Equal(2, snapshot.Shots.Count);
        }
    }

    [Theory]
    [InlineData(-1)]
    [InlineData(0)]
    [InlineData(int.MinValue)]
    [InlineData(int.MaxValue)]
    public void SignedNativeCycleDoesNotReplaceTheUnsignedSchedulingOrHistoryTick(int cycle)
    {
        var json = System.Text.Json.JsonSerializer.Serialize(new
        {
            tick = ulong.MaxValue,
            native_cycle = cycle,
            organisms = Array.Empty<object>(),
            history = new[] { new { tick = ulong.MaxValue, population = 0 } },
        });

        var snapshot = NativeSnapshotParser.Parse(json, "CPU");

        Assert.Equal(cycle, snapshot.NativeCycle);
        Assert.Equal(ulong.MaxValue, snapshot.Tick);
        Assert.Equal(ulong.MaxValue, Assert.Single(snapshot.History).Tick);
    }

    [Theory]
    [InlineData("{\"tick\":91,\"organisms\":[]}")]
    [InlineData("{\"tick\":91,\"native_cycle\":null,\"organisms\":[]}")]
    public void MissingAndNullNativeCycleRetainTheExistingSnapshotContract(string json)
    {
        var snapshot = NativeSnapshotParser.Parse(json, "CPU");
        Assert.Null(snapshot.NativeCycle);
        Assert.Equal(91ul, snapshot.Tick);
    }

    [Fact]
    public void RetainsNativeSpeciesGraphSamplesAndSixCostSeries()
    {
        const string json = """
            { "tick": 100, "organisms": [], "history": [{ "tick": 100, "population": 3,
              "species_graphs": [{ "name": "Alga.txt", "color": 4278255360,
                 "values": { "population": 3, "average_age": 2.5, "slow_genetic_distance": 123.5 } }],
              "dynamic_cost_series": [0.5, 1.2, 1.1, 0.9, 0.2, 0.4] }] }
            """;
        var history = NativeSnapshotParser.Parse(json, "CPU").History[0];
        Assert.Equal("Alga.txt", history.SpeciesGraphs[0].Name);
        Assert.Equal(123.5, history.SpeciesGraphs[0].Values["slow_genetic_distance"]);
        Assert.Equal([0.5, 1.2, 1.1, 0.9, 0.2, 0.4], history.DynamicCostSeries);
    }

    [Fact]
    public void RetainsCompleteLocalModeConfigurationForLoadedEditors()
    {
        const string json = """
            {
              "tick": 17, "organisms": [],
              "local_mode": {
                "status": "Configured", "current_round": 0,
                "cycle_in_round": 0, "restart_count": 0, "results": [],
                "config": {
                  "kind": "Contest", "preset": "F2",
                  "minimum_rounds": 2, "maximum_rounds": 7,
                  "maximum_cycles": 123, "maximum_population": 37,
                  "restart_on_extinction": false, "energy_handicap": 25,
                  "source_species": 4, "selection_population_weight": 0.25,
                  "selection_energy_weight": 0.75
                }
              }
            }
            """;
        var mode = NativeSnapshotParser.Parse(json, "CPU").LocalMode;
        var configuration = Assert.IsType<LocalModeConfig>(mode.Configuration);
        Assert.Equal(37, configuration.MaximumPopulation);
        Assert.Equal(2u, configuration.MinimumRounds);
        Assert.False(configuration.RestartOnExtinction);
        Assert.Equal(4u, configuration.SourceSpecies);
        Assert.Equal(0.25f, configuration.SelectionPopulationWeight);
        Assert.Equal(0.75f, configuration.SelectionEnergyWeight);
    }

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
        Assert.Equal(4, snapshot.Organisms[0].Phenotype.Skin.Length);
        Assert.Equal(9, snapshot.Organisms[0].Vision.Eyes.Length);
        Assert.Equal(4, snapshot.Organisms[0].Vision.FocusEye);
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
        Assert.Equal(0U, snapshot.RenderInstances[0].Generation);
        Assert.Equal(0, snapshot.RenderInstances[0].Aim);
        Assert.Equal(0UL, snapshot.RenderInstances[0].LineageId);
        Assert.Equal(4, snapshot.RenderInstances[0].Skin.Length);
    }

    [Fact]
    public void ParsesPhenotypeSkinAndNineEyeVision()
    {
        const string json = """
            {
              "tick": 8,
              "organisms": [
                {
                  "id": { "slot": 3, "generation": 2 },
                  "position": [100.0, 200.0],
                  "velocity": [0.0, 0.0],
                  "energy": 900,
                  "age": 4,
                  "phenotype": {
                    "lineage_id": 77,
                    "color": 4280527552,
                    "skin": [
                      { "radius": 0.2, "angle": 0 },
                      { "radius": 0.4, "angle": 314 },
                      { "radius": 0.6, "angle": 628 },
                      { "radius": 0.8, "angle": 942 }
                    ],
                    "accumulated_mutations": 5
                  },
                  "vision": {
                    "focus_eye": 4,
                    "eyes": [
                      { "direction": 0, "width": 0, "center_radians": 0.7, "half_width_radians": 0.087, "range": 1440.0, "value": 1 },
                      { "direction": 0, "width": 0, "center_radians": 0.5, "half_width_radians": 0.087, "range": 1440.0, "value": 2 },
                      { "direction": 0, "width": 0, "center_radians": 0.3, "half_width_radians": 0.087, "range": 1440.0, "value": 3 },
                      { "direction": 0, "width": 0, "center_radians": 0.2, "half_width_radians": 0.087, "range": 1440.0, "value": 4 },
                      { "direction": 314, "width": 70, "center_radians": 1.57, "half_width_radians": 0.262, "range": 1044.0, "value": 32000 },
                      { "direction": 0, "width": 0, "center_radians": 6.1, "half_width_radians": 0.087, "range": 1440.0, "value": 6 },
                      { "direction": 0, "width": 0, "center_radians": 5.9, "half_width_radians": 0.087, "range": 1440.0, "value": 7 },
                      { "direction": 0, "width": 0, "center_radians": 5.7, "half_width_radians": 0.087, "range": 1440.0, "value": 8 },
                      { "direction": 0, "width": 0, "center_radians": 5.5, "half_width_radians": 0.087, "range": 1440.0, "value": 9 }
                    ]
                  }
                }
              ],
              "render_instances": [
                {
                  "slot": 3,
                  "generation": 2,
                  "position": [100.0, 200.0],
                  "radius": 8.0,
                  "color": 4280527552,
                  "aim": 314,
                  "skin": [
                    { "radius": 0.2, "angle": 0 },
                    { "radius": 0.4, "angle": 314 },
                    { "radius": 0.6, "angle": 628 },
                    { "radius": 0.8, "angle": 942 }
                  ],
                  "lineage_id": 77,
                  "vegetable": false
                }
              ]
            }
            """;

        var snapshot = NativeSnapshotParser.Parse(json, "CPU");

        Assert.Equal(77UL, snapshot.RenderInstances[0].LineageId);
        Assert.Equal(4, snapshot.RenderInstances[0].Skin.Length);
        Assert.Equal(314, snapshot.RenderInstances[0].Aim);
        Assert.Equal(9, snapshot.Organisms[0].Vision.Eyes.Length);
        Assert.Equal(4, snapshot.Organisms[0].Vision.FocusEye);
        Assert.Equal(32000, snapshot.Organisms[0].Vision.Eyes[4].Value);
        Assert.Equal(5U, snapshot.Organisms[0].Phenotype.AccumulatedMutations);
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

    [Fact]
    public void ParsesShapePoliciesAndOfflineTeleporterState()
    {
        const string json = """
            {
              "tick": 12,
              "organisms": [],
              "obstacles": [
                {
                  "id": 7,
                  "minimum": [10, 20],
                  "maximum": [30, 40],
                  "color": 4279312947,
                  "opacity": 0.35,
                  "drift": [2, -3],
                  "vision_policy": "see_through",
                  "shot_policy": "pass_through"
                }
              ],
              "file_teleporters": [
                {
                  "id": 9,
                  "center": [50, 60],
                  "radius": 15,
                  "inbound_path": "in",
                  "outbound_path": "out",
                  "poll_interval": 3,
                  "poll_countdown": 1,
                  "capacity": 4,
                  "filters": { "teleport_vegetables": false, "teleport_corpses": true, "teleport_heterotrophs": true },
                  "sent": 8,
                  "received": 5,
                  "backflow_guard": "out/db2-guard.dbo"
                }
              ]
            }
            """;

        var snapshot = NativeSnapshotParser.Parse(json, "CPU");

        Assert.Equal("see_through", snapshot.Obstacles[0].VisionPolicy);
        Assert.Equal("pass_through", snapshot.Obstacles[0].ShotPolicy);
        Assert.Equal([2f, -3f], snapshot.Obstacles[0].Drift);
        Assert.Equal(0.35f, snapshot.Obstacles[0].Opacity);
        var teleporter = Assert.Single(snapshot.FileTeleporters);
        Assert.Equal("in", teleporter.InboundPath);
        Assert.Equal(8UL, teleporter.Sent);
        Assert.Equal(5UL, teleporter.Received);
        Assert.False(teleporter.Filters.TeleportVegetables);
        Assert.Equal("out/db2-guard.dbo", teleporter.BackflowGuardPath);
    }
}
