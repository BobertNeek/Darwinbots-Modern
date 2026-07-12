using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class SimulationSessionTests
{
    [Fact]
    public async Task StepPublishesTheImmutableSnapshotReturnedByTheEngine()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);
        EngineSnapshot? published = null;
        session.SnapshotPublished += snapshot => published = snapshot;

        await session.StepAsync();

        Assert.NotNull(published);
        Assert.Equal(1UL, published.Tick);
        Assert.Equal(1UL, session.LatestSnapshot.Tick);
    }

    [Fact]
    public async Task CommandsAreSerializedOnOneDedicatedWorker()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);

        await Task.WhenAll(Enumerable.Range(0, 20).Select(_ => session.StepAsync()));

        Assert.Equal(20UL, session.LatestSnapshot.Tick);
        Assert.Single(engine.ExecutionThreads);
    }

    [Fact]
    public async Task ImportSendsDnaToTheEngineAndPublishesPopulation()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);

        await session.ImportDnaAsync("start\nstop");

        Assert.Equal("start\nstop", engine.LastDna);
        Assert.Equal(1, session.LatestSnapshot.Population);
    }

    [Fact]
    public async Task ImportForwardsTheRequestedWorldPosition()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);

        await session.ImportDnaAsync("start\nstop", [8000f, 6000f]);

        Assert.Equal([8000f, 6000f], engine.LastImportPosition!);
    }

    [Fact]
    public async Task SpeciesImportIsForwardedAsOneSerializedEngineCommand()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);
        var species = new SpeciesImport(
            "Alga Minimalis",
            "start\nstop",
            true,
            0xff4c963b,
            3,
            2_500,
            10,
            true,
            0.5f);

        await session.ImportSpeciesAsync(species);

        Assert.Equal(species, engine.LastSpeciesImport);
        Assert.Equal(3, session.LatestSnapshot.Population);
    }

    [Fact]
    public async Task SpeciesImportReturnsCompatibilityWarningsToTheCaller()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);
        var species = new SpeciesImport("Legacy", "start\nstop", false, 0xff000000, 1, 1000, 0, false, 1f);

        var report = await session.ImportSpeciesAsync(species);

        Assert.Contains(report.CompatibilityWarnings, warning => warning.Contains("legacy warning"));
    }

    [Fact]
    public async Task RemoveOrganismRunsOnTheEngineThreadAndPublishesPopulation()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);
        await session.ImportDnaAsync("start\nstop");

        await session.RemoveAsync(0, 0);

        Assert.Equal((0U, 0U), engine.LastRemoved);
        Assert.Equal(0, session.LatestSnapshot.Population);
        Assert.Single(engine.ExecutionThreads);
    }

    [Fact]
    public async Task ManualToolsReturnIdsAndDnaThroughTheSession()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);
        await session.ImportDnaAsync("start\nstop");

        await session.MoveAsync(0, 0, [25f, 30f]);
        var clone = await session.CloneAsync(0, 0, [35f, 30f]);
        await session.ReplaceDnaAsync(0, 0, "start\n10 .up store\nstop");
        var dna = await session.ExportDnaAsync(0, 0);
        var child = await session.ReproduceAsync(0, 0, clone.Slot, clone.Generation, [30f, 30f]);

        Assert.Equal(new OrganismKey(1, 0), clone);
        Assert.Equal(new OrganismKey(2, 0), child);
        Assert.Contains(".up", dna);
        Assert.Equal([25f, 30f], engine.LastMovePosition!);
        Assert.Equal(3, session.LatestSnapshot.Population);
    }

    [Fact]
    public async Task BackendSwitchRunsAtTheSerializedCommandBoundary()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);

        await session.SwitchBackendAsync("Gpu");

        Assert.Equal("GPU", session.LatestSnapshot.Backend);
        Assert.Single(engine.ExecutionThreads);
    }

    [Fact]
    public async Task EnvironmentEditsRunOnTheDedicatedEngineThread()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);
        var obstacle = new ObstacleSnapshot(7, [10f, 20f], [30f, 40f]);
        var teleporter = new TeleporterSnapshot(8, [50f, 60f], 15f, [500f, 600f]);

        await session.AddObstacleAsync(obstacle);
        await session.AddTeleporterAsync(teleporter);
        await session.RemoveObstacleAsync(7);
        await session.RemoveTeleporterAsync(8);

        Assert.Equal(obstacle, engine.LastObstacleAdded);
        Assert.Equal(teleporter, engine.LastTeleporterAdded);
        Assert.Equal(7U, engine.LastObstacleRemoved);
        Assert.Equal(8U, engine.LastTeleporterRemoved);
        Assert.Single(engine.ExecutionThreads);
    }

    [Fact]
    public async Task RuntimeBrownianUpdateRunsOnTheDedicatedEngineThread()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);

        await session.SetBrownianMotionAsync(0f);

        Assert.Equal(0f, engine.LastBrownianMotion);
        Assert.Single(engine.ExecutionThreads);
    }

    [Fact]
    public async Task CompleteEnvironmentUpdateRunsAtomicallyOnTheEngineThread()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);
        var update = new EnvironmentUpdate(0, 10, 200, [0f, 2f], 0.5f, 3f);

        await session.UpdateEnvironmentAsync(update);

        Assert.Equal(update, engine.LastEnvironmentUpdate);
        Assert.Single(engine.ExecutionThreads);
    }

    [Fact]
    public async Task SaveAndLoadRunOnEngineThreadAndRepublishLoadedSnapshot()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);
        await session.StepAsync(7);
        var save = await session.SaveAsync();
        await session.StepAsync(5);
        Assert.Equal(12UL, session.LatestSnapshot.Tick);

        await session.LoadAsync(save);

        Assert.Equal(7UL, session.LatestSnapshot.Tick);
        Assert.Single(engine.ExecutionThreads);
    }
}

