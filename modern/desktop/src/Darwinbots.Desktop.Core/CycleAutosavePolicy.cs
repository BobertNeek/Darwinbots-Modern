namespace Darwinbots.Desktop.Core;

public static class CycleAutosavePolicy
{
    public const ulong SafeModeInterval = 2_000;

    public static bool SafeModeEligible(LocalModeSnapshot mode) =>
        (mode.Configuration is null && mode.Status == LocalModeStatus.Idle)
        || ((mode.Kind is LocalModeKind.SimpleSurvival or LocalModeKind.EcoSurvival or LocalModeKind.Zerobot)
            && mode.Evolution?.Phase is not ("testing" or "zerobot_testing" or "complete"));

    public static bool IsDue(ulong tick, bool safeMode, ulong explicitInterval, LocalModeSnapshot mode, int? nativeCycle = null) =>
        (safeMode && SafeModeEligible(mode) && (nativeCycle is { } cycle
            ? cycle > 0 && cycle % (long)SafeModeInterval == 0
            : tick > 0 && tick % SafeModeInterval == 0))
        || (tick > 0 && explicitInterval > 0 && tick % explicitInterval == 0);

    public static uint LimitBatch(ulong tick, uint requested, bool safeMode, ulong explicitInterval, LocalModeSnapshot mode, int? nativeCycle = null)
    {
        var count = (ulong)requested;
        if (safeMode && SafeModeEligible(mode))
        {
            var distance = nativeCycle is { } cycle
                ? (ulong)(cycle < 0 ? (long)SafeModeInterval - cycle : (long)SafeModeInterval - cycle % (long)SafeModeInterval)
                : SafeModeInterval - tick % SafeModeInterval;
            count = Math.Min(count, distance);
        }
        // Staged testing can become eligible and restart inside the same batch.
        // Observe native restart-capable worlds while SafeMode is enabled.
        if (safeMode && nativeCycle.HasValue && (mode.Evolution is { Phase: not "complete" }
            || (SafeModeEligible(mode) && mode.Configuration is { RestartOnExtinction: true })))
            count = Math.Min(count, 1ul);
        if (explicitInterval > 0)
            count = Math.Min(count, explicitInterval - tick % explicitInterval);
        return (uint)count;
    }
}
