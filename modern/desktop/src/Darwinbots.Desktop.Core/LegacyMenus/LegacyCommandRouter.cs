namespace Darwinbots.Desktop.Core.LegacyMenus;

public sealed class LegacyCommandRouter
{
    private readonly IReadOnlyDictionary<LegacyCommandId, ILegacyCommandHandler> _handlers;

    public LegacyCommandRouter(IReadOnlyDictionary<LegacyCommandId, ILegacyCommandHandler> handlers)
    {
        ArgumentNullException.ThrowIfNull(handlers);

        var expected = LegacyMenuCatalog.FlattenLeaves()
            .Select(item => item.Command!.Value)
            .ToHashSet();
        var registered = handlers.Keys.ToHashSet();
        var missing = expected.Except(registered).Order().ToArray();
        var unexpected = registered.Except(expected).Order().ToArray();
        if (missing.Length > 0 || unexpected.Length > 0)
        {
            var details = new List<string>();
            if (missing.Length > 0)
                details.Add($"missing: {string.Join(", ", missing)}");
            if (unexpected.Length > 0)
                details.Add($"unexpected: {string.Join(", ", unexpected.Select(value => $"{value} ({(int)value})"))}");
            throw new ArgumentException($"Historical menu handler registration is incomplete ({string.Join("; ", details)}).", nameof(handlers));
        }

        _handlers = new Dictionary<LegacyCommandId, ILegacyCommandHandler>(handlers);
    }

    public LegacyCommandState GetState(LegacyCommandId command) => Handler(command).GetState();

    public async Task<LegacyCommandResult> ExecuteAsync(
        LegacyCommandId command,
        CancellationToken cancellationToken = default)
    {
        var handler = Handler(command);
        var state = handler.GetState();
        if (!state.Enabled)
            return LegacyCommandResult.Failure(state.DisabledReason ?? $"{command} is unavailable.");

        try
        {
            return await handler.ExecuteAsync(cancellationToken).ConfigureAwait(false);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            return LegacyCommandResult.Cancel();
        }
        catch (Exception error)
        {
            return LegacyCommandResult.Failure($"{command} failed: {error.Message}", error);
        }
    }

    private ILegacyCommandHandler Handler(LegacyCommandId command) =>
        _handlers.TryGetValue(command, out var handler)
            ? handler
            : throw new ArgumentOutOfRangeException(nameof(command), command, "Command is not in the historical menu catalog.");
}
