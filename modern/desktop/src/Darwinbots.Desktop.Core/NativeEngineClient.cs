using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;

namespace Darwinbots.Desktop.Core;

public sealed partial class NativeEngineClient : IEngineClient
{
    private const string LibraryName = "darwinbots_engine";
    private IntPtr _engine;
    private readonly float _worldWidth;
    private readonly float _worldHeight;
    private readonly SeededSpawnPlacement _spawnPlacement;

    public NativeEngineClient(WorldSetupOptions options) : this(
        options.Backend,
        options.PopulationCapacity,
        options.Seed,
        options.WorldWidth,
        options.WorldHeight,
        options.EffectiveMetabolismCost,
        options.VegetableEnergyPerTick,
        options.SunlightEnergy,
        options.Gravity,
        options.Drag,
        options.BrownianMotion,
        options.VegetablePopulationCap,
        options.Physics,
        options.Shots,
        options.Vegetation,
        options.AutoSpeciation,
        options.SpeciationGeneticDistancePercent,
        options.ToroidalWorld)
    {
    }

    public NativeEngineClient(
        string backend = "Auto",
        int capacity = 100_000,
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
        bool toroidalWorld = true)
    {
        _worldWidth = worldWidth;
        _worldHeight = worldHeight;
        _spawnPlacement = new SeededSpawnPlacement(seed);
        gravity ??= [0f, 0f];
        physics ??= Db2PhysicsOptions.Default;
        shots ??= Db2ShotOptions.Default;
        vegetation ??= Db2VegetationOptions.Default;
        ValidatePosition(gravity);
        var bytes = JsonSerializer.SerializeToUtf8Bytes(new
        {
            seed,
            organism_capacity = capacity,
            vegetable_population_cap = vegetablePopulationCap,
            world_width = worldWidth,
            world_height = worldHeight,
            backend,
            allow_cpu_fallback = true,
            metabolism_cost = metabolismCost,
            vegetable_energy_per_tick = vegetableEnergyPerTick,
            sunlight_energy = sunlightEnergy,
            gravity,
            drag,
            brownian_motion = brownianMotion,
            physics = NativeCommandSerializer.Physics(physics),
            shots = NativeCommandSerializer.Shots(shots),
            vegetation = NativeCommandSerializer.Vegetation(vegetation),
            auto_speciation = autoSpeciation,
            speciation_genetic_distance_percent = speciationGeneticDistancePercent,
            toroidal_world = toroidalWorld,
        });
        ThrowIfFailed(NativeMethods.EngineCreate(bytes, (nuint)bytes.Length, out _engine), "create engine");
        if (_engine == IntPtr.Zero)
        {
            throw new InvalidOperationException("Native engine creation returned a null handle.");
        }
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
        ExecuteBatch(new { version = 1, commands = new[] { new { type = "tick", count } } }, "advance simulation");
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
                    minimum_population = species.MinimumPopulation,
                    reseed = species.Reseed,
                    mutation_rate = species.MutationRate,
                    initial_energy = species.InitialEnergy,
                    positions = _spawnPlacement.Next(species.InitialCount, _worldWidth, _worldHeight),
                },
            },
        }, "import species");
        return ReadImportReport(result.GetProperty("results")[0]);
    }

    private static DnaImportReport ReadImportReport(JsonElement value)
    {
        if (!value.TryGetProperty("compatibility_warnings", out var warnings)) return DnaImportReport.Compatible;
        return new DnaImportReport(warnings.EnumerateArray().Select(item => item.GetString() ?? string.Empty)
            .Where(item => item.Length > 0).ToArray());
    }

    public void Remove(uint slot, uint generation)
    {
        EnsureAlive();
        ExecuteBatch(new
        {
            version = 1,
            commands = new[] { new { type = "remove", slot, generation } },
        }, "remove organism");
    }

    public void RemoveMany(IReadOnlyList<OrganismKey> organisms)
    {
        ArgumentNullException.ThrowIfNull(organisms);
        if (organisms.Count == 0)
            throw new ArgumentException("At least one organism is required.", nameof(organisms));
        EnsureAlive();
        ExecuteBatch(NativeCommandSerializer.CreateRemoveManyBatch(organisms), "remove organisms");
    }

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

    public void RemoveObstacle(uint id) => ExecuteBatch(
        new { version = 1, commands = new[] { new { type = "remove_obstacle", id } } }, "remove obstacle");

    public void AddTeleporter(TeleporterSnapshot teleporter)
    {
        ArgumentNullException.ThrowIfNull(teleporter);
        ValidatePosition(teleporter.Center);
        ValidatePosition(teleporter.Destination);
        if (!float.IsFinite(teleporter.Radius) || teleporter.Radius <= 0) throw new ArgumentOutOfRangeException(nameof(teleporter));
        ExecuteBatch(new { version = 1, commands = new[] { new { type = "add_teleporter", id = teleporter.Id, center = teleporter.Center, radius = teleporter.Radius, destination = teleporter.Destination } } }, "add teleporter");
    }

    public void RemoveTeleporter(uint id) => ExecuteBatch(
        new { version = 1, commands = new[] { new { type = "remove_teleporter", id } } }, "remove teleporter");

    public void SetBrownianMotion(float value)
    {
        if (!float.IsFinite(value) || value < 0) throw new ArgumentOutOfRangeException(nameof(value));
        ExecuteBatch(new { version = 1, commands = new[] { new { type = "set_brownian_motion", value } } }, "set Brownian motion");
    }

    public void UpdateMutationSettings(MutationSettingsUpdate update)
    {
        ArgumentNullException.ThrowIfNull(update);
        if (!float.IsFinite(update.GlobalMultiplier) || update.GlobalMultiplier is < 0 or > 100)
            throw new ArgumentOutOfRangeException(nameof(update));
        if (update.SpeciesRates.Any(value => !float.IsFinite(value.Rate) || value.Rate is < 0 or > 100))
            throw new ArgumentOutOfRangeException(nameof(update));
        ExecuteBatch(new
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
                },
            },
        }, "update mutation settings");
    }

    public void SetTiesEnabled(bool enabled) => ExecuteBatch(
        new { version = 1, commands = new[] { new { type = "set_ties_enabled", enabled } } },
        "update tie policy");

    public void SetSpeciesColor(uint species, uint color) => ExecuteBatch(
        new { version = 1, commands = new[] { new { type = "set_species_color", species, color } } },
        "change species color");

    public void MakeNewSpecies(uint slot, uint generation, string name, uint color)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(name);
        ExecuteBatch(new { version = 1, commands = new[] { new { type = "make_new_species", slot, generation, name, color } } }, "make new species");
    }

    public void UpdateLegacyRestrictions(bool robotFixingAllowed, bool asexualReproductionAllowed) => ExecuteBatch(
        new { version = 1, commands = new[] { new { type = "update_legacy_restrictions", robot_fixing_allowed = robotFixingAllowed, asexual_reproduction_allowed = asexualReproductionAllowed } } },
        "update legacy restrictions");

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
                        minimum_rounds = config.MinimumRounds,
                        maximum_rounds = config.MaximumRounds,
                        maximum_cycles = config.MaximumCycles,
                        maximum_population = config.MaximumPopulation,
                        restart_on_extinction = config.RestartOnExtinction,
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
    }

    public void StartLocalMode() => ExecuteBatch(
        new { version = 1, commands = new[] { new { type = "start_local_mode" } } },
        "start local mode");

    public void RestartLocalMode() => ExecuteBatch(
        new { version = 1, commands = new[] { new { type = "restart_local_mode" } } },
        "restart local mode");

    public void AdjudicateLocalRound(uint? winner, string reason)
    {
        EnsureAlive();
        ArgumentException.ThrowIfNullOrWhiteSpace(reason);
        ExecuteBatch(new
        {
            version = 1,
            commands = new[] { new { type = "adjudicate_local_round", winner, reason } },
        }, "adjudicate local round");
    }

    public void SetOrganismAim(uint slot, uint generation, int aim) => ExecuteBatch(
        new { version = 1, commands = new[] { new { type = "set_organism_aim", slot, generation, aim } } },
        "set organism aim");

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

    private static string ModeKindName(LocalModeKind kind) => kind switch
    {
        LocalModeKind.Contest => "contest",
        LocalModeKind.SimpleSurvival => "simple_survival",
        LocalModeKind.EcoSurvival => "eco_survival",
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

    public byte[] Save()
    {
        EnsureAlive();
        var buffer = new NativeBuffer();
        ThrowIfFailed(NativeMethods.EngineSave(_engine, ref buffer), "save simulation");
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

        [LibraryImport(LibraryName, EntryPoint = "db_last_error_json")]
        internal static partial DbStatus LastErrorJson(ref NativeBuffer buffer);

        [LibraryImport(LibraryName, EntryPoint = "db_engine_save")]
        internal static partial DbStatus EngineSave(IntPtr engine, ref NativeBuffer buffer);

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
