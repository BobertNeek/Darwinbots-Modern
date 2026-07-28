namespace Darwinbots.Desktop.Core.LegacyMenus;

public sealed record LegacyCommandState(bool Enabled, bool Checked, string? DisabledReason = null)
{
    public static LegacyCommandState Available { get; } = new(true, false);
}
