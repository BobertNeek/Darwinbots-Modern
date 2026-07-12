namespace Darwinbots.Desktop.Core;

public sealed record DnaImportReport(IReadOnlyList<string> CompatibilityWarnings)
{
    public static DnaImportReport Compatible { get; } = new([]);
}
