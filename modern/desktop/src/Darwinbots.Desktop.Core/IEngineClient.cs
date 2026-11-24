namespace Darwinbots.Desktop.Core;

public interface IEngineClient : IDisposable
{
    string Backend { get; }
    EngineCapabilities Capabilities();
    void Tick(uint count = 1);
    void Tick(uint count, NativeRedrawCadence nativeRedraw) => Tick(count);
    void RedrawNative() { }
    void RedrawNative(bool afterContinuousCycle) => RedrawNative();
    void TickRecordingDeaths(uint count, Action<IReadOnlyList<DeathRecordSnapshot>> onCommitted) => Tick(count);
    void TickRecordingDeaths(uint count, int weightPercent, Action<IReadOnlyList<DeathRecordSnapshot>> onCommitted,
        NativeRedrawCadence? nativeRedraw = null)
    {
        if (nativeRedraw is { } scheduled) TickRecordingDeaths(count, onCommitted, scheduled);
        else TickRecordingDeaths(count, onCommitted);
    }
    GenomeRecordCapture? CaptureLivingSnp(int weightPercent) => null;
    // A null result means recording removal is unavailable and nothing was removed.
    IReadOnlyList<DeathRecordSnapshot>? RemoveRecordingDeaths(
        IReadOnlyList<OrganismKey> organisms, bool suppressDeathPoffs, bool excludeVegetables, int weightPercent) => null;
    void Tick(uint count, bool requireNativeRepopulationSource, IReadOnlyList<ulong> cancelledLineages,
        NativeRedrawCadence? nativeRedraw = null)
    {
        if (requireNativeRepopulationSource)
            throw new NotSupportedException("Original repopulation sources are unavailable in this engine client.");
        if (nativeRedraw is { } scheduled) Tick(count, scheduled);
        else Tick(count);
    }

    void TickRecordingDeaths(uint count, int weightPercent, IReadOnlyList<ulong> cancelledLineages,
        Action<IReadOnlyList<DeathRecordSnapshot>> onCommitted, NativeRedrawCadence? nativeRedraw = null) =>
        Tick(count, requireNativeRepopulationSource: true, cancelledLineages, nativeRedraw);

    void TickRecordingGraphs(uint count, Action<IReadOnlyList<NativeGraphRecordingSample>> onCommitted,
        bool requireNativeRepopulationSource, IReadOnlyList<ulong> cancelledLineages,
        int weightPercent = 100, Action<IReadOnlyList<DeathRecordSnapshot>>? onDeathsCommitted = null,
        NativeRedrawCadence? nativeRedraw = null)
    {
        if (onDeathsCommitted is null)
            Tick(count, requireNativeRepopulationSource, cancelledLineages, nativeRedraw);
        else if (requireNativeRepopulationSource)
            TickRecordingDeaths(count, weightPercent, cancelledLineages, onDeathsCommitted, nativeRedraw);
        else
            TickRecordingDeaths(count, weightPercent, onDeathsCommitted, nativeRedraw);
    }
    NativeGraphRecordingSample? ReadNativeGraphSample() => null;
    NativeGraphRecordingSample? ReadNativeGraphResetSample() => ReadNativeGraphSample();
    NativeGraphRecordingSample? FeedNativeGraphSample(GraphSamplingSettings settings, bool dueOnly = false) =>
        ReadNativeGraphSample();
    void TickRecordingDeaths(uint count, Action<IReadOnlyList<DeathRecordSnapshot>> onCommitted,
        NativeRedrawCadence nativeRedraw) => TickRecordingDeaths(count, onCommitted);

    IReadOnlyList<NativeConsoleActivationFrame> TickWatchingNativeConsoles(IReadOnlyList<OrganismKey> focuses,
        bool requireNativeRepopulationSource, IReadOnlyList<ulong> cancelledLineages,
        int weightPercent = 100, Action<IReadOnlyList<DeathRecordSnapshot>>? onDeathsCommitted = null,
        Action<IReadOnlyList<NativeGraphRecordingSample>>? onGraphSamplesCommitted = null,
        NativeRedrawCadence? nativeRedraw = null)
    {
        if (onGraphSamplesCommitted is not null)
            TickRecordingGraphs(1, onGraphSamplesCommitted, requireNativeRepopulationSource, cancelledLineages,
                weightPercent, onDeathsCommitted, nativeRedraw);
        else if (onDeathsCommitted is not null && requireNativeRepopulationSource)
            TickRecordingDeaths(1, weightPercent, cancelledLineages, onDeathsCommitted, nativeRedraw);
        else if (onDeathsCommitted is not null)
            TickRecordingDeaths(1, weightPercent, onDeathsCommitted, nativeRedraw);
        else
            Tick(1, requireNativeRepopulationSource, cancelledLineages, nativeRedraw);
        return [];
    }

