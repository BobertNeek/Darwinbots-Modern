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

    public async Task<DnaImportReport> ImportSpeciesAsync(SpeciesImport species, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(species);
        species.Validate();
        DnaImportReport? report = null;
        await Enqueue(engine => report = engine.ImportSpecies(species), cancellationToken).ConfigureAwait(false);
        return report ?? DnaImportReport.Compatible;
    }

    public Task RemoveAsync(uint slot, uint generation, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.Remove(slot, generation), cancellationToken);

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

    public Task RemoveObstacleAsync(uint id, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.RemoveObstacle(id), cancellationToken);

    public Task AddTeleporterAsync(TeleporterSnapshot teleporter, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.AddTeleporter(teleporter), cancellationToken);

    public Task RemoveTeleporterAsync(uint id, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.RemoveTeleporter(id), cancellationToken);

    public Task SetBrownianMotionAsync(float value, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.SetBrownianMotion(value), cancellationToken);

    public Task UpdateEnvironmentAsync(EnvironmentUpdate update, CancellationToken cancellationToken = default) =>
        Enqueue(engine => engine.UpdateEnvironment(update), cancellationToken);

    public async Task<byte[]> SaveAsync(CancellationToken cancellationToken = default)
    {
        byte[]? save = null;
        await Enqueue(engine => save = engine.Save(), cancellationToken).ConfigureAwait(false);
        return save ?? throw new InvalidDataException("Engine returned no save data.");
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
