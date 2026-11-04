using System.Text.Json;

namespace Darwinbots.Desktop.Core;

public enum Db2PhysicsPreset
{
    Fluid,
    Solid,
    Custom,
}

public sealed record Db2PhysicsOptions(
    float MaxVelocity,
    float MovementEfficiency,
    float SurfaceGravity,
    float StaticFriction,
    float KineticFriction,
    double Density,
    double Viscosity,
    float Elasticity,
    bool PlanetEaters = false,
    float PlanetEatersG = 0f,
    bool ZeroMomentum = false)
{
    public static Db2PhysicsOptions Default { get; } = new(60f, 0.66f, 0f, 0f, 0f, 0d, 0d, 0f);

    public double ReynoldsNumber => Density * 60d / Math.Max(Viscosity, 1e-7d);

    public string ReynoldsDescription => ReynoldsNumber < 300_000d ? "LAMINAR" : "TURBULENT";

    public string ContextualHelp =>
        "Reynolds number is the unitless ratio of turbulent to laminar flow. " +
        "Values below 3E+5 are characteristic of laminar flow. " +
        "Mass is measured in Masses. Force is Mass twip per cycle per cycle. " +
        "Energy is NRG, and impulse is a Bang.";

    public static IReadOnlyList<Db2PhysicsOptions> FluidPresets { get; } =
    [
        Default with { Density = 1e-7, Viscosity = 0.01 },
        Default with { Density = 1e-7, Viscosity = 0.0005 },
        Default with { Density = 1e-7, Viscosity = 0.000025 },
        Default with { Density = 0d, Viscosity = 0d },
    ];

    public static IReadOnlyList<Db2PhysicsOptions> SolidPresets { get; } =
    [
        Default with { Density = 0d, Viscosity = 0d, SurfaceGravity = 4f, StaticFriction = 0.9f, KineticFriction = 0.75f },
        Default with { Density = 0d, Viscosity = 0d, SurfaceGravity = 2f, StaticFriction = 0.6f, KineticFriction = 0.4f },
        Default with { Density = 0d, Viscosity = 0d, SurfaceGravity = 1f, StaticFriction = 0.05f, KineticFriction = 0.05f },
        Default with { Density = 0d, Viscosity = 0d, SurfaceGravity = 0f, StaticFriction = 0f, KineticFriction = 0f },
    ];

    public static IReadOnlyDictionary<Db2PhysicsPreset, Db2PhysicsOptions> Presets { get; } =
        new Dictionary<Db2PhysicsPreset, Db2PhysicsOptions>
        {
            [Db2PhysicsPreset.Fluid] = FluidPresets[2],
            [Db2PhysicsPreset.Solid] = SolidPresets[1],
            [Db2PhysicsPreset.Custom] = Default,
        };

    public static Db2PhysicsOptions ForPreset(Db2PhysicsPreset preset) => Presets[preset];
}

public enum Db2CorpseDecayMode
{
    None,
    WasteShot,
    EnergyShot,
}

public sealed record Db2CorpseDecayOptions(
    bool Enabled = false,
    int DeathEnergyThreshold = 15,
    Db2CorpseDecayMode Mode = Db2CorpseDecayMode.None,
    int PacketSize = 0,
    ulong Period = 0);

public sealed record Db2WasteOptions(
    int AlzheimerThreshold = 400,
    int DefecationThreshold = 32_000);

public sealed record Db2RecordingOptions(
    bool SafeModeBackup = false,
    bool DeadSnapshotEnabled = false,
    bool ExcludeRepopulatingRobots = false,
    ulong AutosaveInterval = 0,
    int SamplingInterval = 200);

