using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class SimulationSessionTests
{
    [Fact]
    public async Task StartupTemplateExportIsQueuedBehindEarlierCommandsWithoutAdvancingTheWorld()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);
        await session.ImportDnaAsync("start 1 .up store stop");
        await session.ReplaceDnaAsync(0, 0, "start 2 .up store stop");

        Assert.Equal("start 1 .up store stop", await session.ExportSpeciesTemplateDnaAsync(7));
        Assert.Equal("start 2 .up store stop", await session.ExportDnaAsync(0, 0));
        Assert.Equal(7u, engine.LastTemplateSpecies);
        Assert.Equal(0ul, session.LatestSnapshot.Tick);
        Assert.Single(engine.ExecutionThreads);
    }

    [Fact]
    public async Task RefreshPublishesExistingWorldWithoutAdvancingIt()
    {
        var engine = new FakeEngineClient();
        engine.Tick(42);
        await using var session = new SimulationSession(engine);
        EngineSnapshot? published = null;
        session.SnapshotPublished += snapshot => published = snapshot;

        await session.RefreshSnapshotAsync();

        Assert.NotNull(published);
        Assert.Equal(42UL, published.Tick);
        Assert.Same(published, session.LatestSnapshot);
        Assert.Equal(42UL, engine.Snapshot().Tick);
        Assert.Equal(0, session.LatestSnapshot.TicksPerSecond);

        await session.StepAsync();
        Assert.Equal(43UL, session.LatestSnapshot.Tick);
        Assert.True(session.LatestSnapshot.TicksPerSecond > 0);

        await session.LoadAsync(BitConverter.GetBytes(10UL));
        Assert.Equal(10UL, session.LatestSnapshot.Tick);
        Assert.Equal(0, session.LatestSnapshot.TicksPerSecond);

        await session.StepAsync();
        Assert.Equal(11UL, session.LatestSnapshot.Tick);
        Assert.True(session.LatestSnapshot.TicksPerSecond > 0);
    }

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
    public async Task NativeProcessContextReadWaitsForEarlierWorkOnTheSameEngineThread()
    {
        using var release = new ManualResetEventSlim();
        var started = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var engine = new FakeEngineClient { NativeGaussianSpare = 1.25f };
        engine.BeforeTick = () =>
        {
            started.SetResult();
            if (!release.Wait(TimeSpan.FromSeconds(10))) throw new TimeoutException("Gated engine tick was not released.");
            engine.NativeGaussianSpare = 2.5f;
            engine.NativeSunChange = 12;
        };
        await using var session = new SimulationSession(engine);
        var step = session.StepAsync();
        Task<(float? GaussianSpare, byte SunChange)> read;
        try
        {
            await started.Task.WaitAsync(TimeSpan.FromSeconds(10));
            read = session.ReadNativeProcessContextAsync();
            Assert.False(read.IsCompleted);
        }
        finally { release.Set(); }
        await step;

        var context = await read;
        Assert.Equal(2.5f, context.GaussianSpare);
        Assert.Equal((byte)12, context.SunChange);
        Assert.Equal(1ul, session.LatestSnapshot.Tick);
        Assert.Single(engine.ExecutionThreads);
        Assert.NotEqual(Environment.CurrentManagedThreadId, engine.ExecutionThreads.Single());
        Assert.Equal(2.5f, await session.ReadNativeGaussianSpareAsync());
        Assert.Single(engine.ExecutionThreads);
        engine.NativeGaussianSpare = null;
        Assert.Null(await session.ReadNativeGaussianSpareAsync());
        Assert.Equal(1ul, session.LatestSnapshot.Tick);
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
    public async Task RemoveManyPreservesStableOrganismKeysAndRunsOnTheEngineThread()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);
        OrganismKey[] organisms = [new(3, 2), new(7, 5)];
        var callerThread = Environment.CurrentManagedThreadId;

        await session.RemoveManyAsync(organisms);

        Assert.Equal(organisms, Assert.Single(engine.RemoveManyCalls));
        var executionThread = Assert.Single(engine.ExecutionThreads);
        Assert.NotEqual(callerThread, executionThread);
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
    public async Task CloneWithDnaAppliesEditsToTheChildWithoutReplacingTheSource()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);
        await session.ImportDnaAsync("start\nstop");

        var clone = await session.CloneWithDnaAsync(0, 0, [24f, 24f], "start\n10 .up store\nstop");

        Assert.Equal(new OrganismKey(1, 0), clone);
        Assert.Equal((1U, 0U, "start\n10 .up store\nstop"), engine.LastDnaReplacement);
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
        await session.UpdateTeleporterAsync(teleporter with { Capacity = 3, RespectShapes = true });
        await session.RemoveObstacleAsync(7);
        await session.RemoveTeleporterAsync(8);

        Assert.Equal(obstacle, engine.LastObstacleAdded);
        Assert.Equal(teleporter, engine.LastTeleporterAdded);
        Assert.Equal(3, engine.LastTeleporterUpdated!.Capacity);
        Assert.True(engine.LastTeleporterUpdated.RespectShapes);
        Assert.Equal(7U, engine.LastObstacleRemoved);
        Assert.Equal(8U, engine.LastTeleporterRemoved);
        Assert.Single(engine.ExecutionThreads);
    }

    [Fact]
    public async Task ShapeAndOfflineTeleporterEditorsForwardAllProperties()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);
        var obstacle = new ObstacleSnapshot(7, [10f, 20f], [30f, 40f])
        {
            Color = 0xff112233u,
            Opacity = 0.4f,
            Drift = [2f, -3f],
            VisionPolicy = "see_through",
            ShotPolicy = "pass_through",
        };
        var teleporter = new FileTeleporterSnapshot(
            9, [50f, 60f], 15f, "in", "out", 3, 1, 4,
            new TeleporterFilterSnapshot(false, true, true), 8, 5);

        await session.UpdateObstacleAsync(obstacle);
        await session.AddFileTeleporterAsync(teleporter);
        await session.UpdateFileTeleporterAsync(teleporter with { PollInterval = 5 });

        Assert.Equal(obstacle, engine.LastObstacleUpdated);
        Assert.Equal(teleporter, engine.LastFileTeleporterAdded);
        Assert.Equal(5U, engine.LastFileTeleporterUpdated!.PollInterval);
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
        var update = EnvironmentUpdate.Default with
        {
            MetabolismCost = 0,
            VegetableEnergyPerTick = 10,
            SunlightEnergy = 200,
            Gravity = [0f, 2f],
            Drag = 0.5f,
            BrownianMotion = 3f,
        };

        await session.UpdateEnvironmentAsync(update);

        Assert.Equal(update, engine.LastEnvironmentUpdate);
        Assert.Single(engine.ExecutionThreads);
    }

    [Fact]
    public async Task MutationSettingsRunOnTheEngineThread()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);
        var update = new MutationSettingsUpdate(false, 2.25f, [new SpeciesMutationRateUpdate(1, 3.5f)]);

        await session.UpdateMutationSettingsAsync(update);

        Assert.Equal(update, engine.LastMutationSettings);
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

    [Fact]
    public async Task SaveWithoutMutationsRunsOnEngineThreadWithoutUsingOrdinarySave()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);

        var save = await session.SaveWithoutMutationsAsync();

        Assert.Equal([9, 8, 7], save);
        Assert.True(engine.SavedWithoutMutations);
        Assert.False(engine.SavedNormally);
        Assert.Single(engine.ExecutionThreads);
    }
}

