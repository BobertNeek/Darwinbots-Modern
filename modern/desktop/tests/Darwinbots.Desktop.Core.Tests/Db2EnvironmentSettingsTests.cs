using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class Db2EnvironmentSettingsTests
{
    [Fact]
    public void StarterModeUsesNormalMetabolismEvenWhenZerobotSustenanceDefaultsToDisabled()
    {
        var options = new WorldSetupOptions
        {
            StartingMode = StartingMode.StarterBotsAndVegetables,
            ZerobotSustenance = ZerobotSustenance.DisabledMetabolism,
            MetabolismCost = 1,
        };

        Assert.Equal(1, options.EffectiveMetabolismCost);
    }

    [Fact]
    public void ZerobotModeCanDisableMetabolism()
    {
        var options = new WorldSetupOptions
        {
            StartingMode = StartingMode.Zerobots,
            ZerobotSustenance = ZerobotSustenance.DisabledMetabolism,
            MetabolismCost = 1,
        };

        Assert.Equal(0, options.EffectiveMetabolismCost);
    }

    [Fact]
    public void Db2SettingsSerializeIntoVersionedEnvironmentCommand()
    {
        var json = NativeCommandSerializer.SerializeEnvironment(EnvironmentUpdate.Default);

        Assert.Contains("\"max_velocity\":60", json);
        Assert.Contains("\"movement_efficiency\":0.66", json);
        Assert.Contains("\"speed\":40", json);
        Assert.Contains("\"start_chloroplasts\":16000", json);
    }
}
