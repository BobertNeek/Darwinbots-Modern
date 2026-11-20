namespace Darwinbots.Desktop.Core;

public sealed class RobotColorEditorState
{
    public RobotColorEditorState(uint initialColor)
    {
        Red = (byte)(initialColor >> 16);
        Green = (byte)(initialColor >> 8);
        Blue = (byte)initialColor;
    }

    public byte Red { get; private set; }
    public byte Green { get; private set; }
    public byte Blue { get; private set; }

    public uint Color => 0xff000000u | (uint)(Red << 16) | (uint)(Green << 8) | Blue;

    public string PreviewHex => $"#{Color & 0x00ffffff:X6}";

    public void SetRed(byte value) => Red = value;

    public void SetGreen(byte value) => Green = value;

    public void SetBlue(byte value) => Blue = value;
}

public sealed record RobotColorEdit(uint Color, bool WriteToDna);

public static class RobotColorDna
{
    public static string Apply(string dna, uint color)
    {
        ArgumentNullException.ThrowIfNull(dna);

        var comments = new List<string>();
        var body = new List<string>();
        var inBody = false;
        var lines = dna.Replace("\r\n", "\n", StringComparison.Ordinal).Replace('\r', '\n').Split('\n');
        foreach (var line in lines)
        {
            var trimmed = line.Trim();
            if (!inBody && (trimmed.Length == 0 || trimmed.StartsWith("'", StringComparison.Ordinal)))
            {
                comments.Add(line);
                continue;
            }

            inBody = true;
            if (trimmed.Equals("@", StringComparison.Ordinal)
                || trimmed.StartsWith("def red", StringComparison.OrdinalIgnoreCase)
                || trimmed.StartsWith("def green", StringComparison.OrdinalIgnoreCase)
                || trimmed.StartsWith("def blue", StringComparison.OrdinalIgnoreCase))
                continue;

            body.Add(line);
        }

        while (body.Count > 0 && string.IsNullOrWhiteSpace(body[^1]))
            body.RemoveAt(body.Count - 1);

        var metadata = new[]
        {
            "@",
            $"def red {(byte)(color >> 16)}",
            $"def green {(byte)(color >> 8)}",
            $"def blue {(byte)color}",
        };
        return string.Join("\n", comments.Concat(metadata).Concat(body));
    }
}
