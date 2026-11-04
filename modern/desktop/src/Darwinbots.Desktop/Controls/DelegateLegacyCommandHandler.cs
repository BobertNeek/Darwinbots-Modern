using Darwinbots.Desktop.Core.LegacyMenus;

namespace Darwinbots.Desktop.Controls;

internal sealed class DelegateLegacyCommandHandler(
    Func<LegacyCommandState> state,
    Func<CancellationToken, Task<LegacyCommandResult>> execute) : ILegacyCommandHandler
{
    public LegacyCommandState GetState() => state();

    public Task<LegacyCommandResult> ExecuteAsync(CancellationToken cancellationToken = default) =>
        execute(cancellationToken);

    public static DelegateLegacyCommandHandler Disabled(string reason) => new(
        () => new LegacyCommandState(false, false, reason),
        _ => Task.FromResult(LegacyCommandResult.Failure(reason)));
}
