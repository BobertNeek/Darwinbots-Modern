namespace Darwinbots.Desktop.Core;

public static class ObstacleEditorPolicy
{
    public static ObstacleSnapshot BuildPayload(
        ObstacleSnapshot original,
        float[] minimum,
        float[] maximum,
        uint color,
        float opacity,
        float[] drift,
        string visionPolicy,
        string shotPolicy)
    {
        ArgumentNullException.ThrowIfNull(original);
        ValidateVector(minimum, nameof(minimum));
        ValidateVector(maximum, nameof(maximum));
        ValidateVector(drift, nameof(drift));
        if (!float.IsFinite(opacity) || opacity is < 0f or > 1f)
            throw new ArgumentOutOfRangeException(nameof(opacity));
        ArgumentException.ThrowIfNullOrWhiteSpace(visionPolicy);
        ArgumentException.ThrowIfNullOrWhiteSpace(shotPolicy);

        return original with
        {
            Minimum = [.. minimum],
            Maximum = [.. maximum],
            Color = color,
            Opacity = opacity,
            Drift = [.. drift],
            VisionPolicy = visionPolicy,
            ShotPolicy = shotPolicy,
        };
    }

    public static bool ConfirmDelete(bool confirmed) => confirmed;

    private static void ValidateVector(float[] value, string name)
    {
        ArgumentNullException.ThrowIfNull(value, name);
        if (value.Length != 2 || value.Any(number => !float.IsFinite(number)))
            throw new ArgumentException("Obstacle vectors must contain two finite values.", name);
    }
}
