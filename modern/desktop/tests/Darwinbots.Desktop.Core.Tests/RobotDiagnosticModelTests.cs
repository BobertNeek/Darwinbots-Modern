using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class RobotDiagnosticModelTests
{
    [Fact]
    public void SnapshotActivationFramesReachTheFocusedRobotDiagnosticModel()
    {
        const string json = """
            {
              "tick": 12,
              "world_size": [1000, 800],
              "organisms": [
                {
                  "id": { "slot": 7, "generation": 3 },
                  "position": [100, 200],
                  "velocity": [0, 0],
                  "energy": 1000,
                  "age": 4,
                  "species": 1,
                  "vegetable": false,
                  "body": 1000,
                  "waste": 0,
                  "shell": 0,
                  "slime": 0,
                  "venom": 0,
                  "poison": 0,
                  "chloroplasts": 0,
                  "aim": 0,
                  "paralyzed": 0,
                  "poisoned": 0,
                  "parents": [null, null],
                  "activation_bitmap": [false, true],
                  "activation_history": [
                    { "tick": 11, "activations": [false, true] },
                    { "tick": 12, "activations": [true, false] }
                  ]
                }
              ]
            }
            """;

        var snapshot = NativeSnapshotParser.Parse(json, "CPU");
        var model = new RobotDiagnosticModel();

        model.Update(snapshot, new OrganismKey(7, 3));

        Assert.Equal([11UL, 12UL], model.ActivationTimeline.Select(frame => frame.Tick));
        Assert.Equal([1, 1], model.ActivationTimeline.Select(frame => frame.ActiveGeneCount));
        Assert.Equal("LIVE · TICK 12 · 2 GENE FRAMES", model.ActivationSummary);
    }
}
