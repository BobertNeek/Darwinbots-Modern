using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class NativeSetupAppearanceTests
{
    [Fact]
    public void ColdSetupUsesBothRawTimerSamplesAndTheSuppliedUniformState()
    {
        var clock = new SetupClock(1.25, 12_345.5);
        var result = Assert.IsType<NativeSpeciesAppearance>(NativeEngineClient.StageNativeSetupAppearance(
            "Test.txt", "start stop", useOldColors: true, uniformState: 0x08_d82a, timeProvider: clock));

        Assert.Equal(2, clock.Reads);
        Assert.Equal(0xffff_0000u, result.Argb);
        Assert.Equal(0x31_2200u, result.UniformState);
        Assert.Equal(new SkinPointSnapshot[]
        {
            new(36f / 60, 314 - 2 * 504),
            new(10f / 60, 314 - 2 * 212),
            new(26f / 60, 314 - 2 * 622),
            new(16f / 60, 314 - 2 * 287),
        }, result.Skin);
    }

    [Fact]
    public void LiveStagingCommitsTheUniformStateAndPreservesTheWorldAndGaussianSpare()
    {
        var clock = new SetupClock(1.25, 12_345.5);
        using var engine = new NativeEngineClient(new WorldSetupOptions { Backend = "Cpu", NativeGaussianSpare = 0.125f }, clock);
        var before = engine.Snapshot();
        var expected = Assert.IsType<NativeSpeciesAppearance>(NativeEngineClient.StageNativeSetupAppearance(
            "Palette.txt", "start stop", useOldColors: false, uniformState: engine.ReadNativeSetupUniformState(),
            timeProvider: new SetupClock(1.25, 12_345.5)));

        var applied = Assert.IsType<NativeSpeciesAppearance>(engine.StageNativeSpeciesAppearance("Palette.txt", "start stop", false));
        Assert.Equal(expected.Argb, applied.Argb);
        Assert.Equal(expected.Skin, applied.Skin);
        Assert.Equal(expected.UniformState, engine.ReadNativeSetupUniformState());
        Assert.Equal(0.125f, engine.ReadNativeGaussianSpare());
        Assert.Equal(before.Tick, engine.Snapshot().Tick);
        Assert.Equal(before.NativeCycle, engine.Snapshot().NativeCycle);
        Assert.Equal(before.Organisms.Count, engine.Snapshot().Organisms.Count);
        Assert.Equal(2, clock.Reads);
    }

    [Fact]
    public void UnsupportedLegacyInputLeavesTheEngineStateIntact()
    {
        var clock = new SetupClock(1.25);
        using var engine = new NativeEngineClient(new WorldSetupOptions { Backend = "Cpu" }, clock);
        var before = engine.ReadNativeSetupUniformState();
        Assert.Null(engine.StageNativeSpeciesAppearance("Unknown.txt", "start 32768 stop", false));
        Assert.Null(engine.StageNativeSpeciesAppearance("日本語.txt", "start stop", false));
        Assert.Equal(before, engine.ReadNativeSetupUniformState());
        Assert.Equal(1, clock.Reads);
    }

    private sealed class SetupClock(params double[] seconds) : TimeProvider
    {
        private static readonly DateTimeOffset Midnight = new(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
        public int Reads { get; private set; }
        public override TimeZoneInfo LocalTimeZone => TimeZoneInfo.Utc;
        public override DateTimeOffset GetUtcNow() => Midnight.AddSeconds(seconds[Reads++]);
    }
}
