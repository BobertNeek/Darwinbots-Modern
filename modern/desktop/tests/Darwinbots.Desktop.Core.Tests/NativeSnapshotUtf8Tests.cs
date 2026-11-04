using System.Text;
using System.Text.Json;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class NativeSnapshotUtf8Tests
{
    [Theory]
    [InlineData("{}")]
    [InlineData("{\"tick\":7,\"organisms\":null,\"stats\":null,\"native_cycle\":null,\"world_size\":[]}")]
    [InlineData("{\"TiCk\":7,\"ORGANISMS\":[],\"death_poffs_enabled\":false,\"ties_enabled\":false,\"mutations_enabled\":false}")]
    public void MissingNullDefaultAndCaseInsensitiveFieldsMatchTheStringContract(string json)
    {
        AssertEquivalent(json);
    }

    [Fact]
    public void UnicodeInspectionHistoryMemoryAndSettingsMatchWithoutRetainingInputBytes()
    {
        const string json = """
        {
          "tick":42,"native_cycle":-17,"world_size":[1234,5678],
          "organisms":[{
            "id":{"slot":817,"generation":2},"position":[12,34],"velocity":[5,6],
            "energy":12345,"body":123,"shell":17,"slime":18,"venom":19,"poison":20,
            "current_gene":3,"condition_count":4,"debug_text":"é Ω 漢字 🦎\nline two",
            "activation_bitmap":[true,false,true],
            "activation_history":[{"tick":41,"activations":[false,true]},{"tick":42,"activations":[true,false]}],
            "memory":[{"address":1,"value":-123},{"address":1000,"value":2147483647}],
            "mutation_log":["変異 🦎"],"mutation_class_counts":{"点":7},
            "last_mutation_kind":"é point","flags":{"hidden":true,"corpse":false}
          }],
          "species":[{"name":"Espèce 漢字 🦎","vegetable":false,"color":4294901760,"minimum_population":7,"reseed":true,"mutation_rate":0.5}],
          "stats":{"total_energy":-123,"native_energy_displayed":0,"native_energy_delta":-17,"native_shots_visited":430},
          "history":[{"tick":41,"population":1}],
          "mutations_enabled":false,"global_mutation_multiplier":2.5,
          "robot_fixing_allowed":false,"asexual_reproduction_allowed":false,
          "death_poffs_enabled":false,"ties_enabled":false,
          "mutation_profile":{"point":{"one_in":123.5,"mean":7,"std_dev":0.25}}
        }
        """;
        var expected = NativeSnapshotParser.Parse(json, "CPU");
        var bytes = Encoding.UTF8.GetBytes(json);
        Assert.True(bytes.Length > json.Length);
        var inputBefore = bytes.ToArray();
        var parsed = NativeSnapshotParser.Parse(bytes, "CPU");
        Assert.Equal(inputBefore, bytes);
        var expectedState = JsonSerializer.Serialize(expected);
        Assert.Equal(expectedState, JsonSerializer.Serialize(parsed));
        Assert.Equal("é Ω 漢字 🦎\nline two", parsed.Organisms[0].DebugText);
        Assert.Equal(2, parsed.Organisms[0].ActivationHistory.Count);
        Assert.Equal(2147483647, parsed.Organisms[0].Memory[1].Value);
        Assert.False(parsed.MutationSettings.Enabled);
        Array.Fill(bytes, (byte)0);
        Assert.Equal(expectedState, JsonSerializer.Serialize(parsed));
    }

    [Theory]
    [InlineData("{")]
    [InlineData("[]")]
    [InlineData("{\"tick\":-1}")]
    [InlineData("{\"tick\":1} trailing")]
    public void MalformedJsonFailsThroughBothApis(string json)
    {
        Assert.Throws<JsonException>(() => NativeSnapshotParser.Parse(json, "CPU"));
        Assert.Throws<JsonException>(() => NativeSnapshotParser.Parse(Encoding.UTF8.GetBytes(json), "CPU"));
    }

    [Theory]
    [InlineData("")]
    [InlineData(" \t\r\n")]
    public void EmptyOrJsonWhitespaceInputIsRejected(string json)
    {
        Assert.Throws<ArgumentException>(() => NativeSnapshotParser.Parse(json, "CPU"));
        Assert.Throws<ArgumentException>(() => NativeSnapshotParser.Parse(Encoding.UTF8.GetBytes(json), "CPU"));
    }

    [Fact]
    public void NullSnapshotsAndMalformedUtf8AreRejected()
    {
        Assert.Throws<InvalidDataException>(() => NativeSnapshotParser.Parse("null", "CPU"));
        Assert.Throws<InvalidDataException>(() => NativeSnapshotParser.Parse("null"u8, "CPU"));
        var malformed = Encoding.UTF8.GetBytes("{\"species\":[{\"name\":\"x\"}]}");
        malformed[Array.IndexOf(malformed, (byte)'x')] = 0xff;
        Assert.Throws<JsonException>(() => NativeSnapshotParser.Parse(malformed, "CPU"));
    }

    private static void AssertEquivalent(string json) => Assert.Equal(
        JsonSerializer.Serialize(NativeSnapshotParser.Parse(json, "CPU")),
        JsonSerializer.Serialize(NativeSnapshotParser.Parse(Encoding.UTF8.GetBytes(json), "CPU")));
}
