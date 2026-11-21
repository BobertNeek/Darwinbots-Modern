using System.Text.Json;
using System.Text.Json.Nodes;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class NativeCostPresetTests
{
    [Theory]
    [InlineData(Db2CostPreset.NoCosts, "0")]
    [InlineData(Db2CostPreset.F1, "1")]
    [InlineData(Db2CostPreset.Custom, "2")]
    public void NativeSettingsRoundTripTheSelectorWithoutReplacingCosts(Db2CostPreset preset, string nativeValue)
    {
        const int selector = 175;
        const int retainedCoefficient = 100 + 36;
        var settings = new DesktopSettings
        {
            NativeCostPreset = preset,
            GlobalSettings = new GlobalSettings { Costs = new GlobalSimulationCostSettings { Movement = 0.375 } },
        };
        var encoded = Db2SettingsCodec.Encode(settings);
        var fields = NativeFields(encoded);
        Assert.Equal(nativeValue, fields[selector]);
        fields[retainedCoefficient] = "0.125075";

        var imported = Decode(fields);
        Assert.Equal(preset, imported.NativeCostPreset);
        Assert.Equal(preset, imported.ToWorldSetup().NativeCostPreset);
        Assert.Equal(0.375, imported.GlobalSettings.Costs.Movement);
        Assert.Equal("0.125075", NativeFields(Db2SettingsCodec.Encode(imported))[retainedCoefficient]);
        Assert.Equal(preset, Db2SettingsCodec.Decode(encoded).NativeCostPreset);
    }

    [Fact]
    public void NativeMissingSelectorDefaultsToCustomAndOlderModernJsonCanInferOrRecoverIt()
    {
        const int selector = 175;
        var raw = NativeFields(Db2SettingsCodec.Encode(new DesktopSettings { NativeCostPreset = Db2CostPreset.F1 }));
        Assert.Equal(Db2CostPreset.Custom, Decode(raw[..selector]).NativeCostPreset);

        var retained = new DesktopSettings { LegacySettingsFields = raw };
        var older = JsonNode.Parse(retained.ToJson())!.AsObject();
        Assert.True(older.Remove("nativeCostPreset"));
        Assert.Equal(Db2CostPreset.F1, DesktopSettings.FromJson(older.ToJsonString()).NativeCostPreset);
        Assert.Null(DesktopSettings.FromJson(retained.ToJson()).NativeCostPreset);
        Assert.Null(DesktopSettings.FromJson("{\"schemaVersion\":1}").NativeCostPreset);
        older["legacySettingsFields"] = JsonSerializer.SerializeToNode(raw[..selector]);
        Assert.Null(DesktopSettings.FromJson(older.ToJsonString()).NativeCostPreset);
    }

    [Fact]
    public void EngineStartupConfigurationCarriesTheExplicitNativePreset()
    {
        var setup = new WorldSetupOptions { NativeCostPreset = Db2CostPreset.NoCosts };
        using var config = JsonDocument.Parse(NativeCommandSerializer.SerializeEngineConfig(setup));
        Assert.Equal(0, config.RootElement.GetProperty("native_cost_preset").GetInt32());
    }

    [Theory]
    [InlineData(false, "0")]
    [InlineData(true, "1")]
    public void NativeAgeLinearCostUsesSeparateSetSlotsForTheRateAndEnableFlag(bool enabled, string nativeFlag)
    {
        // Original OptionsForm.SaveSett writes the VB6 Costs array. In an
        // empty roster the array starts at record 100, with rate 33 and flag 60.
        const int rate = 100 + 33;
        const int flag = 100 + 60;
        var fields = NativeFields(Db2SettingsCodec.Encode(new DesktopSettings()));
        fields[rate] = "0.25";
        fields[flag] = nativeFlag;

        var settings = Decode(fields);
        var costs = settings.GlobalSettings.Costs;
        Assert.Equal(0.25, costs.AgeLinear);
        Assert.Equal(enabled, costs.AgeLinearEnabled);
        Assert.Equal(enabled, costs.EffectiveAgeLinearEnabled);

        // The old Modern field remains round-trippable in the extension. It
        // has no meaning for the native SET cost table.
        var withLegacyAlias = settings with
        {
            GlobalSettings = settings.GlobalSettings with
            {
                Costs = costs with { DynamicUpperReinstateThreshold = 89 },
            },
        };
        var encoded = Db2SettingsCodec.Encode(withLegacyAlias);
        Assert.Equal(89, Db2SettingsCodec.Decode(encoded).GlobalSettings.Costs.DynamicUpperReinstateThreshold);
        var saved = NativeFields(encoded);
        Assert.Equal("0.25", saved[rate]);
        Assert.Equal(nativeFlag, saved[flag]);

        using var payload = JsonDocument.Parse(NativeCommandSerializer.SerializeEngineConfig(settings.ToWorldSetup()));
        Assert.Equal(enabled, payload.RootElement.GetProperty("dna_costs").GetProperty("age_linear_enabled").GetBoolean());
    }

    [Fact]
    public void OlderModernCostsKeepTheNonzeroRateAsTheDefaultLinearAgeEnable()
    {
        var old = new GlobalSettings { Costs = new GlobalSimulationCostSettings { AgeLinear = 0.25 } };
        var restored = GlobalSettings.FromJson(old.ToJson()).Costs;
        Assert.Null(restored.AgeLinearEnabled);
        Assert.True(restored.EffectiveAgeLinearEnabled);
    }

    private static string[] NativeFields(byte[] bytes) => LegacyDnaText.Decode(bytes)
        .Split("#DARWINBOTS_MODERN_SETTINGS_V1#")[0].Split("\r\n");

    private static DesktopSettings Decode(string[] fields) =>
        Db2SettingsCodec.Decode(LegacyDnaText.Encode(string.Join("\r\n", fields)));
}
