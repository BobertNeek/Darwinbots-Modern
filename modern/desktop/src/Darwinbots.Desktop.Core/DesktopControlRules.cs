namespace Darwinbots.Desktop.Core;

public static class DesktopControlRules
{
    private const double DragThresholdSquared = 16.0;

    public static uint TicksForSelection(int selection) => selection switch
    {
        0 => 1,
        1 => 5,
        2 => 20,
        _ => 100,
    };

    public static bool IsMaximumSpeed(int selection) => selection >= 3;

    public static bool IsDragGesture(double deltaX, double deltaY) =>
        deltaX * deltaX + deltaY * deltaY >= DragThresholdSquared;

    public static uint AdaptMaximumBatch(uint current, TimeSpan elapsed, bool turbo)
    {
        var maximum = turbo ? 10_000u : 2_000u;
        current = Math.Clamp(current, 1u, maximum);
        var targetMilliseconds = turbo ? 100.0 : 32.0;
        var elapsedMilliseconds = Math.Max(0.1, elapsed.TotalMilliseconds);
        var scaled = current * targetMilliseconds / elapsedMilliseconds;
        var lower = Math.Max(1u, current / 2);
        var upper = Math.Min(maximum, Math.Max(current + 1, current * 2));
        return (uint)Math.Clamp(Math.Round(scaled), lower, upper);
    }
}
