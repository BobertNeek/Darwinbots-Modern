namespace Darwinbots.Desktop.Core;

public interface IEngineClient : IDisposable
{
    string Backend { get; }
    EngineCapabilities Capabilities();
    void Tick(uint count = 1);
    DnaImportReport ImportDna(string dna, float[]? position = null);
    int ImportOrganism(ReadOnlyMemory<byte> payload) => throw new NotSupportedException("This engine client does not support organism interchange.");
    DnaImportReport ImportSpecies(SpeciesImport species);
    DnaImportReport InsertSpeciesAt(SpeciesImport species, float[] position);
    void Remove(uint slot, uint generation);
    void RemoveMany(IReadOnlyList<OrganismKey> organisms);
    void Move(uint slot, uint generation, float[] position);
    OrganismKey CloneOrganism(uint slot, uint generation, float[] position);
    void ReplaceDna(uint slot, uint generation, string dna);
    string ExportDna(uint slot, uint generation);
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
    void SwitchBackend(string backend);
    void AddObstacle(ObstacleSnapshot obstacle);
    void UpdateObstacle(ObstacleSnapshot obstacle);
    void RemoveObstacle(uint id);
    void AddTeleporter(TeleporterSnapshot teleporter);
    void UpdateTeleporter(TeleporterSnapshot teleporter);
    void RemoveTeleporter(uint id);
    void AddFileTeleporter(FileTeleporterSnapshot teleporter);
    void UpdateFileTeleporter(FileTeleporterSnapshot teleporter);
    void RemoveFileTeleporter(uint id);
    void SetBrownianMotion(float value);
    void UpdateMutationSettings(MutationSettingsUpdate update);
    void UpdateOrganismMutationProfile(uint slot, uint generation, MutationProfileUpdate profile) =>
        throw new NotSupportedException("This engine client does not support per-organism mutation profiles.");
    void SetTiesEnabled(bool enabled);
    void SetSpeciesColor(uint species, uint color);
    void MakeNewSpecies(uint slot, uint generation, string name, uint color);
    void UpdateLegacyRestrictions(bool robotFixingAllowed, bool asexualReproductionAllowed);
    void ConfigurePlayerBot(PlayerBotConfig config);
    void SetPlayerBotKey(uint slot, uint generation, int keyCode, bool pressed);
    void SetOrganismMemory(uint slot, uint generation, ushort address, int value);
    ushort ResolveMemoryAddress(string name) => ushort.TryParse(name, out var address) && address is >= 1 and <= 1000
        ? address : throw new FormatException($"Unknown memory address '{name}'.");
    void ExecuteRobots() => throw new NotSupportedException("DNA-only execution is unavailable.");
    OrganismKey? FindBestRobot(bool energyWeighting) => null;
    OrganismKey? FindBestRobot(int weightPercent) => FindBestRobot(weightPercent > 0);
    void UpdateMainRules(GlobalMainSettings settings) { }
    void UpdateAdvancedMutationSettings(GlobalAdvancedMutationSettings settings, GlobalMainSettings main) { }
    void UpdateCostSettings(GlobalSimulationCostSettings settings) { }
    void UpdateDisqualificationRules(int level) { }
    void UpdateRecognitionFudging(int mode) { }
    void SetOrganismEnergy(uint slot, uint generation, int energy);
    void UpdateRestrictionOptions(RestrictionOptions options);
    void ConfigureLocalMode(LocalModeConfig config);
    void StartLocalMode();
    void RestartLocalMode();
    void AdjudicateLocalRound(uint? winner, string reason);
    void SetOrganismAim(uint slot, uint generation, int aim);
    void UpdateEnvironment(EnvironmentUpdate update);
    void ConfigureGraphSampling(GraphSamplingSettings settings)
    {
        if (settings.Selective || settings.Generational || settings.Slow)
            throw new NotSupportedException("Native graph sampling is unavailable.");
    }
    byte[] Save();
    byte[] SaveLegacy() => throw new NotSupportedException("This engine client does not support legacy simulation export.");
    byte[] SaveWithoutMutations();
    void Load(byte[] save);
    EngineSnapshot Snapshot();
    WorldConfiguration? ReadConfiguration() => null;
}
