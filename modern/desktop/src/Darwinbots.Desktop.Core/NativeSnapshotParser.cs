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
        var organisms = (native.Organisms ?? []).Select(organism => new OrganismSnapshot(
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
            IsCorpse = organism.Flags?.Corpse ?? false,
            IsHidden = organism.Flags?.Hidden ?? false,
            Phenotype = ParsePhenotype(organism.Phenotype),
            Vision = ParseVision(organism.Vision),
            CurrentGene = organism.CurrentGene,
            ConditionCount = organism.ConditionCount,
            BirthCycle = organism.BirthCycle,
            ActivationBitmap = organism.ActivationBitmap ?? [],
            ActivationHistory = (organism.ActivationHistory ?? [])
                .Select(frame => new ActivationFrameSnapshot(frame.Tick, frame.Activations ?? []))
                .ToArray(),
            DebugText = organism.DebugText ?? string.Empty,
            Memory = (organism.Memory ?? [])
                .Select(cell => new MemoryCellSnapshot(cell.Address, cell.Value))
                .ToArray(),
            Mass = organism.Mass,
            Radius = organism.Radius,
            PermanentWaste = organism.PermanentWaste,
            Offspring = organism.Offspring,
            Kills = organism.Kills,
            Descendants = organism.Descendants,
            SimulationId = organism.SimulationId,
            UniqueId = organism.UniqueId,
            VirusTimer = organism.VirusTimer,
            Touch = organism.Touch,
            Taste = organism.Taste,
            NewMutations = organism.NewMutations,
            TotalMutations = organism.TotalMutations,
            LastMutationCycle = organism.LastMutationCycle,
            LastMutationKind = organism.LastMutationKind,
            MutationLog = organism.MutationLog ?? [],
            MutationClassCounts = organism.MutationClassCounts ?? new Dictionary<string, ulong>(),
            Subspecies = organism.Subspecies,
            MutationProfile = ParseMutationProfile(organism.MutationProfile),
            DnaLength = organism.DnaLength,
            GeneCount = organism.GeneCount,
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
            value.Reseed,
            value.MutationRate)).ToArray();
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
        var obstacles = (native.Obstacles ?? []).Select(value => new ObstacleSnapshot(value.Id, value.Minimum, value.Maximum)
        {
            Color = value.Color ?? 0xffb9bdb6u,
            Opacity = value.Opacity ?? 1f,
            Drift = value.Drift is { Length: 2 } ? value.Drift : [0f, 0f],
            VisionPolicy = value.VisionPolicy ?? "opaque",
            ShotPolicy = value.ShotPolicy ?? "absorb",
        }).ToArray();
        var teleporters = (native.Teleporters ?? []).Select(value => new TeleporterSnapshot(value.Id, value.Center, value.Radius, value.Destination)
        {
            Mode = TeleporterMode(value),
            Color = value.Color ?? 0xffb9bdb6u,
            Velocity = value.Velocity is { Length: 2 } ? value.Velocity : [0f, 0f],
            Path = value.Path ?? string.Empty,
            InboundPath = value.InboundPath ?? string.Empty,
            OutboundPath = value.OutboundPath ?? string.Empty,
            Inbound = value.Inbound ?? true,
            Outbound = value.Outbound ?? false,
            Local = value.Local ?? true,
            Internet = value.Internet ?? false,
            DriftHorizontal = value.DriftHorizontal ?? false,
            DriftVertical = value.DriftVertical ?? false,
            FixedPosition = value.FixedPosition ?? true,
            Highlight = value.Highlight ?? false,
            TeleportVegetables = value.Filters?.TeleportVegetables ?? true,
            TeleportCorpses = value.Filters?.TeleportCorpses ?? true,
            TeleportHeterotrophs = value.Filters?.TeleportHeterotrophs ?? true,
            RespectShapes = value.RespectShapes ?? false,
            Sent = value.Sent ?? 0,
            Received = value.Received ?? 0,
            PollInterval = value.PollInterval ?? 1,
            PollCountdown = value.PollCountdown ?? 0,
            Capacity = value.Capacity ?? 1,
            BackflowLimit = value.BackflowLimit ?? 0,
        }).ToArray();
        var fileTeleporters = (native.FileTeleporters ?? []).Select(value => new FileTeleporterSnapshot(
            value.Id,
            value.Center,
            value.Radius,
            value.InboundPath ?? string.Empty,
            value.OutboundPath ?? string.Empty,
            value.PollInterval,
            value.PollCountdown,
            value.Capacity,
            value.Filters is { } filters
                ? new TeleporterFilterSnapshot(filters.TeleportVegetables, filters.TeleportCorpses, filters.TeleportHeterotrophs)
                : new TeleporterFilterSnapshot(true, true, true),
            value.Sent,
            value.Received,
            value.BackflowGuard)).ToArray();
        var corpses = (native.Corpses ?? []).Select(value => new CorpseSnapshot(value.Position, value.Velocity, value.Energy, value.Body, value.Age, value.Species, value.Repopulating)
        {
            Dna = value.Dna ?? string.Empty,
            Generation = value.Generation,
            BirthCycle = value.BirthCycle,
            Parent = value.Parent is null ? null : new OrganismKey(value.Parent.Slot, value.Parent.Generation),
            Offspring = value.Offspring,
            Kills = value.Kills,
            NewMutations = value.NewMutations,
            TotalMutations = value.TotalMutations,
            DnaLength = value.DnaLength,
            UniqueId = value.UniqueId,
            Chloroplasts = value.Chloroplasts,
        }).ToArray();
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
            value.Tick, value.Population, value.TotalEnergy, value.Births, value.Deaths, value.Mutations, value.ShotsFired)
        {
            AverageMutations = value.AverageMutations,
            AverageMutationsPerDnaLength = value.AverageMutationsPerDnaLength,
            AverageAge = value.AverageAge,
            AverageDescendents = value.AverageDescendents,
            AverageEnergy = value.AverageEnergy,
            AverageDnaLength = value.AverageDnaLength,
            AverageDnaConditions = value.AverageDnaConditions,
            DynamicCostStats = value.DynamicCostStats,
            SpeciesDiversity = value.SpeciesDiversity,
            AverageChloroplasts = value.AverageChloroplasts,
            SelectiveGeneticDistance = value.SelectiveGeneticDistance,
            GenerationalDistance = value.GenerationalDistance,
            SlowGeneticDistance = value.SlowGeneticDistance,
            SpeciesGraphs = (value.SpeciesGraphs ?? []).Select(species => new SpeciesGraphSampleSnapshot(
                species.Name, species.Color, species.Values)).ToArray(),
            DynamicCostSeries = value.DynamicCostSeries ?? [],
        }).ToArray();
        var ties = (native.Ties ?? []).Select(value => new TieSnapshot(
            new OrganismKey(value.First.Slot, value.First.Generation),
            new OrganismKey(value.Second.Slot, value.Second.Generation),
            value.RestLength)).ToArray();
        var timings = native.PhaseTimings is null ? PhaseTimingsSnapshot.Empty : new PhaseTimingsSnapshot(
            native.PhaseTimings.Dna, native.PhaseTimings.Intent, native.PhaseTimings.Spatial,
            native.PhaseTimings.Sensing, native.PhaseTimings.Interactions, native.PhaseTimings.Physics,
            native.PhaseTimings.Lifecycle, native.PhaseTimings.Mutation, native.PhaseTimings.Snapshot);
        var playerBot = ParsePlayerBot(native.PlayerBot);
        var restrictionOptions = ParseRestrictionOptions(native.RestrictionOptions);
        return new EngineSnapshot(native.Tick, organisms.Length, backend, organisms)
        {
            RenderInstances = renderInstances,
            Species = species,
            MutationSettings = new MutationSettingsSnapshot(
                native.MutationsEnabled ?? true,
                native.GlobalMutationMultiplier ?? 1f)
            {
                Profile = ParseMutationProfile(native.MutationProfile),
            },
            TiesEnabled = native.TiesEnabled ?? true,
            RobotFixingAllowed = native.RobotFixingAllowed ?? true,
            AsexualReproductionAllowed = native.AsexualReproductionAllowed ?? true,
            PlayerBot = playerBot,
            RestrictionOptions = restrictionOptions,
            Stats = stats,
            WorldSize = native.WorldSize is { Length: 2 } ? native.WorldSize : [16_000f, 12_000f],
            Obstacles = obstacles,
            Teleporters = teleporters,
            FileTeleporters = fileTeleporters,
            Corpses = corpses,
            Shots = shots,
            History = history,
            Ties = ties,
            PhaseTimings = timings,
            LocalMode = ParseLocalMode(native.LocalMode),
        };
    }

    private static MutationProfileUpdate ParseMutationProfile(NativeMutationProfile? native)
    {
        if (native is null)
            return MutationProfileUpdate.Default;

        var defaults = MutationProfileUpdate.Default;
        return new MutationProfileUpdate(
            ParseMutationRate(native.Point, defaults.Point),
            ParseMutationRate(native.Point2, defaults.Point2),
            ParseMutationRate(native.Delta, defaults.Delta),
            ParseMutationRate(native.CopyError, defaults.CopyError),
            ParseMutationRate(native.CopyError2, defaults.CopyError2),
            ParseMutationRate(native.Insertion, defaults.Insertion),
            ParseMutationRate(native.Reversal, defaults.Reversal),
            ParseMutationRate(native.MinorDeletion, defaults.MinorDeletion),
            ParseMutationRate(native.MajorDeletion, defaults.MajorDeletion),
            ParseMutationRate(native.Amplification, defaults.Amplification),
            ParseMutationRate(native.Translocation, defaults.Translocation),
            native.PointValueChangePercent ?? defaults.PointValueChangePercent,
            native.CopyErrorValueChangePercent ?? defaults.CopyErrorValueChangePercent,
            ParseMutationCategories(native.PointCategoryDistribution, defaults.PointCategoryDistribution),
            ParseMutationCategories(native.CopyErrorCategoryDistribution, defaults.CopyErrorCategoryDistribution));
    }

    private static MutationRateUpdate ParseMutationRate(
        NativeMutationRate? native,
        MutationRateUpdate fallback) =>
        native is null ? fallback : new(native.OneIn, native.Mean, native.StdDev);

    private static MutationCategoryDistributionUpdate ParseMutationCategories(
        NativeMutationCategoryDistribution? native,
        MutationCategoryDistributionUpdate fallback) =>
        native is null
            ? fallback
            : new(native.Number, native.Dereference, native.Operator, native.Logic, native.Comparison, native.Flow);

    private static LocalModeSnapshot ParseLocalMode(NativeLocalMode? native)
    {
        if (native?.Config is not { } config)
            return LocalModeSnapshot.Idle;

        var parsedConfig = new LocalModeConfig(
            ParseKind(config.Kind),
            ParsePreset(config.Preset),
            config.MinimumRounds,
            config.MaximumRounds,
            config.MaximumCycles,
            config.MaximumPopulation,
            config.RestartOnExtinction,
            config.SourceSpecies,
            config.SelectionPopulationWeight,
            config.SelectionEnergyWeight,
            config.Restrictions is { } restrictions
                ? new LocalModeRestrictions(
                    restrictions.ProhibitInformationShots,
                    restrictions.ProhibitViruses,
                    restrictions.ProhibitSexualReproduction,
                    restrictions.ProhibitCrossSpeciesTieTransfer,
                    restrictions.RobotFixingAllowed,
                    restrictions.AsexualReproductionAllowed)
                : new(),
            config.EnergyHandicap)
        {
            StagedEvolution = ParseStagedEvolution(config.StagedEvolution),
            Workflow = ParseWorkflow(config.Workflow),
            EntrantOrder = config.EntrantOrder ?? [],
            InitialLadder = config.InitialLadder ?? [],
            SeedingOpponent = config.SeedingOpponent,
        };
        return new LocalModeSnapshot(
            ParseStatus(native.Status),
            parsedConfig.Kind,
            parsedConfig.Preset,
            native.CurrentRound,
            parsedConfig.MaximumRounds,
            native.CycleInRound,
            native.ContestRuntime is { EffectiveCycleLimit: > 0 } runtime ? runtime.EffectiveCycleLimit : parsedConfig.MaximumCycles,
            native.RestartCount,
            (native.Results ?? []).Select(result => new LocalModeResult(
                result.Round,
                result.Winner,
                result.WinningPopulation,
                result.WinningEnergy,
                result.Reason,
                result.Automatic)).ToArray(),
            native.Adjudication is { } adjudication
                ? new LocalModeAdjudication(adjudication.Winner, adjudication.Reason, adjudication.Automatic)
                : null,
            parsedConfig.Restrictions)
        {
            MinimumRounds = parsedConfig.MinimumRounds,
            EnergyHandicap = parsedConfig.EnergyHandicap,
            Configuration = parsedConfig,
            Evolution = native.Evolution is { } evolution ? new StagedEvolutionSnapshot(evolution.Stage,
                evolution.Phase ?? "evolving", evolution.Hidden, evolution.ReductionUnit, evolution.HideCycles,
                evolution.TestWins, evolution.OpponentStage, evolution.Handicap - evolution.InverseHandicap) {
                    BaselineSpecies = evolution.BaselineSpecies ?? [], MutatorSpecies = evolution.MutatorSpecies ?? [],
                    TestResultCursor = evolution.TestResultCursor,
                } : null,
            Series = native.Series is { } series ? new LeagueSeriesSnapshot(
                ParseWorkflow(series.Workflow), series.ActiveMatch ?? [], series.MatchNumber,
                series.Ladder ?? [], series.Seeded ?? [], (series.Matches ?? []).Select(match =>
                    new LeagueMatchSnapshot(match.Number, match.Entrants ?? [], match.Winner,
                        match.FirstResult, match.ResultCount)).ToArray())
                    { MatchStartResult = series.MatchStartResult } : null,
            Disqualifications = (native.Disqualifications ?? []).Select(value => new DisqualificationRecord(
                value.Tick, value.Round, value.Species, value.FileName ?? string.Empty,
                value.Tag ?? string.Empty, value.Reason ?? string.Empty)).ToArray(),
        };
    }

    private static PlayerBotSnapshot ParsePlayerBot(NativePlayerBot? native)
    {
        if (native is null) return PlayerBotSnapshot.Default;
        var bindings = (native.Bindings ?? [])
            .Select(binding => new PlayerBotBinding(binding.Key, binding.MemoryAddress, binding.Value, binding.Invert))
            .ToArray();
        return new PlayerBotSnapshot(
            native.Enabled,
            native.Focus is { } focus ? new OrganismKey(focus.Slot, focus.Generation) : null,
            bindings,
            (native.PressedKeys ?? []).Select(key => checked((int)key)).ToArray());
    }

    private static RestrictionOptions ParseRestrictionOptions(NativeRestrictionOptions? native)
    {
        if (native is null) return RestrictionOptions.Default;
        return new RestrictionOptions(
            ParseRestrictionColumn(native.NonRepopulating),
            ParseRestrictionColumn(native.Repopulating));
    }

    private static RestrictionColumn ParseRestrictionColumn(NativeRestrictionColumn? native) => native is null
        ? new()
        : new(
            native.KillMultibot,
            native.KillDisqualified,
            native.DisableChloroplasts,
            native.FixedInPlace,
            native.DisableVision,
            native.DisableDna,
            native.DisableReproduction,
            native.VirusImmune,
            native.DisableMutations,
            native.DisableMovement);

    private static LocalModeKind ParseKind(string? value) => value?.ToLowerInvariant() switch
    {
        "contest" => LocalModeKind.Contest,
        "simple_survival" => LocalModeKind.SimpleSurvival,
        "eco_survival" => LocalModeKind.EcoSurvival,
        "zerobot" => LocalModeKind.Zerobot,
        _ => LocalModeKind.League,
    };

    private static StagedEvolutionConfiguration? ParseStagedEvolution(NativeEvolutionConfig? value) => value is null ? null : new(
        value.Mode switch { "eco" => EvolutionMode.Eco, "zerobot" => EvolutionMode.Zerobot, _ => EvolutionMode.SimpleSurvival },
        value.SourceDna ?? string.Empty, value.ReductionUnit, value.CycleLength, value.NormalizeDnaSize,
        value.ZerobotInitialDnaLength, value.AutoGraphs) { FitnessWeight = value.FitnessWeight ?? 100 };

    private static LeagueWorkflow ParseWorkflow(string? value) => value?.ToLowerInvariant() switch
    {
        "ladder" => LeagueWorkflow.Ladder,
        "tournament" => LeagueWorkflow.Tournament,
        "seeding" => LeagueWorkflow.Seeding,
        _ => LeagueWorkflow.Direct,
    };

    private static LeaguePreset ParsePreset(string? value) => value?.ToLowerInvariant() switch
    {
        "f2" => LeaguePreset.F2,
        "f3" => LeaguePreset.F3,
        "multibot" => LeaguePreset.Multibot,
        "short_bot" => LeaguePreset.ShortBot,
        "single_gene" => LeaguePreset.SingleGene,
        "single_store" => LeaguePreset.SingleStore,
        _ => LeaguePreset.F1,
    };

    private static LocalModeStatus ParseStatus(string? value) => value?.ToLowerInvariant() switch
    {
        "configured" => LocalModeStatus.Configured,
        "running" => LocalModeStatus.Running,
        "round_complete" => LocalModeStatus.RoundComplete,
        "completed" => LocalModeStatus.Completed,
        _ => LocalModeStatus.Idle,
    };

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
        [property: JsonPropertyName("mutations_enabled")] bool? MutationsEnabled,
        [property: JsonPropertyName("global_mutation_multiplier")] float? GlobalMutationMultiplier,
        [property: JsonPropertyName("mutation_profile")] NativeMutationProfile? MutationProfile,
        [property: JsonPropertyName("ties_enabled")] bool? TiesEnabled,
        [property: JsonPropertyName("robot_fixing_allowed")] bool? RobotFixingAllowed,
        [property: JsonPropertyName("asexual_reproduction_allowed")] bool? AsexualReproductionAllowed,
        [property: JsonPropertyName("player_bot")] NativePlayerBot? PlayerBot,
        [property: JsonPropertyName("restriction_options")] NativeRestrictionOptions? RestrictionOptions,
        [property: JsonPropertyName("stats")] NativeStats? Stats,
        [property: JsonPropertyName("obstacles")] NativeObstacle[]? Obstacles,
        [property: JsonPropertyName("teleporters")] NativeTeleporter[]? Teleporters,
        [property: JsonPropertyName("file_teleporters")] NativeFileTeleporter[]? FileTeleporters,
        [property: JsonPropertyName("corpses")] NativeCorpse[]? Corpses,
        [property: JsonPropertyName("shots")] NativeShot[]? Shots,
        [property: JsonPropertyName("history")] NativeHistorySample[]? History,
        [property: JsonPropertyName("ties")] NativeTie[]? Ties,
        [property: JsonPropertyName("phase_timings")] NativePhaseTimings? PhaseTimings,
        [property: JsonPropertyName("local_mode")] NativeLocalMode? LocalMode);

    private sealed record NativePlayerBot(
        [property: JsonPropertyName("enabled")] bool Enabled,
        [property: JsonPropertyName("focus")] NativeId? Focus,
        [property: JsonPropertyName("bindings")] NativePlayerBotBinding[]? Bindings,
        [property: JsonPropertyName("pressed_keys")] uint[]? PressedKeys);

    private sealed record NativePlayerBotBinding(
        [property: JsonPropertyName("key")] int Key,
        [property: JsonPropertyName("memory_address")] ushort MemoryAddress,
        [property: JsonPropertyName("value")] int Value,
        [property: JsonPropertyName("invert")] bool Invert);

    private sealed record NativeRestrictionOptions(
        [property: JsonPropertyName("non_repopulating")] NativeRestrictionColumn? NonRepopulating,
        [property: JsonPropertyName("repopulating")] NativeRestrictionColumn? Repopulating);

    private sealed record NativeRestrictionColumn(
        [property: JsonPropertyName("kill_multibot")] bool KillMultibot,
        [property: JsonPropertyName("kill_disqualified")] bool KillDisqualified,
        [property: JsonPropertyName("disable_chloroplasts")] bool DisableChloroplasts,
        [property: JsonPropertyName("fixed_in_place")] bool FixedInPlace,
        [property: JsonPropertyName("disable_vision")] bool DisableVision,
        [property: JsonPropertyName("disable_dna")] bool DisableDna,
        [property: JsonPropertyName("disable_reproduction")] bool DisableReproduction,
        [property: JsonPropertyName("virus_immune")] bool VirusImmune,
        [property: JsonPropertyName("disable_mutations")] bool DisableMutations,
        [property: JsonPropertyName("disable_movement")] bool DisableMovement);

    private sealed record NativeLocalMode(
        [property: JsonPropertyName("config")] NativeLocalModeConfig? Config,
        [property: JsonPropertyName("status")] string? Status,
        [property: JsonPropertyName("current_round")] uint CurrentRound,
        [property: JsonPropertyName("cycle_in_round")] ulong CycleInRound,
        [property: JsonPropertyName("restart_count")] uint RestartCount,
        [property: JsonPropertyName("results")] NativeLocalModeResult[]? Results,
        [property: JsonPropertyName("adjudication")] NativeLocalModeAdjudication? Adjudication,
        [property: JsonPropertyName("contest_runtime")] NativeContestRuntime? ContestRuntime,
        [property: JsonPropertyName("disqualifications")] NativeDisqualification[]? Disqualifications,
        [property: JsonPropertyName("series")] NativeLeagueSeries? Series,
        [property: JsonPropertyName("evolution")] NativeEvolutionState? Evolution);

    private sealed record NativeEvolutionState(
        [property: JsonPropertyName("stage")] uint Stage,
        [property: JsonPropertyName("phase")] string? Phase,
        [property: JsonPropertyName("hidden")] bool Hidden,
        [property: JsonPropertyName("reduction_unit")] float ReductionUnit,
        [property: JsonPropertyName("hide_cycles")] uint HideCycles,
        [property: JsonPropertyName("test_wins")] uint TestWins,
        [property: JsonPropertyName("opponent_stage")] uint OpponentStage,
        [property: JsonPropertyName("handicap")] double Handicap,
        [property: JsonPropertyName("inverse_handicap")] double InverseHandicap,
        [property: JsonPropertyName("baseline_species")] uint[]? BaselineSpecies,
        [property: JsonPropertyName("mutator_species")] uint[]? MutatorSpecies,
        [property: JsonPropertyName("test_result_cursor")] int TestResultCursor);

    private sealed record NativeEvolutionConfig(
        [property: JsonPropertyName("mode")] string? Mode,
        [property: JsonPropertyName("source_dna")] string? SourceDna,
        [property: JsonPropertyName("reduction_unit")] float ReductionUnit,
        [property: JsonPropertyName("cycle_length")] uint CycleLength,
        [property: JsonPropertyName("normalize_dna_size")] bool NormalizeDnaSize,
        [property: JsonPropertyName("zerobot_initial_dna_length")] int ZerobotInitialDnaLength,
        [property: JsonPropertyName("auto_graphs")] bool AutoGraphs,
        [property: JsonPropertyName("fitness_weight")] ushort? FitnessWeight);

    private sealed record NativeLeagueSeries(
        [property: JsonPropertyName("workflow")] string? Workflow,
        [property: JsonPropertyName("active_match")] uint[]? ActiveMatch,
        [property: JsonPropertyName("match_number")] uint MatchNumber,
        [property: JsonPropertyName("match_start_result")] int MatchStartResult,
        [property: JsonPropertyName("ladder")] uint[]? Ladder,
        [property: JsonPropertyName("seeded")] uint[]? Seeded,
        [property: JsonPropertyName("matches")] NativeLeagueMatch[]? Matches);

    private sealed record NativeLeagueMatch(
        [property: JsonPropertyName("number")] uint Number,
        [property: JsonPropertyName("entrants")] uint[]? Entrants,
        [property: JsonPropertyName("winner")] uint Winner,
        [property: JsonPropertyName("first_result")] int FirstResult,
        [property: JsonPropertyName("result_count")] int ResultCount);

    private sealed record NativeDisqualification(
        [property: JsonPropertyName("tick")] ulong Tick,
        [property: JsonPropertyName("round")] uint Round,
        [property: JsonPropertyName("species")] uint Species,
        [property: JsonPropertyName("file_name")] string? FileName,
        [property: JsonPropertyName("tag")] string? Tag,
        [property: JsonPropertyName("reason")] string? Reason);

    private sealed record NativeContestRuntime([property: JsonPropertyName("effective_cycle_limit")] ulong EffectiveCycleLimit);

    private sealed record NativeLocalModeConfig(
        [property: JsonPropertyName("kind")] string? Kind,
        [property: JsonPropertyName("preset")] string? Preset,
        [property: JsonPropertyName("minimum_rounds")] uint MinimumRounds,
        [property: JsonPropertyName("maximum_rounds")] uint MaximumRounds,
        [property: JsonPropertyName("maximum_cycles")] ulong MaximumCycles,
        [property: JsonPropertyName("maximum_population")] int MaximumPopulation,
        [property: JsonPropertyName("restart_on_extinction")] bool RestartOnExtinction,
        [property: JsonPropertyName("energy_handicap")] int EnergyHandicap,
        [property: JsonPropertyName("source_species")] uint? SourceSpecies,
        [property: JsonPropertyName("selection_population_weight")] float SelectionPopulationWeight,
        [property: JsonPropertyName("selection_energy_weight")] float SelectionEnergyWeight,
        [property: JsonPropertyName("restrictions")] NativeLocalModeRestrictions? Restrictions,
        [property: JsonPropertyName("workflow")] string? Workflow,
        [property: JsonPropertyName("entrant_order")] uint[]? EntrantOrder,
        [property: JsonPropertyName("initial_ladder")] uint[]? InitialLadder,
        [property: JsonPropertyName("seeding_opponent")] uint? SeedingOpponent,
        [property: JsonPropertyName("staged_evolution")] NativeEvolutionConfig? StagedEvolution);

    private sealed record NativeLocalModeRestrictions(
        [property: JsonPropertyName("prohibit_information_shots")] bool ProhibitInformationShots,
        [property: JsonPropertyName("prohibit_viruses")] bool ProhibitViruses,
        [property: JsonPropertyName("prohibit_sexual_reproduction")] bool ProhibitSexualReproduction,
        [property: JsonPropertyName("prohibit_cross_species_tie_transfer")] bool ProhibitCrossSpeciesTieTransfer,
        [property: JsonPropertyName("robot_fixing_allowed")] bool RobotFixingAllowed,
        [property: JsonPropertyName("asexual_reproduction_allowed")] bool AsexualReproductionAllowed);

    private sealed record NativeLocalModeResult(
        [property: JsonPropertyName("round")] uint Round,
        [property: JsonPropertyName("winner")] uint? Winner,
        [property: JsonPropertyName("winning_population")] int WinningPopulation,
        [property: JsonPropertyName("winning_energy")] long WinningEnergy,
        [property: JsonPropertyName("reason")] string Reason,
        [property: JsonPropertyName("automatic")] bool Automatic);

    private sealed record NativeLocalModeAdjudication(
        [property: JsonPropertyName("winner")] uint? Winner,
        [property: JsonPropertyName("reason")] string Reason,
        [property: JsonPropertyName("automatic")] bool Automatic);

    private sealed record NativeCorpse(
        [property: JsonPropertyName("position")] float[] Position,
        [property: JsonPropertyName("velocity")] float[] Velocity,
        [property: JsonPropertyName("energy")] int Energy,
        [property: JsonPropertyName("body")] int Body,
        [property: JsonPropertyName("age")] ulong Age,
        [property: JsonPropertyName("species")] uint Species,
        [property: JsonPropertyName("repopulating")] bool Repopulating,
        [property: JsonPropertyName("dna")] string? Dna,
        [property: JsonPropertyName("generation")] uint Generation,
        [property: JsonPropertyName("birth_cycle")] ulong BirthCycle,
        [property: JsonPropertyName("parent")] NativeId? Parent,
        [property: JsonPropertyName("offspring")] ulong Offspring,
        [property: JsonPropertyName("kills")] ulong Kills,
        [property: JsonPropertyName("new_mutations")] ulong NewMutations,
        [property: JsonPropertyName("total_mutations")] ulong TotalMutations,
        [property: JsonPropertyName("dna_length")] uint DnaLength,
        [property: JsonPropertyName("unique_id")] ulong UniqueId,
        [property: JsonPropertyName("chloroplasts")] int Chloroplasts);

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
        [property: JsonPropertyName("shots_fired")] ulong ShotsFired,
        [property: JsonPropertyName("average_mutations")] double AverageMutations,
        [property: JsonPropertyName("average_mutations_per_dna_length")] double AverageMutationsPerDnaLength,
        [property: JsonPropertyName("average_age")] double AverageAge,
        [property: JsonPropertyName("average_descendents")] double AverageDescendents,
        [property: JsonPropertyName("average_energy")] double AverageEnergy,
        [property: JsonPropertyName("average_dna_length")] double AverageDnaLength,
        [property: JsonPropertyName("average_dna_conditions")] double AverageDnaConditions,
        [property: JsonPropertyName("dynamic_cost_stats")] double DynamicCostStats,
        [property: JsonPropertyName("species_diversity")] double SpeciesDiversity,
        [property: JsonPropertyName("average_chloroplasts")] double AverageChloroplasts,
        [property: JsonPropertyName("selective_genetic_distance")] double SelectiveGeneticDistance,
        [property: JsonPropertyName("generational_distance")] double GenerationalDistance,
        [property: JsonPropertyName("slow_genetic_distance")] double SlowGeneticDistance,
        [property: JsonPropertyName("species_graphs")] NativeSpeciesGraphSample[]? SpeciesGraphs,
        [property: JsonPropertyName("dynamic_cost_series")] double[]? DynamicCostSeries);

    private sealed record NativeSpeciesGraphSample(
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("color")] uint Color,
        [property: JsonPropertyName("values")] Dictionary<string, double> Values);

    private sealed record NativeTie(
        [property: JsonPropertyName("first")] NativeId First,
        [property: JsonPropertyName("second")] NativeId Second,
        [property: JsonPropertyName("rest_length")] float RestLength);
        
    private sealed record NativeObstacle(
        [property: JsonPropertyName("id")] uint Id,
        [property: JsonPropertyName("minimum")] float[] Minimum,
        [property: JsonPropertyName("maximum")] float[] Maximum,
        [property: JsonPropertyName("color")] uint? Color,
        [property: JsonPropertyName("opacity")] float? Opacity,
        [property: JsonPropertyName("drift")] float[]? Drift,
        [property: JsonPropertyName("vision_policy")] string? VisionPolicy,
        [property: JsonPropertyName("shot_policy")] string? ShotPolicy);

    private static string TeleporterMode(NativeTeleporter value) =>
        value.Internet == true ? "internet" :
        value.Local != false ? "intrasim" :
        value.Outbound == true ? "outbound" :
        "inbound";

    private sealed record NativeTeleporter(
        [property: JsonPropertyName("id")] uint Id,
        [property: JsonPropertyName("center")] float[] Center,
        [property: JsonPropertyName("radius")] float Radius,
        [property: JsonPropertyName("destination")] float[] Destination,
        [property: JsonPropertyName("width")] float? Width = null,
        [property: JsonPropertyName("height")] float? Height = null,
        [property: JsonPropertyName("color")] uint? Color = null,
        [property: JsonPropertyName("velocity")] float[]? Velocity = null,
        [property: JsonPropertyName("path")] string? Path = null,
        [property: JsonPropertyName("inbound_path")] string? InboundPath = null,
        [property: JsonPropertyName("outbound_path")] string? OutboundPath = null,
        [property: JsonPropertyName("inbound")] bool? Inbound = null,
        [property: JsonPropertyName("outbound")] bool? Outbound = null,
        [property: JsonPropertyName("local")] bool? Local = null,
        [property: JsonPropertyName("internet")] bool? Internet = null,
        [property: JsonPropertyName("drift_horizontal")] bool? DriftHorizontal = null,
        [property: JsonPropertyName("drift_vertical")] bool? DriftVertical = null,
        [property: JsonPropertyName("fixed_position")] bool? FixedPosition = null,
        [property: JsonPropertyName("highlight")] bool? Highlight = null,
        [property: JsonPropertyName("filters")] NativeTeleporterFilters? Filters = null,
        [property: JsonPropertyName("respect_shapes")] bool? RespectShapes = null,
        [property: JsonPropertyName("sent")] ulong? Sent = null,
        [property: JsonPropertyName("received")] ulong? Received = null,
        [property: JsonPropertyName("poll_interval")] uint? PollInterval = null,
        [property: JsonPropertyName("capacity")] int? Capacity = null,
        [property: JsonPropertyName("poll_countdown")] uint? PollCountdown = null,
        [property: JsonPropertyName("backflow_limit")] uint? BackflowLimit = null);

    private sealed record NativeFileTeleporter(
        [property: JsonPropertyName("id")] uint Id,
        [property: JsonPropertyName("center")] float[] Center,
        [property: JsonPropertyName("radius")] float Radius,
        [property: JsonPropertyName("inbound_path")] string? InboundPath,
        [property: JsonPropertyName("outbound_path")] string? OutboundPath,
        [property: JsonPropertyName("poll_interval")] uint PollInterval,
        [property: JsonPropertyName("poll_countdown")] uint PollCountdown,
        [property: JsonPropertyName("capacity")] int Capacity,
        [property: JsonPropertyName("filters")] NativeTeleporterFilters? Filters,
        [property: JsonPropertyName("sent")] ulong Sent,
        [property: JsonPropertyName("received")] ulong Received,
        [property: JsonPropertyName("backflow_guard")] string? BackflowGuard);

    private sealed record NativeTeleporterFilters(
        [property: JsonPropertyName("teleport_vegetables")] bool TeleportVegetables,
        [property: JsonPropertyName("teleport_corpses")] bool TeleportCorpses,
        [property: JsonPropertyName("teleport_heterotrophs")] bool TeleportHeterotrophs);

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
        [property: JsonPropertyName("reseed")] bool Reseed,
        [property: JsonPropertyName("mutation_rate")] float MutationRate);

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
        [property: JsonPropertyName("birth_cycle")] ulong BirthCycle,
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
        [property: JsonPropertyName("touch")] int Touch,
        [property: JsonPropertyName("taste")] int Taste,
        [property: JsonPropertyName("parents")] NativeId?[]? Parents,
        [property: JsonPropertyName("phenotype")] NativeVisualPhenotype? Phenotype,
        [property: JsonPropertyName("vision")] NativeVision? Vision,
        [property: JsonPropertyName("current_gene")] uint CurrentGene,
        [property: JsonPropertyName("condition_count")] uint ConditionCount,
        [property: JsonPropertyName("activation_bitmap")] bool[]? ActivationBitmap,
        [property: JsonPropertyName("activation_history")] NativeActivationFrame[]? ActivationHistory,
        [property: JsonPropertyName("debug_text")] string? DebugText,
        [property: JsonPropertyName("memory")] NativeMemoryCell[]? Memory,
        [property: JsonPropertyName("mass")] float Mass,
        [property: JsonPropertyName("radius")] float Radius,
        [property: JsonPropertyName("permanent_waste")] int PermanentWaste,
        [property: JsonPropertyName("offspring")] ulong Offspring,
        [property: JsonPropertyName("kills")] ulong Kills,
        [property: JsonPropertyName("descendants")] ulong Descendants,
        [property: JsonPropertyName("simulation_id")] ulong SimulationId,
        [property: JsonPropertyName("unique_id")] ulong UniqueId,
        [property: JsonPropertyName("virus_timer")] int VirusTimer,
        [property: JsonPropertyName("new_mutations")] ulong NewMutations,
        [property: JsonPropertyName("total_mutations")] ulong TotalMutations,
        [property: JsonPropertyName("last_mutation_cycle")] ulong? LastMutationCycle,
        [property: JsonPropertyName("last_mutation_kind")] string? LastMutationKind,
        [property: JsonPropertyName("mutation_log")] string[]? MutationLog,
        [property: JsonPropertyName("mutation_class_counts")] Dictionary<string, ulong>? MutationClassCounts,
        [property: JsonPropertyName("subspecies")] int Subspecies,
        [property: JsonPropertyName("mutation_profile")] NativeMutationProfile? MutationProfile,
        [property: JsonPropertyName("dna_length")] uint DnaLength,
        [property: JsonPropertyName("gene_count")] uint GeneCount,
        [property: JsonPropertyName("flags")] NativeOrganismFlags? Flags);

    private sealed record NativeOrganismFlags([property: JsonPropertyName("corpse")] bool Corpse,
        [property: JsonPropertyName("hidden")] bool Hidden);

    private sealed record NativeMutationProfile(
        [property: JsonPropertyName("point")] NativeMutationRate? Point,
        [property: JsonPropertyName("point2")] NativeMutationRate? Point2,
        [property: JsonPropertyName("delta")] NativeMutationRate? Delta,
        [property: JsonPropertyName("copy_error")] NativeMutationRate? CopyError,
        [property: JsonPropertyName("copy_error2")] NativeMutationRate? CopyError2,
        [property: JsonPropertyName("insertion")] NativeMutationRate? Insertion,
        [property: JsonPropertyName("reversal")] NativeMutationRate? Reversal,
        [property: JsonPropertyName("minor_deletion")] NativeMutationRate? MinorDeletion,
        [property: JsonPropertyName("major_deletion")] NativeMutationRate? MajorDeletion,
        [property: JsonPropertyName("amplification")] NativeMutationRate? Amplification,
        [property: JsonPropertyName("translocation")] NativeMutationRate? Translocation,
        [property: JsonPropertyName("point_value_change_percent")] byte? PointValueChangePercent,
        [property: JsonPropertyName("copy_error_value_change_percent")] byte? CopyErrorValueChangePercent,
        [property: JsonPropertyName("point_category_distribution")] NativeMutationCategoryDistribution? PointCategoryDistribution,
        [property: JsonPropertyName("copy_error_category_distribution")] NativeMutationCategoryDistribution? CopyErrorCategoryDistribution);

    private sealed record NativeMutationRate(
        [property: JsonPropertyName("one_in")] double OneIn,
        [property: JsonPropertyName("mean")] double Mean,
        [property: JsonPropertyName("std_dev")] double StdDev);

    private sealed record NativeMutationCategoryDistribution(
        [property: JsonPropertyName("number")] double Number,
        [property: JsonPropertyName("dereference")] double Dereference,
        [property: JsonPropertyName("operator")] double Operator,
        [property: JsonPropertyName("logic")] double Logic,
        [property: JsonPropertyName("comparison")] double Comparison,
        [property: JsonPropertyName("flow")] double Flow);

    private sealed record NativeActivationFrame(
        [property: JsonPropertyName("tick")] ulong Tick,
        [property: JsonPropertyName("activations")] bool[]? Activations);

    private sealed record NativeMemoryCell(
        [property: JsonPropertyName("address")] ushort Address,
        [property: JsonPropertyName("value")] int Value);

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
