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