    DnaImportReport ImportDna(string dna, float[]? position = null);
    int ImportOrganism(ReadOnlyMemory<byte> payload) => throw new NotSupportedException("This engine client does not support organism interchange.");
    DnaImportReport ImportSpecies(SpeciesImport species);
    DnaImportReport InsertSpeciesAt(SpeciesImport species, float[] position);
    void Remove(uint slot, uint generation);
    void RemoveMany(IReadOnlyList<OrganismKey> organisms);
    void RemoveMany(IReadOnlyList<OrganismKey> organisms, bool suppressDeathPoffs)
    {
        if (suppressDeathPoffs)
            throw new NotSupportedException("This engine client does not support suppressing death particles for a connected-organism deletion.");
        RemoveMany(organisms);
    }
    void SetDeathPoffsEnabled(bool enabled) =>
        throw new NotSupportedException("This engine client does not support changing death-particle creation.");
    void Move(uint slot, uint generation, float[] position);
    void DragHighlightedRobots(HighlightedRobotDrag drag) =>
        throw new NotSupportedException("This engine client does not support dragging highlighted robots.");
    OrganismKey CloneOrganism(uint slot, uint generation, float[] position);
    void ReplaceDna(uint slot, uint generation, string dna);
    int TagOrganismsByName(string founderName, string tag) =>
        throw new NotSupportedException("Robot tagging is unavailable in this engine client.");
    string ReadRobotTag(OrganismKey focus) => string.Empty;
    string SetRobotTag(OrganismKey focus, string tag) =>
        throw new NotSupportedException("Robot descriptions are unavailable in this engine client.");
    string ExportDna(uint slot, uint generation);
    string ReadRobotDna(uint slot, uint generation, uint? markerPosition = null) => ExportDna(slot, generation);
    string ExportOriginalDnaFile(uint slot, uint generation) => ExportDna(slot, generation);
    string ExportSpeciesTemplateDna(uint species) =>
        throw new NotSupportedException("This engine client does not support startup species DNA export.");
    IReadOnlyList<OrganismKey> ListLegacyOrganismGroup(OrganismKey seed) =>
        throw new NotSupportedException("This engine client does not support organism group lookup.");
    byte[] ExportOrganismGroup(IReadOnlyList<OrganismKey> organisms) =>
        throw new NotSupportedException("This engine client does not support native organism export.");
    OrganismKey Reproduce(uint firstSlot, uint firstGeneration, uint? secondSlot, uint? secondGeneration, float[] position);
    OrganismKey Reproduce(ManualReproductionRequest request) =>
        Reproduce(
            request.Parent.Slot,
            request.Parent.Generation,
            request.SecondParent?.Slot,
            request.SecondParent?.Generation,
            request.Position);
    OrganismKey? ReproduceNativeRobotInfo(OrganismKey parent) =>
        throw new NotSupportedException("Native Robot Info reproduction is unavailable in this engine client.");
    void SwitchBackend(string backend);
    void AddObstacle(ObstacleSnapshot obstacle);
    (int Added, Db2ShapeDriftOptions Drift)? AddNativePolarIce() => null;
    int? AddNativeSimpleMaze(bool horizontal, MazeDimensions dimensions) => null;
    int? AddNativePatternMaze(MazePreset pattern, MazeDimensions dimensions) => null;
    void AddTrashCompactor(ObstacleSnapshot left, ObstacleSnapshot right)
    {
        AddObstacle(left);
        AddObstacle(right);
    }
    int? AddNativeRandomObstacles(int count, ShapeCreationDimensions? dimensions = null) => null;
    int? AddNativeCenteredObstacle(ShapeCreationDimensions dimensions) => null;
    Db2ShapeDriftOptions? SetShapeDrift(bool? horizontal, bool? vertical, short? rate) => null;
    void UpdateObstacle(ObstacleSnapshot obstacle);
    void RemoveObstacle(uint id);
    void RemoveObstacles(IReadOnlyList<uint> ids)
    {
        foreach (var id in ids) RemoveObstacle(id);
    }
    int? RemoveNativeRandomObstacles() => null;
    void AddTeleporter(TeleporterSnapshot teleporter);
    void AddNativeTeleporter(NativeTeleporterOptions options) =>
        throw new NotSupportedException("Original teleporter creation is unavailable in this engine client.");
    void DragNativeTeleporter(uint id, float[] center) =>
        throw new NotSupportedException("Original teleporter dragging is unavailable in this engine client.");
    void UpdateTeleporter(TeleporterSnapshot teleporter);
    void RemoveTeleporter(uint id);
    void AddFileTeleporter(FileTeleporterSnapshot teleporter);
    void UpdateFileTeleporter(FileTeleporterSnapshot teleporter);
    void RemoveFileTeleporter(uint id);
    void RemoveTeleporters(IReadOnlyList<uint> teleporterIds, IReadOnlyList<uint> fileTeleporterIds)
    {
        foreach (var id in teleporterIds) RemoveTeleporter(id);
        foreach (var id in fileTeleporterIds) RemoveFileTeleporter(id);
    }
    void SetBrownianMotion(float value);
    void UpdateMutationSettings(MutationSettingsUpdate update);
    void UpdateLiveSettings(EnvironmentUpdate environment, MutationSettingsUpdate mutation, GlobalSimulationCostSettings? costs)
    {
        UpdateEnvironment(environment);
        UpdateMutationSettings(mutation);
        if (costs is not null) UpdateCostSettings(costs);
    }
    void UpdateLiveSettings(EnvironmentUpdate environment, MutationSettingsUpdate mutation,
        GlobalSimulationCostSettings? costs, Db2CostPreset? preset)
    {
        if (preset is null) UpdateLiveSettings(environment, mutation, costs);
        else
        {
            UpdateLiveSettings(environment, mutation, null);
            UpdateCostSettings(costs, preset);
        }
    }
    void UpdateGlobalMutationSettings(MutationSettingsUpdate update, GlobalAdvancedMutationSettings? advanced, GlobalMainSettings main)
    {
        UpdateMutationSettings(update);
        if (advanced is not null) UpdateAdvancedMutationSettings(advanced, main);
    }
    void UpdateOrganismMutationProfile(uint slot, uint generation, MutationProfileUpdate profile) =>
        throw new NotSupportedException("This engine client does not support per-organism mutation profiles.");
    void SetShotDecay(bool energy, bool doNotDecay) =>
        throw new NotSupportedException("This engine client does not support changing shot decay.");
    void SetTiesEnabled(bool enabled);
    void UpdateSpeciesSettings(IReadOnlyList<SpeciesImport> species) =>
        throw new NotSupportedException("Species settings are unavailable in this engine client.");
    void SetSpeciesColor(uint species, uint color);
    void SetOrganismColor(uint slot, uint generation, uint color) =>
        throw new NotSupportedException("Robot color editing is unavailable in this engine client.");
    void MakeNewSpecies(uint slot, uint generation, string name, uint color);
    string ForkRobotAndDescendants(uint slot, uint generation) =>
        throw new NotSupportedException("Forking robot descendants is unavailable in this engine client.");
    bool RenameRobotFounder(uint slot, uint generation, string name) => false;
    void UpdateLegacyRestrictions(bool robotFixingAllowed, bool asexualReproductionAllowed);
    void ConfigurePlayerBot(PlayerBotConfig config);
    void ConfigureNativePostDnaMemoryWatch(ulong requestId, OrganismKey? focus, bool captureMemory) { }
    void ConfigureGeneActivationWatch(ulong watchId, bool enabled, OrganismKey? focus) { }
    void SetPlayerBotKey(uint slot, uint generation, int keyCode, bool pressed);
    void SetPlayerBotKeyState(int keyCode, bool pressed) =>
        throw new NotSupportedException("This engine client does not support independent PlayerBot key state.");
    void SetPlayerBotTargets(OrganismKey? focus, IReadOnlyList<OrganismKey> highlights) { }
    void SetPlayerBotPointer(float[]? position) { }
    void SetOrganismMemory(uint slot, uint generation, ushort address, int value);
    ushort ResolveMemoryAddress(string name) => ushort.TryParse(name, out var address) && address is >= 1 and <= 1000
        ? address : throw new FormatException($"Unknown memory address '{name}'.");
    ushort ResolveMemoryAddress(string name, OrganismKey focus) => ResolveMemoryAddress(name);
    void ExecuteRobots() => throw new NotSupportedException("DNA-only execution is unavailable.");
    OrganismKey? FindBestRobot(bool energyWeighting) => null;
    OrganismKey? FindBestRobot(int weightPercent) => FindBestRobot(weightPercent > 0);
    RobotSelectionResult SelectBestRobot(int fallbackWeight) => new(FindBestRobot(fallbackWeight), false);
    void UpdateMainRules(GlobalMainSettings settings) { }
    void UpdateAdvancedMutationSettings(GlobalAdvancedMutationSettings settings, GlobalMainSettings main) { }
    void UpdateCostSettings(GlobalSimulationCostSettings settings) { }
    void UpdateCostSettings(GlobalSimulationCostSettings? settings, Db2CostPreset? preset)
    {
        if (settings is not null) UpdateCostSettings(settings);
    }
    void UpdateDisqualificationRules(int level) { }
    void UpdateRecognitionFudging(int mode) { }
    void SetOrganismEnergy(uint slot, uint generation, int energy);
    void SetNativeConsoleEnergy(OrganismKey focus, float energy) =>
        throw new NotSupportedException("This engine client does not support native console energy.");
    void UpdateRestrictionOptions(RestrictionOptions options);
    void ApplyNativeRestrictionOverwrites(RestrictionOptions options) => UpdateRestrictionOptions(options);
    void ConfigureLocalMode(LocalModeConfig config);
    void StartLocalMode();
    void RestartLocalMode();
    void SetOrdinaryRestart(bool enabled) =>
        throw new NotSupportedException("Ordinary simulation restart is unavailable in this engine client.");
    void FinishOrdinaryRestart() =>
        throw new NotSupportedException("Ordinary simulation restart is unavailable in this engine client.");
    NativeRestartSources ReadNativeRestartSources() => NativeRestartSources.Empty;
    bool HasUnresolvedNativeRepopulationSources() => false;
    int BindNativeRestartSources(IReadOnlyList<NativeRestartBinding> sources) => 0;
    int BindNativeRestartSources(IReadOnlyList<NativeRestartBinding> sources, bool preserveExisting) =>
        preserveExisting ? 0 : BindNativeRestartSources(sources);
    void ResumeRestartMode() => throw new NotSupportedException("This engine client cannot resume a restart mode.");
    void ClearRestartModeHiding() { }
    void AdjudicateLocalRound(uint? winner, string reason);
    void SetOrganismAim(uint slot, uint generation, int aim);
    void UpdateEnvironment(EnvironmentUpdate update);
    void UpdateLiveSettings(EnvironmentUpdate environment, MutationSettingsUpdate mutation)
    {
        UpdateEnvironment(environment);
        UpdateMutationSettings(mutation);
    }
    void AcceptNativeAdvancedSettings(EnvironmentUpdate? environment, MutationSettingsUpdate? mutation,
        GlobalSimulationCostSettings? costs, Db2CostPreset? preset)
    {
        if (environment is not null && mutation is not null) UpdateLiveSettings(environment, mutation, costs, preset);
        else if (costs is not null || preset is not null) UpdateCostSettings(costs, preset);
    }

