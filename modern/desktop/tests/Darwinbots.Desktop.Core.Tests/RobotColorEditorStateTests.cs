using System.Text.Json;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class RobotColorEditorStateTests
{
    [Fact]
    public void EditorStatePreservesRgbDnaMetadataAndAuthoritativeColorPayload()
    {
        var state = new RobotColorEditorState(0xff123456u);
        state.SetRed(0x1a);
        state.SetGreen(0x2b);
        state.SetBlue(0x3c);

        var color = state.Color;
        Assert.Equal(0xff1a2b3cu, color);
        Assert.Equal("#1A2B3C", state.PreviewHex);

        var metadata = RobotColorDna.Apply(
            "' bot\n\ndef red 1\ndef green 2\ndef blue 3\n@\nstart\nstop",
            color);
        Assert.StartsWith("' bot\n\n@\ndef red 26\ndef green 43\ndef blue 60\nstart\nstop", metadata, StringComparison.Ordinal);
        Assert.DoesNotContain("def red 1", metadata, StringComparison.Ordinal);

        var payload = JsonDocument.Parse(NativeCommandSerializer.SerializeSpeciesColor(7u, color));
        var command = payload.RootElement.GetProperty("commands")[0];
        Assert.Equal("set_species_color", command.GetProperty("type").GetString());
        Assert.Equal(7u, command.GetProperty("species").GetUInt32());
        Assert.Equal(color, command.GetProperty("color").GetUInt32());
    }
}
