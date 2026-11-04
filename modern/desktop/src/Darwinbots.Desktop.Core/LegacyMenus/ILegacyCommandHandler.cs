namespace Darwinbots.Desktop.Core.LegacyMenus;

public interface ILegacyCommandHandler
{
    LegacyCommandState GetState();

    Task<LegacyCommandResult> ExecuteAsync(CancellationToken cancellationToken = default);
}
