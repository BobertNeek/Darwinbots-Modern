using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class CommunityLeagueNativeTests
{
    [Theory]
    [InlineData(LeaguePreset.ShortBot, "at most 6 genes")]
    [InlineData(LeaguePreset.SingleGene, "at most 6 genes")]
    [InlineData(LeaguePreset.SingleStore, "exactly 1 store")]
    public void InvalidEntrantsReachTheDesktopAsActionableErrorsWithoutChangingTheWorld(
        LeaguePreset preset, string explanation)
    {
        using var engine = new NativeEngineClient(backend: "Cpu");
        var dna = preset == LeaguePreset.SingleStore
            ? "start .up inc .dn dec stop"
            : string.Concat(Enumerable.Repeat("start stop\n", 7));
        engine.ImportDna(dna, [500, 500]);
        var before = engine.Save();
        var error = Assert.Throws<InvalidOperationException>(() =>
            engine.ConfigureLocalMode(LocalModeConfig.Default with { Preset = preset }));
        Assert.Contains(explanation, error.Message);
        Assert.Equal(before, engine.Save());
    }

    [Fact]
    public void SgConfigurationKeepsItsLegacySaveTokenAndShortBotArenaAfterResume()
    {
        using var engine = new NativeEngineClient(backend: "Cpu");
        engine.ImportDna(string.Concat(Enumerable.Repeat("start stop\n", 6)), [500, 500]);
        engine.ConfigureLocalMode(LocalModeConfig.Default with { Preset = LeaguePreset.SingleGene });
        using var resumed = new NativeEngineClient(backend: "Cpu");
        resumed.Load(engine.Save());
        resumed.StartLocalMode();
        Assert.Equal(new float[] { 16000, 12000 }, resumed.Snapshot().WorldSize);
        Assert.Equal(LeaguePreset.SingleGene, resumed.Snapshot().LocalMode.Preset);
    }
}
