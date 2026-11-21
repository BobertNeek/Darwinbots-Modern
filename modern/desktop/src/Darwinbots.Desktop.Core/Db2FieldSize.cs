namespace Darwinbots.Desktop.Core;

public static class Db2FieldSize
{
    public const int Minimum = 1;
    public const int Maximum = 25;

    public static bool IsValid(int size) => size is >= Minimum and <= Maximum;

    public static (int Width, int Height) Dimensions(int size) => size switch
    {
        1 => (9_237, 6_928),
        >= 2 and <= 12 => (8_000 * size, 6_000 * size),
        >= 13 and <= Maximum => (192_000 * (size - 12), 144_000 * (size - 12)),
        _ => throw new ArgumentOutOfRangeException(nameof(size)),
    };
}
