namespace Darwinbots.Desktop.Core;

public enum ManualReproductionMutationMode
{
    Preserve,
    Apply,
}

public sealed record ManualReproductionRequest(
    OrganismKey Parent,
    OrganismKey? SecondParent,
    float[] Position,
    int Energy,
    int Body,
    ManualReproductionMutationMode MutationMode,
    bool Focus)
{
    public void Validate()
    {
        if (Position is null || Position.Length != 2 || Position.Any(value => !float.IsFinite(value)))
            throw new ArgumentException("World position must contain two finite coordinates.", nameof(Position));
        if (Energy < 0)
            throw new ArgumentOutOfRangeException(nameof(Energy));
        if (Body < 0)
            throw new ArgumentOutOfRangeException(nameof(Body));
    }
}
