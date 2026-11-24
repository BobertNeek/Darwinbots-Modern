namespace Darwinbots.Desktop.Core.LegacyMenus;

public sealed record LegacyHelpTopic(
    LegacyCommandId Command,
    string Title,
    string Section,
    string Content,
    bool Visible = true);

public static class LegacyHelpResolver
{
    public static IReadOnlyList<LegacyHelpTopic> Topics { get; } =
    [
        new(
            LegacyCommandId.HelpDna,
            "DNA Help",
            "Legacy DNA quick reference",
            "Darwinbots DNA is plain text. Conditions appear between cond and start; actions appear between start and stop. Values are pushed onto the stack, sysvars are read with * and written with store.\n\nExample:\ncond\n*.eye5 0 >\nstart\n30 .up store\nstop\nend\n\nApostrophes begin comments. Imported bot files keep the historical .txt format and sysvar names."),
        new(
            LegacyCommandId.HelpAbout,
            "About Darwinbots Modern",
            "Behavioral modernization",
            "Darwinbots Modern is a cross-platform continuation of Darwinbots 2. It preserves legacy text DNA and sysvars while running a Rust simulation engine behind an Avalonia desktop interface. CPU and GPU backends share the same simulation contract.\n\nThe VB6 Darwinbots 2 runtime remains the behavioral authority for compatibility decisions."),
    ];

    public static LegacyHelpTopic Resolve(LegacyCommandId command) =>
        Topics.FirstOrDefault(topic => topic.Command == command)
        ?? throw new ArgumentOutOfRangeException(nameof(command), command, "Command has no local help topic.");
}