public sealed record Db2ShotOptions(
    float Speed,
    float RangeMultiplier,
    float Decay,
    bool EnergyShotsDoNotDecay,
    bool WasteShotsDoNotDecay,
    bool FeedingProportional = true,
    int FeedingFixedPower = 200,
    float FeedingProportion = 1f)
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
    int TidesOffset = 0,
    bool WeatherEnabled = false,
    bool SunUpEnabled = false,
    int SunUpThreshold = 500_000,
    bool SunDownEnabled = false,
    int SunDownThreshold = 1_000_000,
    int SunThresholdMode = 0,
    bool SunOnRandom = false)
{
    public static Db2VegetationOptions Default { get; } = new(16_000, 100, 50, 10, 10, 0.75f, true, false, 10_000, Gradient: 1.02f);
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
    bool ToroidalWorld = true,
    bool WrapX = true,
    bool WrapY = true,
    Db2CorpseDecayOptions? CorpseDecay = null,
    Db2WasteOptions? Waste = null,
    Db2RecordingOptions? Recording = null,
    bool? ShapesVisible = null)
{
    public static EnvironmentUpdate Default { get; } = new(
        0, 0, 100, [0f, 0f], 0f, 0.5f,
        Db2PhysicsOptions.Default,
        Db2ShotOptions.Default,
        Db2VegetationOptions.Default,
        false,
        20f,
        true,
        true,
        true,
        new Db2CorpseDecayOptions(),
        new Db2WasteOptions(),
        new Db2RecordingOptions(SafeModeBackup: true));
}

public static class NativeCommandSerializer
{
    public static string SerializeEnvironment(EnvironmentUpdate update) =>
        JsonSerializer.Serialize(CreateEnvironmentBatch(update));

    public static string SerializeMutationSettings(MutationSettingsUpdate update) =>
        JsonSerializer.Serialize(CreateMutationSettingsBatch(update));

    public static string SerializeSpeciesColor(uint species, uint color) =>
        JsonSerializer.Serialize(CreateSpeciesColorBatch(species, color));

    public static string SerializeOrganismMutationProfile(
        OrganismKey organism,
        MutationProfileUpdate profile) =>
        JsonSerializer.Serialize(CreateOrganismMutationProfileBatch(organism, profile));

    public static string SerializeManualReproduction(ManualReproductionRequest request) =>
        JsonSerializer.Serialize(CreateManualReproductionBatch(request));

    internal static object CreateManualReproductionBatch(ManualReproductionRequest request) => new
    {
        version = 1,
        commands = new[]
        {
            new
            {
                type = "manual_reproduce",
                first_slot = request.Parent.Slot,
                first_generation = request.Parent.Generation,
                second_slot = request.SecondParent?.Slot,
                second_generation = request.SecondParent?.Generation,
                position = request.Position,
                energy = request.Energy,
                body = request.Body,
                mutation_mode = request.MutationMode.ToString().ToLowerInvariant(),
                focus = request.Focus,
            },
        },
    };

    internal static object CreateOrganismMutationProfileBatch(
        OrganismKey organism,
        MutationProfileUpdate profile) => new
        {
            version = 1,
            commands = new[]
            {
                new
                {
                    type = "update_organism_mutation_profile",
                    slot = organism.Slot,
                    generation = organism.Generation,
                    profile = MutationProfile(profile),
                },
            },
        };

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
            corpse_decay = CorpseDecay(options.CorpseDecay),
            waste = Waste(options.Waste),
            vegetation = Vegetation(options.Vegetation),
            recording = Recording(options.Recording with
            {
                SafeModeBackup = options.Recording.SafeModeBackup || options.GlobalSettings.Main.SafeMode,
            }),
            species = options.Species.Select(Species).ToArray(),
            auto_speciation = options.AutoSpeciation,
            speciation_genetic_distance_percent = options.SpeciationGeneticDistancePercent,
            toroidal_world = options.ToroidalWorld,
            fixed_bot_radii = options.FixedBotRadii,
            shapes_visible = options.ShapesVisible,
            dna_costs = DnaCosts(options.GlobalSettings.Costs),
            advanced_mutation = AdvancedMutation(options.GlobalSettings.AdvancedMutation, options.GlobalSettings.Main),
            disqualification = options.GlobalSettings.League.DisqualificationRule,
            recognition_fudging = options.GlobalSettings.League.RecognitionFudging,
            body_cutoff = options.GlobalSettings.Main.BodyCutoff,
            greedy_reproduction = options.GlobalSettings.Main.GreedyEnforcement,
            axis_topology = AxisTopology(options.ToroidalWorld, options.WrapX, options.WrapY),
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

