namespace Darwinbots.Desktop.Core;

public sealed record GeneActivationMeterSnapshot(
    ulong WatchId,
    OrganismKey? Focus,
    IReadOnlyList<bool> Current,
    IReadOnlyList<int> Weighted,
    ulong Executions);

public sealed record GeneActivationLabel(int Number, double Center, double? Tick);

public static class GeneActivationMeterLayout
{
    public static IEnumerable<GeneActivationLabel> Labels(int geneCount)
    {
        if (geneCount < 25)
        {
            for (var column = 0; column < geneCount; column++)
                yield return new(column + 1, (column + 0.5) / geneCount, null);
        }
        else
        {
            for (var boundary = 5; boundary < geneCount; boundary += 5)
                yield return new(boundary, (boundary + 0.5) / geneCount, (double)boundary / geneCount);
        }
    }
}
