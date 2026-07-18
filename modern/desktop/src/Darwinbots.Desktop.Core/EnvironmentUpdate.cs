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
    ulong CycleLength)
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
    };
}
