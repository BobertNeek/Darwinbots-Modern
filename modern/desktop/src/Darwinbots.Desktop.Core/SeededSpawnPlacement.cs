namespace Darwinbots.Desktop.Core;

public sealed class SeededSpawnPlacement(ulong seed)
{
    private ulong _state = seed == 0 ? 0x9e3779b97f4a7c15UL : seed;

    public float[][] Next(int count, float worldWidth, float worldHeight)
    {
        if (count < 0) throw new ArgumentOutOfRangeException(nameof(count));
        if (!float.IsFinite(worldWidth) || worldWidth <= 0) throw new ArgumentOutOfRangeException(nameof(worldWidth));
        if (!float.IsFinite(worldHeight) || worldHeight <= 0) throw new ArgumentOutOfRangeException(nameof(worldHeight));

        var marginX = Math.Min(60f, worldWidth / 4f);
        var marginY = Math.Min(60f, worldHeight / 4f);
        var spanX = Math.Max(0f, worldWidth - marginX * 2f);
        var spanY = Math.Max(0f, worldHeight - marginY * 2f);
        var positions = new float[count][];
        for (var index = 0; index < count; index++)
        {
            positions[index] =
            [
                marginX + NextUnit() * spanX,
                marginY + NextUnit() * spanY,
            ];
        }
        return positions;
    }

    public float[][] Next(SpeciesImport species, float worldWidth, float worldHeight)
    {
        ArgumentNullException.ThrowIfNull(species);
        species.Validate();
        return species.Placement.Mode switch
        {
            SpeciesPlacementMode.Random => NextInBounds(species, worldWidth, worldHeight),
            SpeciesPlacementMode.Center => Enumerable.Range(0, species.InitialCount)
                .Select(_ => new[] { worldWidth / 2f, worldHeight / 2f })
                .ToArray(),
            SpeciesPlacementMode.Fixed => Enumerable.Range(0, species.InitialCount)
                .Select(_ => new[] { species.Placement.X, species.Placement.Y })
                .ToArray(),
            _ => throw new ArgumentOutOfRangeException(nameof(species.Placement)),
        };
    }

    private float[][] NextInBounds(SpeciesImport species, float worldWidth, float worldHeight)
    {
        var placement = species.Placement;
        if (placement.Left == 0 && placement.Right == 1 && placement.Top == 0 && placement.Bottom == 1)
            return Next(species.InitialCount, worldWidth, worldHeight);
        return Enumerable.Range(0, species.InitialCount).Select(_ => new[]
        {
            worldWidth * (placement.Left + NextUnit() * (placement.Right - placement.Left)),
            worldHeight * (placement.Top + NextUnit() * (placement.Bottom - placement.Top)),
        }).ToArray();
    }

    public (float[] Minimum, float[] Maximum)[] NextObstacles(int count, float worldWidth, float worldHeight)
    {
        if (count < 0) throw new ArgumentOutOfRangeException(nameof(count));
        if (!float.IsFinite(worldWidth) || worldWidth <= 0) throw new ArgumentOutOfRangeException(nameof(worldWidth));
        if (!float.IsFinite(worldHeight) || worldHeight <= 0) throw new ArgumentOutOfRangeException(nameof(worldHeight));

        var shapes = new (float[] Minimum, float[] Maximum)[count];
        for (var index = 0; index < count; index++)
        {
            var shapeWidth = worldWidth * (0.025f + NextUnit() * 0.09f);
            var shapeHeight = worldHeight * (0.015f + NextUnit() * 0.07f);
            var x = NextUnit() * Math.Max(1, worldWidth - shapeWidth);
            var y = NextUnit() * Math.Max(1, worldHeight - shapeHeight);
            shapes[index] = ([x, y], [x + shapeWidth, y + shapeHeight]);
        }
        return shapes;
    }

    private float NextUnit() => (NextRandom() >> 40) / 16_777_216f;

    private ulong NextRandom()
    {
        var value = _state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        _state = value == 0 ? 1UL : value;
        return _state;
    }
}
