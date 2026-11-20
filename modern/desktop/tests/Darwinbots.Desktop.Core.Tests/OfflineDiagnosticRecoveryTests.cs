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

    [Fact]
    public async Task FailedLoadExposesSafeModeFaultWindowAndValidRecoveryPublishesCheckpointWithoutMutatingFailedState()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);
        EngineSnapshot? visibleSnapshot = null;
        session.SnapshotPublished += snapshot => visibleSnapshot = snapshot;

        await session.StepAsync(7);
        var checkpointSave = await session.SaveAsync();
        var checkpoint = session.LatestSnapshot;
        var recoveryArtifact = @"C:\recovery\autosave-7.db3s";
        var diagnostics = new OfflineDiagnosticRecovery();
        diagnostics.BeginSession(@"C:\recovery\last-start.json");
        diagnostics.RecordCheckpoint(checkpoint.Tick, recoveryArtifact);

        await session.StepAsync(5);
        var failedPreState = session.LatestSnapshot;
        Assert.Same(failedPreState, visibleSnapshot);

        var fault = await Assert.ThrowsAsync<ArgumentOutOfRangeException>(() => session.LoadAsync([1, 2, 3]));
        var decision = diagnostics.HandleError(fault, failedPreState.Tick, diagnostics.RecoveryArtifactPath);

        Assert.Equal(DiagnosticRecoveryAction.Recover, decision.Action);
        Assert.True(decision.StopSimulation);
        Assert.Equal(new FaultWindow(checkpoint.Tick, failedPreState.Tick), decision.FaultWindow);
        Assert.Contains("RECOVERY READY", decision.Status, StringComparison.Ordinal);
        Assert.Contains("FAULT WINDOW 7–12", decision.Status, StringComparison.Ordinal);
        Assert.Equal(failedPreState, session.LatestSnapshot);
        Assert.Same(failedPreState, visibleSnapshot);

        await session.LoadAsync(checkpointSave);

        Assert.NotSame(failedPreState, session.LatestSnapshot);
        Assert.Equal(checkpoint.Tick, session.LatestSnapshot.Tick);
        Assert.Equal(checkpoint.Tick, visibleSnapshot!.Tick);
    }
}
