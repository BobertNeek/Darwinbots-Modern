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
            {
                var lineCount = RoundedInteger(width / (CorridorWidth + WallThickness)) - 1;
                var random = new Random(unchecked((int)width * 31 + (int)height));
                for (var line = 1; line <= lineCount; line++)
                {
                    var opening = Math.Max(1f, (float)(random.NextDouble() * Math.Max(1f, height - CorridorWidth)));
                    var x = line * (CorridorWidth + WallThickness);
                    Add(x, -100f, WallThickness, opening);
                    if (opening + CorridorWidth < height + 100f)
                        Add(x, opening + CorridorWidth, WallThickness, height + 100f - opening - CorridorWidth);
                }
                break;
            }
            case MazePreset.Vertical:
            {
                var lineCount = RoundedInteger(height / (CorridorWidth + WallThickness)) - 1;
                var random = new Random(unchecked((int)width * 31 + (int)height));
                for (var line = 1; line <= lineCount; line++)
                {
                    var opening = Math.Max(1f, (float)(random.NextDouble() * Math.Max(1f, width - CorridorWidth)));
                    var y = line * (CorridorWidth + WallThickness);
                    Add(-100f, y, opening, WallThickness);
                    if (opening + CorridorWidth < width + 100f)
                        Add(opening + CorridorWidth, y, width + 100f - opening - CorridorWidth, WallThickness);
                }
                break;
            }
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
                var blockHeight = height * 1.2f;
                Add(-blockWidth + 1f, height * -0.1f, blockWidth, blockHeight, 2f);
                Add(width - 1f, height * -0.1f, blockWidth, blockHeight, -2f);
                break;
            }
        }

        return shapes;
    }

    private static int RoundedInteger(float value) => (int)Math.Round(value, MidpointRounding.ToEven);
}
