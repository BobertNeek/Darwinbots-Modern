using System.Text.Json;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class RecognitionFudgingTests
{
    [Theory]
    [InlineData(0)]
    [InlineData(1)]
    [InlineData(2)]
    public void OriginalModesSurviveGlobalSettingsAndInitialEngineConfiguration(int mode)
    {
        var settings = new GlobalSettings
        {
            League = new GlobalLeagueSettings { RecognitionFudging = mode },
        };
        var restored = GlobalSettings.FromJson(settings.ToJson());
        Assert.Equal(mode, restored.League.RecognitionFudging);

        using var config = JsonDocument.Parse(NativeCommandSerializer.SerializeEngineConfig(
            new WorldSetupOptions { GlobalSettings = restored }));
        Assert.Equal(mode, config.RootElement.GetProperty("recognition_fudging").GetInt32());
    }

    [Fact]
    public void OlderGlobalSettingsDefaultToNoRecognitionFudging()
    {
        Assert.Equal(0, GlobalSettings.FromJson("{}").League.RecognitionFudging);
        var legacy = GlobalSettings.FromJson("{\"league\":{\"disqualificationRule\":2}}");
        Assert.Equal(2, legacy.League.DisqualificationRule);
        Assert.Equal(0, legacy.League.RecognitionFudging);
    }

    [Theory]
    [InlineData(-1)]
    [InlineData(3)]
    public void UnknownRecognitionFudgingModesAreRejected(int mode)
    {
        var settings = new GlobalSettings
        {
            League = new GlobalLeagueSettings { RecognitionFudging = mode },
        };
        Assert.Throws<InvalidDataException>(() => GlobalSettings.FromJson(settings.ToJson()));
    }

    [Fact]
    public async Task LiveRecognitionFudgingChangesRunOnTheSimulationWorkerWithoutTicking()
    {
        var engine = new FakeEngineClient();
        await using var session = new SimulationSession(engine);
        var callerThread = Environment.CurrentManagedThreadId;

        await session.UpdateRecognitionFudgingAsync(1);
        await session.UpdateRecognitionFudgingAsync(2);

        Assert.Equal(new[] { 1, 2 }, engine.RecognitionFudgingUpdates);
        Assert.Equal(0ul, session.LatestSnapshot.Tick);
        Assert.NotEqual(callerThread, Assert.Single(engine.ExecutionThreads));
    }
}
