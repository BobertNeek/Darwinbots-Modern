namespace Darwinbots.Desktop.Core;

public enum DiagnosticRecoveryAction
{
    Report,
    Ignore,
    Recover,
}

public sealed record DiagnosticRecoveryDecision(
    DiagnosticRecoveryAction Action,
    bool StopSimulation,
    string Status);

public sealed class OfflineDiagnosticRecovery
{
    public bool IgnoreErrors { get; private set; }
    public bool IsInTray { get; private set; }
    public string? LastErrorMessage { get; private set; }
    public ulong? LastErrorTick { get; private set; }
    public string? RecoveryArtifactPath { get; private set; }

    public void SetIgnoreErrors(bool enabled) => IgnoreErrors = enabled;

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

        if (IgnoreErrors)
            return new(DiagnosticRecoveryAction.Ignore, false, $"ERROR IGNORED · {error.Message}");

        if (!string.IsNullOrWhiteSpace(RecoveryArtifactPath))
        {
            return new(
                DiagnosticRecoveryAction.Recover,
                true,
                $"ENGINE ERROR · TICK {tick:N0} · RECOVERY READY · {Path.GetFileName(RecoveryArtifactPath)}");
        }

        return new(DiagnosticRecoveryAction.Report, true, $"ENGINE ERROR · {error.Message}");
    }

    public void HideToTray() => IsInTray = true;

    public void RestoreFromTray() => IsInTray = false;
}
