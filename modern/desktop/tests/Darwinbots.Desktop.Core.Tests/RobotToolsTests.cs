using Darwinbots.Desktop.Core;
using System.Text;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class RobotToolsTests
{
    [Fact]
    public void RobotConsoleParsesCausalCommandsAndArguments()
    {
        var command = RobotConsoleCommand.Parse("  set   50   -12 ");

        Assert.Equal("set", command.Name);
        Assert.Equal(["50", "-12"], command.Arguments);
    }

    [Fact]
    public void PlayerBotKeyMapRoundTripsFourLineBindings()
    {
        var path = Path.Combine(Path.GetTempPath(), $"darwinbots-{Guid.NewGuid():N}.pbkp");
        var bindings = new[]
        {
            new PlayerBotBinding(65, 50, 12),
            new PlayerBotBinding(66, 51, -9, true),
        };
        try
        {
            PlayerBotKeyMapFile.Save(path, bindings);
            Assert.Equal(8, File.ReadAllLines(path).Length);
            Assert.Equal(
                Encoding.ASCII.GetBytes("65\r\n50\r\n12\r\nFalse\r\n66\r\n51\r\n-9\r\nTrue\r\n"),
                File.ReadAllBytes(path));
            Assert.Equal(bindings, PlayerBotKeyMapFile.Load(path));
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public void RestrictionPresetWritesOriginalNineteenNumericFieldsWithCrLf()
    {
        var path = Path.Combine(Path.GetTempPath(), $"darwinbots-{Guid.NewGuid():N}.resp");
        var options = new RestrictionOptions(
            new RestrictionColumn(true, false, true, true, false, true, false, true, false, true),
            new RestrictionColumn(false, true, false, false, true, false, true, false, true, false));
        try
        {
            RestrictionPresetFile.Save(path, options);
            Assert.Equal(
                Encoding.ASCII.GetBytes(
                    "1\r\n0\r\n1\r\n1\r\n0\r\n1\r\n0\r\n1\r\n0\r\n1\r\n"
                    + "0\r\n1\r\n0\r\n1\r\n0\r\n1\r\n0\r\n1\r\n0\r\n"),
                File.ReadAllBytes(path));
            Assert.Equal(19, File.ReadAllLines(path).Length);
            Assert.Equal(options, RestrictionPresetFile.Load(path));
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public void OriginalNineteenFieldRestrictionPresetKeepsAsymmetricColumnPositions()
    {
        var path = Path.Combine(Path.GetTempPath(), $"darwinbots-{Guid.NewGuid():N}.resp");
        const string original = "1\r\n0\r\n1\r\n1\r\n0\r\n1\r\n0\r\n1\r\n0\r\n1\r\n"
            + "0\r\n1\r\n0\r\n1\r\n0\r\n1\r\n0\r\n1\r\n0\r\n";
        var expected = new RestrictionOptions(
            new RestrictionColumn(
                KillMultibot: true, KillDisqualified: false, DisableChloroplasts: true,
                FixedInPlace: true, DisableVision: false, DisableDna: true,
                DisableReproduction: false, VirusImmune: true, DisableMutations: false,
                DisableMovement: true),
            new RestrictionColumn(
                KillMultibot: false, KillDisqualified: true, DisableChloroplasts: false,
                FixedInPlace: false, DisableVision: true, DisableDna: false,
                DisableReproduction: true, VirusImmune: false, DisableMutations: true,
                DisableMovement: false));
        try
        {
            File.WriteAllBytes(path, Encoding.ASCII.GetBytes(original));

            Assert.Equal(expected, RestrictionPresetFile.Load(path));
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public void HistoricalTwentyBooleanRestrictionPresetRetainsModernOnlyTrueFlag()
    {
        var path = Path.Combine(Path.GetTempPath(), $"darwinbots-{Guid.NewGuid():N}.resp");
        const string historical = "True\r\nFalse\r\nTrue\r\nTrue\r\nFalse\r\nTrue\r\nFalse\r\nTrue\r\nFalse\r\nTrue\r\n"
            + "False\r\nTrue\r\nTrue\r\nFalse\r\nTrue\r\nFalse\r\nTrue\r\nFalse\r\nTrue\r\nFalse\r\n";
        var expected = new RestrictionOptions(
            new RestrictionColumn(true, false, true, true, false, true, false, true, false, true),
            new RestrictionColumn(false, true, true, false, true, false, true, false, true, false));
        try
        {
            File.WriteAllBytes(path, Encoding.ASCII.GetBytes(historical));

            var loaded = RestrictionPresetFile.Load(path);
            Assert.Equal(expected, loaded);
            Assert.True(loaded.Repopulating.DisableChloroplasts);
            Assert.Throws<InvalidDataException>(() => RestrictionPresetFile.Save(path, loaded));
            Assert.Equal(Encoding.ASCII.GetBytes(historical), File.ReadAllBytes(path));
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void NativeRestrictionPresetRejectsUnrepresentableFlagBeforeTouchingTarget(bool existing)
    {
        var path = Path.Combine(Path.GetTempPath(), $"darwinbots-{Guid.NewGuid():N}.resp");
        byte[] priorBytes = [0x00, 0xE9, 0x80, 0x0D, 0x0A];
        var options = RestrictionOptions.Default with
        {
            Repopulating = new RestrictionColumn(DisableChloroplasts: true),
        };
        try
        {
            if (existing) File.WriteAllBytes(path, priorBytes);

            Assert.Throws<InvalidDataException>(() => RestrictionPresetFile.Save(path, options));

            if (existing) Assert.Equal(priorBytes, File.ReadAllBytes(path));
            else Assert.False(File.Exists(path));
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public void NativeSnapshotCarriesRobotToolState()
    {
        const string json = """
            {
              "tick": 4,
              "organisms": [],
              "player_bot": {
                "enabled": true,
                "focus": { "slot": 2, "generation": 1 },
                "bindings": [{ "key": 65, "memory_address": 50, "value": 77, "invert": false }],
                "pressed_keys": [65]
              },
              "restriction_options": {
                "non_repopulating": { "fixed_in_place": true },
                "repopulating": { "disable_dna": true }
              }
            }
            """;

        var snapshot = NativeSnapshotParser.Parse(json, "CPU");

        Assert.True(snapshot.PlayerBot.Enabled);
        Assert.Equal(new OrganismKey(2, 1), snapshot.PlayerBot.Focus);
        Assert.Equal(new PlayerBotBinding(65, 50, 77), snapshot.PlayerBot.Bindings.Single());
        Assert.Equal([65], snapshot.PlayerBot.PressedKeys);
        Assert.True(snapshot.RestrictionOptions.NonRepopulating.FixedInPlace);
        Assert.True(snapshot.RestrictionOptions.Repopulating.DisableDna);
    }
}
