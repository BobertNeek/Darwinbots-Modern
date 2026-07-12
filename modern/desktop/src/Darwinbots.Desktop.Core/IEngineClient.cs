namespace Darwinbots.Desktop.Core;

public interface IEngineClient : IDisposable
{
    string Backend { get; }
    EngineCapabilities Capabilities();
    void Tick(uint count = 1);
    DnaImportReport ImportDna(string dna, float[]? position = null);
    DnaImportReport ImportSpecies(SpeciesImport species);
    void Remove(uint slot, uint generation);
    void Move(uint slot, uint generation, float[] position);
    OrganismKey CloneOrganism(uint slot, uint generation, float[] position);
    void ReplaceDna(uint slot, uint generation, string dna);
    string ExportDna(uint slot, uint generation);
    OrganismKey Reproduce(uint firstSlot, uint firstGeneration, uint? secondSlot, uint? secondGeneration, float[] position);
    void SwitchBackend(string backend);
    void AddObstacle(ObstacleSnapshot obstacle);
    void RemoveObstacle(uint id);
    void AddTeleporter(TeleporterSnapshot teleporter);
    void RemoveTeleporter(uint id);
    void SetBrownianMotion(float value);
    void UpdateEnvironment(EnvironmentUpdate update);
    byte[] Save();
    void Load(byte[] save);
    EngineSnapshot Snapshot();
}
