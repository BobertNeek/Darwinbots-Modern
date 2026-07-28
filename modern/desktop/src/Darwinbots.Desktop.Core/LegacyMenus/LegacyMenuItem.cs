namespace Darwinbots.Desktop.Core.LegacyMenus;

public sealed record LegacyMenuItem
{
    private LegacyMenuItem(
        string label,
        LegacyCommandId? command,
        string? iconKey,
        string? shortcut,
        LegacyMenuCheckKind checkKind,
        IReadOnlyList<LegacyMenuItem> children,
        bool separator)
    {
        Label = label;
        Command = command;
        IconKey = iconKey;
        Shortcut = shortcut;
        CheckKind = checkKind;
        Children = children;
        IsSeparator = separator;
    }

    public string Label { get; }
    public LegacyCommandId? Command { get; }
    public string? IconKey { get; }
    public string? Shortcut { get; }
    public LegacyMenuCheckKind CheckKind { get; }
    public IReadOnlyList<LegacyMenuItem> Children { get; }
    public bool IsSeparator { get; }
    public bool IsLeaf => Command is not null;

    public static LegacyMenuItem Top(string label, params LegacyMenuItem[] children) =>
        new(label, null, null, null, LegacyMenuCheckKind.None, children, false);

    public static LegacyMenuItem Group(string label, string iconKey, params LegacyMenuItem[] children) =>
        new(label, null, iconKey, null, LegacyMenuCheckKind.None, children, false);

    public static LegacyMenuItem Leaf(
        string label,
        LegacyCommandId command,
        string iconKey,
        string? shortcut = null,
        LegacyMenuCheckKind checkKind = LegacyMenuCheckKind.None) =>
        new(label, command, iconKey, shortcut, checkKind, [], false);

    public static LegacyMenuItem Separator() =>
        new("-", null, null, null, LegacyMenuCheckKind.None, [], true);
}
