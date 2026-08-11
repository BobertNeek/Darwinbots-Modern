using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class ObjectEditorPolicyTests
{
    [Fact]
    public void RoutesObjectMenuCommandsToEditorPayloadsAndAllowsOnlyMatchingSelection()
    {
        var newShape = ObjectEditorPolicy.Route(LegacyCommandId.ObjectsNewShape, null);
        var newTeleporter = ObjectEditorPolicy.Route(LegacyCommandId.ObjectsNewTeleporter, null);

        Assert.Equal(ObjectEditorAction.Create, newShape.Action);
        Assert.Equal(ObjectEditorKind.Shape, newShape.Kind);
        Assert.True(newShape.Allowed);
        Assert.Equal(ObjectEditorAction.Create, newTeleporter.Action);
        Assert.Equal(ObjectEditorKind.Teleporter, newTeleporter.Kind);
        Assert.True(newTeleporter.Allowed);

        var obstacle = new ObstacleSnapshot(7, [10f, 20f], [30f, 40f]);
        var editedObstacle = ObjectEditorPolicy.BuildObstaclePayload(
            obstacle,
            [110f, 120f],
            [310f, 420f],
            0xa1b2c3d4U,
            0.35f,
            [-3.5f, 4.5f],
            "see_through",
            "pass_through");
        Assert.Equal(7U, editedObstacle.Id);
        Assert.Equal([110f, 120f], editedObstacle.Minimum);
        Assert.Equal(0xa1b2c3d4U, editedObstacle.Color);

        var teleporter = new TeleporterSnapshot(8, [10f, 20f], 12f, [30f, 40f]);
        var editedTeleporter = ObjectEditorPolicy.BuildTeleporterPayload(
            teleporter,
            "outbound",
            [110f, 120f],
            25f,
            [310f, 420f],
            7,
            3,
            2,
            false,
            true,
            true,
            false,
            true,
            "local",
            "inbound",
            "outbound");
        Assert.Equal(8U, editedTeleporter.Id);
        Assert.Equal("outbound", editedTeleporter.Mode);
        Assert.Equal([110f, 120f], editedTeleporter.Center);
        Assert.Equal([310f, 420f], editedTeleporter.Destination);
        Assert.Equal(7U, editedTeleporter.PollInterval);
        Assert.Equal(3, editedTeleporter.Capacity);

        var obstacleSelection = new ObjectEditorSelection(ObjectEditorKind.Shape, obstacle.Id);
        var teleporterSelection = new ObjectEditorSelection(ObjectEditorKind.Teleporter, teleporter.Id);
        Assert.True(ObjectEditorPolicy.RouteEdit(ObjectEditorKind.Shape, obstacleSelection).Allowed);
        Assert.False(ObjectEditorPolicy.RouteEdit(ObjectEditorKind.Teleporter, obstacleSelection).Allowed);
        Assert.True(ObjectEditorPolicy.RouteDelete(ObjectEditorKind.Teleporter, teleporterSelection).Allowed);
        Assert.False(ObjectEditorPolicy.RouteDelete(ObjectEditorKind.Shape, teleporterSelection).Allowed);
    }
}
