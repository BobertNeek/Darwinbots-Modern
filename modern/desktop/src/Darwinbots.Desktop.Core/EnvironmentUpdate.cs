using System.Text.Json;

namespace Darwinbots.Desktop.Core;

public sealed record Db2PhysicsOptions(
    float MaxVelocity,
    float MovementEfficiency,
    float SurfaceGravity,
    float StaticFriction,
    float KineticFriction,
    double Density,
    double Viscosity,
    float Elasticity)
{
    public static Db2PhysicsOptions Default { get; } = new(180f, 0.66f, 2f, 0.6f, 0.4f, 0d, 0d, 0f);
}

public sealed record Db2ShotOptions(
    float Speed,
    float RangeMultiplier,
    float Decay,
    bool EnergyShotsDoNotDecay,
    bool WasteShotsDoNotDecay)
{
    public static Db2ShotOptions Default { get; } = new(40f, 1f, 40f, false, false);
}

public sealed record Db2VegetationOptions(
    int StartChloroplasts,
    int MaxEnergyPerTick,
    int MinimumChloroplastEquivalents,
    int RepopulationAmount,
    ulong RepopulationCooldown,
    float FeedingToBody,
    bool Daytime,
    bool DayNightEnabled,
    ulong CycleLength,
    bool PondMode = false,
    float LightIntensity = 100f,
    float Gradient = 1f,
    uint Tides = 0,
    int TidesOffset = 0)
{
    public static Db2VegetationOptions Default { get; } = new(16_000, 40, 10, 10, 25, 0.5f, true, false, 10_000);
}

public sealed record EnvironmentUpdate(
    int MetabolismCost,
    int VegetableEnergyPerTick,
    int SunlightEnergy,
    float[] Gravity,
    float Drag,
    float BrownianMotion,
    Db2PhysicsOptions Physics,
    Db2ShotOptions Shots,
    Db2VegetationOptions Vegetation,
    bool AutoSpeciation,
    float SpeciationGeneticDistancePercent,
    bool ToroidalWorld = true)
{
    public static EnvironmentUpdate Default { get; } = new(
        0, 0, 100, [0f, 0f], 0f, 0f,
        Db2PhysicsOptions.Default,
        Db2ShotOptions.Default,
        Db2VegetationOptions.Default,
        false,
        20f,
        true);
}

public static class NativeCommandSerializer
{
    public static string SerializeEnvironment(EnvironmentUpdate update) =>
        JsonSerializer.Serialize(CreateEnvironmentBatch(update));

    public static string SerializeEngineConfig(WorldSetupOptions options) =>
        JsonSerializer.Serialize(new
        {
            seed = options.Seed,
            organism_capacity = options.PopulationCapacity,
            vegetable_population_cap = options.VegetablePopulationCap,
            world_width = options.WorldWidth,
            world_height = options.WorldHeight,
            backend = options.Backend,
            allow_cpu_fallback = true,
            metabolism_cost = options.EffectiveMetabolismCost,
            vegetable_energy_per_tick = options.VegetableEnergyPerTick,
            sunlight_energy = options.SunlightEnergy,
            gravity = options.Gravity,
            drag = options.Drag,
            brownian_motion = options.BrownianMotion,
            physics = Physics(options.Physics),
            shots = Shots(options.Shots),
            vegetation = Vegetation(options.Vegetation),
            auto_speciation = options.AutoSpeciation,
            speciation_genetic_distance_percent = options.SpeciationGeneticDistancePercent,
            toroidal_world = options.ToroidalWorld,
            dna_costs = DnaCosts(options.GlobalSettings.Costs),
            advanced_mutation = AdvancedMutation(options.GlobalSettings.AdvancedMutation, options.GlobalSettings.Main),
        });

    public static object CreateRemoveManyBatch(IReadOnlyList<OrganismKey> organisms) => new
    {
        version = 1,
        commands = new[]
        {
            new
            {
                type = "remove_many",
                organisms = organisms.Select(value => new { slot = value.Slot, generation = value.Generation }).ToArray(),
            },
        },
    };

    internal static object CreateEnvironmentBatch(EnvironmentUpdate update) => new
    {
        version = 1,
        commands = new[]
        {
            new
            {
                type = "update_environment",
                metabolism_cost = update.MetabolismCost,
                vegetable_energy_per_tick = update.VegetableEnergyPerTick,
                sunlight_energy = update.SunlightEnergy,
                gravity = update.Gravity,
                drag = update.Drag,
                brownian_motion = update.BrownianMotion,
                physics = Physics(update.Physics),
                shots = Shots(update.Shots),
                vegetation = Vegetation(update.Vegetation),
                auto_speciation = update.AutoSpeciation,
                speciation_genetic_distance_percent = update.SpeciationGeneticDistancePercent,
                toroidal_world = update.ToroidalWorld,
            },
        },
    };

