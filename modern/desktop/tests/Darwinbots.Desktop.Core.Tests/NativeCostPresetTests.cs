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

    private static string[] NativeFields(byte[] bytes) => LegacyDnaText.Decode(bytes)
        .Split("#DARWINBOTS_MODERN_SETTINGS_V1#")[0].Split("\r\n");

    private static DesktopSettings Decode(string[] fields) =>
        Db2SettingsCodec.Decode(LegacyDnaText.Encode(string.Join("\r\n", fields)));
}
