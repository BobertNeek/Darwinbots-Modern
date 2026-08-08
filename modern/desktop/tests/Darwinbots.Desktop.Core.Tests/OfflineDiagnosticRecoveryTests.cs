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
}
