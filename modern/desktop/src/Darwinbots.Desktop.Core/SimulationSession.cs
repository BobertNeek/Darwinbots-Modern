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
    private ulong _lastPublishedTick;

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

    public Task StepAsync(uint count = 1, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.Tick(count), cancellationToken);

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

    public Task RemoveManyAsync(IReadOnlyList<OrganismKey> organisms, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(organisms);
        if (organisms.Count == 0)
            throw new ArgumentException("At least one organism is required.", nameof(organisms));
        OrganismKey[] stableOrganisms = [.. organisms];
        return Enqueue(engine => engine.RemoveMany(stableOrganisms), cancellationToken);
    }

    public Task MoveAsync(uint slot, uint generation, float[] position, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.Move(slot, generation, position), cancellationToken);

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

    public async Task<string> ExportDnaAsync(uint slot, uint generation, CancellationToken cancellationToken = default)
    {
        string? result = null;
        await Enqueue(engine => result = engine.ExportDna(slot, generation), cancellationToken).ConfigureAwait(false);
        return result ?? throw new InvalidDataException("Engine returned no DNA.");
    }

    public async Task<OrganismKey> ReproduceAsync(
        uint firstSlot, uint firstGeneration, uint? secondSlot, uint? secondGeneration,
        float[] position, CancellationToken cancellationToken = default)
    {
        OrganismKey? result = null;
        await Enqueue(engine => result = engine.Reproduce(firstSlot, firstGeneration, secondSlot, secondGeneration, position), cancellationToken).ConfigureAwait(false);
        return result ?? throw new InvalidDataException("Engine returned no child ID.");
    }

    public Task SwitchBackendAsync(string backend, CancellationToken cancellationToken = default)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(backend);
        return Enqueue(engine => engine.SwitchBackend(backend), cancellationToken);
    }

    public Task AddObstacleAsync(ObstacleSnapshot obstacle, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.AddObstacle(obstacle), cancellationToken);

    public Task UpdateObstacleAsync(ObstacleSnapshot obstacle, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateObstacle(obstacle), cancellationToken);

    public Task RemoveObstacleAsync(uint id, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.RemoveObstacle(id), cancellationToken);

    public Task AddTeleporterAsync(TeleporterSnapshot teleporter, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.AddTeleporter(teleporter), cancellationToken);

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

    public Task SetTiesEnabledAsync(bool enabled, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.SetTiesEnabled(enabled), cancellationToken);

    public Task SetSpeciesColorAsync(uint species, uint color, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.SetSpeciesColor(species, color), cancellationToken);

    public Task MakeNewSpeciesAsync(uint slot, uint generation, string name, uint color, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.MakeNewSpecies(slot, generation, name, color), cancellationToken);

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

    public Task SetOrganismMemoryAsync(uint slot, uint generation, ushort address, int value, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.SetOrganismMemory(slot, generation, address, value), cancellationToken);

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

    public Task AdjudicateLocalRoundAsync(uint? winner, string reason, CancellationToken cancellationToken = default)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(reason);
        return Enqueue(engine => engine.AdjudicateLocalRound(winner, reason), cancellationToken);
    }

    public Task SetOrganismAimAsync(uint slot, uint generation, int aim, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.SetOrganismAim(slot, generation, aim), cancellationToken);

    public Task UpdateEnvironmentAsync(EnvironmentUpdate update, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateEnvironment(update), cancellationToken);

    public async Task<byte[]> SaveAsync(CancellationToken cancellationToken = default)
    {
        byte[]? save = null;
        await Enqueue(engine => save = engine.Save(), cancellationToken).ConfigureAwait(false);
        return save ?? throw new InvalidDataException("Engine returned no save data.");
    }

    public async Task<byte[]> SaveWithoutMutationsAsync(CancellationToken cancellationToken = default)
    {
        byte[]? save = null;
        await Enqueue(engine => save = engine.SaveWithoutMutations(), cancellationToken).ConfigureAwait(false);
        return save ?? throw new InvalidDataException("Engine returned no mutation-free save data.");
    }

    public Task LoadAsync(byte[] save, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(save);
        return Enqueue(engine => engine.Load(save), cancellationToken);
    }

    private Task Enqueue(Action<IEngineClient> action, CancellationToken cancellationToken)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        cancellationToken.ThrowIfCancellationRequested();
        var completion = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var command = new SessionCommand(action, completion, cancellationToken);
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
                var snapshot = _engine.Snapshot();
                var elapsed = _performanceClock.Elapsed.TotalSeconds;
                var advanced = snapshot.Tick - _lastPublishedTick;
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
        CancellationToken CancellationToken);
}
