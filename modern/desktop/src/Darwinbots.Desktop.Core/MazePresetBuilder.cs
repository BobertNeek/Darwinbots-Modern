namespace Darwinbots.Desktop.Core;

public enum MazePreset
{
    Horizontal,
    Vertical,
    Spiral,
    Checkerboard,
    PolarIce,
    TrashCompactor,
}

public readonly record struct MazeDimensions(int CorridorWidth, int WallThickness)
{
    public static MazeDimensions Default { get; } = new(500, 50);
}

public sealed record MazePresetShape(float[] Minimum, float[] Maximum, float[] Drift)
{
    public MazePresetShape(float[] minimum, float[] maximum) : this(minimum, maximum, [0f, 0f])
    {
    }
}

public static class MazePresetBuilder
{
    private const float CorridorWidth = 500f;
    private const float WallThickness = 50f;

    public static IReadOnlyList<MazePresetShape> Build(MazePreset preset, float width, float height)
    {
        if (!float.IsFinite(width) || !float.IsFinite(height) || width <= 0f || height <= 0f)
            throw new ArgumentOutOfRangeException(nameof(width));

        var shapes = new List<MazePresetShape>();
        void Add(float x, float y, float shapeWidth, float shapeHeight, float driftX = 0f, float driftY = 0f)
        {
            if (shapeWidth <= 0f || shapeHeight <= 0f) return;
            shapes.Add(new MazePresetShape(
                [x, y],
                [x + shapeWidth, y + shapeHeight],
                [driftX, driftY]));
        }

        switch (preset)
        {
            case MazePreset.Horizontal:
            case MazePreset.Vertical:
                return BuildSimple(preset == MazePreset.Horizontal, width, height, MazeDimensions.Default);
            case MazePreset.Spiral:
            {
                var horizontalLines = RoundedInteger(height / (CorridorWidth + WallThickness)) - 1;
                var verticalLines = RoundedInteger(width / (CorridorWidth + WallThickness)) - 1;
                var lineCount = Math.Min(horizontalLines, verticalLines);
                if (lineCount % 2 != 0) lineCount--;
                for (var line = 1; line <= lineCount / 2; line++)
                {
                    Add((line - 1) * CorridorWidth, line * CorridorWidth,
                        width - CorridorWidth * (2 * (line - 1) + 1), WallThickness);
                    Add(line * CorridorWidth, height - line * CorridorWidth,
                        width - CorridorWidth * 2f * line + WallThickness, WallThickness);
                    Add(width - line * CorridorWidth, line * CorridorWidth,
                        WallThickness, height - CorridorWidth * (2 * line));
                    Add(line * CorridorWidth, (line + 1) * CorridorWidth,
                        WallThickness, height - CorridorWidth * (2 * line + 1));
                }
                break;
            }
            case MazePreset.Checkerboard:
            {
                var blockWidth = Math.Min(5_000f, width / 10f);
                var blocksAcross = (int)Math.Floor(width / (blockWidth + CorridorWidth));
                var acrossGap = (blocksAcross * (blockWidth + CorridorWidth) + CorridorWidth - width) / 2f;
                var blocksDown = (int)Math.Floor(height / (blockWidth + CorridorWidth));
                var downGap = (blocksDown * (blockWidth + CorridorWidth) + CorridorWidth - height) / 2f;
                for (var across = 0; across < blocksAcross; across++)
                for (var down = 0; down < blocksDown; down++)
                    Add(
                        across * blockWidth + (across + 1) * CorridorWidth - acrossGap,
                        down * blockWidth + (down + 1) * CorridorWidth - downGap,
                        blockWidth,
                        blockWidth);
                break;
            }
            case MazePreset.PolarIce:
            {
                var blockWidth = width / 2f;
                var blockHeight = height / 2f;
                for (var block = 0; block <= 8; block++)
                    Add(blockWidth / 2f, blockHeight / 2f, blockWidth, blockHeight, 20f, 20f);
                break;
            }
            case MazePreset.TrashCompactor:
            {
                const float blockWidth = 1_000f;
                // VB6 evaluates these unsuffixed literals as Double, then
                // stores the dimensions/coordinates as Single.
                var blockHeight = (float)(height * 1.2);
                var y = (float)(height * -0.1);
                Add(-blockWidth + 1f, y, blockWidth, blockHeight, 2f);
                Add(width - 1f, y, blockWidth, blockHeight, -2f);
                break;
            }
        }

        return shapes;
    }

    public static IReadOnlyList<MazePresetShape> BuildSimple(bool horizontal, float width, float height, MazeDimensions dimensions)
    {
        if (!float.IsFinite(width) || !float.IsFinite(height) || width <= 0f || height <= 0f)
            throw new ArgumentOutOfRangeException(nameof(width));
        if (dimensions.CorridorWidth is < 300 or > 5000 || dimensions.WallThickness is < 1 or > 500)
            throw new ArgumentOutOfRangeException(nameof(dimensions));

        var across = horizontal ? width : height;
        var along = horizontal ? height : width;
        var corridor = (float)dimensions.CorridorWidth;
        var thickness = (float)dimensions.WallThickness;
        var pitch = corridor + thickness;
        var lineCount = RoundedInteger(across / pitch) - 1;
        var random = new Random(unchecked((int)width * 31 + (int)height));
        var shapes = new List<MazePresetShape>();
        for (var line = 1; line <= lineCount; line++)
        {
            var opening = Math.Max(1f, (float)(random.NextDouble() * Math.Max(1f, along - corridor)));
            var cross = line * pitch;
            AddSegment(-100f, opening);
            if (opening + corridor < along + 100f)
                AddSegment(opening + corridor, along + 100f - opening - corridor);

            void AddSegment(float start, float length)
            {
                if (length <= 0f) return;
                shapes.Add(horizontal
                    ? new([cross, start], [cross + thickness, start + length])
                    : new([start, cross], [start + length, cross + thickness]));
            }
        }
        return shapes;
    }

    private static int RoundedInteger(float value) => (int)Math.Round(value, MidpointRounding.ToEven);
}
