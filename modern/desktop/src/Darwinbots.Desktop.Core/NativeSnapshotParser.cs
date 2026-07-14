using System.Text.Json;
using System.Text.Json.Serialization;

namespace Darwinbots.Desktop.Core;

public static class NativeSnapshotParser
{
    private static readonly JsonSerializerOptions Options = new()
    {
        PropertyNameCaseInsensitive = true,
    };

    public static EngineSnapshot Parse(string json, string backend)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(json);
        var native = JsonSerializer.Deserialize<NativeSnapshot>(json, Options)
            ?? throw new InvalidDataException("The native engine returned an empty snapshot.");
        var organisms = native.Organisms.Select(organism => new OrganismSnapshot(
            organism.Id.Slot,
            organism.Id.Generation,
            organism.Position,
            organism.Velocity,
            organism.Energy,
            organism.Age,
            organism.Species,
            organism.Vegetable,
            organism.Body,
            organism.Waste,
            organism.Shell,
            organism.Slime,
            organism.Venom,
            organism.Poison,
            organism.Chloroplasts,
            organism.Aim,
            organism.Paralyzed,
            organism.Poisoned,
            organism.Parents?.Select(parent => parent is null ? (OrganismKey?)null : new OrganismKey(parent.Slot, parent.Generation)).ToArray())
        {
            Phenotype = ParsePhenotype(organism.Phenotype),
            Vision = ParseVision(organism.Vision),
        }).ToArray();
        var renderInstances = (native.RenderInstances ?? []).Select(instance => new RenderInstanceSnapshot(
            instance.Slot,
            instance.Position,
            instance.Radius,
            instance.Color)
        {
            Generation = instance.Generation,
            Aim = instance.Aim,
            Skin = ParseSkin(instance.Skin),
            LineageId = instance.LineageId,
            Vegetable = instance.Vegetable,
        }).ToArray();
        var species = (native.Species ?? []).Select(value => new SpeciesSnapshot(
            value.Name,
            value.Vegetable,
            value.Color,
            value.MinimumPopulation,
            value.Reseed)).ToArray();
        var stats = native.Stats is null ? SimulationStatsSnapshot.Empty : new SimulationStatsSnapshot(
            native.Stats.Births,
            native.Stats.Deaths,
            native.Stats.ShotsFired,
            native.Stats.EnergyHarvested,
            native.Stats.Mutations,
            native.Stats.TiesCreated,
            native.Stats.TotalEnergy,
            native.Stats.EnergyDonated,
            native.Stats.Reseeds,
            native.Stats.SelfReproductions,
            native.Stats.FeedingEvents,
            native.Stats.IntentionalMovementEvents,
            native.Stats.ProjectileImpacts,
            native.Stats.ProjectileEffects,
            native.Stats.PlantEnergyGenerated);
        var obstacles = (native.Obstacles ?? []).Select(value => new ObstacleSnapshot(value.Id, value.Minimum, value.Maximum)).ToArray();
        var teleporters = (native.Teleporters ?? []).Select(value => new TeleporterSnapshot(value.Id, value.Center, value.Radius, value.Destination)).ToArray();
        var corpses = (native.Corpses ?? []).Select(value => new CorpseSnapshot(value.Position, value.Velocity, value.Energy, value.Body, value.Age)).ToArray();
        var shots = (native.Shots ?? []).Select(value => new ShotSnapshot(
            new OrganismKey(value.Owner.Slot, value.Owner.Generation),
            value.Start,
            value.End,
            value.Velocity is { Length: 2 } ? value.Velocity : [0f, 0f],
            value.Kind,
            value.Value,
            value.Age,
            value.Range,
            value.Energy,
            value.ImpactFlash)).ToArray();
        var history = (native.History ?? []).Select(value => new HistorySampleSnapshot(
            value.Tick, value.Population, value.TotalEnergy, value.Births, value.Deaths, value.Mutations, value.ShotsFired)).ToArray();
        var ties = (native.Ties ?? []).Select(value => new TieSnapshot(
            new OrganismKey(value.First.Slot, value.First.Generation),
            new OrganismKey(value.Second.Slot, value.Second.Generation),
            value.RestLength)).ToArray();
        var timings = native.PhaseTimings is null ? PhaseTimingsSnapshot.Empty : new PhaseTimingsSnapshot(
            native.PhaseTimings.Dna, native.PhaseTimings.Intent, native.PhaseTimings.Spatial,
            native.PhaseTimings.Sensing, native.PhaseTimings.Interactions, native.PhaseTimings.Physics,
            native.PhaseTimings.Lifecycle, native.PhaseTimings.Mutation, native.PhaseTimings.Snapshot);
        return new EngineSnapshot(native.Tick, organisms.Length, backend, organisms)
        {
            RenderInstances = renderInstances,
            Species = species,
            Stats = stats,
            WorldSize = native.WorldSize is { Length: 2 } ? native.WorldSize : [16_000f, 12_000f],
            Obstacles = obstacles,
            Teleporters = teleporters,
            Corpses = corpses,
            Shots = shots,
            History = history,
            Ties = ties,
            PhaseTimings = timings,
        };
    }

    private static VisualPhenotypeSnapshot ParsePhenotype(NativeVisualPhenotype? phenotype) =>
        phenotype is null
            ? VisualPhenotypeSnapshot.Default
            : new VisualPhenotypeSnapshot(
                phenotype.LineageId,
                phenotype.Color,
                ParseSkin(phenotype.Skin),
                phenotype.AccumulatedMutations);

    private static VisionSnapshot ParseVision(NativeVision? vision)
    {
        var defaults = VisualSnapshotDefaults.DefaultEyes();
        if (vision?.Eyes is { } eyes)
        {
            for (var index = 0; index < Math.Min(defaults.Length, eyes.Length); index++)
            {
                var eye = eyes[index];
                defaults[index] = new EyeSnapshot(
                    eye.Direction,
                    eye.Width,
                    eye.CenterRadians,
                    eye.HalfWidthRadians,
                    eye.Range,
                    eye.Value);
            }
        }
        return new VisionSnapshot((byte)Math.Clamp(vision?.FocusEye ?? 4, 0, 8), defaults);
    }

    private static SkinPointSnapshot[] ParseSkin(NativeSkinPoint[]? skin)
    {
        var defaults = VisualSnapshotDefaults.DefaultSkin();
        if (skin is null)
        {
            return defaults;
        }
        for (var index = 0; index < Math.Min(defaults.Length, skin.Length); index++)
        {
            defaults[index] = new SkinPointSnapshot(
                Math.Clamp(skin[index].Radius, 0.15f, 0.82f),
                ((skin[index].Angle % 1257) + 1257) % 1257);
        }
        return defaults;
    }

    private sealed record NativeSnapshot(
        [property: JsonPropertyName("tick")] ulong Tick,
        [property: JsonPropertyName("world_size")] float[]? WorldSize,
        [property: JsonPropertyName("organisms")] NativeOrganism[] Organisms,
        [property: JsonPropertyName("render_instances")] NativeRenderInstance[]? RenderInstances,
        [property: JsonPropertyName("species")] NativeSpecies[]? Species,
        [property: JsonPropertyName("stats")] NativeStats? Stats,
        [property: JsonPropertyName("obstacles")] NativeObstacle[]? Obstacles,
        [property: JsonPropertyName("teleporters")] NativeTeleporter[]? Teleporters,
        [property: JsonPropertyName("corpses")] NativeCorpse[]? Corpses,
        [property: JsonPropertyName("shots")] NativeShot[]? Shots,
        [property: JsonPropertyName("history")] NativeHistorySample[]? History,
        [property: JsonPropertyName("ties")] NativeTie[]? Ties,
        [property: JsonPropertyName("phase_timings")] NativePhaseTimings? PhaseTimings);

    private sealed record NativeCorpse(
        [property: JsonPropertyName("position")] float[] Position,
        [property: JsonPropertyName("velocity")] float[] Velocity,
        [property: JsonPropertyName("energy")] int Energy,
        [property: JsonPropertyName("body")] int Body,
        [property: JsonPropertyName("age")] ulong Age);

    private sealed record NativeShot(
        [property: JsonPropertyName("owner")] NativeId Owner,
        [property: JsonPropertyName("start")] float[] Start,
        [property: JsonPropertyName("end")] float[] End,
        [property: JsonPropertyName("velocity")] float[]? Velocity,
        [property: JsonPropertyName("age")] uint Age,
        [property: JsonPropertyName("range")] uint Range,
        [property: JsonPropertyName("energy")] float Energy,
        [property: JsonPropertyName("kind")] int Kind,
        [property: JsonPropertyName("value")] int Value,
        [property: JsonPropertyName("impact_flash")] bool ImpactFlash);

    private sealed record NativeHistorySample(
        [property: JsonPropertyName("tick")] ulong Tick,
        [property: JsonPropertyName("population")] int Population,
        [property: JsonPropertyName("total_energy")] long TotalEnergy,
        [property: JsonPropertyName("births")] ulong Births,
        [property: JsonPropertyName("deaths")] ulong Deaths,
        [property: JsonPropertyName("mutations")] ulong Mutations,
        [property: JsonPropertyName("shots_fired")] ulong ShotsFired);

    private sealed record NativeTie(
        [property: JsonPropertyName("first")] NativeId First,
        [property: JsonPropertyName("second")] NativeId Second,
        [property: JsonPropertyName("rest_length")] float RestLength);
        
    private sealed record NativeObstacle(
        [property: JsonPropertyName("id")] uint Id,
        [property: JsonPropertyName("minimum")] float[] Minimum,
        [property: JsonPropertyName("maximum")] float[] Maximum);

    private sealed record NativeTeleporter(
        [property: JsonPropertyName("id")] uint Id,
        [property: JsonPropertyName("center")] float[] Center,
        [property: JsonPropertyName("radius")] float Radius,
        [property: JsonPropertyName("destination")] float[] Destination);

    private sealed record NativePhaseTimings(
        [property: JsonPropertyName("dna")] double Dna,
        [property: JsonPropertyName("intent")] double Intent,
        [property: JsonPropertyName("spatial")] double Spatial,
        [property: JsonPropertyName("sensing")] double Sensing,
        [property: JsonPropertyName("interactions")] double Interactions,
        [property: JsonPropertyName("physics")] double Physics,
        [property: JsonPropertyName("lifecycle")] double Lifecycle,
        [property: JsonPropertyName("mutation")] double Mutation,
        [property: JsonPropertyName("snapshot")] double Snapshot);

    private sealed record NativeSpecies(
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("vegetable")] bool Vegetable,
        [property: JsonPropertyName("color")] uint Color,
        [property: JsonPropertyName("minimum_population")] int MinimumPopulation,
        [property: JsonPropertyName("reseed")] bool Reseed);

    private sealed record NativeStats(
        [property: JsonPropertyName("births")] ulong Births,
        [property: JsonPropertyName("deaths")] ulong Deaths,
        [property: JsonPropertyName("shots_fired")] ulong ShotsFired,
        [property: JsonPropertyName("energy_harvested")] ulong EnergyHarvested,
        [property: JsonPropertyName("mutations")] ulong Mutations,
        [property: JsonPropertyName("ties_created")] ulong TiesCreated,
        [property: JsonPropertyName("total_energy")] long TotalEnergy,
        [property: JsonPropertyName("energy_donated")] ulong EnergyDonated,
        [property: JsonPropertyName("reseeds")] ulong Reseeds,
        [property: JsonPropertyName("self_reproductions")] ulong SelfReproductions,
        [property: JsonPropertyName("feeding_events")] ulong FeedingEvents,
        [property: JsonPropertyName("intentional_movement_events")] ulong IntentionalMovementEvents,
        [property: JsonPropertyName("projectile_impacts")] ulong ProjectileImpacts,
        [property: JsonPropertyName("projectile_effects")] ulong ProjectileEffects,
        [property: JsonPropertyName("plant_energy_generated")] ulong PlantEnergyGenerated);

    private sealed record NativeRenderInstance(
        [property: JsonPropertyName("slot")] uint Slot,
        [property: JsonPropertyName("position")] float[] Position,
        [property: JsonPropertyName("radius")] float Radius,
        [property: JsonPropertyName("color")] uint Color,
        [property: JsonPropertyName("generation")] uint Generation,
        [property: JsonPropertyName("aim")] int Aim,
        [property: JsonPropertyName("skin")] NativeSkinPoint[]? Skin,
        [property: JsonPropertyName("lineage_id")] ulong LineageId,
        [property: JsonPropertyName("vegetable")] bool Vegetable);

    private sealed record NativeOrganism(
        [property: JsonPropertyName("id")] NativeId Id,
        [property: JsonPropertyName("position")] float[] Position,
        [property: JsonPropertyName("velocity")] float[] Velocity,
        [property: JsonPropertyName("energy")] int Energy,
        [property: JsonPropertyName("age")] ulong Age,
        [property: JsonPropertyName("species")] uint Species,
        [property: JsonPropertyName("vegetable")] bool Vegetable,
        [property: JsonPropertyName("body")] int Body,
        [property: JsonPropertyName("waste")] int Waste,
        [property: JsonPropertyName("shell")] int Shell,
        [property: JsonPropertyName("slime")] int Slime,
        [property: JsonPropertyName("venom")] int Venom,
        [property: JsonPropertyName("poison")] int Poison,
        [property: JsonPropertyName("chloroplasts")] int Chloroplasts,
        [property: JsonPropertyName("aim")] int Aim,
        [property: JsonPropertyName("paralyzed")] int Paralyzed,
        [property: JsonPropertyName("poisoned")] int Poisoned,
        [property: JsonPropertyName("parents")] NativeId?[]? Parents,
        [property: JsonPropertyName("phenotype")] NativeVisualPhenotype? Phenotype,
        [property: JsonPropertyName("vision")] NativeVision? Vision);

    private sealed record NativeSkinPoint(
        [property: JsonPropertyName("radius")] float Radius,
        [property: JsonPropertyName("angle")] int Angle);

    private sealed record NativeVisualPhenotype(
        [property: JsonPropertyName("lineage_id")] ulong LineageId,
        [property: JsonPropertyName("color")] uint Color,
        [property: JsonPropertyName("skin")] NativeSkinPoint[]? Skin,
        [property: JsonPropertyName("accumulated_mutations")] uint AccumulatedMutations);

    private sealed record NativeVision(
        [property: JsonPropertyName("focus_eye")] int FocusEye,
        [property: JsonPropertyName("eyes")] NativeEye[]? Eyes);

    private sealed record NativeEye(
        [property: JsonPropertyName("direction")] int Direction,
        [property: JsonPropertyName("width")] int Width,
        [property: JsonPropertyName("center_radians")] float CenterRadians,
        [property: JsonPropertyName("half_width_radians")] float HalfWidthRadians,
        [property: JsonPropertyName("range")] float Range,
        [property: JsonPropertyName("value")] int Value);

    private sealed record NativeId(
        [property: JsonPropertyName("slot")] uint Slot,
        [property: JsonPropertyName("generation")] uint Generation);
}
