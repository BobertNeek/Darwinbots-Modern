namespace Darwinbots.Desktop.Core;

public readonly record struct VisualPoint(float X, float Y);

public readonly record struct EyeSectorGeometry(
    float StartRadians,
    float SweepRadians,
    float Range,
    bool Focused);

public static class OrganismVisualGeometry
{
    private const float AimUnitsPerRadian = 200f;

    public static VisualPoint[] SkinPoints(
        IReadOnlyList<SkinPointSnapshot> skin,
        float radius,
        int aim)
    {
        ArgumentNullException.ThrowIfNull(skin);
        var points = new VisualPoint[skin.Count];
        var aimRadians = aim / AimUnitsPerRadian;
        for (var index = 0; index < skin.Count; index++)
        {
            var point = skin[index];
            var angle = aimRadians + point.Angle / AimUnitsPerRadian;
            var distance = radius * Math.Clamp(point.Radius, 0.15f, 0.82f);
            points[index] = new VisualPoint(
                MathF.Sin(angle) * distance,
                MathF.Cos(angle) * distance);
        }
        return points;
    }

    public static VisualPoint HeadingPoint(float radius, int aim)
    {
        var angle = aim / AimUnitsPerRadian;
        return new VisualPoint(
            MathF.Sin(angle) * radius,
            MathF.Cos(angle) * radius);
    }

    public static float SelectionRingRadius(float renderedRadius) =>
        MathF.Max(renderedRadius + 2.2f, 5.5f);

    public static EyeSectorGeometry[] EyeSectors(
        VisionSnapshot vision,
        int aim,
        float radius)
    {
        ArgumentNullException.ThrowIfNull(vision);
        var eyes = vision.Eyes ?? [];
        var sectors = new EyeSectorGeometry[eyes.Length];
        var aimRadians = aim / AimUnitsPerRadian;
        for (var index = 0; index < eyes.Length; index++)
        {
            var eye = eyes[index];
            var center = aimRadians
                + eye.Direction / AimUnitsPerRadian
                + (4 - index) * MathF.PI / 18f;
            var halfWidth = float.IsFinite(eye.HalfWidthRadians)
                ? Math.Clamp(MathF.Abs(eye.HalfWidthRadians), MathF.PI / 360f, MathF.PI)
                : MathF.PI / 36f;
            sectors[index] = new EyeSectorGeometry(
                NormalizeRadians(center - halfWidth),
                Math.Clamp(halfWidth * 2f, MathF.PI / 180f, MathF.Tau),
                Math.Max(radius, float.IsFinite(eye.Range) ? eye.Range : 1_440f),
                index == vision.FocusEye);
        }
        return sectors;
    }

    private static float NormalizeRadians(float value) =>
        (value % MathF.Tau + MathF.Tau) % MathF.Tau;
}