    void ConfigureGraphSampling(GraphSamplingSettings settings)
    {
        if (settings.Selective || settings.Generational || settings.Slow)
            throw new NotSupportedException("Native graph sampling is unavailable.");
    }
    NativeGraphRecordingState? ReadNativeGraphRecording() => null;
    void UpdateNativeGraphRecording(string? runStamp, IReadOnlyList<NativeGraphRecordingRow> graphs) { }
    NativeGraphLayoutState? ReadNativeGraphLayout() => null;
    void UpdateNativeGraphLayout(IReadOnlyList<NativeGraphLayoutRow> graphs,
        IReadOnlyList<NativeGraphCustomQuery>? customQueries = null) { }
    byte[] Save();
    byte[] SaveWithoutRestartMode() => throw new NotSupportedException("This engine client cannot save without restart mode state.");
    byte[] SaveLegacy() => throw new NotSupportedException("This engine client does not support legacy simulation export.");
    byte[] SaveLegacyWithoutMutationDetails() => throw new NotSupportedException("This engine client does not support legacy simulation export without mutation details.");
    byte[] SaveWithoutMutations();
    int CaptureNativeLoadSeed(bool generateSeed) => 0;
    void SetNativeSeedMetadata(int seed) { }
    void Load(byte[] save);
    void LoadWithNativeSeed(byte[] save, int seed) => Load(save);
    EngineSnapshot Snapshot();
    WorldConfiguration? ReadConfiguration() => null;
    uint? ReadNativeSetupUniformState() => null;
    NativeSpeciesAppearance? StageNativeSpeciesAppearance(string filename, string dna, bool useOldColors) => null;
    float? ReadNativeGaussianSpare() => null;
    byte ReadNativeSunChange() => 0;
    bool? ToggleNativeSun() => null;
    NativeProcessCachesSnapshot ReadNativeProcessCaches() => NativeProcessCachesSnapshot.Empty;
    string ReadRobotTagReport() => throw new NotSupportedException("Robot tag information is unavailable in this engine client.");
}