    public static object CreateRemoveManyBatch(IReadOnlyList<OrganismKey> organisms, bool suppressDeathPoffs)
    {
        // Preserve the existing ordinary removal JSON, including absent optional policy.
        if (!suppressDeathPoffs) return CreateRemoveManyBatch(organisms);
        return new
        {
            version = 1,
            commands = new[]
            {
                new
                {
                    type = "remove_many",
                    organisms = organisms.Select(value => new { slot = value.Slot, generation = value.Generation }).ToArray(),
                    suppress_death_poffs = true,
                },
            },
        };
    }

    public static object CreateDeathPoffsBatch(bool enabled) => new
    {
        version = 1,
        commands = new[] { new { type = "set_death_poffs_enabled", enabled } },
    };

    internal static object CreateSpeciesColorBatch(uint species, uint color) => new
    {
        version = 1,
        commands = new[]
        {
            new { type = "set_species_color", species, color },
        },
    };

    internal static object CreateMutationSettingsBatch(MutationSettingsUpdate update) => new
    {
        version = 1,
        commands = new[]
        {
            new
            {
                type = "update_mutation_settings",
                enabled = update.Enabled,
                global_multiplier = update.GlobalMultiplier,
                species_rates = update.SpeciesRates.Select(value => new { species = value.Species, rate = value.Rate }).ToArray(),
                profile = update.Profile is null ? null : MutationProfile(update.Profile),
            },
        },
    };

    internal static object MutationProfile(MutationProfileUpdate value) => new
    {
        point = MutationRate(value.Point),
        point2 = MutationRate(value.Point2),
        delta = MutationRate(value.Delta),
        copy_error = MutationRate(value.CopyError),
        copy_error2 = MutationRate(value.CopyError2),
        insertion = MutationRate(value.Insertion),
        reversal = MutationRate(value.Reversal),
        minor_deletion = MutationRate(value.MinorDeletion),
        major_deletion = MutationRate(value.MajorDeletion),
        amplification = MutationRate(value.Amplification),
        translocation = MutationRate(value.Translocation),
        point_value_change_percent = value.PointValueChangePercent,
        copy_error_value_change_percent = value.CopyErrorValueChangePercent,
        point_category_distribution = MutationCategories(value.PointCategoryDistribution),
        copy_error_category_distribution = MutationCategories(value.CopyErrorCategoryDistribution),
    };

    private static object MutationRate(MutationRateUpdate value) => new
    {
        one_in = value.OneIn,
        mean = value.Mean,
        std_dev = value.StdDev,
    };

    private static object MutationCategories(MutationCategoryDistributionUpdate value) => new
    {
        number = value.Number,
        dereference = value.Dereference,
        @operator = value.Operator,
        logic = value.Logic,
        comparison = value.Comparison,
        flow = value.Flow,
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
                corpse_decay = CorpseDecay(update.CorpseDecay),
                waste = Waste(update.Waste),
                vegetation = Vegetation(update.Vegetation),
                recording = Recording(update.Recording),
                auto_speciation = update.AutoSpeciation,
                speciation_genetic_distance_percent = update.SpeciationGeneticDistancePercent,
                toroidal_world = update.ToroidalWorld,
                shapes_visible = update.ShapesVisible,
                axis_topology = AxisTopology(update.ToroidalWorld, update.WrapX, update.WrapY),
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
        planet_eaters = value.PlanetEaters,
        planet_eaters_g = value.PlanetEatersG,
        zero_momentum = value.ZeroMomentum,
        reynolds = value.ReynoldsNumber,
    };

    internal static object CorpseDecay(Db2CorpseDecayOptions? value) => new
    {
        enabled = value?.Enabled ?? false,
        death_energy_threshold = value?.DeathEnergyThreshold ?? 15,
        mode = (value?.Mode ?? Db2CorpseDecayMode.None) switch
        {
            Db2CorpseDecayMode.WasteShot => "waste_shot",
            Db2CorpseDecayMode.EnergyShot => "energy_shot",
            _ => "none",
        },
        packet_size = value?.PacketSize ?? 0,
        period = value?.Period ?? 0,
    };

    internal static object Waste(Db2WasteOptions? value) => new
    {
        alzheimer_threshold = value?.AlzheimerThreshold ?? 400,
        defecation_threshold = value?.DefecationThreshold ?? 32_000,
    };

