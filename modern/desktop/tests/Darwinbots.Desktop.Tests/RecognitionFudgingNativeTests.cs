using System.Buffers.Binary;
using System.Text.Json;
using System.Text.Json.Nodes;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class RecognitionFudgingNativeTests
{
    [Theory]
    [InlineData(0, 1)]
    [InlineData(1, 2)]
    [InlineData(2, 0)]
    public void InitialAndLiveRecognitionModesPersistThroughTheNativeClient(int initialMode, int updatedMode)
    {
        using var engine = new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu",
            GlobalSettings = new GlobalSettings
            {
                League = new GlobalLeagueSettings { RecognitionFudging = initialMode },
            },
        });
        Assert.Equal(initialMode, engine.ReadConfiguration().RecognitionFudging);

        engine.UpdateRecognitionFudging(updatedMode);
        var save = engine.Save();
        Assert.Equal(updatedMode, engine.ReadConfiguration().RecognitionFudging);
        Assert.Equal(0ul, engine.Snapshot().Tick);
        Assert.Equal(save, engine.Save());

        using var restored = new NativeEngineClient(backend: "Cpu");
        restored.Load(save);
        Assert.Equal(updatedMode, restored.ReadConfiguration().RecognitionFudging);
        Assert.Equal(0ul, restored.Snapshot().Tick);
        Assert.Equal(save, restored.Save());
    }

    [Theory]
    [InlineData(-1)]
    [InlineData(3)]
    public void InvalidLiveModeDoesNotChangeTheNativeWorld(int mode)
    {
        using var engine = new NativeEngineClient(backend: "Cpu");
        var before = engine.Save();

        Assert.Throws<ArgumentOutOfRangeException>(() => engine.UpdateRecognitionFudging(mode));

        Assert.Equal(before, engine.Save());
        Assert.Equal(0, engine.ReadConfiguration().RecognitionFudging);
    }

    [Fact]
    public void OriginalSimulationRetainsCurrentLeaguePreferencesAndModernSaveRestoresThem()
    {
        var original = OriginalSimulation();

        using var engine = new NativeEngineClient(backend: "Cpu");
        engine.UpdateRecognitionFudging(2);
        engine.UpdateDisqualificationRules(2);
        engine.Load(original);
        Assert.Equal(2, engine.ReadConfiguration().RecognitionFudging);
        Assert.Equal(2, engine.ReadConfiguration().DisqualificationRule);
        Assert.Equal(281ul, engine.Snapshot().Tick);
        var modern = engine.Save();

        using var resumed = new NativeEngineClient(backend: "Cpu");
        resumed.UpdateRecognitionFudging(1);
        resumed.UpdateDisqualificationRules(1);
        resumed.Load(modern);
        Assert.Equal(2, resumed.ReadConfiguration().RecognitionFudging);
        Assert.Equal(2, resumed.ReadConfiguration().DisqualificationRule);
        Assert.Equal(281ul, resumed.Snapshot().Tick);

        resumed.UpdateRecognitionFudging(1);
        resumed.UpdateDisqualificationRules(1);
        resumed.Load(original);
        Assert.Equal(1, resumed.ReadConfiguration().RecognitionFudging);
        Assert.Equal(1, resumed.ReadConfiguration().DisqualificationRule);
        Assert.Equal(281ul, resumed.Snapshot().Tick);
    }

    [Fact]
    public void OriginalSimulationLoadsSavedOscillationAndRetainsGlobalMainAndMutationPolicies()
    {
        var original = OriginalSimulation();
        using var reference = new NativeEngineClient(backend: "Cpu");
        reference.Load(original);
        var originalConfiguration = reference.ReadConfiguration();
        var originalOscillation = originalConfiguration.AdvancedMutation;
        var originalRecording = originalConfiguration.Environment.Recording!;
        var advanced = new GlobalAdvancedMutationSettings
        {
            Point2Enabled = true, CopyError2Enabled = true,
            AmplificationEnabled = true, TranslocationEnabled = true,
            Delta2Enabled = true, Delta2Interval = 7, Delta2RateStep = 1.25,
            Delta2DistributionStep = 0.5, Delta2PreferenceStepPercent = 3,
            NativeDelta2 = new NativeDelta2Settings
            {
                RateExponential = 2, RateLinear = 3,
                DistributionExponential = 5, DistributionLinear = 7,
                RateChancePercent = 80, DistributionChancePercent = 40,
                PreferenceStep = 9, MaximumRatePerBase = 111,
            },
            NormalizationEnabled = true, NormalizationDnaLength = 1234,
            EpigeneticReset = true, EpigeneticAmplification = 1.25,
            EpigeneticExponent = 1.7, EpigeneticOverloadThreshold = 21,
            SaveAsTemporaryGene = true,
            OscillationMode = GlobalMutationOscillationMode.Sine,
            OscillationHighCycles = 123, OscillationLowCycles = 456,
            OscillationPeriod = 43, OscillationAmplitude = 2.75,
        };
        using var engine = new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu",
            Vegetation = Db2VegetationOptions.Default with
            {
                StartChloroplasts = 12345,
                MaxEnergyPerTick = originalConfiguration.Environment.Vegetation.MaxEnergyPerTick + 10,
            },
            Recording = originalRecording with
            {
                SafeModeBackup = true,
                DeadSnapshotEnabled = !originalRecording.DeadSnapshotEnabled,
                ExcludeRepopulatingRobots = !originalRecording.ExcludeRepopulatingRobots,
                SamplingInterval = originalRecording.SamplingInterval + 17,
            },
            GlobalSettings = new GlobalSettings
            {
                Main = new GlobalMainSettings { BodyCutoff = 12345, GreedyEnforcement = true },
                AdvancedMutation = advanced,
            },
        });
        // The native exponent is a Single, so compare the actual initial policy.
        var current = engine.ReadConfiguration();
        var expectedMutation = current.AdvancedMutation with
        {
            OscillationMode = originalOscillation.OscillationMode,
            OscillationHighCycles = originalOscillation.OscillationHighCycles,
            OscillationLowCycles = originalOscillation.OscillationLowCycles,
            OscillationPeriod = originalOscillation.OscillationPeriod,
            OscillationAmplitude = originalOscillation.OscillationAmplitude,
        };

        engine.Load(original);

        var loaded = engine.ReadConfiguration();
        Assert.Equal(12345, loaded.BodyCutoff);
        Assert.True(loaded.GreedyReproduction);
        Assert.Equal(expectedMutation, loaded.AdvancedMutation);
        var expectedVegetation = originalConfiguration.Environment.Vegetation with { StartChloroplasts = 12345 };
        var expectedRecording = originalRecording with { SafeModeBackup = true };
        Assert.Equal(expectedVegetation, loaded.Environment.Vegetation);
        Assert.Equal(expectedRecording, loaded.Environment.Recording);
        Assert.Equal(281ul, engine.Snapshot().Tick);
        var modern = engine.Save();
        engine.UpdateMainRules(new GlobalMainSettings { BodyCutoff = 999 });
        engine.UpdateAdvancedMutationSettings(new GlobalAdvancedMutationSettings(), new GlobalMainSettings());
        engine.UpdateEnvironment(loaded.Environment with
        {
            Vegetation = loaded.Environment.Vegetation with { StartChloroplasts = 2468 },
            Recording = loaded.Environment.Recording! with { SafeModeBackup = false },
        });

        engine.Load(modern);

        var resumed = engine.ReadConfiguration();
        Assert.Equal(12345, resumed.BodyCutoff);
        Assert.True(resumed.GreedyReproduction);
        Assert.Equal(expectedMutation, resumed.AdvancedMutation);
        Assert.Equal(expectedVegetation, resumed.Environment.Vegetation);
        Assert.Equal(expectedRecording, resumed.Environment.Recording);
        Assert.Equal(281ul, engine.Snapshot().Tick);
    }

    [Fact]
    public void LoadingAnOlderModernSaveResetsRecognitionFudgingToItsCompatibilityDefault()
    {
        using var engine = new NativeEngineClient(backend: "Cpu");
        engine.UpdateRecognitionFudging(2);
        var save = engine.Save();
        var payload = JsonNode.Parse(save.AsSpan(10))!.AsObject();
        Assert.True(payload["config"]!.AsObject().Remove("recognition_fudging"));
        var olderPayload = JsonSerializer.SerializeToUtf8Bytes(payload);
        var olderSave = new byte[10 + olderPayload.Length];
        save.AsSpan(0, 10).CopyTo(olderSave);
        BinaryPrimitives.WriteUInt32LittleEndian(olderSave.AsSpan(6, 4), (uint)olderPayload.Length);
        olderPayload.CopyTo(olderSave, 10);

        engine.Load(olderSave);

        Assert.Equal(0, engine.ReadConfiguration().RecognitionFudging);
        Assert.Equal(0ul, engine.Snapshot().Tick);
    }

    private static byte[] OriginalSimulation()
    {
        const string relative = "modern/engine/tests/fixtures/db24832-original-user.sim";
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        while (directory is not null && !File.Exists(Path.Combine(directory.FullName, relative)))
            directory = directory.Parent;
        Assert.NotNull(directory);
        return File.ReadAllBytes(Path.Combine(directory.FullName, relative));
    }
}
