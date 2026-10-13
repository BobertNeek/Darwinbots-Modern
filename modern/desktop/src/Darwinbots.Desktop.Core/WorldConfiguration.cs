using System.Text.Json;
using System.Text.Json.Serialization;

namespace Darwinbots.Desktop.Core;

public sealed record WorldConfiguration(
    EnvironmentUpdate Environment,
    double GlobalCostMultiplier,
    int PopulationCapacity,
    int VegetablePopulationCap,
    IReadOnlyList<SpeciesImport> Species);

internal static class NativeConfigurationParser
{
    private static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        Converters = { new JsonStringEnumConverter(JsonNamingPolicy.SnakeCaseLower) },
    };

    public static WorldConfiguration Parse(ReadOnlyMemory<byte> bytes)
    {
        using var document = JsonDocument.Parse(bytes);
        var root = document.RootElement;
        var config = root.GetProperty("config");
        T Read<T>(string name) => config.GetProperty(name).Deserialize<T>(Options)
            ?? throw new InvalidDataException($"Missing native {name} configuration.");
        var toroidal = config.GetProperty("toroidal_world").GetBoolean();
        var topology = config.GetProperty("axis_topology");
        var wrapX = topology.ValueKind == JsonValueKind.Array ? topology[0].GetBoolean() : toroidal;
        var wrapY = topology.ValueKind == JsonValueKind.Array ? topology[1].GetBoolean() : toroidal;
        var recording = config.GetProperty("recording");
        var environment = new EnvironmentUpdate(
            Read<int>("metabolism_cost"), Read<int>("vegetable_energy_per_tick"), Read<int>("sunlight_energy"),
            Read<float[]>("gravity"), Read<float>("drag"), Read<float>("brownian_motion"),
            Read<Db2PhysicsOptions>("physics"), Read<Db2ShotOptions>("shots"), Read<Db2VegetationOptions>("vegetation"),
            Read<bool>("auto_speciation"), Read<float>("speciation_genetic_distance_percent"), toroidal, wrapX, wrapY,
            Read<Db2CorpseDecayOptions>("corpse_decay"), Read<Db2WasteOptions>("waste"),
            new Db2RecordingOptions(
                recording.GetProperty("safemode_backup").GetBoolean(),
                recording.GetProperty("dead_snapshot_enabled").GetBoolean(),
                recording.GetProperty("exclude_repopulating_robots").GetBoolean(),
                recording.GetProperty("autosave_interval").GetUInt64(),
                recording.GetProperty("sampling_interval").GetInt32()));
        var species = root.GetProperty("species").EnumerateArray()
            .Where(value => value.GetProperty("dna").ValueKind == JsonValueKind.String)
            .Select(value =>
            {
                var species = value.Deserialize<SpeciesImport>(Options)
                    ?? throw new InvalidDataException("Missing native species template.");
                return species with { Restrictions = species.Restrictions ?? new(), Skin = species.Skin ?? SpeciesImport.DefaultSkin() };
            })
            .ToArray();
        return new WorldConfiguration(environment,
            config.GetProperty("dna_costs").GetProperty("global_multiplier").GetDouble(),
            Read<int>("organism_capacity"), Read<int>("vegetable_population_cap"), species);
    }
}
