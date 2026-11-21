using System.Collections.Concurrent;
using System.Diagnostics;

namespace Darwinbots.Desktop.Core;

public sealed class SimulationSession : IAsyncDisposable
{
    private readonly IEngineClient _engine;
    private readonly BlockingCollection<SessionCommand> _commands = new();
    private readonly Thread _worker;
    private bool _disposed;
    private readonly Stopwatch _performanceClock = Stopwatch.StartNew();
    private ulong? _lastPublishedTick;
    private float[]? _pendingPlayerBotPointer;
    private float[]? _appliedPlayerBotPointer;

    public SimulationSession(IEngineClient engine)
    {
        _engine = engine;
        _worker = new Thread(ProcessCommands)
        {
            IsBackground = true,
            Name = "Darwinbots simulation",
        };
        _worker.Start();
    }

    public EngineSnapshot LatestSnapshot { get; private set; } = EngineSnapshot.Empty;

    public event Action<EngineSnapshot>? SnapshotPublished;

    public Task RefreshSnapshotAsync(CancellationToken cancellationToken = default) =>
        Enqueue(_ => { }, cancellationToken);

    public Task StepAsync(uint count = 1, CancellationToken cancellationToken = default) =>
        Enqueue(engine =>
        {
            ApplyPlayerBotPointer(engine);
            engine.Tick(count);
        }, cancellationToken, measureTicks: true);

    public Task SetPlayerBotTargetsAsync(OrganismKey? focus, IReadOnlyList<OrganismKey> highlights,
        CancellationToken cancellationToken = default)
    {
        var targets = highlights.ToArray();
        return Enqueue(engine => engine.SetPlayerBotTargets(focus, targets), cancellationToken);
    }

    public void SetPlayerBotPointer(float[]? position)
    {
        if (position is not null) ValidatePosition(position);
        Volatile.Write(ref _pendingPlayerBotPointer, position?.ToArray());
    }

    private void ApplyPlayerBotPointer(IEngineClient engine)
    {
        // Pointer motion can outpace the simulation worker. Apply its latest
        // position just before ticking, including a pending button release.
        var position = Volatile.Read(ref _pendingPlayerBotPointer);
        if (ReferenceEquals(_appliedPlayerBotPointer, position))
            return;
        engine.SetPlayerBotPointer(position);
        _appliedPlayerBotPointer = position;
    }

    public Task StepRecordingDeathsAsync(
        uint count, Action<IReadOnlyList<DeathRecordSnapshot>> onCommitted, CancellationToken cancellationToken = default) =>
        StepRecordingDeathsAsync(count, 100, onCommitted, cancellationToken);

    public Task StepRecordingDeathsAsync(uint count, int weightPercent,
        Action<IReadOnlyList<DeathRecordSnapshot>> onCommitted, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.TickRecordingDeaths(count, weightPercent, onCommitted), cancellationToken, measureTicks: true);

    public async Task<OrganismKey?> FindBestRobotAsync(bool energyWeighting, CancellationToken cancellationToken = default)
        => await FindBestRobotAsync(energyWeighting ? 100 : 0, cancellationToken).ConfigureAwait(false);

    public async Task<OrganismKey?> FindBestRobotAsync(int weightPercent, CancellationToken cancellationToken = default)
    {
        OrganismKey? result = null;
        await Enqueue(engine => result = engine.FindBestRobot(weightPercent), cancellationToken).ConfigureAwait(false);
        return result;
    }
    public async Task<RobotSelectionResult> SelectBestRobotAsync(int fallbackWeight, CancellationToken cancellationToken = default)
    {
        RobotSelectionResult result = new(null, false);
        await Enqueue(engine => result = engine.SelectBestRobot(fallbackWeight), cancellationToken).ConfigureAwait(false);
        return result;
    }

    public Task UpdateRecognitionFudgingAsync(int mode, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateRecognitionFudging(mode), cancellationToken);

    public Task UpdateDisqualificationRulesAsync(int level, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateDisqualificationRules(level), cancellationToken);

    public Task UpdateMainRulesAsync(GlobalMainSettings settings, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateMainRules(settings), cancellationToken);

    public Task UpdateAdvancedMutationSettingsAsync(GlobalAdvancedMutationSettings settings, GlobalMainSettings main,
        CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateAdvancedMutationSettings(settings, main), cancellationToken);

