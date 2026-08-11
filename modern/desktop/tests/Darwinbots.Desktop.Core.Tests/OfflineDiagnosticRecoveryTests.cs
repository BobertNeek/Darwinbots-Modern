using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class OfflineDiagnosticRecoveryTests
{
    [Fact]
    public void OfflineErrorsCanBeIgnoredRecoveredAndRestoredFromTray()
    {
        var diagnostics = new OfflineDiagnosticRecovery();

        var recovered = diagnostics.HandleError(
            new InvalidOperationException("bad DNA"),
            tick: 42,
            recoveryArtifactPath: @"C:\recovery\autosave-42.db3s");

        Assert.Equal(DiagnosticRecoveryAction.Recover, recovered.Action);
        Assert.True(recovered.StopSimulation);
        Assert.Equal(@"C:\recovery\autosave-42.db3s", diagnostics.RecoveryArtifactPath);
        Assert.Equal((ulong)42, diagnostics.LastErrorTick);

        diagnostics.SetIgnoreErrors(true);
        var ignored = diagnostics.HandleError(
            new InvalidOperationException("transient"),
            tick: 43,
            recoveryArtifactPath: null);

        Assert.Equal(DiagnosticRecoveryAction.Ignore, ignored.Action);
        Assert.False(ignored.StopSimulation);
        Assert.Equal("transient", diagnostics.LastErrorMessage);

        diagnostics.HideToTray();
        Assert.True(diagnostics.IsInTray);

        diagnostics.RestoreFromTray();
        Assert.False(diagnostics.IsInTray);
    }

    [Fact]
    public void FaultsReportTheLastStartArtifactAndABoundedTwoThousandCycleWindow()
    {
        var diagnostics = new OfflineDiagnosticRecovery();
        diagnostics.BeginSession(@"C:\recovery\last-start.json");
        diagnostics.RecordCheckpoint(10_000, @"C:\recovery\autosave-10000.db3s");

        var decision = diagnostics.HandleError(
            new InvalidOperationException("bad DNA"),
            tick: 11_987,
            recoveryArtifactPath: @"C:\recovery\autosave-10000.db3s");

        Assert.Equal(new FaultWindow(10_000, 11_987), decision.FaultWindow);
        Assert.Equal(@"C:\recovery\last-start.json", diagnostics.LastStartArtifactPath);
        Assert.Contains("FAULT WINDOW 10,000–11,987", decision.Status, StringComparison.Ordinal);
        Assert.Contains("LAST START READY", decision.Status, StringComparison.Ordinal);
    }

    [Fact]
    public void DecisionPolicyLogsRecoverableFaultsButRecoversNativeCorruptionEvenWhenIgnoreErrorsIsEnabled()
    {
        var diagnostics = new OfflineDiagnosticRecovery();
        diagnostics.SetIgnoreErrors(true);

        var ignored = diagnostics.HandleError(
            new InvalidOperationException("transient simulation fault"),
            tick: 200,
            recoveryArtifactPath: null);

        Assert.Equal(DiagnosticRecoveryAction.Ignore, ignored.Action);
        Assert.False(ignored.StopSimulation);
        Assert.Equal("transient simulation fault", diagnostics.LastErrorMessage);
        Assert.Equal((ulong)200, diagnostics.LastErrorTick);
        Assert.False(diagnostics.IsInTray);

        var recovered = diagnostics.HandleError(
            new AccessViolationException("native corruption"),
            tick: 201,
            recoveryArtifactPath: @"C:\recovery\autosave-201.db3s");

        Assert.Equal(DiagnosticRecoveryAction.Recover, recovered.Action);
        Assert.True(recovered.StopSimulation);
        Assert.Equal(@"C:\recovery\autosave-201.db3s", diagnostics.RecoveryArtifactPath);
        Assert.Contains("RECOVERY READY", recovered.Status, StringComparison.Ordinal);
        Assert.False(diagnostics.IsInTray);
    }
}
