using Darwinbots.Desktop.Core;
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
            Assert.Equal(bindings, PlayerBotKeyMapFile.Load(path));
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public void RestrictionPresetRoundTripsBothEditableColumns()
    {
        var path = Path.Combine(Path.GetTempPath(), $"darwinbots-{Guid.NewGuid():N}.resp");
        var options = new RestrictionOptions(
            new RestrictionColumn(true, false, true, true, false, true, false, true, false, true),
            new RestrictionColumn(false, true, false, false, true, false, true, false, true, false));
        try
        {
            RestrictionPresetFile.Save(path, options);
            Assert.Equal(20, File.ReadAllLines(path).Length);
            Assert.Equal(options, RestrictionPresetFile.Load(path));
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