    internal static object Recording(Db2RecordingOptions? value) => new
    {
        safemode_backup = value?.SafeModeBackup ?? false,
        dead_snapshot_enabled = value?.DeadSnapshotEnabled ?? false,
        exclude_repopulating_robots = value?.ExcludeRepopulatingRobots ?? false,
        autosave_interval = value?.AutosaveInterval ?? 0,
        sampling_interval = value?.SamplingInterval ?? 200,
    };

    private static object Species(SpeciesImport value) => new
    {
        name = value.Name,
        dna = value.Dna,
        vegetable = value.Vegetable,
        color = value.Color,
        initial_count = value.InitialCount,
        initial_energy = value.InitialEnergy,
        minimum_population = value.MinimumPopulation,
        reseed = value.Reseed,
        mutation_rate = value.MutationRate,
        placement = new
        {
            mode = value.Placement.Mode.ToString().ToLowerInvariant(),
            x = value.Placement.X,
            y = value.Placement.Y,
        },
        skin = value.Skin.Select(point => new { radius = point.Radius, angle = point.Angle }).ToArray(),
        restrictions = new
        {
            kill_multibot = value.Restrictions.KillMultibot,
            kill_disqualified = value.Restrictions.KillDisqualified,
            fixed_in_place = value.Restrictions.FixedInPlace,
            disable_vision = value.Restrictions.DisableVision,
            disable_dna = value.Restrictions.DisableDna,
            disable_movement = value.Restrictions.DisableMovement,
            disable_reproduction = value.Restrictions.DisableReproduction,
            virus_immune = value.Restrictions.VirusImmune,
            disable_mutations = value.Restrictions.DisableMutations,
            disable_chloroplasts = value.Restrictions.DisableChloroplasts,
        },
    };

    internal static object Shots(Db2ShotOptions value) => new
    {
        speed = value.Speed,
        range_multiplier = value.RangeMultiplier,
        decay = value.Decay,
        energy_shots_do_not_decay = value.EnergyShotsDoNotDecay,
        waste_shots_do_not_decay = value.WasteShotsDoNotDecay,
        feeding_proportional = value.FeedingProportional,
        feeding_fixed_power = value.FeedingFixedPower,
        feeding_proportion = value.FeedingProportion,
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
        weather_enabled = value.WeatherEnabled,
        sun_on_random = value.SunOnRandom,
        sun_up_enabled = value.SunUpEnabled,
        sun_up_threshold = value.SunUpThreshold,
        sun_down_enabled = value.SunDownEnabled,
        sun_down_threshold = value.SunDownThreshold,
        sun_threshold_mode = value.SunThresholdMode,
    };

    private static bool[] AxisTopology(bool toroidalWorld, bool wrapX, bool wrapY) =>
        !toroidalWorld && wrapX && wrapY ? [false, false] : [wrapX, wrapY];

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
        dynamic_upper_range_percent = value.DynamicUpperRangePercent ?? value.DynamicPopulationRange,
        dynamic_lower_range_percent = value.DynamicLowerRangePercent ?? value.DynamicUpperZeroThreshold,
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
        native_delta2 = new {
            rate_exponential = value.NativeDelta2.RateExponential,
            rate_linear = value.NativeDelta2.RateLinear,
            distribution_exponential = value.NativeDelta2.DistributionExponential,
            distribution_linear = value.NativeDelta2.DistributionLinear,
            rate_chance_percent = value.NativeDelta2.RateChancePercent,
            distribution_chance_percent = value.NativeDelta2.DistributionChancePercent,
            preference_step = value.NativeDelta2.PreferenceStep,
            maximum_rate_per_base = value.NativeDelta2.MaximumRatePerBase,
        },
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
            high_cycles = value.OscillationHighCycles,
            low_cycles = value.OscillationLowCycles,
        },
        epigenetic = new
        {
            native_exponent = value.EpigeneticExponent,
            reset_enabled = value.EpigeneticReset || main.EpigeneticReset,
            amplification = value.EpigeneticAmplification * main.EpigeneticAmplification,
            overload_threshold = value.EpigeneticOverloadThreshold,
            save_as_temporary_gene = value.SaveAsTemporaryGene || main.TemporaryGeneData,
        },
    };
}
