using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.ViewModels;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class FinalToolsVisualRouteTests
{
    [Fact]
    public void NativeSnapshotRouteProjectsCompleteRobotStatisticsAndStatusEvents()
    {
        var snapshot = NativeSnapshotParser.Parse(NativeSnapshotJson, "CPU");
        var organism = Assert.Single(snapshot.Organisms);
        var viewModel = new MainWindowViewModel();

        viewModel.Update(snapshot);
        viewModel.Select(new OrganismKey(7, 3));

        Assert.Equal(new[] { 875, 100, 48 }, organism.Memory.Select(cell => cell.Value));
        Assert.Equal(0xff123456u, organism.Phenotype.Color);
        Assert.Equal(-1, Assert.Single(snapshot.Shots).Kind);
        Assert.Equal(40f, Assert.Single(snapshot.Shots).Energy);
        Assert.Equal(3, Assert.Single(snapshot.Ties).RestLength);
        Assert.Contains("FEED 13", viewModel.CombatSummary, StringComparison.Ordinal);
        Assert.Contains("MOVE 14", viewModel.CombatSummary, StringComparison.Ordinal);
        Assert.Contains("EFFECT 16", viewModel.CombatSummary, StringComparison.Ordinal);
        Assert.Contains(viewModel.StatusPanels, panel => panel.Label == "Feed");
        Assert.Contains(viewModel.StatusPanels, panel => panel.Label == "Events");
    }

    private const string NativeSnapshotJson = """
        {
          "tick": 27,
          "world_size": [1000, 800],
          "organisms": [
            {
              "id": { "slot": 7, "generation": 3 },
              "position": [100, 200],
              "velocity": [1.5, -0.5],
              "energy": 875,
              "age": 19,
              "species": 1,
              "vegetable": false,
              "body": 321,
              "waste": 8,
              "shell": 3,
              "slime": 4,
              "venom": 5,
              "poison": 6,
              "chloroplasts": 7,
              "aim": 314,
              "touch": 4,
              "taste": -2,
              "mass": 333,
              "radius": 12.5,
              "permanent_waste": 9,
              "offspring": 6,
              "descendants": 14,
              "simulation_id": 22,
              "unique_id": 7003,
              "virus_timer": 8,
              "new_mutations": 2,
              "total_mutations": 9,
              "dna_length": 17,
              "gene_count": 3,
              "current_gene": 2,
              "condition_count": 3,
              "memory": [
                { "address": 310, "value": 875 },
                { "address": 311, "value": 100 },
                { "address": 312, "value": 48 }
              ],
              "phenotype": {
                "lineage_id": 44,
                "color": 4279383126,
                "accumulated_mutations": 9,
                "skin": [
                  { "radius": 0.45, "angle": 0 },
                  { "radius": 0.72, "angle": 314 },
                  { "radius": 0.45, "angle": 628 }
                ]
              },
              "vision": {
                "focus_eye": 4,
                "eyes": [
                  { "direction": 314, "width": 70, "center_radians": 0.1, "half_width_radians": 0.05, "range": 1440, "value": 875 }
                ]
              }
            }
          ],
          "species": [
            { "name": "Unassigned", "vegetable": false, "color": 4286931330, "minimum_population": 0, "reseed": false },
            { "name": "Audit Bot", "vegetable": false, "color": 4280507072, "minimum_population": 0, "reseed": false }
          ],
          "stats": {
            "births": 11,
            "deaths": 4,
            "shots_fired": 9,
            "energy_harvested": 123,
            "mutations": 6,
            "ties_created": 3,
            "total_energy": 9999,
            "energy_donated": 12,
            "reseed": 2,
            "self_reproductions": 1,
            "feeding_events": 13,
            "intentional_movement_events": 14,
            "projectile_impacts": 15,
            "projectile_effects": 16,
            "plant_energy_generated": 17
          },
          "shots": [
            {
              "owner": { "slot": 7, "generation": 3 },
              "start": [20, 40],
              "end": [80, 40],
              "velocity": [40, 0],
              "kind": -1,
              "value": 40,
              "age": 1,
              "range": 40,
              "energy": 40,
              "impact_flash": false
            }
          ],
          "ties": [
            {
              "first": { "slot": 7, "generation": 3 },
              "second": { "slot": 7, "generation": 3 },
              "rest_length": 3
            }
          ]
        }
        """;
}
