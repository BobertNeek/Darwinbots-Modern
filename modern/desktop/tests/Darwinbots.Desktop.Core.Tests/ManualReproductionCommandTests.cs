using System.Text.Json;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class ManualReproductionCommandTests
{
    [Fact]
    public async Task ManualReproductionCarriesFullDb2PayloadAndRoutesChildResponse()
    {
        var request = new ManualReproductionRequest(
            Parent: new OrganismKey(7, 3),
            SecondParent: null,
            Position: [123f, 456f],
            Energy: 321,
            Body: 654,
            MutationMode: ManualReproductionMutationMode.Apply,
            Focus: true);

        using var payload = JsonDocument.Parse(NativeCommandSerializer.SerializeManualReproduction(request));
        var command = payload.RootElement.GetProperty("commands")[0];

        Assert.Equal("manual_reproduce", command.GetProperty("type").GetString());
        Assert.Equal(7u, command.GetProperty("first_slot").GetUInt32());
        Assert.Equal(3u, command.GetProperty("first_generation").GetUInt32());
        Assert.Equal([123f, 456f], command.GetProperty("position").Deserialize<float[]>() ?? []);
        Assert.Equal(321, command.GetProperty("energy").GetInt32());
        Assert.Equal(654, command.GetProperty("body").GetInt32());
        Assert.Equal("apply", command.GetProperty("mutation_mode").GetString());
        Assert.True(command.GetProperty("focus").GetBoolean());

        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);

        var child = await session.ReproduceAsync(request);

        Assert.Equal(new OrganismKey(0, 0), child);
        Assert.Equal(request.Parent, engine.LastManualReproduction!.Parent);
        Assert.Equal(request.SecondParent, engine.LastManualReproduction.SecondParent);
        Assert.Equal(request.Energy, engine.LastManualReproduction.Energy);
        Assert.Equal(request.Body, engine.LastManualReproduction.Body);
        Assert.Equal(request.MutationMode, engine.LastManualReproduction.MutationMode);
        Assert.Equal(request.Focus, engine.LastManualReproduction.Focus);
        Assert.Equal(request.Position, engine.LastManualReproduction.Position);
    }
}
