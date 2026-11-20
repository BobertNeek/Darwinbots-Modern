using System.Text.Json;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class CycleAutosavePolicyTests
{
    [Fact]
    public void NativeSafeModeUsesPositiveOriginalCyclesWhileExplicitIntervalUsesSchedulingTicks()
    {
        var mode = LocalModeSnapshot.Idle;
        Assert.False(CycleAutosavePolicy.IsDue(2000, true, 0, mode, -1));
        Assert.False(CycleAutosavePolicy.IsDue(2000, true, 0, mode, 0));
        Assert.False(CycleAutosavePolicy.IsDue(2000, true, 0, mode, 1999));
        Assert.True(CycleAutosavePolicy.IsDue(2001, true, 0, mode, 2000));
        Assert.True(CycleAutosavePolicy.IsDue(4007, true, 0, mode, 2000));
        Assert.True(CycleAutosavePolicy.IsDue(7, false, 7, mode, -1));
        Assert.False(CycleAutosavePolicy.IsDue(8, false, 7, mode, 2000));
        Assert.Equal(2001u, CycleAutosavePolicy.LimitBatch(0, 4000, true, 0, mode, -1));
        Assert.Equal(2000u, CycleAutosavePolicy.LimitBatch(1, 4000, true, 0, mode, 0));
        Assert.Equal(1u, CycleAutosavePolicy.LimitBatch(1999, 100, true, 0, mode, 1999));
        Assert.Equal(2u, CycleAutosavePolicy.LimitBatch(5, 4000, true, 7, mode, -1));
        Assert.Equal(0u, CycleAutosavePolicy.LimitBatch(0, 0, true, 0, mode, -1));
    }

    [Fact]
    public void OnlyNativeSafeModeObservesRestartCapableBatchesIncludingPhasesThatCanBecomeEligible()
    {
        var mode = LocalModeSnapshot.Idle with
        {
            Status = LocalModeStatus.Running,
            Kind = LocalModeKind.SimpleSurvival,
            Configuration = LocalModeConfig.Default with { Kind = LocalModeKind.SimpleSurvival, RestartOnExtinction = true },
        };
        Assert.Equal(1u, CycleAutosavePolicy.LimitBatch(100, 100, true, 0, mode, 10));
        Assert.Equal(100u, CycleAutosavePolicy.LimitBatch(100, 100, false, 0, mode, 10));
        Assert.Equal(100u, CycleAutosavePolicy.LimitBatch(100, 100, true, 0, mode));
        Assert.Equal(100u, CycleAutosavePolicy.LimitBatch(100, 100, true, 0, LocalModeSnapshot.Idle, 10));
        Assert.Equal(100u, CycleAutosavePolicy.LimitBatch(100, 100, true, 0,
            mode with { Kind = LocalModeKind.Contest, Configuration = mode.Configuration! with { Kind = LocalModeKind.Contest } }, 10));
        var staged = mode with
        {
            Configuration = mode.Configuration! with { RestartOnExtinction = false },
            Evolution = new StagedEvolutionSnapshot(1, "evolving", false, 10, 1500, 0, 0, 0),
        };
        Assert.Equal(1u, CycleAutosavePolicy.LimitBatch(100, 100, true, 0, staged, 10));
        Assert.Equal(1u, CycleAutosavePolicy.LimitBatch(100, 100, true, 0,
            staged with { Evolution = staged.Evolution! with { Phase = "testing" } }, 10));
        Assert.Equal(100u, CycleAutosavePolicy.LimitBatch(100, 100, true, 0,
            staged with { Evolution = staged.Evolution! with { Phase = "complete" } }, 10));
    }

    [Fact]
    public void OriginalSafeModeDefaultIsEnabledAndExplicitFalseRemainsDisabled()
    {
        Assert.True(new GlobalSettings().Main.SafeMode);
        Assert.True(GlobalSettings.FromJson("{}").Main.SafeMode);
        Assert.True(EnvironmentUpdate.Default.Recording!.SafeModeBackup);
        Assert.True(new WorldSetupOptions().ToEnvironmentUpdate().Recording!.SafeModeBackup);
        Assert.False(new WorldSetupOptions
        {
            GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { SafeMode = false } },
        }.ToEnvironmentUpdate().Recording!.SafeModeBackup);
    }

    [Fact]
    public void NormalWorldSavesAtPositiveTwoThousandCycleBoundaries()
    {
        Assert.False(CycleAutosavePolicy.IsDue(0, true, 0, LocalModeSnapshot.Idle));
        Assert.False(CycleAutosavePolicy.IsDue(1999, true, 0, LocalModeSnapshot.Idle));
        Assert.True(CycleAutosavePolicy.IsDue(2000, true, 0, LocalModeSnapshot.Idle));
        Assert.True(CycleAutosavePolicy.IsDue(4000, true, 0, LocalModeSnapshot.Idle));
        Assert.False(CycleAutosavePolicy.IsDue(2000, false, 0, LocalModeSnapshot.Idle));
        Assert.Equal(1u, CycleAutosavePolicy.LimitBatch(1999, 100, true, 0, LocalModeSnapshot.Idle));
        Assert.Equal(2000u, CycleAutosavePolicy.LimitBatch(2000, 4000, true, 0, LocalModeSnapshot.Idle));
    }

    [Theory]
    [InlineData(LocalModeKind.League, null, false)]
    [InlineData(LocalModeKind.Contest, null, false)]
    [InlineData(LocalModeKind.EcoSurvival, "evolving", true)]
    [InlineData(LocalModeKind.EcoSurvival, "testing", false)]
    [InlineData(LocalModeKind.SimpleSurvival, "evolving", true)]
    [InlineData(LocalModeKind.SimpleSurvival, "testing", false)]
    [InlineData(LocalModeKind.Zerobot, "zerobot_evolving", true)]
    [InlineData(LocalModeKind.Zerobot, "zerobot_testing", false)]
    [InlineData(LocalModeKind.Zerobot, "complete", false)]
    public void OriginalRestartModesControlSafeModeEligibility(LocalModeKind kind, string? phase, bool eligible)
    {
        var mode = LocalModeSnapshot.Idle with
        {
            Status = LocalModeStatus.Running,
            Kind = kind,
            Configuration = LocalModeConfig.Default with { Kind = kind },
            Evolution = phase is null ? null : new StagedEvolutionSnapshot(1, phase, false, 10, 1500, 0, 0, 0),
        };
        Assert.Equal(eligible, CycleAutosavePolicy.IsDue(2000, true, 0, mode));
        Assert.True(CycleAutosavePolicy.IsDue(2000, false, 100, mode));
    }

    [Fact]
    public void ExplicitModernIntervalIsIndependentAndCapsBatchesAtTheEarlierBoundary()
    {
        Assert.True(CycleAutosavePolicy.IsDue(7, false, 7, LocalModeSnapshot.Idle));
        Assert.False(CycleAutosavePolicy.IsDue(8, false, 7, LocalModeSnapshot.Idle));
        Assert.Equal(3u, CycleAutosavePolicy.LimitBatch(1997, 100, true, 13, LocalModeSnapshot.Idle));
        Assert.Equal(1u, CycleAutosavePolicy.LimitBatch(1999, 100, false, 8, LocalModeSnapshot.Idle));
        Assert.Equal(0u, CycleAutosavePolicy.LimitBatch(0, 0, true, 7, LocalModeSnapshot.Idle));
    }

    [Theory]
    [InlineData(false, false, false)]
    [InlineData(true, false, true)]
    [InlineData(false, true, true)]
    public void InitialGlobalAndRecordingPreferencesReachTheEngine(bool global, bool recording, bool expected)
    {
        using var config = JsonDocument.Parse(NativeCommandSerializer.SerializeEngineConfig(new WorldSetupOptions
        {
            GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { SafeMode = global } },
            Recording = new Db2RecordingOptions(SafeModeBackup: recording),
        }));
        Assert.Equal(expected, config.RootElement.GetProperty("recording").GetProperty("safemode_backup").GetBoolean());
    }
}