    public Task UpdateCostSettingsAsync(GlobalSimulationCostSettings settings, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateCostSettings(settings), cancellationToken);

    public Task SetGlobalCostMultiplierAsync(double multiplier, CancellationToken cancellationToken = default) =>
        Enqueue(engine =>
        {
            var configuration = engine.ReadConfiguration()
                ?? throw new InvalidOperationException("The simulation's current costs are unavailable.");
            engine.UpdateCostSettings(configuration.Costs with { GlobalMultiplier = multiplier });
        }, cancellationToken);

    public async Task<DnaImportReport> ImportDnaAsync(
        string dna,
        float[]? position = null,
        CancellationToken cancellationToken = default)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(dna);
        DnaImportReport? report = null;
        await Enqueue(engine => report = engine.ImportDna(dna, position), cancellationToken).ConfigureAwait(false);
        return report ?? DnaImportReport.Compatible;
    }

    public async Task<int> ImportOrganismAsync(
        ReadOnlyMemory<byte> payload,
        CancellationToken cancellationToken = default)
    {
        if (payload.IsEmpty) throw new ArgumentException("Organism interchange payload is empty.", nameof(payload));
        var imported = 0;
        await Enqueue(engine => imported = engine.ImportOrganism(payload), cancellationToken).ConfigureAwait(false);
        return imported;
    }

    public async Task<DnaImportReport> ImportSpeciesAsync(SpeciesImport species, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(species);
        species.Validate();
        DnaImportReport? report = null;
        await Enqueue(engine => report = engine.ImportSpecies(species), cancellationToken).ConfigureAwait(false);
        return report ?? DnaImportReport.Compatible;
    }

    public async Task<DnaImportReport> InsertSpeciesAtAsync(
        SpeciesImport species,
        float[] position,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(species);
        species.Validate();
        ValidatePosition(position);
        DnaImportReport? report = null;
        float[] requestedPosition = [.. position];
        await Enqueue(engine => report = engine.InsertSpeciesAt(species, requestedPosition), cancellationToken)
            .ConfigureAwait(false);
        return report ?? DnaImportReport.Compatible;
    }

    public Task RemoveAsync(uint slot, uint generation, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.Remove(slot, generation), cancellationToken);

    public Task RemoveManyAsync(IReadOnlyList<OrganismKey> organisms, CancellationToken cancellationToken = default) =>
        RemoveManyAsync(organisms, false, cancellationToken);

    public Task RemoveManyAsync(IReadOnlyList<OrganismKey> organisms, bool suppressDeathPoffs, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(organisms);
        if (organisms.Count == 0)
            throw new ArgumentException("At least one organism is required.", nameof(organisms));
        OrganismKey[] stableOrganisms = [.. organisms];
        return Enqueue(engine => engine.RemoveMany(stableOrganisms, suppressDeathPoffs), cancellationToken);
    }

    public async Task<(EngineSnapshot Snapshot, IReadOnlyDictionary<OrganismKey, string> Dna)> RemoveWithDnaAsync(
        IReadOnlyList<OrganismKey> organisms,
        bool suppressDeathPoffs,
        bool excludeVegetables,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(organisms);
        if (organisms.Count == 0)
            throw new ArgumentException("At least one organism is required.", nameof(organisms));
        var stableKeys = organisms.ToArray();
        var snapshot = EngineSnapshot.Empty;
        var dna = new Dictionary<OrganismKey, string>();
        await Enqueue(engine =>
        {
            // Read the still-live state and remove it at one session boundary.
            snapshot = engine.Snapshot();
            dna = CaptureSelectedDna(engine, snapshot, stableKeys, excludeVegetables);
            RemoveSelected(engine, stableKeys, suppressDeathPoffs);
        }, cancellationToken).ConfigureAwait(false);
        return (snapshot, dna);
    }

    public async Task<IReadOnlyList<DeathRecordSnapshot>> RemoveRecordingDeathsAsync(
        IReadOnlyList<OrganismKey> organisms, bool suppressDeathPoffs, bool excludeVegetables,
        int weightPercent, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(organisms);
        if (organisms.Count == 0)
            throw new ArgumentException("At least one organism is required.", nameof(organisms));
        var stableKeys = organisms.ToArray();
        IReadOnlyList<DeathRecordSnapshot> records = [];
        await Enqueue(engine =>
        {
            if (engine.RemoveRecordingDeaths(stableKeys, suppressDeathPoffs, excludeVegetables, weightPercent) is { } native)
            {
                records = native;
                return;
            }
            var snapshot = engine.Snapshot();
            var dna = CaptureSelectedDna(engine, snapshot, stableKeys, excludeVegetables);
            records = Db2GenomeRecordFile.ProjectLiving(snapshot, dna).ToArray();
            RemoveSelected(engine, stableKeys, suppressDeathPoffs);
        }, cancellationToken).ConfigureAwait(false);
        return records;
    }

    private static Dictionary<OrganismKey, string> CaptureSelectedDna(IEngineClient engine, EngineSnapshot snapshot,
        IReadOnlyList<OrganismKey> keys, bool excludeVegetables)
    {
        var dna = new Dictionary<OrganismKey, string>();
        foreach (var key in keys)
        {
            var robot = snapshot.Organisms.FirstOrDefault(value => value.Slot == key.Slot && value.Generation == key.Generation)
                ?? throw new InvalidDataException("A selected robot is no longer available.");
            if (robot.DnaLength == 1 || (excludeVegetables && robot.Vegetable)) continue;
            var source = engine.ExportDna(key.Slot, key.Generation);
            if (!string.IsNullOrWhiteSpace(source)) dna[key] = source;
        }
        return dna;
    }

    private static void RemoveSelected(IEngineClient engine, OrganismKey[] keys, bool suppressDeathPoffs)
    {
        if (keys.Length == 1 && !suppressDeathPoffs)
            engine.Remove(keys[0].Slot, keys[0].Generation);
        else
            engine.RemoveMany(keys, suppressDeathPoffs);
    }

    public Task SetDeathPoffsEnabledAsync(bool enabled, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.SetDeathPoffsEnabled(enabled), cancellationToken);

    public async Task<(EngineSnapshot Snapshot, IReadOnlyDictionary<OrganismKey, string> Dna)> CaptureLivingGenomeRecordsAsync(
        CancellationToken cancellationToken = default)
    {
        var snapshot = EngineSnapshot.Empty;
        var dna = new Dictionary<OrganismKey, string>();
        await Enqueue(engine =>
        {
            snapshot = engine.Snapshot();
            foreach (var robot in snapshot.Organisms)
                dna[new OrganismKey(robot.Slot, robot.Generation)] = engine.ExportDna(robot.Slot, robot.Generation);
        }, cancellationToken).ConfigureAwait(false);
        return (snapshot, dna);
    }

    public async Task<GenomeRecordCapture> CaptureLivingSnpAsync(int weightPercent, CancellationToken cancellationToken = default)
    {
        GenomeRecordCapture capture = new(0, []);
        await Enqueue(engine =>
        {
            if (engine.CaptureLivingSnp(weightPercent) is { } native)
            {
                capture = native;
                return;
            }
            var snapshot = engine.Snapshot();
            var dna = new Dictionary<OrganismKey, string>();
            foreach (var robot in snapshot.Organisms)
            {
                if (robot.DnaLength == 1) continue;
                dna[new OrganismKey(robot.Slot, robot.Generation)] = engine.ExportDna(robot.Slot, robot.Generation);
            }
            capture = new(snapshot.Tick, Db2GenomeRecordFile.ProjectLiving(snapshot, dna).ToArray());
        }, cancellationToken).ConfigureAwait(false);
        return capture;
    }

    public Task MoveAsync(uint slot, uint generation, float[] position, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.Move(slot, generation, position), cancellationToken);

    public Task DragHighlightedRobotsAsync(HighlightedRobotDrag drag) =>
        Enqueue(engine => engine.DragHighlightedRobots(drag), CancellationToken.None);

    public async Task<OrganismKey> CloneAsync(uint slot, uint generation, float[] position, CancellationToken cancellationToken = default)
    {
        OrganismKey? result = null;
        await Enqueue(engine => result = engine.CloneOrganism(slot, generation, position), cancellationToken).ConfigureAwait(false);
        return result ?? throw new InvalidDataException("Engine returned no clone ID.");
    }

    public async Task<OrganismKey> CloneWithDnaAsync(
        uint slot,
        uint generation,
        float[] position,
        string dna,
        CancellationToken cancellationToken = default)
    {
        var clone = await CloneAsync(slot, generation, position, cancellationToken).ConfigureAwait(false);
        await ReplaceDnaAsync(clone.Slot, clone.Generation, dna, cancellationToken).ConfigureAwait(false);
        return clone;
    }

    public Task ReplaceDnaAsync(uint slot, uint generation, string dna, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.ReplaceDna(slot, generation, dna), cancellationToken);

    public async Task<int> TagOrganismsByNameAsync(
        string founderName, string tag, CancellationToken cancellationToken = default)
    {
        var count = 0;
        await Enqueue(engine => count = engine.TagOrganismsByName(founderName, tag), cancellationToken)
            .ConfigureAwait(false);
        return count;
    }

    public async Task<string> ExportDnaAsync(uint slot, uint generation, CancellationToken cancellationToken = default)
    {
        string? result = null;
        await Enqueue(engine => result = engine.ExportDna(slot, generation), cancellationToken).ConfigureAwait(false);
        return result ?? throw new InvalidDataException("Engine returned no DNA.");
    }

    public async Task<Db2RobotFile> ExportRobotFileAsync(
        uint slot, uint generation, CancellationToken cancellationToken = default)
    {
        Db2RobotFile? result = null;
        await Enqueue(engine =>
        {
            var dna = engine.ExportDna(slot, generation);
            var robot = engine.Snapshot().Organisms.SingleOrDefault(value => value.Slot == slot && value.Generation == generation)
                ?? throw new InvalidDataException("The selected robot is no longer available.");
            result = new Db2RobotFile(dna, robot.MutationProfile);
        }, cancellationToken).ConfigureAwait(false);
        return result ?? throw new InvalidDataException("Engine returned no robot DNA file.");
    }

    public async Task<string> ExportSpeciesTemplateDnaAsync(uint species, CancellationToken cancellationToken = default)
    {
        string? result = null;
        await Enqueue(engine => result = engine.ExportSpeciesTemplateDna(species), cancellationToken).ConfigureAwait(false);
        return result ?? throw new InvalidDataException("Engine returned no startup species DNA.");
    }

    public async Task<IReadOnlyList<OrganismKey>> ListLegacyOrganismGroupAsync(OrganismKey seed)
    {
        IReadOnlyList<OrganismKey> members = [];
        await Enqueue(engine => members = engine.ListLegacyOrganismGroup(seed), CancellationToken.None)
            .ConfigureAwait(false);
        return members;
    }

    public async Task<byte[]> ExportOrganismGroupAsync(IReadOnlyList<OrganismKey> organisms, CancellationToken cancellationToken = default)
    {
        var captured = organisms.ToArray();
        byte[]? bytes = null;
        await Enqueue(engine => bytes = engine.ExportOrganismGroup(captured), cancellationToken).ConfigureAwait(false);
        return bytes ?? throw new InvalidDataException("Engine returned no organism group.");
    }

    public async Task<OrganismKey> ReproduceAsync(
        uint firstSlot, uint firstGeneration, uint? secondSlot, uint? secondGeneration,
        float[] position, CancellationToken cancellationToken = default)
    {
        OrganismKey? result = null;
        await Enqueue(engine => result = engine.Reproduce(firstSlot, firstGeneration, secondSlot, secondGeneration, position), cancellationToken).ConfigureAwait(false);
        return result ?? throw new InvalidDataException("Engine returned no child ID.");
    }

    public async Task<OrganismKey> ReproduceAsync(
        ManualReproductionRequest request,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        request.Validate();
        OrganismKey? result = null;
        await Enqueue(engine => result = engine.Reproduce(request), cancellationToken).ConfigureAwait(false);
        return result ?? throw new InvalidDataException("Engine returned no child ID.");
    }

    public Task SwitchBackendAsync(string backend, CancellationToken cancellationToken = default)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(backend);
        return Enqueue(engine => engine.SwitchBackend(backend), cancellationToken);
    }

    public Task AddObstacleAsync(ObstacleSnapshot obstacle, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.AddObstacle(obstacle), cancellationToken);

    public Task AddTrashCompactorAsync(ObstacleSnapshot left, ObstacleSnapshot right, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.AddTrashCompactor(left, right), cancellationToken);

    public async Task<int?> AddNativeRandomObstaclesAsync(
        int count,
        ShapeCreationDimensions dimensions,
        CancellationToken cancellationToken = default)
    {
        int? added = null;
        await Enqueue(engine => added = engine.AddNativeRandomObstacles(count, dimensions), cancellationToken).ConfigureAwait(false);
        return added;
    }

    public async Task<int?> AddNativeCenteredObstacleAsync(
        ShapeCreationDimensions dimensions,
        CancellationToken cancellationToken = default)
    {
        int? added = null;
        await Enqueue(engine => added = engine.AddNativeCenteredObstacle(dimensions), cancellationToken).ConfigureAwait(false);
        return added;
    }

    public async Task<(int Added, Db2ShapeDriftOptions Drift)?> AddNativePolarIceAsync(CancellationToken cancellationToken = default)
    {
        (int Added, Db2ShapeDriftOptions Drift)? result = null;
        await Enqueue(engine => result = engine.AddNativePolarIce(), cancellationToken).ConfigureAwait(false);
        return result;
    }

    public async Task<int?> AddNativeSimpleMazeAsync(bool horizontal, MazeDimensions dimensions, CancellationToken cancellationToken = default)
    {
        int? result = null;
        await Enqueue(engine => result = engine.AddNativeSimpleMaze(horizontal, dimensions), cancellationToken).ConfigureAwait(false);
        return result;
    }

    public async Task<int?> AddNativePatternMazeAsync(MazePreset pattern, MazeDimensions dimensions, CancellationToken cancellationToken = default)
    {
        int? result = null;
        await Enqueue(engine => result = engine.AddNativePatternMaze(pattern, dimensions), cancellationToken).ConfigureAwait(false);
        return result;
    }

    public async Task<Db2ShapeDriftOptions?> SetShapeDriftAsync(
        bool? horizontal, bool? vertical, short? rate, CancellationToken cancellationToken = default)
    {
        Db2ShapeDriftOptions? settings = null;
        await Enqueue(engine => settings = engine.SetShapeDrift(horizontal, vertical, rate), cancellationToken).ConfigureAwait(false);
        return settings;
    }

    public Task UpdateObstacleAsync(ObstacleSnapshot obstacle, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateObstacle(obstacle), cancellationToken);

    public Task RemoveObstacleAsync(uint id, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.RemoveObstacle(id), cancellationToken);

    public Task RemoveObstaclesAsync(IReadOnlyList<uint> ids, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.RemoveObstacles(ids), cancellationToken);

    public async Task<int?> RemoveNativeRandomObstaclesAsync(CancellationToken cancellationToken = default)
    {
        int? removed = null;
        await Enqueue(engine => removed = engine.RemoveNativeRandomObstacles(), cancellationToken).ConfigureAwait(false);
        return removed;
    }

    public Task AddTeleporterAsync(TeleporterSnapshot teleporter, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.AddTeleporter(teleporter), cancellationToken);

    public Task AddNativeTeleporterAsync(NativeTeleporterOptions options, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.AddNativeTeleporter(options), cancellationToken);

    public Task DragNativeTeleporterAsync(uint id, float[] center, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.DragNativeTeleporter(id, center), cancellationToken);

    public Task UpdateTeleporterAsync(TeleporterSnapshot teleporter, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateTeleporter(teleporter), cancellationToken);

    public Task RemoveTeleporterAsync(uint id, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.RemoveTeleporter(id), cancellationToken);

    public Task AddFileTeleporterAsync(FileTeleporterSnapshot teleporter, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.AddFileTeleporter(teleporter), cancellationToken);

    public Task UpdateFileTeleporterAsync(FileTeleporterSnapshot teleporter, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateFileTeleporter(teleporter), cancellationToken);

    public Task RemoveFileTeleporterAsync(uint id, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.RemoveFileTeleporter(id), cancellationToken);

    public Task SetBrownianMotionAsync(float value, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.SetBrownianMotion(value), cancellationToken);

    public Task UpdateMutationSettingsAsync(MutationSettingsUpdate update, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateMutationSettings(update), cancellationToken);

    public Task UpdateGlobalMutationSettingsAsync(MutationSettingsUpdate update, GlobalAdvancedMutationSettings? advanced,
        GlobalMainSettings main, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateGlobalMutationSettings(update, advanced, main), cancellationToken);

    public Task UpdateOrganismMutationProfileAsync(
        uint slot,
        uint generation,
        MutationProfileUpdate profile,
        CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateOrganismMutationProfile(slot, generation, profile), cancellationToken);

    public Task SetShotDecayAsync(bool energy, bool doNotDecay, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.SetShotDecay(energy, doNotDecay), cancellationToken);

    public Task SetTiesEnabledAsync(bool enabled, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.SetTiesEnabled(enabled), cancellationToken);

    public Task UpdateSpeciesSettingsAsync(IReadOnlyList<SpeciesImport> species, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateSpeciesSettings(species), cancellationToken);

    public Task SetSpeciesColorAsync(uint species, uint color, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.SetSpeciesColor(species, color), cancellationToken);

    public Task SetOrganismColorAsync(uint slot, uint generation, uint color, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.SetOrganismColor(slot, generation, color), cancellationToken);

    public Task MakeNewSpeciesAsync(uint slot, uint generation, string name, uint color, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.MakeNewSpecies(slot, generation, name, color), cancellationToken);

    public async Task<string> ForkRobotAndDescendantsAsync(uint slot, uint generation, CancellationToken cancellationToken = default)
    {
        var name = string.Empty;
        await Enqueue(engine => name = engine.ForkRobotAndDescendants(slot, generation), cancellationToken).ConfigureAwait(false);
        return name;
    }

    public Task UpdateLegacyRestrictionsAsync(bool robotFixingAllowed, bool asexualReproductionAllowed, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateLegacyRestrictions(robotFixingAllowed, asexualReproductionAllowed), cancellationToken);

    public Task ConfigurePlayerBotAsync(PlayerBotConfig config, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(config);
        foreach (var binding in config.Bindings) binding.Validate();
        return Enqueue(engine => engine.ConfigurePlayerBot(config), cancellationToken);
    }

    public Task SetPlayerBotKeyAsync(OrganismKey focus, int keyCode, bool pressed, CancellationToken cancellationToken = default)
    {
        if (keyCode is < 0 or > 65_535)
            throw new ArgumentOutOfRangeException(nameof(keyCode));
        return Enqueue(engine => engine.SetPlayerBotKey(focus.Slot, focus.Generation, keyCode, pressed), cancellationToken);
    }

    public Task SetPlayerBotKeyStateAsync(int keyCode, bool pressed, CancellationToken cancellationToken = default)
    {
        if (keyCode is < 0 or > 65_535)
            throw new ArgumentOutOfRangeException(nameof(keyCode));
        return Enqueue(engine => engine.SetPlayerBotKeyState(keyCode, pressed), cancellationToken);
    }

    public Task SetOrganismMemoryAsync(uint slot, uint generation, ushort address, int value, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.SetOrganismMemory(slot, generation, address, value), cancellationToken);

    public async Task<ushort> ResolveMemoryAddressAsync(string name, CancellationToken cancellationToken = default)
    {
        ushort address = 0;
        await Enqueue(engine => address = engine.ResolveMemoryAddress(name), cancellationToken).ConfigureAwait(false);
        return address;
    }

    public Task ExecuteRobotsAsync(CancellationToken cancellationToken = default) => Enqueue(engine => engine.ExecuteRobots(), cancellationToken);

    public Task SetOrganismEnergyAsync(uint slot, uint generation, int energy, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.SetOrganismEnergy(slot, generation, energy), cancellationToken);

    public Task UpdateRestrictionOptionsAsync(RestrictionOptions options, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(options);
        return Enqueue(engine => engine.UpdateRestrictionOptions(options), cancellationToken);
    }

    public Task ConfigureLocalModeAsync(LocalModeConfig config, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(config);
        return Enqueue(engine => engine.ConfigureLocalMode(config), cancellationToken);
    }

    public Task StartLocalModeAsync(CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.StartLocalMode(), cancellationToken);

    public Task RestartLocalModeAsync(CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.RestartLocalMode(), cancellationToken);

    public Task SetOrdinaryRestartAsync(bool enabled, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.SetOrdinaryRestart(enabled), cancellationToken);

    public Task FinishOrdinaryRestartAsync(CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.FinishOrdinaryRestart(), cancellationToken);

    public Task AdjudicateLocalRoundAsync(uint? winner, string reason, CancellationToken cancellationToken = default)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(reason);
        return Enqueue(engine => engine.AdjudicateLocalRound(winner, reason), cancellationToken);
    }

    public Task SetOrganismAimAsync(uint slot, uint generation, int aim, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.SetOrganismAim(slot, generation, aim), cancellationToken);

    public Task ConfigureGraphSamplingAsync(GraphSamplingSettings settings, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.ConfigureGraphSampling(settings), cancellationToken);

    public Task UpdateEnvironmentAsync(EnvironmentUpdate update, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateEnvironment(update), cancellationToken);

    public Task UpdateLiveSettingsAsync(EnvironmentUpdate environment, MutationSettingsUpdate mutation,
        GlobalSimulationCostSettings? costs, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateLiveSettings(environment, mutation, costs), cancellationToken);

    public async Task<WorldConfiguration?> ReadConfigurationAsync(CancellationToken cancellationToken = default)
    {
        WorldConfiguration? configuration = null;
        await Enqueue(engine => configuration = engine.ReadConfiguration(), cancellationToken).ConfigureAwait(false);
        return configuration;
    }

    public async Task<byte[]> SaveAsync(CancellationToken cancellationToken = default)
    {
        byte[]? save = null;
        await Enqueue(engine => save = engine.Save(), cancellationToken).ConfigureAwait(false);
        return save ?? throw new InvalidDataException("Engine returned no save data.");
    }

    public async Task<string> ReadRobotTagReportAsync(CancellationToken cancellationToken = default)
    {
        string? report = null;
        await Enqueue(engine => report = engine.ReadRobotTagReport(), cancellationToken).ConfigureAwait(false);
        return report ?? throw new InvalidDataException("Engine returned no robot tag information.");
    }

    public async Task<uint?> ReadNativeSetupUniformStateAsync()
    {
        uint? state = null;
        await Enqueue(engine => state = engine.ReadNativeSetupUniformState(), CancellationToken.None, publishSnapshot: false)
            .ConfigureAwait(false);
        return state;
    }

    public async Task<NativeSpeciesAppearance?> StageNativeSpeciesAppearanceAsync(string filename, string dna, bool useOldColors)
    {
        NativeSpeciesAppearance? appearance = null;
        await Enqueue(engine => appearance = engine.StageNativeSpeciesAppearance(filename, dna, useOldColors),
            CancellationToken.None, publishSnapshot: false).ConfigureAwait(false);
        return appearance;
    }

    public async Task<float?> ReadNativeGaussianSpareAsync(CancellationToken cancellationToken = default)
    {
        float? spare = null;
        await Enqueue(engine => spare = engine.ReadNativeGaussianSpare(), cancellationToken).ConfigureAwait(false);
        return spare;
    }

    public async Task<(float? GaussianSpare, byte SunChange)> ReadNativeProcessContextAsync(CancellationToken cancellationToken = default)
    {
        (float? GaussianSpare, byte SunChange) context = default;
        await Enqueue(engine => context = (engine.ReadNativeGaussianSpare(), engine.ReadNativeSunChange()), cancellationToken).ConfigureAwait(false);
        return context;
    }

    public async Task<bool?> ToggleNativeSunAsync(CancellationToken cancellationToken = default)
    {
        bool? enabled = null;
        await Enqueue(engine => enabled = engine.ToggleNativeSun(), cancellationToken).ConfigureAwait(false);
        return enabled;
    }

    public async Task<NativeProcessStateSnapshot> ReadNativeProcessStateAsync(CancellationToken cancellationToken = default)
    {
        NativeProcessStateSnapshot? context = null;
        await Enqueue(engine => context = new NativeProcessStateSnapshot(
            engine.ReadNativeGaussianSpare(), engine.ReadNativeSunChange(), engine.ReadNativeProcessCaches()),
            cancellationToken).ConfigureAwait(false);
        return context ?? throw new InvalidDataException("Engine returned no native process state.");
    }

    public async Task<byte[]> SaveLegacyAsync(CancellationToken cancellationToken = default)
    {
        byte[]? save = null;
        await Enqueue(engine => save = engine.SaveLegacy(), cancellationToken).ConfigureAwait(false);
        return save ?? throw new InvalidDataException("Engine returned no legacy save data.");
    }

    public async Task<byte[]> SaveWithoutMutationsAsync(CancellationToken cancellationToken = default)
    {
        byte[]? save = null;
        await Enqueue(engine => save = engine.SaveWithoutMutations(), cancellationToken).ConfigureAwait(false);
        return save ?? throw new InvalidDataException("Engine returned no mutation-free save data.");
    }

    public async Task<byte[]> SaveLegacyWithoutMutationDetailsAsync(CancellationToken cancellationToken = default)
    {
        byte[]? save = null;
        await Enqueue(engine => save = engine.SaveLegacyWithoutMutationDetails(), cancellationToken).ConfigureAwait(false);
        return save ?? throw new InvalidDataException("Engine returned no legacy save data.");
    }

    public Task LoadAsync(byte[] save, CancellationToken cancellationToken = default) =>
        LoadWithNativeRestartSourcesAsync(save, null, cancellationToken);

    public Task LoadWithNativeRestartSourcesAsync(byte[] save, Func<NativeRestartSource, string?>? resolveSource,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(save);
        return Enqueue(engine =>
        {
            engine.Load(save);
            _appliedPlayerBotPointer = null;
            SetPlayerBotPointer(null);
            // Modern checkpoints already contain their original source bindings.
            if (resolveSource is not null && !save.AsSpan().StartsWith("DB3S"u8))
            {
                var restart = engine.ReadNativeRestartSources();
                if (restart.Requested || restart.Sources.Count > 0)
                {
                    var bindings = new List<NativeRestartBinding>();
                    foreach (var source in restart.Sources)
                        if (resolveSource(source) is { } dna)
                            bindings.Add(new(source.LineageId, dna));
                    engine.BindNativeRestartSources(bindings);
                }
            }
        }, cancellationToken);
    }

    private Task Enqueue(Action<IEngineClient> action, CancellationToken cancellationToken, bool measureTicks = false,
        bool publishSnapshot = true)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        cancellationToken.ThrowIfCancellationRequested();
        var completion = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var command = new SessionCommand(action, completion, cancellationToken, measureTicks, publishSnapshot);
        _commands.Add(command, cancellationToken);
        return completion.Task;
    }

    private static void ValidatePosition(float[] position)
    {
        ArgumentNullException.ThrowIfNull(position);
        if (position.Length != 2 || position.Any(value => !float.IsFinite(value)))
            throw new ArgumentException("World position must contain two finite coordinates.", nameof(position));
    }

    private void ProcessCommands()
    {
        foreach (var command in _commands.GetConsumingEnumerable())
        {
            if (command.CancellationToken.IsCancellationRequested)
            {
                command.Completion.TrySetCanceled(command.CancellationToken);
                continue;
            }

            try
            {
                command.Action(_engine);
                if (!command.PublishSnapshot)
                {
                    command.Completion.TrySetResult();
                    continue;
                }
                var snapshot = _engine.Snapshot();
                var elapsed = _performanceClock.Elapsed.TotalSeconds;
                var advanced = command.MeasureTicks && _lastPublishedTick is { } previous && snapshot.Tick >= previous
                    ? snapshot.Tick - previous
                    : 0;
                if (elapsed > 0)
                    snapshot = snapshot with {
                        TicksPerSecond = advanced / elapsed,
                        SnapshotsPerSecond = 1.0 / elapsed,
                    };
                _lastPublishedTick = snapshot.Tick;
                _performanceClock.Restart();
                LatestSnapshot = snapshot;
                SnapshotPublished?.Invoke(snapshot);
                command.Completion.TrySetResult();
            }
            catch (Exception error)
            {
                command.Completion.TrySetException(error);
            }
        }
    }

    public ValueTask DisposeAsync()
    {
        if (_disposed)
        {
            return ValueTask.CompletedTask;
        }

        _disposed = true;
        _commands.CompleteAdding();
        _worker.Join();
        _commands.Dispose();
        _engine.Dispose();
        return ValueTask.CompletedTask;
    }

    private sealed record SessionCommand(
        Action<IEngineClient> Action,
        TaskCompletionSource Completion,
        CancellationToken CancellationToken,
        bool MeasureTicks,
        bool PublishSnapshot);
}
