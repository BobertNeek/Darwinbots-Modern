using System.Text;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class ShapeShotPolicyCompatibilityTests
{
    private const string ModernExtensionMarker = "#DARWINBOTS_MODERN_SETTINGS_V1#";

    [Theory]
    [InlineData("absorb")]
    [InlineData("reflect")]
    [InlineData("pass_through")]
    public void ModernSettingsEditorAndNativeSnapshotPreserveDistinctShotPolicies(string policy)
    {
        var original = Assert.Single(SettingsWith(policy).Obstacles);
        var edited = ObjectEditorPolicy.BuildObstaclePayload(
            original, [11f, 22f], [33f, 44f], original.Color, original.Opacity,
            [-2f, 3f], original.VisionPolicy, policy);
        var settings = new DesktopSettings { Obstacles = [edited] };
        var restored = DesktopSettings.FromJson(settings.ToJson());
        Assert.Equal(policy, Assert.Single(restored.Obstacles).ShotPolicy);
        Assert.Equal(policy, Assert.Single(restored.ToWorldSetup().Obstacles).ShotPolicy);
        Assert.Equal(policy, Assert.Single(DesktopSettings.FromWorldSetup(restored.ToWorldSetup()).Obstacles).ShotPolicy);

        var snapshot = NativeSnapshotParser.Parse($$"""
            {
              "tick": 0,
              "organisms": [],
              "obstacles": [
                { "id": 7, "minimum": [11, 22], "maximum": [33, 44], "shot_policy": "{{policy}}" }
              ]
            }
            """, "CPU");
        Assert.Equal(policy, Assert.Single(snapshot.Obstacles).ShotPolicy);
    }

    [Theory]
    [InlineData("absorb", "#TRUE#")]
    [InlineData("reflect", "#FALSE#")]
    public void OriginalSettingsUseOneBooleanForAbsorbOrReflect(string policy, string expectedFlag)
    {
        var settings = SettingsWith(policy, policy);
        var native = NativeText(settings);
        // OptionsForm.SaveSett: with zero species, the shared shapesAbsorbShots record is 195.
        Assert.Equal(expectedFlag, native.Split("\r\n")[195]);
        var imported = Db2SettingsCodec.Decode(LegacyDnaText.Encode(native));
        Assert.Equal(2, imported.Obstacles.Count);
        Assert.All(imported.Obstacles, obstacle => Assert.Equal(policy, obstacle.ShotPolicy));
        Assert.Equal([25f, 35f], imported.Obstacles[0].Minimum);
        Assert.Equal([251f, 450f], imported.Obstacles[1].Maximum);
        Assert.Equal(expectedFlag, NativeText(imported).Split("\r\n")[195]);
    }

    [Theory]
    [InlineData("pass_through")]
    [InlineData("unknown")]
    public void OriginalSettingsRejectPoliciesThatTheirBooleanCannotRepresent(string policy)
    {
        var error = Assert.Throws<InvalidDataException>(() => Db2SettingsCodec.Encode(SettingsWith(policy)));
        Assert.Contains("one global Absorb or Reflect", error.Message, StringComparison.Ordinal);
        // The same modern JSON remains lossless; only the original-format boundary rejects it.
        Assert.Equal(policy, Assert.Single(DesktopSettings.FromJson(SettingsWith(policy).ToJson()).Obstacles).ShotPolicy);
    }

    [Fact]
    public void OriginalSettingsRejectMixedPerShapeAbsorbAndReflect()
    {
        var error = Assert.Throws<InvalidDataException>(() => Db2SettingsCodec.Encode(SettingsWith("absorb", "reflect")));
        Assert.Contains("one global Absorb or Reflect", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void ExistingModernExtensionKeepsPassThroughDistinctFromOriginalFalse()
    {
        var native = NativeText(SettingsWith("reflect"));
        var modern = SettingsWith("pass_through");
        var encoded = LegacyDnaText.Encode(native + ModernExtensionMarker + "\r\n"
            + Convert.ToBase64String(Encoding.UTF8.GetBytes(modern.ToJson())) + "\r\n");
        Assert.Equal("reflect", Assert.Single(Db2SettingsCodec.Decode(LegacyDnaText.Encode(native)).Obstacles).ShotPolicy);
        Assert.Equal("pass_through", Assert.Single(Db2SettingsCodec.Decode(encoded).Obstacles).ShotPolicy);
    }

    private static DesktopSettings SettingsWith(params string[] policies) => new()
    {
        Obstacles = policies.Select((policy, index) => new ObstacleSnapshot(
            (uint)index + 1, [25f + index, 35f], [250f + index, 450f]) { ShotPolicy = policy }).ToArray(),
    };

    private static string NativeText(DesktopSettings settings)
    {
        var text = LegacyDnaText.Decode(Db2SettingsCodec.Encode(settings));
        return text[..text.IndexOf(ModernExtensionMarker, StringComparison.Ordinal)];
    }
}
