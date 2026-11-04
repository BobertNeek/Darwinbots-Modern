namespace Darwinbots.Desktop.Core.LegacyMenus;

public sealed record LegacyCommandResult(
    bool Succeeded,
    bool Canceled,
    string Status,
    Exception? Error = null)
{
    public static LegacyCommandResult Success(string status) => new(true, false, status);

    public static LegacyCommandResult Cancel(string status = "CANCELED") => new(false, true, status);

    public static LegacyCommandResult Failure(string status, Exception? error = null) =>
        new(false, false, status, error);
}
