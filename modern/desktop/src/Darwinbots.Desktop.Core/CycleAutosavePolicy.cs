namespace Darwinbots.Desktop.Core;

public static class CycleAutosavePolicy
{
    public const ulong SafeModeInterval = 2_000;

    public static bool SafeModeEligible(LocalModeSnapshot mode) =>
        (mode.Configuration is null && mode.Status == LocalModeStatus.Idle)
        || ((mode.Kind is LocalModeKind.SimpleSurvival or LocalModeKind.EcoSurvival or LocalModeKind.Zerobot)
            && mode.Evolution?.Phase is not ("testing" or "zerobot_testing" or "complete"));

    public static bool IsDue(ulong tick, bool safeMode, ulong explicitInterval, LocalModeSnapshot mode) =>
        tick > 0 && ((safeMode && SafeModeEligible(mode) && tick % SafeModeInterval == 0)
            || (explicitInterval > 0 && tick % explicitInterval == 0));

    public static uint LimitBatch(ulong tick, uint requested, bool safeMode, ulong explicitInterval, LocalModeSnapshot mode)
    {
        var count = (ulong)requested;
        if (safeMode && SafeModeEligible(mode))
            count = Math.Min(count, SafeModeInterval - tick % SafeModeInterval);
        if (explicitInterval > 0)
            count = Math.Min(count, explicitInterval - tick % explicitInterval);
        return (uint)count;
    }
}
