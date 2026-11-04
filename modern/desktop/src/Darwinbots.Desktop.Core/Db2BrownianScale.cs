namespace Darwinbots.Desktop.Core;

public static class Db2BrownianScale
{
    public const float MediumValue = 0.5f;
    public const float HighValue = 7f;
    public const decimal MediumPercent = 75m;

    public static float FromPercent(decimal percent)
    {
        var clamped = Math.Clamp(percent, 0m, 100m);
        if (clamped <= MediumPercent)
            return (float)(clamped / MediumPercent * (decimal)MediumValue);

        return (float)((decimal)MediumValue
            + ((clamped - MediumPercent) / (100m - MediumPercent))
            * ((decimal)HighValue - (decimal)MediumValue));
    }

    public static decimal ToPercent(float value)
    {
        if (!float.IsFinite(value) || value <= 0f)
            return 0m;

        var clamped = Math.Min(value, HighValue);
        if (clamped <= MediumValue)
            return (decimal)clamped / (decimal)MediumValue * MediumPercent;

        return MediumPercent
            + (((decimal)clamped - (decimal)MediumValue)
            / ((decimal)HighValue - (decimal)MediumValue))
            * (100m - MediumPercent);
    }
}