internal sealed class FakeEngineClient : IEngineClient
{
    private ulong _tick;
    private int _population;

    public string Backend { get; private set; } = "CPU";
    public string? LastDna { get; private set; }
    public string? StartupDna { get; private set; }
    public uint? LastTemplateSpecies { get; private set; }
    public float[]? LastImportPosition { get; private set; }
    public SpeciesImport? LastSpeciesImport { get; private set; }
    public SpeciesImport? LastInsertedSpecies { get; private set; }
    public float[]? LastInsertedPosition { get; private set; }
    public (uint Slot, uint Generation)? LastRemoved { get; private set; }
    public List<IReadOnlyList<OrganismKey>> RemoveManyCalls { get; } = [];
    public float[]? LastMovePosition { get; private set; }
    public ObstacleSnapshot? LastObstacleAdded { get; private set; }
    public ObstacleSnapshot? LastObstacleUpdated { get; private set; }
    public TeleporterSnapshot? LastTeleporterAdded { get; private set; }
    public TeleporterSnapshot? LastTeleporterUpdated { get; private set; }
    public FileTeleporterSnapshot? LastFileTeleporterAdded { get; private set; }
    public FileTeleporterSnapshot? LastFileTeleporterUpdated { get; private set; }
    public uint? LastObstacleRemoved { get; private set; }
    public uint? LastTeleporterRemoved { get; private set; }
    public float? LastBrownianMotion { get; private set; }
    public EnvironmentUpdate? LastEnvironmentUpdate { get; private set; }
    public MutationSettingsUpdate? LastMutationSettings { get; private set; }
    public ManualReproductionRequest? LastManualReproduction { get; private set; }
    public bool? LastTiesEnabled { get; private set; }
    public (uint Slot, uint Generation, string Dna)? LastDnaReplacement { get; private set; }
    private string _dna = "start\nstop";
    public HashSet<int> ExecutionThreads { get; } = [];
    public bool SavedNormally { get; private set; }
    public bool SavedWithoutMutations { get; private set; }
    public List<int> RecognitionFudgingUpdates { get; } = [];
    public Action? BeforeTick { get; set; }
    public Action? BeforeDnaExport { get; set; }
    public OrganismSnapshot[] SnapshotOrganisms { get; set; } = [];
    public float? NativeGaussianSpare { get; set; }
    public byte NativeSunChange { get; set; }
    public byte ReadNativeSunChange()
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        return NativeSunChange;
    }
    public float? ReadNativeGaussianSpare()
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        return NativeGaussianSpare;
    }

    public void UpdateRecognitionFudging(int mode)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        RecognitionFudgingUpdates.Add(mode);
    }

    public EngineSnapshot Snapshot()
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        return new EngineSnapshot(_tick, _population, Backend, SnapshotOrganisms);
    }

    public EngineCapabilities Capabilities() => new(1, Backend, false, null);

    public byte[] Save()
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        SavedNormally = true;
        return BitConverter.GetBytes(_tick);
    }

    public byte[] SaveWithoutMutations()
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        SavedWithoutMutations = true;
        return [9, 8, 7];
    }

    public void Load(byte[] save) => _tick = BitConverter.ToUInt64(save);

    public void Tick(uint count = 1)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        BeforeTick?.Invoke();
        _tick += count;
    }

    public DnaImportReport ImportDna(string dna, float[]? position = null)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastDna = dna;
        StartupDna ??= dna;
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

    public DnaImportReport InsertSpeciesAt(SpeciesImport species, float[] position)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastInsertedSpecies = species;
        LastInsertedPosition = position;
        _population++;
        return new DnaImportReport(["legacy warning"]);
    }

    public void Remove(uint slot, uint generation)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastRemoved = (slot, generation);
        _population = Math.Max(0, _population - 1);
    }

    public void RemoveMany(IReadOnlyList<OrganismKey> organisms)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        RemoveManyCalls.Add([.. organisms]);
        _population = Math.Max(0, _population - organisms.Count);
    }

    public void Move(uint slot, uint generation, float[] position) => LastMovePosition = position;

    public OrganismKey CloneOrganism(uint slot, uint generation, float[] position)
    {
        _population++;
        return new OrganismKey((uint)(_population - 1), 0);
    }

    public void ReplaceDna(uint slot, uint generation, string dna)
    {
        _dna = dna;
        LastDnaReplacement = (slot, generation, dna);
    }

    public string ExportDna(uint slot, uint generation)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        BeforeDnaExport?.Invoke();
        return _dna;
    }

    public void UpdateOrganismMutationProfile(uint slot, uint generation, MutationProfileUpdate profile)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        SnapshotOrganisms = SnapshotOrganisms.Select(value => value.Slot == slot && value.Generation == generation
            ? value with { MutationProfile = profile } : value).ToArray();
    }

    public string ExportSpeciesTemplateDna(uint species)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastTemplateSpecies = species;
        return StartupDna ?? throw new InvalidOperationException("No startup template.");
    }

    public OrganismKey Reproduce(uint firstSlot, uint firstGeneration, uint? secondSlot, uint? secondGeneration, float[] position)
    {
        _population++;
        return new OrganismKey((uint)(_population - 1), 0);
    }

    public OrganismKey Reproduce(ManualReproductionRequest request)
    {
        LastManualReproduction = request;
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

    public void UpdateObstacle(ObstacleSnapshot obstacle)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastObstacleUpdated = obstacle;
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

    public void UpdateTeleporter(TeleporterSnapshot teleporter)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastTeleporterUpdated = teleporter;
    }

    public void RemoveTeleporter(uint id)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastTeleporterRemoved = id;
    }

    public void AddFileTeleporter(FileTeleporterSnapshot teleporter)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastFileTeleporterAdded = teleporter;
    }

    public void UpdateFileTeleporter(FileTeleporterSnapshot teleporter)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastFileTeleporterUpdated = teleporter;
    }

    public void RemoveFileTeleporter(uint id)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
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

    public void UpdateMutationSettings(MutationSettingsUpdate update)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastMutationSettings = update;
    }

    public void SetTiesEnabled(bool enabled)
    {
        ExecutionThreads.Add(Environment.CurrentManagedThreadId);
        LastTiesEnabled = enabled;
    }
    public void SetSpeciesColor(uint species, uint color) { }
    public void MakeNewSpecies(uint slot, uint generation, string name, uint color) { }
    public void UpdateLegacyRestrictions(bool robotFixingAllowed, bool asexualReproductionAllowed) { }
    public void ConfigurePlayerBot(PlayerBotConfig config) { }
    public void SetPlayerBotKey(uint slot, uint generation, int keyCode, bool pressed) { }
    public void SetOrganismMemory(uint slot, uint generation, ushort address, int value) { }
    public void SetOrganismEnergy(uint slot, uint generation, int energy) { }
    public void UpdateRestrictionOptions(RestrictionOptions options) { }
    public void ConfigureLocalMode(LocalModeConfig config) { }
    public void StartLocalMode() { }
    public void RestartLocalMode() { }
    public void AdjudicateLocalRound(uint? winner, string reason) { }
    public void SetOrganismAim(uint slot, uint generation, int aim) { }

    public void Dispose() { }
}
