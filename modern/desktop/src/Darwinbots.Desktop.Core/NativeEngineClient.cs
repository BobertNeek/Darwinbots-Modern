using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;

namespace Darwinbots.Desktop.Core;

public sealed partial class NativeEngineClient : IEngineClient
{
    private const string LibraryName = "darwinbots_engine";
    private IntPtr _engine;
    private float _worldWidth;
    private float _worldHeight;
    private readonly SeededSpawnPlacement _spawnPlacement;
    private readonly TimeProvider _clock;
    private bool _sampleRestartClockPerCycle;
    private bool _sampleClassicSkinClock;

    public NativeEngineClient(WorldSetupOptions options, TimeProvider? timeProvider = null)
    {
        _clock = timeProvider ?? TimeProvider.System;
        if (options.GlobalSettings.Main.NewSeed)
            options = options with { Seed = NativeTimerSeed() };
        _worldWidth = options.WorldWidth;
        _worldHeight = options.WorldHeight;
        _spawnPlacement = new SeededSpawnPlacement(options.Seed);
        ValidatePosition(options.Gravity);
        var bytes = Encoding.UTF8.GetBytes(NativeCommandSerializer.SerializeEngineConfig(options));
        ThrowIfFailed(NativeMethods.EngineCreate(bytes, (nuint)bytes.Length, out _engine), "create engine");
        if (_engine == IntPtr.Zero)
        {
            throw new InvalidOperationException("Native engine creation returned a null handle.");
        }
        try
        {
            if (options.NativeGaussianSpare is { } spare)
                ExecuteBatch(new { version = 1, commands = new[] { new { type = "set_native_gaussian_spare", spare } } }, "inherit native Gaussian spare");
            if (options.NativeSunChange is { } sunChange && !(options.Vegetation.WeatherEnabled || options.Vegetation.SunOnRandom))
                ExecuteBatch(new { version = 1, commands = new[] { new { type = "set_native_sun_change", sun_change = sunChange } } }, "inherit native sun change");
            if (options.NativeProcessCaches is { } caches)
                ExecuteBatch(new { version = 1, commands = new[] { new { type = "set_native_process_caches", caches } } }, "inherit native process caches");
            UpdateMutationSettings(options.MutationSettings);
            SetTiesEnabled(options.TiesEnabled);
            UpdateLegacyRestrictions(options.RobotFixingAllowed, options.AsexualReproductionAllowed);
            foreach (var obstacle in options.Obstacles) { AddObstacle(obstacle); UpdateObstacle(obstacle); }
        }
        catch { Dispose(); throw; }
    }

    public NativeEngineClient(
        string backend = "Auto",
        int capacity = 32_000,
        ulong seed = 1,
        float worldWidth = 16_000f,
        float worldHeight = 12_000f,
        int metabolismCost = 0,
        int vegetableEnergyPerTick = 4,
        int sunlightEnergy = 100,
        float[]? gravity = null,
        float drag = 0f,
        float brownianMotion = 0f,
        int vegetablePopulationCap = 500,
        Db2PhysicsOptions? physics = null,
        Db2ShotOptions? shots = null,
        Db2VegetationOptions? vegetation = null,
        bool autoSpeciation = false,
        float speciationGeneticDistancePercent = 20f,
        bool toroidalWorld = true,
        bool wrapX = true,
        bool wrapY = true,
        GlobalSettings? globalSettings = null,
        TimeProvider? timeProvider = null) : this(new WorldSetupOptions
    {
        Backend = backend,
        PopulationCapacity = capacity,
        Seed = seed,
        WorldWidth = worldWidth,
        WorldHeight = worldHeight,
        MetabolismCost = metabolismCost,
        VegetableEnergyPerTick = vegetableEnergyPerTick,
        SunlightEnergy = sunlightEnergy,
        Gravity = gravity ?? [0f, 0f],
        Drag = drag,
        BrownianMotion = brownianMotion,
        VegetablePopulationCap = vegetablePopulationCap,
        Physics = physics ?? Db2PhysicsOptions.Default,
        Shots = shots ?? Db2ShotOptions.Default,
        Vegetation = vegetation ?? Db2VegetationOptions.Default,
        AutoSpeciation = autoSpeciation,
        SpeciationGeneticDistancePercent = speciationGeneticDistancePercent,
        ToroidalWorld = toroidalWorld,
        WrapX = wrapX,
        WrapY = wrapY,
        GlobalSettings = globalSettings ?? new GlobalSettings(),
    }, timeProvider)
    {
    }

    public string Backend => NativeMethods.EngineBackend(_engine) switch
    {
        0 => "CPU",
        1 => "GPU",
        _ => "UNKNOWN",
    };

    public void Tick(uint count = 1)
    {
        EnsureAlive();
        if (count == 0) return;
        if (_sampleRestartClockPerCycle && count is > 1 and <= 1_000_000)
        {
            // External league/staged restarts take a new Timer sample for each
            // original cycle. Each successful cycle commits before the next begins.
            for (uint cycle = 0; cycle < count; cycle++) Tick(1);
            return;
        }
        ExecuteBatch(new { version = 1, commands = new object[] { NativeRestartSeedCommand(), new { type = "tick", count } } }, "advance simulation");
    }

    public DnaImportReport ImportDna(string dna, float[]? position = null)
    {
        EnsureAlive();
        ArgumentException.ThrowIfNullOrWhiteSpace(dna);
        var result = ExecuteBatchForResult(
            new { version = 1, commands = new[] { new { type = "import_dna", dna, position } } },
            "import DNA");
        return ReadImportReport(result.GetProperty("results")[0]);
    }

