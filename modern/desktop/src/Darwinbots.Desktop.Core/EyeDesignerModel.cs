using System.Globalization;
using System.Text;

namespace Darwinbots.Desktop.Core;

public sealed record EyeDesignRow(int Eye, int Direction, int Width)
{
    public string Name => $"eye{Eye}";
}

public sealed class EyeDesignerModel
{
    public const int EyeCount = 9;
    public const int MinimumValue = -32_000;
    public const int MaximumValue = 32_000;

    private readonly EyeDesignRow[] _rows;

    private EyeDesignerModel(int aim, EyeDesignRow[] rows, byte focusEye)
    {
        Aim = aim;
        _rows = rows;
        FocusEye = focusEye;
    }

    public int Aim { get; }
    public byte FocusEye { get; }
    public IReadOnlyList<EyeDesignRow> Rows => _rows;

    public static EyeDesignerModel FromOrganism(OrganismSnapshot organism)
    {
        ArgumentNullException.ThrowIfNull(organism);
        var eyes = organism.Vision.Eyes ?? [];
        var rows = Enumerable.Range(0, EyeCount)
            .Select(index =>
            {
                var eye = eyes.ElementAtOrDefault(index);
                return new EyeDesignRow(index + 1, eye?.Direction ?? 0, eye?.Width ?? 0);
            })
            .ToArray();
        return new EyeDesignerModel(organism.Aim, rows, organism.Vision.FocusEye);
    }

    public bool TrySet(int index, string directionText, string widthText, out string error)
    {
        if (!int.TryParse(directionText, NumberStyles.Integer, CultureInfo.InvariantCulture, out var direction))
        {
            error = $"EYE {index + 1} DIRECTION MUST BE AN INTEGER";
            return false;
        }
        if (!int.TryParse(widthText, NumberStyles.Integer, CultureInfo.InvariantCulture, out var width))
        {
            error = $"EYE {index + 1} WIDTH MUST BE AN INTEGER";
            return false;
        }
        return TrySet(index, direction, width, out error);
    }

    public bool TrySet(int index, int direction, int width, out string error)
    {
        if (index is < 0 or >= EyeCount)
        {
            error = "EYE ROW IS OUT OF RANGE";
            return false;
        }
        if (!IsValidValue(direction) || !IsValidValue(width))
        {
            error = "EYE VALUES MUST BE BETWEEN -32000 AND 32000";
            return false;
        }
        _rows[index] = _rows[index] with { Direction = direction, Width = width };
        error = string.Empty;
        return true;
    }

    public VisionSnapshot Preview
    {
        get
        {
            var eyes = _rows.Select((row, index) =>
            {
                var effectiveWidth = AbsoluteWidth(row.Width);
                var halfWidth = effectiveWidth / 200f / 2f;
                var range = SightDistance(effectiveWidth);
                var center = NormalizeRadians(
                    Aim / 200f + row.Direction / 200f + (4 - index) * MathF.PI / 18f);
                return new EyeSnapshot(row.Direction, row.Width, center, halfWidth, range, 0);
            }).ToArray();
            return new VisionSnapshot(FocusEye, eyes);
        }
    }

    public string WriteDna()
    {
        var builder = new StringBuilder("cond\n*.robage 0 =\nstart\n");
        foreach (var row in _rows)
        {
            builder.Append(row.Direction.ToString(CultureInfo.InvariantCulture))
                .Append(" .eye").Append(row.Eye).Append("dir store\n")
                .Append(row.Width.ToString(CultureInfo.InvariantCulture))
                .Append(" .eye").Append(row.Eye).Append("width store\n'\n");
        }
        builder.Append("stop");
        return builder.ToString();
    }

    private static bool IsValidValue(int value) => value is >= MinimumValue and <= MaximumValue;

    private static int AbsoluteWidth(int width)
    {
        if (width == 0) return 35;
        var absolute = width % 1256 + 35;
        return absolute <= 0 ? absolute + 1256 : absolute;
    }

    private static float SightDistance(int effectiveWidth)
    {
        if (effectiveWidth == 35) return 1_440f;
        var range = 1_440f * (1f - MathF.Log(effectiveWidth / 35f) / 4f);
        return float.IsFinite(range) ? MathF.Max(0f, range) : 0f;
    }

    private static float NormalizeRadians(float value) =>
        (value % MathF.Tau + MathF.Tau) % MathF.Tau;
}
