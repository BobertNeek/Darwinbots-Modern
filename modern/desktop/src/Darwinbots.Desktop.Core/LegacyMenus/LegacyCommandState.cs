namespace Darwinbots.Desktop.Core.LegacyMenus;

public sealed record LegacyCommandState(bool Enabled, bool Checked, string? DisabledReason = null, bool Visible = true)
{
    public static LegacyCommandState Available { get; } = new(true, false);
}
