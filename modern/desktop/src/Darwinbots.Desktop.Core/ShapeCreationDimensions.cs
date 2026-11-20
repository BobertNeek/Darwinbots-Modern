namespace Darwinbots.Desktop.Core;

public readonly record struct ShapeCreationDimensions(int WidthPermille, int HeightPermille)
{
    public static ShapeCreationDimensions Default { get; } = new(200, 200);
}