    internal static object Physics(Db2PhysicsOptions value) => new
    {
        max_velocity = value.MaxVelocity,
        movement_efficiency = value.MovementEfficiency,
        surface_gravity = value.SurfaceGravity,
        static_friction = value.StaticFriction,
        kinetic_friction = value.KineticFriction,
        density = value.Density,
        viscosity = value.Viscosity,
        elasticity = value.Elasticity,
    };

    internal static object Shots(Db2ShotOptions value) => new
    {
        speed = value.Speed,
        range_multiplier = value.RangeMultiplier,
        decay = value.Decay,
        energy_shots_do_not_decay = value.EnergyShotsDoNotDecay,
        waste_shots_do_not_decay = value.WasteShotsDoNotDecay,
    };

    internal static object Vegetation(Db2VegetationOptions value) => new
    {
        start_chloroplasts = value.StartChloroplasts,
        max_energy_per_tick = value.MaxEnergyPerTick,
        minimum_chloroplast_equivalents = value.MinimumChloroplastEquivalents,
        repopulation_amount = value.RepopulationAmount,
        repopulation_cooldown = value.RepopulationCooldown,
        feeding_to_body = value.FeedingToBody,
        daytime = value.Daytime,
        day_night_enabled = value.DayNightEnabled,
        cycle_length = value.CycleLength,
        pond_mode = value.PondMode,
        light_intensity = value.LightIntensity,
        gradient = value.Gradient,
        tides = value.Tides,
        tides_offset = value.TidesOffset,
    };

    internal static object DnaCosts(GlobalSimulationCostSettings value) => new
    {
        number = value.DnaNumber,
        dereference = value.DnaDereference,
        basic = value.DnaBasic,
        advanced = value.DnaAdvanced,
        bitwise = value.DnaBitwise,
        condition = value.DnaCondition,
        logic = value.DnaLogic,
        store = value.DnaStore,
        flow = value.DnaFlow,
        dna_upkeep = value.DnaUpkeep,
        dna_copy = value.DnaCopy,
        movement = value.Movement,
        rotation = value.Rotation,
        tie_formation = value.TieFormation,
        shot_formation = value.ShotFormation,
        venom = value.Venom,
        poison = value.Poison,
        slime = value.Slime,
        shell = value.Shell,
        body_upkeep = value.BodyUpkeep,
        chloroplast_construction = value.ChloroplastConstruction,
        virus = value.Virus,
        age_base = value.AgeBase,
        age_start = value.AgeStart,
        age_linear = value.AgeLinear,
        age_logarithmic = value.AgeLogarithmic,
        dynamic_cost_multiplier = value.DynamicMultiplier,
        dynamic_all_robots = value.DynamicAllRobots,
        dynamic_target_population = value.DynamicTargetPopulation,
        dynamic_population_range = value.DynamicPopulationRange,
        dynamic_sensitivity = value.DynamicSensitivity,
        dynamic_allow_negative = value.DynamicAllowNegative,
        dynamic_lower_zero_threshold = value.DynamicLowerZeroThreshold,
        dynamic_upper_zero_threshold = value.DynamicUpperZeroThreshold,
        dynamic_lower_reinstate_threshold = value.DynamicLowerReinstateThreshold,
        dynamic_upper_reinstate_threshold = value.DynamicUpperReinstateThreshold,
        global_multiplier = value.GlobalMultiplier,
    };

    internal static object AdvancedMutation(GlobalAdvancedMutationSettings value, GlobalMainSettings main) => new
    {
        point2_enabled = value.Point2Enabled,
        copy_error2_enabled = value.CopyError2Enabled,
        amplification_enabled = value.AmplificationEnabled,
        translocation_enabled = value.TranslocationEnabled,
        normalization_enabled = value.NormalizationEnabled,
        normalization_dna_length = value.NormalizationDnaLength,
        delta2 = new
        {
            enabled = value.Delta2Enabled,
            interval = value.Delta2Interval,
            rate_step = value.Delta2RateStep,
            distribution_step = value.Delta2DistributionStep,
            preference_step_percent = value.Delta2PreferenceStepPercent,
        },
        oscillation = new
        {
            mode = value.OscillationMode.ToString(),
            period = value.OscillationPeriod,
            amplitude = value.OscillationAmplitude,
        },
        epigenetic = new
        {
            reset_enabled = value.EpigeneticReset || main.EpigeneticReset,
            amplification = value.EpigeneticAmplification * main.EpigeneticAmplification,
            overload_threshold = value.EpigeneticOverloadThreshold,
            save_as_temporary_gene = value.SaveAsTemporaryGene || main.TemporaryGeneData,
        },
    };
}
