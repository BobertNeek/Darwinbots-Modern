namespace Darwinbots.Desktop.Core;

public enum DiagnosticRecoveryAction
{
    Report,
    Ignore,
    Recover,
}

public sealed record FaultWindow(ulong StartTick, ulong EndTick)
{
    public ulong Width => EndTick >= StartTick ? EndTick - StartTick + 1 : 0;
}

public sealed record DiagnosticRecoveryDecision(
    DiagnosticRecoveryAction Action,
    bool StopSimulation,
    string Status)
{
    public FaultWindow? FaultWindow { get; init; }
}

public sealed class OfflineDiagnosticRecovery
{
    public const ulong FaultWindowSize = 2_000;

    public bool IgnoreErrors { get; private set; }
    public bool IsInTray { get; private set; }
    public string? LastErrorMessage { get; private set; }
    public ulong? LastErrorTick { get; private set; }
    public string? RecoveryArtifactPath { get; private set; }
    public string? LastStartArtifactPath { get; private set; }
    public ulong? LastCheckpointTick { get; private set; }
    public FaultWindow? CurrentFaultWindow { get; private set; }

    public void SetIgnoreErrors(bool enabled) => IgnoreErrors = enabled;

    public void BeginSession(string? lastStartArtifactPath)
    {
        LastStartArtifactPath = string.IsNullOrWhiteSpace(lastStartArtifactPath) ? null : lastStartArtifactPath;
        LastCheckpointTick = null;
        CurrentFaultWindow = null;
    }

    public void RecordCheckpoint(ulong tick, string? artifactPath)
    {
        if (LastCheckpointTick is null || tick >= LastCheckpointTick)
            LastCheckpointTick = tick;
        if (!string.IsNullOrWhiteSpace(artifactPath)) RecoveryArtifactPath = artifactPath;
    }

    public DiagnosticRecoveryDecision HandleError(
        Exception error,
        ulong tick,
        string? recoveryArtifactPath)
    {
        ArgumentNullException.ThrowIfNull(error);

        LastErrorMessage = error.Message;
        LastErrorTick = tick;
        if (!string.IsNullOrWhiteSpace(recoveryArtifactPath))
            RecoveryArtifactPath = recoveryArtifactPath;
        CurrentFaultWindow = NarrowFaultWindow(tick);
        var faultStatus = $" · FAULT WINDOW {CurrentFaultWindow.StartTick:N0}–{CurrentFaultWindow.EndTick:N0}";
        var lastStartStatus = LastStartArtifactPath is null ? string.Empty : " · LAST START READY";

        if (IgnoreErrors)
            return new(DiagnosticRecoveryAction.Ignore, false, $"ERROR IGNORED · {error.Message}{faultStatus}{lastStartStatus}")
            {
                FaultWindow = CurrentFaultWindow,
            };

        if (!string.IsNullOrWhiteSpace(RecoveryArtifactPath))
        {
            return new(
                DiagnosticRecoveryAction.Recover,
                true,
                $"ENGINE ERROR · TICK {tick:N0} · RECOVERY READY · {Path.GetFileName(RecoveryArtifactPath)}{faultStatus}{lastStartStatus}")
            {
                FaultWindow = CurrentFaultWindow,
            };
        }

        return new(DiagnosticRecoveryAction.Report, true, $"ENGINE ERROR · {error.Message}{faultStatus}{lastStartStatus}")
        {
            FaultWindow = CurrentFaultWindow,
        };
    }

    public void HideToTray() => IsInTray = true;

    public void RestoreFromTray() => IsInTray = false;

    private FaultWindow NarrowFaultWindow(ulong tick)
    {
        var start = LastCheckpointTick is { } checkpoint && checkpoint <= tick
            && tick - checkpoint < FaultWindowSize
            ? checkpoint
            : tick / FaultWindowSize * FaultWindowSize;
        var end = start > ulong.MaxValue - (FaultWindowSize - 1)
            ? ulong.MaxValue
            : start + FaultWindowSize - 1;
        return new(start, Math.Min(tick, end));
    }
}