    public int ImportOrganism(ReadOnlyMemory<byte> payload)
    {
        EnsureAlive();
        if (payload.IsEmpty) throw new ArgumentException("Organism interchange payload is empty.", nameof(payload));
        var result = ExecuteBatchForResult(new
        {
            version = 1,
            // System.Text.Json encodes byte[] as base64. The native Vec<u8>
            // command contract requires a JSON sequence of byte values.
            commands = new[] { new { type = "import_organism", payload = payload.ToArray().Select(value => (int)value).ToArray() } },
        }, "import organism");
        return result.GetProperty("results")[0].GetProperty("count").GetInt32();
    }

    public DnaImportReport ImportSpecies(SpeciesImport species)
    {
        EnsureAlive();
        ArgumentNullException.ThrowIfNull(species);
        species.Validate();
        var result = ExecuteBatchForResult(new
        {
            version = 1,
            commands = new[]
            {
                new
                {
                    type = "import_species",
                    dna = species.Dna,
                    name = species.Name,
                    vegetable = species.Vegetable,
                    color = species.Color,
                    initial_count = species.InitialCount,
                    minimum_population = species.MinimumPopulation,
                    reseed = species.Reseed,
                    mutation_rate = species.MutationRate,
                    mutation_profile = species.MutationProfile is null ? null : NativeCommandSerializer.MutationProfile(species.MutationProfile),
                    initial_energy = species.InitialEnergy,
                    placement = new
                    {
                        mode = species.Placement.Mode.ToString().ToLowerInvariant(),
                        left = species.Placement.Left, right = species.Placement.Right,
                        top = species.Placement.Top, bottom = species.Placement.Bottom,
                        x = species.Placement.X,
                        y = species.Placement.Y,
                    },
                    skin = species.Skin.Select(point => new { radius = point.Radius, angle = point.Angle }).ToArray(),
                    restrictions = SpeciesRestrictions(species.Restrictions),
                    positions = _spawnPlacement.Next(species, _worldWidth, _worldHeight),
                },
            },
        }, "import species");
        return ReadImportReport(result.GetProperty("results")[0]);
    }

    public DnaImportReport InsertSpeciesAt(SpeciesImport species, float[] position)
    {
        EnsureAlive();
        ArgumentNullException.ThrowIfNull(species);
        species.Validate();
        ValidatePosition(position);
        var result = ExecuteBatchForResult(new
        {
            version = 1,
            commands = new[]
            {
                new
                {
                    type = "insert_species",
                    name = species.Name,
                    dna = species.Dna,
                    position,
                    initial_energy = species.InitialEnergy,
                },
            },
        }, "insert species");
        return ReadImportReport(result.GetProperty("results")[0]);
    }

    private static DnaImportReport ReadImportReport(JsonElement value)
    {
        if (!value.TryGetProperty("compatibility_warnings", out var warnings)) return DnaImportReport.Compatible;
        return new DnaImportReport(warnings.EnumerateArray().Select(item => item.GetString() ?? string.Empty)
            .Where(item => item.Length > 0).ToArray());
    }

    private static object SpeciesRestrictions(SpeciesRestrictionSettings value) => new
    {
        kill_multibot = value.KillMultibot,
        kill_disqualified = value.KillDisqualified,
        fixed_in_place = value.FixedInPlace,
        disable_vision = value.DisableVision,
        disable_dna = value.DisableDna,
        disable_movement = value.DisableMovement,
        disable_reproduction = value.DisableReproduction,
        virus_immune = value.VirusImmune,
        disable_mutations = value.DisableMutations,
        disable_chloroplasts = value.DisableChloroplasts,
    };

    public void Remove(uint slot, uint generation)
    {
        EnsureAlive();
        ExecuteBatch(new
        {
            version = 1,
            commands = new[] { new { type = "remove", slot, generation } },
        }, "remove organism");
    }

    public void RemoveMany(IReadOnlyList<OrganismKey> organisms) => RemoveMany(organisms, false);

    public void RemoveMany(IReadOnlyList<OrganismKey> organisms, bool suppressDeathPoffs)
    {
        ArgumentNullException.ThrowIfNull(organisms);
        if (organisms.Count == 0)
            throw new ArgumentException("At least one organism is required.", nameof(organisms));
        EnsureAlive();
        ExecuteBatch(NativeCommandSerializer.CreateRemoveManyBatch(organisms, suppressDeathPoffs), "remove organisms");
    }

    public void SetDeathPoffsEnabled(bool enabled) => ExecuteBatch(
        NativeCommandSerializer.CreateDeathPoffsBatch(enabled), "change death-particle creation");

    public void Move(uint slot, uint generation, float[] position)
    {
        ValidatePosition(position);
        ExecuteBatch(new { version = 1, commands = new[] { new { type = "move_organism", slot, generation, position } } }, "move organism");
    }

    public OrganismKey CloneOrganism(uint slot, uint generation, float[] position)
    {
        ValidatePosition(position);
        var result = ExecuteBatchForResult(new { version = 1, commands = new[] { new { type = "clone_organism", slot, generation, position } } }, "clone organism");
        return ReadKey(result.GetProperty("results")[0]);
    }