internal sealed class FakeEngineClient : IEngineClient
{
    private ulong _tick;
    private int _population;

    public string Backend { get; private set; } = "CPU";
    public string? LastDna { get; private set; }
    public float[]? LastImportPosition { get; private set; }
    public SpeciesImport? LastSpeciesImport { get; private set; }
    public (uint Slot, uint Generation)? LastRemoved { get; private set; }
    public float[]? LastMovePosition { get; private set; }
    public ObstacleSnapshot? LastObstacleAdded { get; private set; }
    public TeleporterSnapshot? LastTeleporterAdded { get; private set; }
    public uint? LastObstacleRemoved { get; private set; }
    public uint? LastTeleporterRemoved { get; private set; }
    public float? LastBrownianMotion { get; private set; }
    public EnvironmentUpdate? LastEnvironmentUpdate { get; private set; }
    private string _dna = "start\nstop";
    public HashSet<int> ExecutionThreads { get; } = [];

    public EngineSnapshot Snapshot()
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        return new EngineSnapshot(_tick, _population, Backend, []);
    }

    public EngineCapabilities Capabilities() => new(1, Backend, false, null);

    public byte[] Save() => BitConverter.GetBytes(_tick);

    public void Load(byte[] save) => _tick = BitConverter.ToUInt64(save);

    public void Tick(uint count = 1)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        _tick += count;
    }

    public DnaImportReport ImportDna(string dna, float[]? position = null)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastDna = dna;
        LastImportPosition = position;
        _population++;
        return new DnaImportReport(["legacy warning"]);
    }

    public DnaImportReport ImportSpecies(SpeciesImport species)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastSpeciesImport = species;
        _population += species.InitialCount;
        return new DnaImportReport(["legacy warning"]);
    }

    public void Remove(uint slot, uint generation)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastRemoved = (slot, generation);
        _population = Math.Max(0, _population - 1);
    }

    public void Move(uint slot, uint generation, float[] position) => LastMovePosition = position;

    public OrganismKey CloneOrganism(uint slot, uint generation, float[] position)
    {
        _population++;
        return new OrganismKey((uint)(_population - 1), 0);
    }

    public void ReplaceDna(uint slot, uint generation, string dna) => _dna = dna;

    public string ExportDna(uint slot, uint generation) => _dna;

    public OrganismKey Reproduce(uint firstSlot, uint firstGeneration, uint? secondSlot, uint? secondGeneration, float[] position)
    {
        _population++;
        return new OrganismKey((uint)(_population - 1), 0);
    }

    public void SwitchBackend(string backend)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        Backend = backend.ToUpperInvariant();
    }

    public void AddObstacle(ObstacleSnapshot obstacle)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastObstacleAdded = obstacle;
    }

    public void RemoveObstacle(uint id)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastObstacleRemoved = id;
    }

    public void AddTeleporter(TeleporterSnapshot teleporter)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastTeleporterAdded = teleporter;
    }

    public void RemoveTeleporter(uint id)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastTeleporterRemoved = id;
    }

    public void SetBrownianMotion(float value)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastBrownianMotion = value;
    }

    public void UpdateEnvironment(EnvironmentUpdate update)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastEnvironmentUpdate = update;
    }

    public void Dispose() { }
}
