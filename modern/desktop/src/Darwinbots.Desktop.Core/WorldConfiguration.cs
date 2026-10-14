using System.Text.Json;
using System.Text.Json.Serialization;

namespace Darwinbots.Desktop.Core;

public sealed record WorldConfiguration(
    EnvironmentUpdate Environment,
    double GlobalCostMultiplier,
    int PopulationCapacity,
    int VegetablePopulationCap,
    IReadOnlyList<SpeciesImport> Species)
{
    public GlobalSimulationCostSettings Costs { get; init; } = new();
    public GlobalAdvancedMutationSettings AdvancedMutation { get; init; } = new();
    public int BodyCutoff { get; init; }
    public bool GreedyReproduction { get; init; }
    public int DisqualificationRule { get; init; }
}

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
        var costJson = config.GetProperty("dna_costs");
        var costs = Read<GlobalSimulationCostSettings>("dna_costs") with
        {
            DnaNumber = costJson.GetProperty("number").GetDouble(),
            DnaDereference = costJson.GetProperty("dereference").GetDouble(),
            DnaBasic = costJson.GetProperty("basic").GetDouble(),
            DnaAdvanced = costJson.GetProperty("advanced").GetDouble(),
            DnaBitwise = costJson.GetProperty("bitwise").GetDouble(),
            DnaCondition = costJson.GetProperty("condition").GetDouble(),
            DnaLogic = costJson.GetProperty("logic").GetDouble(),
            DnaStore = costJson.GetProperty("store").GetDouble(),
            DnaFlow = costJson.GetProperty("flow").GetDouble(),
            DynamicMultiplier = costJson.GetProperty("dynamic_cost_multiplier").GetDouble(),
        };
        var advancedJson = config.GetProperty("advanced_mutation");
        var delta = advancedJson.GetProperty("delta2");
        var oscillation = advancedJson.GetProperty("oscillation");
        var epigenetic = advancedJson.GetProperty("epigenetic");
        var advanced = Read<GlobalAdvancedMutationSettings>("advanced_mutation") with
        {
            Delta2Enabled = delta.GetProperty("enabled").GetBoolean(),
            Delta2Interval = delta.GetProperty("interval").GetUInt64(),
            Delta2RateStep = delta.GetProperty("rate_step").GetDouble(),
            Delta2DistributionStep = delta.GetProperty("distribution_step").GetDouble(),
            Delta2PreferenceStepPercent = delta.GetProperty("preference_step_percent").GetInt32(),
            OscillationMode = oscillation.GetProperty("mode").Deserialize<GlobalMutationOscillationMode>(Options),
            OscillationPeriod = oscillation.GetProperty("period").GetUInt64(),
            OscillationAmplitude = oscillation.GetProperty("amplitude").GetDouble(),
            OscillationHighCycles = oscillation.TryGetProperty("high_cycles", out var highCycles) ? highCycles.GetUInt64() : 10000,
            OscillationLowCycles = oscillation.TryGetProperty("low_cycles", out var lowCycles) ? lowCycles.GetUInt64() : 10000,
            EpigeneticReset = epigenetic.GetProperty("reset_enabled").GetBoolean(),
            EpigeneticAmplification = epigenetic.GetProperty("amplification").GetDouble(),
            EpigeneticExponent = epigenetic.TryGetProperty("native_exponent", out var exponent) ? exponent.GetDouble() : 1.3,
            EpigeneticOverloadThreshold = epigenetic.GetProperty("overload_threshold").GetUInt64(),
            SaveAsTemporaryGene = epigenetic.GetProperty("save_as_temporary_gene").GetBoolean(),
        };
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
                recording.GetProperty("sampling_interval").GetInt32()),
            config.TryGetProperty("shapes_visible", out var visible) ? visible.GetBoolean() : false);
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
            Read<int>("organism_capacity"), Read<int>("vegetable_population_cap"), species)
        {
            Costs = costs,
            AdvancedMutation = advanced,
            BodyCutoff = Read<int>("body_cutoff"),
            GreedyReproduction = Read<bool>("greedy_reproduction"),
            DisqualificationRule = config.TryGetProperty("disqualification", out var rule) ? rule.GetInt32() : 0,
        };
    }
}