    public void ReplaceDna(uint slot, uint generation, string dna)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(dna);
        ExecuteBatch(new { version = 1, commands = new[] { new { type = "replace_dna", slot, generation, dna } } }, "replace DNA");
    }

    public string ExportDna(uint slot, uint generation)
    {
        var result = ExecuteBatchForResult(new { version = 1, commands = new[] { new { type = "export_dna", slot, generation } } }, "export DNA");
        return result.GetProperty("results")[0].GetProperty("dna").GetString()
            ?? throw new InvalidDataException("Native engine returned empty DNA.");
    }

    public string ExportSpeciesTemplateDna(uint species)
    {
        var result = ExecuteBatchForResult(new
        {
            version = 1,
            commands = new[] { new { type = "export_species_template_dna", species } },
        }, "export startup species DNA");
        return result.GetProperty("results")[0].GetProperty("dna").GetString()
            ?? throw new InvalidDataException("Native engine returned no startup species DNA.");
    }

    public byte[] ExportOrganismGroup(IReadOnlyList<OrganismKey> organisms)
    {
        ArgumentNullException.ThrowIfNull(organisms);
        if (organisms.Count is < 1 or > 50 || organisms.Distinct().Count() != organisms.Count)
            throw new ArgumentException("A Darwinbots 2 organism must contain 1 to 50 distinct robots.", nameof(organisms));
        var result = ExecuteBatchForResult(new
        {
            version = 1,
            commands = new[] { new { type = "export_organism_group", organisms = organisms.Select(key => new { slot = key.Slot, generation = key.Generation }).ToArray() } },
        }, "export organism group");
        return result.GetProperty("results")[0].GetProperty("payload").EnumerateArray().Select(value => value.GetByte()).ToArray();
    }

    public OrganismKey Reproduce(uint firstSlot, uint firstGeneration, uint? secondSlot, uint? secondGeneration, float[] position)
    {
        ValidatePosition(position);
        var result = ExecuteBatchForResult(new
        {
            version = 1,
            commands = new[] { new { type = "manual_reproduce", first_slot = firstSlot, first_generation = firstGeneration, second_slot = secondSlot, second_generation = secondGeneration, position } },
        }, "manually reproduce organism");
        return ReadKey(result.GetProperty("results")[0]);
    }

    public OrganismKey Reproduce(ManualReproductionRequest request)
    {
        ArgumentNullException.ThrowIfNull(request);
        request.Validate();
        var result = ExecuteBatchForResult(NativeCommandSerializer.CreateManualReproductionBatch(request), "manually reproduce organism");
        return ReadKey(result.GetProperty("results")[0]);
    }

    public void SwitchBackend(string backend)
    {
        EnsureAlive();
        if (!new[] { "Auto", "Cpu", "Gpu" }.Contains(backend, StringComparer.OrdinalIgnoreCase))
            throw new ArgumentException($"Unknown backend '{backend}'.", nameof(backend));
        ExecuteBatch(new { version = 1, commands = new[] { new { type = "switch_backend", backend } } }, "switch backend");
    }

    public void AddObstacle(ObstacleSnapshot obstacle)
    {
        ArgumentNullException.ThrowIfNull(obstacle);
        ValidatePosition(obstacle.Minimum);
        ValidatePosition(obstacle.Maximum);
        ExecuteBatch(new { version = 1, commands = new[] { new { type = "add_obstacle", id = obstacle.Id, minimum = obstacle.Minimum, maximum = obstacle.Maximum } } }, "add obstacle");
    }

    public void UpdateObstacle(ObstacleSnapshot obstacle)
    {
        ArgumentNullException.ThrowIfNull(obstacle);
        ValidatePosition(obstacle.Minimum);
        ValidatePosition(obstacle.Maximum);
        if (!float.IsFinite(obstacle.Opacity) || obstacle.Opacity is < 0 or > 1)
            throw new ArgumentOutOfRangeException(nameof(obstacle));
        if (obstacle.Drift is not { Length: 2 } || obstacle.Drift.Any(value => !float.IsFinite(value)))
            throw new ArgumentException("Obstacle drift must contain two finite values.", nameof(obstacle));
        ExecuteBatch(new
        {
            version = 1,
            commands = new[]
            {
                new
                {
                    type = "update_obstacle",
                    obstacle = new
                    {
                        id = obstacle.Id,
                        minimum = obstacle.Minimum,
                        maximum = obstacle.Maximum,
                        color = obstacle.Color,
                        opacity = obstacle.Opacity,
                        drift = obstacle.Drift,
                        vision_policy = obstacle.VisionPolicy,
                        shot_policy = obstacle.ShotPolicy,
                    },
                },
            },
        }, "update obstacle");
    }

    public void RemoveObstacle(uint id) => ExecuteBatch(
        new { version = 1, commands = new[] { new { type = "remove_obstacle", id } } }, "remove obstacle");

    public void AddTeleporter(TeleporterSnapshot teleporter)
    {
        ValidateTeleporter(teleporter);
        ExecuteBatch(new
        {
            version = 1,
            commands = new[] { TeleporterPayload("add_teleporter", teleporter) },
        }, "add teleporter");
    }

    public void UpdateTeleporter(TeleporterSnapshot teleporter)
    {
        ValidateTeleporter(teleporter);
        ExecuteBatch(new
        {
            version = 1,
            commands = new[] { TeleporterPayload("update_teleporter", teleporter) },
        }, "update teleporter");
    }

    public void RemoveTeleporter(uint id) => ExecuteBatch(
        new { version = 1, commands = new[] { new { type = "remove_teleporter", id } } }, "remove teleporter");

    private static object TeleporterPayload(string command, TeleporterSnapshot teleporter)
    {
        var mode = teleporter.Mode.Trim().ToLowerInvariant();
        return new
        {
            type = command,
            id = teleporter.Id,
            center = teleporter.Center,
            radius = teleporter.Radius,
            destination = teleporter.Destination,
            width = teleporter.Radius * 2f,
            height = teleporter.Radius * 2f,
            color = teleporter.Color,
            velocity = teleporter.Velocity,
            path = teleporter.Path,
            inbound_path = teleporter.InboundPath,
            outbound_path = teleporter.OutboundPath,
            inbound = mode is "intrasim" or "inbound",
            outbound = mode == "outbound",
            local = mode == "intrasim",
            internet = mode == "internet",
            drift_horizontal = teleporter.DriftHorizontal,
            drift_vertical = teleporter.DriftVertical,
            fixed_position = teleporter.FixedPosition,
            highlight = teleporter.Highlight,
            filters = new
            {
                teleport_vegetables = teleporter.TeleportVegetables,
                teleport_corpses = teleporter.TeleportCorpses,
                teleport_heterotrophs = teleporter.TeleportHeterotrophs,
            },
            respect_shapes = teleporter.RespectShapes,
            sent = teleporter.Sent,
            received = teleporter.Received,
            poll_interval = teleporter.PollInterval,
            capacity = teleporter.Capacity,
            poll_countdown = teleporter.PollCountdown,
            backflow_limit = teleporter.BackflowLimit,
        };
    }

    private static void ValidateTeleporter(TeleporterSnapshot teleporter)
    {
        ArgumentNullException.ThrowIfNull(teleporter);
        ValidatePosition(teleporter.Center);
        ValidatePosition(teleporter.Destination);
        if (!float.IsFinite(teleporter.Radius) || teleporter.Radius <= 0)
            throw new ArgumentOutOfRangeException(nameof(teleporter));
        if (teleporter.PollInterval == 0 || teleporter.Capacity <= 0)
            throw new ArgumentOutOfRangeException(nameof(teleporter));
        if (teleporter.Mode.Trim().ToLowerInvariant() is not ("intrasim" or "inbound" or "outbound"))
            throw new ArgumentException("Online teleporters are disabled in offline mode.", nameof(teleporter));
    }

    public void AddFileTeleporter(FileTeleporterSnapshot teleporter)
    {
        teleporter = ValidateFileTeleporter(teleporter);
        ExecuteBatch(new
        {
            version = 1,
            commands = new[] { new { type = "add_file_teleporter", teleporter = FileTeleporterPayload(teleporter) } },
        }, "add file teleporter");
    }

    public void UpdateFileTeleporter(FileTeleporterSnapshot teleporter)
    {
        teleporter = ValidateFileTeleporter(teleporter);
        ExecuteBatch(new
        {
            version = 1,
            commands = new[] { new { type = "update_file_teleporter", teleporter = FileTeleporterPayload(teleporter) } },
        }, "update file teleporter");
    }

    public void RemoveFileTeleporter(uint id) => ExecuteBatch(
        new { version = 1, commands = new[] { new { type = "remove_file_teleporter", id } } }, "remove file teleporter");

    private static object FileTeleporterPayload(FileTeleporterSnapshot teleporter) => new
    {
        id = teleporter.Id,
        center = teleporter.Center,
        radius = teleporter.Radius,
        inbound_path = teleporter.InboundPath,
        outbound_path = teleporter.OutboundPath,
        poll_interval = teleporter.PollInterval,
        poll_countdown = teleporter.PollCountdown,
        capacity = teleporter.Capacity,
        filters = new
        {
            teleport_vegetables = teleporter.Filters.TeleportVegetables,
            teleport_corpses = teleporter.Filters.TeleportCorpses,
            teleport_heterotrophs = teleporter.Filters.TeleportHeterotrophs,
        },
        sent = teleporter.Sent,
        received = teleporter.Received,
    };

    private static FileTeleporterSnapshot ValidateFileTeleporter(FileTeleporterSnapshot teleporter)
    {
        ArgumentNullException.ThrowIfNull(teleporter);
        ValidatePosition(teleporter.Center);
        if (!float.IsFinite(teleporter.Radius) || teleporter.Radius <= 0)
            throw new ArgumentOutOfRangeException(nameof(teleporter));
        if (string.IsNullOrWhiteSpace(teleporter.InboundPath) && string.IsNullOrWhiteSpace(teleporter.OutboundPath))
            throw new ArgumentException("A file teleporter needs an inbound or outbound path.", nameof(teleporter));
        if (teleporter.PollInterval == 0 || teleporter.Capacity <= 0)
            throw new ArgumentOutOfRangeException(nameof(teleporter));
        var paths = FileTeleporterPaths.Create(teleporter.InboundPath, teleporter.OutboundPath);
        paths.ValidateExistingDirectories();
        return teleporter with
        {
            InboundPath = paths.InboundDirectory,
            OutboundPath = paths.OutboundDirectory,
        };
    }

    public void SetBrownianMotion(float value)
    {
        if (!float.IsFinite(value) || value < 0) throw new ArgumentOutOfRangeException(nameof(value));
        ExecuteBatch(new { version = 1, commands = new[] { new { type = "set_brownian_motion", value } } }, "set Brownian motion");
    }

    public ushort ResolveMemoryAddress(string name)
    {
        var result = ExecuteBatchForResult(new { version = 1, commands = new[] { new { type = "resolve_memory_address", name } } }, "resolve memory address");
        return result.GetProperty("results")[0].GetProperty("address").GetUInt16();
    }

    public void ExecuteRobots() => ExecuteBatch(new { version = 1, commands = new[] { new { type = "execute_robots" } } }, "execute robots");
    public OrganismKey? FindBestRobot(bool energyWeighting)
        => FindBestRobot(energyWeighting ? 100 : 0);

    public OrganismKey? FindBestRobot(int weightPercent)
    {
        if (weightPercent is < 0 or > 200) throw new ArgumentOutOfRangeException(nameof(weightPercent));
        var value = ExecuteBatchForResult(new { version = 1, commands = new[] { new { type = "find_best_robot", weight_percent = weightPercent } } }, "find best robot")
            .GetProperty("results")[0];
        return value.ValueKind == JsonValueKind.Null ? null : ReadKey(value);
    }
    public RobotSelectionResult SelectBestRobot(int fallbackWeight)
    {
        EnsureAlive();
        if (fallbackWeight is < 0 or > 200) throw new ArgumentOutOfRangeException(nameof(fallbackWeight));
        var mode = Snapshot().LocalMode;
        var canRestart = mode.Status == LocalModeStatus.Running
            && mode.Kind == LocalModeKind.Zerobot && mode.Evolution?.Phase == "evolving";
        // Sample the same Single Timer/Long seed as automatic staged restarts,
        // but the engine stores/uses it only when this action actually restarts.
        ulong? restartSeed = canRestart ? NativeTimerSeed() : null;
        var value = ExecuteBatchForResult(new { version = 1, commands = new[] {
            new { type = "select_best_robot", weight_percent = fallbackWeight, restart_seed = restartSeed }
        } }, "select best robot").GetProperty("results")[0];
        var selected = value.GetProperty("selected");
        return new(selected.ValueKind == JsonValueKind.Null ? null : ReadKey(selected), value.GetProperty("restarted").GetBoolean())
        {
            EffectiveWeight = value.GetProperty("effective_weight").GetUInt16(),
        };
    }

    public void UpdateRecognitionFudging(int mode)
    {
        if (mode is < 0 or > 2) throw new ArgumentOutOfRangeException(nameof(mode));
        ExecuteBatch(new { version = 1, commands = new[] { new { type = "update_recognition_fudging", mode } } }, "update recognition fudging");
    }
    public void UpdateDisqualificationRules(int level)
    {
        if (level is < 0 or > 2) throw new ArgumentOutOfRangeException(nameof(level));
        ExecuteBatch(new { version = 1, commands = new[] { new { type = "update_disqualification_rules", level } } }, "update disqualification rules");
    }
    public void UpdateAdvancedMutationSettings(GlobalAdvancedMutationSettings settings, GlobalMainSettings main) =>
        ExecuteBatch(new { version = 1, commands = new[] { new {
            type = "update_advanced_mutation_settings", settings = NativeCommandSerializer.AdvancedMutation(settings, main),
        } } }, "update advanced mutation settings");

    public void UpdateMainRules(GlobalMainSettings settings) => ExecuteBatch(new { version = 1, commands = new[]
    {
        new { type = "update_main_rules", body_cutoff = settings.BodyCutoff, greedy_reproduction = settings.GreedyEnforcement },
    } }, "update global main rules");

    public void UpdateCostSettings(GlobalSimulationCostSettings settings) =>
        ExecuteBatch(new { version = 1, commands = new[] { new {
            type = "update_cost_settings", costs = NativeCommandSerializer.DnaCosts(settings),
        } } }, "update simulation costs");

    public void UpdateMutationSettings(MutationSettingsUpdate update)
    {
        ArgumentNullException.ThrowIfNull(update);
        if (!float.IsFinite(update.GlobalMultiplier) || update.GlobalMultiplier is < 0 or > 100)
            throw new ArgumentOutOfRangeException(nameof(update));
        if (update.SpeciesRates.Any(value => !float.IsFinite(value.Rate) || value.Rate is < 0 or > 100))
            throw new ArgumentOutOfRangeException(nameof(update));
        ExecuteBatch(NativeCommandSerializer.CreateMutationSettingsBatch(update), "update mutation settings");
    }

    public void UpdateOrganismMutationProfile(uint slot, uint generation, MutationProfileUpdate profile)
    {
        ArgumentNullException.ThrowIfNull(profile);
        ExecuteBatch(
            NativeCommandSerializer.CreateOrganismMutationProfileBatch(
                new OrganismKey(slot, generation), profile),
            "update organism mutation profile");
    }

    public void SetTiesEnabled(bool enabled) => ExecuteBatch(
        new { version = 1, commands = new[] { new { type = "set_ties_enabled", enabled } } },
        "update tie policy");

    public void SetSpeciesColor(uint species, uint color) => ExecuteBatch(
        NativeCommandSerializer.CreateSpeciesColorBatch(species, color),
        "change species color");

    public void MakeNewSpecies(uint slot, uint generation, string name, uint color)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(name);
        ExecuteBatch(new { version = 1, commands = new[] { new { type = "make_new_species", slot, generation, name, color } } }, "make new species");
    }

    public void UpdateLegacyRestrictions(bool robotFixingAllowed, bool asexualReproductionAllowed) => ExecuteBatch(
        new { version = 1, commands = new[] { new { type = "update_legacy_restrictions", robot_fixing_allowed = robotFixingAllowed, asexual_reproduction_allowed = asexualReproductionAllowed } } },
        "update legacy restrictions");

    public void ConfigurePlayerBot(PlayerBotConfig config)
    {
        ArgumentNullException.ThrowIfNull(config);
        foreach (var binding in config.Bindings) binding.Validate();
        ExecuteBatch(new
        {
            version = 1,
            commands = new[]
            {
                new
                {
                    type = "configure_player_bot",
                    enabled = config.Enabled,
                    focus = OrganismPayload(config.Focus),
                    bindings = config.Bindings.Select(binding => new
                    {
                        key = binding.KeyCode,
                        memory_address = binding.MemoryAddress,
                        value = binding.Value,
                        invert = binding.Invert,
                    }).ToArray(),
                },
            },
        }, "configure PlayerBot");
    }

    public void SetPlayerBotKey(uint slot, uint generation, int keyCode, bool pressed)
    {
        if (keyCode is < 0 or > 65_535)
            throw new ArgumentOutOfRangeException(nameof(keyCode));
        ExecuteBatch(new
        {
            version = 1,
            commands = new[] { new { type = "set_player_bot_key", slot, generation, key = keyCode, pressed } },
        }, "update PlayerBot key state");
    }

    public void SetOrganismMemory(uint slot, uint generation, ushort address, int value) => ExecuteBatch(
        new
        {
            version = 1,
            commands = new[] { new { type = "set_organism_memory", slot, generation, address, value } },
        }, "set organism memory");

    public void SetOrganismEnergy(uint slot, uint generation, int energy) => ExecuteBatch(
        new
        {
            version = 1,
            commands = new[] { new { type = "set_organism_energy", slot, generation, energy } },
        }, "set organism energy");

    public void UpdateRestrictionOptions(RestrictionOptions options) => ExecuteBatch(
        new
        {
            version = 1,
            commands = new[]
            {
                new
                {
                    type = "update_restriction_options",
                    options = new
                    {
                        non_repopulating = RestrictionColumnPayload(options.NonRepopulating),
                        repopulating = RestrictionColumnPayload(options.Repopulating),
                    },
                },
            },
        }, "update restriction options");

    public void ConfigureLocalMode(LocalModeConfig config)
    {
        EnsureAlive();
        ArgumentNullException.ThrowIfNull(config);
        ExecuteBatch(new
        {
            version = 1,
            commands = new[]
            {
                new
                {
                    type = "configure_local_mode",
                    config = new
                    {
                        kind = ModeKindName(config.Kind),
                        preset = PresetName(config.Preset),
                        workflow = config.Workflow == LeagueWorkflow.PopulationSeeding
                            ? "population_seeding" : config.Workflow.ToString().ToLowerInvariant(),
                        entrant_order = config.EntrantOrder,
                        initial_ladder = config.InitialLadder,
                        seeding_opponent = config.Workflow == LeagueWorkflow.PopulationSeeding ? null : config.SeedingOpponent,
                        finish_with_ladder = config.FinishWithLadder,
                        staged_evolution = config.StagedEvolution is { } evolution ? new {
                            mode = evolution.Mode switch { EvolutionMode.Eco => "eco", EvolutionMode.Zerobot => "zerobot", _ => "simple_survival" },
                            source_dna = evolution.SourceDna, reduction_unit = evolution.ReductionUnit,
                            source_mutation_profile = evolution.SourceMutationProfile is null ? null : NativeCommandSerializer.MutationProfile(evolution.SourceMutationProfile),
                            cycle_length = evolution.CycleLength, normalize_dna_size = evolution.NormalizeDnaSize,
                            zerobot_initial_dna_length = evolution.ZerobotInitialDnaLength,
                            auto_graphs = evolution.AutoGraphs, fitness_weight = evolution.FitnessWeight,
                        } : null,
                        minimum_rounds = config.MinimumRounds,
                        maximum_rounds = config.MaximumRounds,
                        maximum_cycles = config.MaximumCycles,
                        maximum_population = config.MaximumPopulation,
                        restart_on_extinction = config.RestartOnExtinction,
                        energy_handicap = config.EnergyHandicap,
                        source_species = config.SourceSpecies,
                        selection_population_weight = config.SelectionPopulationWeight,
                        selection_energy_weight = config.SelectionEnergyWeight,
                        restrictions = new
                        {
                            prohibit_information_shots = config.Restrictions.ProhibitInformationShots,
                            prohibit_viruses = config.Restrictions.ProhibitViruses,
                            prohibit_sexual_reproduction = config.Restrictions.ProhibitSexualReproduction,
                            prohibit_cross_species_tie_transfer = config.Restrictions.ProhibitCrossSpeciesTieTransfer,
                            robot_fixing_allowed = config.Restrictions.RobotFixingAllowed,
                            asexual_reproduction_allowed = config.Restrictions.AsexualReproductionAllowed,
                        },
                    },
                },
            },
        }, "configure local mode");
        UpdateRestartClockPolicy(Snapshot().LocalMode);
    }

    public void StartLocalMode() => ExecuteBatch(
        new { version = 1, commands = new object[] { NativeRestartSeedCommand(), new { type = "start_local_mode" } } },
        "start local mode");

    public void RestartLocalMode() => ExecuteBatch(
        new { version = 1, commands = new object[] { NativeRestartSeedCommand(), new { type = "restart_local_mode" } } },
        "restart local mode");

    public void AdjudicateLocalRound(uint? winner, string reason)
    {
        EnsureAlive();
        ArgumentException.ThrowIfNullOrWhiteSpace(reason);
        ExecuteBatch(new
        {
            version = 1,
            commands = new object[] { NativeRestartSeedCommand(), new { type = "adjudicate_local_round", winner, reason } },
        }, "adjudicate local round");
    }

    public void SetOrganismAim(uint slot, uint generation, int aim) => ExecuteBatch(
        new { version = 1, commands = new[] { new { type = "set_organism_aim", slot, generation, aim } } },
        "set organism aim");

    public void ConfigureGraphSampling(GraphSamplingSettings settings) => ExecuteBatch(new
    {
        version = 1,
        commands = new[] { new { type = "configure_graph_sampling", settings = new
        {
            selective = settings.Selective,
            generational = settings.Generational,
            slow = settings.Slow,
        } } },
    }, "configure graph sampling");

    public void UpdateEnvironment(EnvironmentUpdate update)
    {
        ArgumentNullException.ThrowIfNull(update);
        ValidatePosition(update.Gravity);
        ExecuteBatch(NativeCommandSerializer.CreateEnvironmentBatch(update), "update environment");
    }

    private static OrganismKey ReadKey(JsonElement value) => new(
        value.GetProperty("slot").GetUInt32(),
        value.GetProperty("generation").GetUInt32());

    private static void ValidatePosition(float[] position)
    {
        ArgumentNullException.ThrowIfNull(position);
        if (position.Length != 2 || position.Any(value => !float.IsFinite(value)))
            throw new ArgumentException("Position must contain two finite coordinates.", nameof(position));
    }

    private static object? OrganismPayload(OrganismKey? key) => key is { } value
        ? new { slot = value.Slot, generation = value.Generation }
        : null;

    private static object RestrictionColumnPayload(RestrictionColumn value) => new
    {
        kill_multibot = value.KillMultibot,
        kill_disqualified = value.KillDisqualified,
        disable_chloroplasts = value.DisableChloroplasts,
        fixed_in_place = value.FixedInPlace,
        disable_vision = value.DisableVision,
        disable_dna = value.DisableDna,
        disable_reproduction = value.DisableReproduction,
        virus_immune = value.VirusImmune,
        disable_mutations = value.DisableMutations,
        disable_movement = value.DisableMovement,
    };

    private static string ModeKindName(LocalModeKind kind) => kind switch
    {
        LocalModeKind.Contest => "contest",
        LocalModeKind.SimpleSurvival => "simple_survival",
        LocalModeKind.EcoSurvival => "eco_survival",
        LocalModeKind.Zerobot => "zerobot",
        _ => "league",
    };

    private static string PresetName(LeaguePreset preset) => preset switch
    {
        LeaguePreset.F2 => "f2",
        LeaguePreset.F3 => "f3",
        LeaguePreset.Multibot => "multibot",
        LeaguePreset.ShortBot => "short_bot",
        LeaguePreset.SingleGene => "single_gene",
        LeaguePreset.SingleStore => "single_store",
        _ => "f1",
    };

    public WorldConfiguration ReadConfiguration()
    {
        EnsureAlive();
        var buffer = new NativeBuffer();
        ThrowIfFailed(NativeMethods.EngineConfigurationJson(_engine, ref buffer), "read world configuration");
        try { return NativeConfigurationParser.Parse(CopyBuffer(buffer)); }
        finally { NativeMethods.BufferFree(ref buffer); }
    }

    public EngineCapabilities Capabilities()
    {
        EnsureAlive();
        var buffer = new NativeBuffer();
        ThrowIfFailed(NativeMethods.EngineCapabilitiesJson(_engine, ref buffer), "read capabilities");
        try
        {
            using var document = JsonDocument.Parse(CopyBuffer(buffer));
            var root = document.RootElement;
            return new EngineCapabilities(
                root.GetProperty("version").GetInt32(),
                root.GetProperty("active").GetString() ?? "Unknown",
                root.GetProperty("gpu_available").GetBoolean(),
                root.GetProperty("fallback_reason").ValueKind == JsonValueKind.Null
                    ? null
                    : root.GetProperty("fallback_reason").GetString());
        }
        finally
        {
            NativeMethods.BufferFree(ref buffer);
        }
    }

    public float? ReadNativeGaussianSpare()
    {
        EnsureAlive();
        var result = ExecuteBatchForResult(
            new { version = 1, commands = new[] { new { type = "read_native_gaussian_spare" } } },
            "read native Gaussian spare").GetProperty("results")[0];
        return result.ValueKind == JsonValueKind.Null ? null : result.GetSingle();
    }

    public byte ReadNativeSunChange()
    {
        EnsureAlive();
        return ExecuteBatchForResult(
            new { version = 1, commands = new[] { new { type = "read_native_sun_change" } } },
            "read native sun change").GetProperty("results")[0].GetByte();
    }

    public NativeProcessCachesSnapshot ReadNativeProcessCaches()
    {
        EnsureAlive();
        var result = ExecuteBatchForResult(
            new { version = 1, commands = new[] { new { type = "read_native_process_caches" } } },
            "read native process caches").GetProperty("results")[0];
        return result.Deserialize<NativeProcessCachesSnapshot>()
            ?? throw new InvalidDataException("The native engine returned no process caches.");
    }

    public byte[] Save()
    {
        EnsureAlive();
        var buffer = new NativeBuffer();
        ThrowIfFailed(NativeMethods.EngineSave(_engine, ref buffer), "save simulation");
        try { return CopyBuffer(buffer); }
        finally { NativeMethods.BufferFree(ref buffer); }
    }

    public byte[] SaveLegacy()
    {
        EnsureAlive();
        var buffer = new NativeBuffer();
        ThrowIfFailed(NativeMethods.EngineSaveLegacy(_engine, ref buffer), "save legacy simulation");
        try { return CopyBuffer(buffer); }
        finally { NativeMethods.BufferFree(ref buffer); }
    }

    public byte[] SaveWithoutMutations()
    {
        EnsureAlive();
        var buffer = new NativeBuffer();
        ThrowIfFailed(NativeMethods.EngineSaveWithoutMutations(_engine, ref buffer), "save simulation without mutations");
        try { return CopyBuffer(buffer); }
        finally { NativeMethods.BufferFree(ref buffer); }
    }

    public void Load(byte[] save)
    {
        EnsureAlive();
        ArgumentNullException.ThrowIfNull(save);
        if (save.Length == 0) throw new InvalidDataException("Save data is empty.");
        ThrowIfFailed(NativeMethods.EngineLoad(_engine, save, (nuint)save.Length), "load simulation");
        var loaded = Snapshot();
        _worldWidth = loaded.WorldSize[0];
        _worldHeight = loaded.WorldSize[1];
        UpdateRestartClockPolicy(loaded.LocalMode);
    }

    public EngineSnapshot Snapshot()
    {
        EnsureAlive();
        var buffer = new NativeBuffer();
        ThrowIfFailed(NativeMethods.EngineSnapshotJson(_engine, ref buffer), "read snapshot");
        try
        {
            if (buffer.Data == IntPtr.Zero || buffer.Length == 0)
            {
                throw new InvalidDataException("Native engine returned an empty snapshot buffer.");
            }
            var bytes = new byte[checked((int)buffer.Length)];
            Marshal.Copy(buffer.Data, bytes, 0, bytes.Length);
            return NativeSnapshotParser.Parse(Encoding.UTF8.GetString(bytes), Backend);
        }
        finally
        {
            NativeMethods.BufferFree(ref buffer);
        }
    }

    public void Dispose()
    {
        if (_engine == IntPtr.Zero)
        {
            return;
        }
        NativeMethods.EngineDestroy(_engine);
        _engine = IntPtr.Zero;
        GC.SuppressFinalize(this);
    }

    private void EnsureAlive() => ObjectDisposedException.ThrowIf(_engine == IntPtr.Zero, this);

    private ulong NativeTimerSeed()
    {
        // VB6 Timer is a Single; assignment of Timer * 100 to Long rounds ties to even.
        float seconds = (float)_clock.GetLocalNow().TimeOfDay.TotalSeconds;
        return (ulong)Math.Round(seconds * 100f, MidpointRounding.ToEven);
    }

    private object NativeRestartSeedCommand()
    {
        // Keep the existing seed rounding and first clock read. AssignSkin's
        // parameterless Randomize also needs the original, unrounded Single.
        float seconds = (float)_clock.GetLocalNow().TimeOfDay.TotalSeconds;
        var seed = (ulong)Math.Round(seconds * 100f, MidpointRounding.ToEven);
        return _sampleClassicSkinClock
            ? new { type = "set_native_restart_seed", seed, skin_timer_seconds = new[] {
                seconds, (float)_clock.GetLocalNow().TimeOfDay.TotalSeconds } }
            : new { type = "set_native_restart_seed", seed };
    }

    private void UpdateRestartClockPolicy(LocalModeSnapshot mode)
    {
        _sampleRestartClockPerCycle = mode.Evolution is not null
            || mode.Configuration is { Workflow: not LeagueWorkflow.Direct };
        _sampleClassicSkinClock = mode.Configuration?.Workflow == LeagueWorkflow.PopulationSeeding;
    }

    private void ExecuteBatch(object batch, string operation)
    {
        _ = ExecuteBatchForResult(batch, operation);
    }

    private JsonElement ExecuteBatchForResult(object batch, string operation)
    {
        var bytes = JsonSerializer.SerializeToUtf8Bytes(batch);
        var buffer = new NativeBuffer();
        ThrowIfFailed(NativeMethods.EngineCommandBatch(_engine, bytes, (nuint)bytes.Length, ref buffer), operation);
        try
        {
            using var document = JsonDocument.Parse(CopyBuffer(buffer));
            return document.RootElement.Clone();
        }
        finally { NativeMethods.BufferFree(ref buffer); }
    }

    private static byte[] CopyBuffer(NativeBuffer buffer)
    {
        if (buffer.Data == IntPtr.Zero || buffer.Length == 0)
        {
            throw new InvalidDataException("Native engine returned an empty owned buffer.");
        }
        var bytes = new byte[checked((int)buffer.Length)];
        Marshal.Copy(buffer.Data, bytes, 0, bytes.Length);
        return bytes;
    }

    private static void ThrowIfFailed(DbStatus status, string operation)
    {
        if (status != DbStatus.Ok)
        {
            var detail = ReadLastError();
            throw new InvalidOperationException($"Native engine could not {operation}; status {(int)status} ({status}). {detail}");
        }
    }

    private static string ReadLastError()
    {
        var buffer = new NativeBuffer();
        if (NativeMethods.LastErrorJson(ref buffer) != DbStatus.Ok || buffer.Data == IntPtr.Zero) return "No error detail was provided.";
        try
        {
            using var document = JsonDocument.Parse(CopyBuffer(buffer));
            return document.RootElement.GetProperty("message").GetString() ?? "No error detail was provided.";
        }
        finally
        {
            NativeMethods.BufferFree(ref buffer);
        }
    }

    private enum DbStatus
    {
        Ok = 0,
        NullPointer = 1,
        InvalidUtf8 = 2,
        InvalidConfig = 3,
        EngineError = 4,
        InvalidCommand = 5,
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct NativeBuffer
    {
        public IntPtr Data;
        public nuint Length;
        public nuint Capacity;
    }

    private static partial class NativeMethods
    {
        [LibraryImport(LibraryName, EntryPoint = "db_engine_create")]
        internal static partial DbStatus EngineCreate(byte[] config, nuint configLength, out IntPtr engine);

        [LibraryImport(LibraryName, EntryPoint = "db_engine_backend")]
        internal static partial int EngineBackend(IntPtr engine);

        [LibraryImport(LibraryName, EntryPoint = "db_engine_destroy")]
        internal static partial DbStatus EngineDestroy(IntPtr engine);

        [LibraryImport(LibraryName, EntryPoint = "db_engine_tick")]
        internal static partial DbStatus EngineTick(IntPtr engine);

        [LibraryImport(LibraryName, EntryPoint = "db_engine_import_dna")]
        internal static partial DbStatus EngineImportDna(IntPtr engine, byte[] dna, nuint dnaLength);

        [LibraryImport(LibraryName, EntryPoint = "db_engine_command_batch")]
        internal static partial DbStatus EngineCommandBatch(IntPtr engine, byte[] commands, nuint commandLength, ref NativeBuffer buffer);

        [LibraryImport(LibraryName, EntryPoint = "db_engine_capabilities_json")]
        internal static partial DbStatus EngineCapabilitiesJson(IntPtr engine, ref NativeBuffer buffer);

        [LibraryImport(LibraryName, EntryPoint = "db_engine_configuration_json")]
        internal static partial DbStatus EngineConfigurationJson(IntPtr engine, ref NativeBuffer buffer);

        [LibraryImport(LibraryName, EntryPoint = "db_last_error_json")]
        internal static partial DbStatus LastErrorJson(ref NativeBuffer buffer);

        [LibraryImport(LibraryName, EntryPoint = "db_engine_save")]
        internal static partial DbStatus EngineSave(IntPtr engine, ref NativeBuffer buffer);

        [LibraryImport(LibraryName, EntryPoint = "db_engine_save_legacy")]
        internal static partial DbStatus EngineSaveLegacy(IntPtr engine, ref NativeBuffer buffer);

        [LibraryImport(LibraryName, EntryPoint = "db_engine_save_without_mutations")]
        internal static partial DbStatus EngineSaveWithoutMutations(IntPtr engine, ref NativeBuffer buffer);

        [LibraryImport(LibraryName, EntryPoint = "db_engine_load")]
        internal static partial DbStatus EngineLoad(IntPtr engine, byte[] save, nuint saveLength);

        [LibraryImport(LibraryName, EntryPoint = "db_engine_snapshot_json")]
        internal static partial DbStatus EngineSnapshotJson(IntPtr engine, ref NativeBuffer buffer);

        [LibraryImport(LibraryName, EntryPoint = "db_buffer_free")]
        internal static partial DbStatus BufferFree(ref NativeBuffer buffer);
    }
}
