using System.Reflection;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class ObstacleEditorPolicyTests
{
    [Fact]
    public void BuildsTheFullObstaclePayloadAndRequiresDeleteConfirmation()
    {
        var original = new ObstacleSnapshot(7, [10f, 20f], [30f, 40f])
        {
            Color = 0xff112233,
            Opacity = 0.5f,
            Drift = [1f, -2f],
            VisionPolicy = "opaque",
            ShotPolicy = "absorb",
        };
        var policyType = typeof(ObstacleSnapshot).Assembly.GetType("Darwinbots.Desktop.Core.ObstacleEditorPolicy");
        Assert.NotNull(policyType);

        var buildPayload = policyType!.GetMethod("BuildPayload", BindingFlags.Public | BindingFlags.Static);
        Assert.NotNull(buildPayload);
        var edited = (ObstacleSnapshot)buildPayload!.Invoke(null,
        [
            original,
            new[] { 110f, 120f },
            new[] { 310f, 420f },
            0xa1b2c3d4U,
            0.35f,
            new[] { -3.5f, 4.5f },
            "see_through",
            "pass_through",
        ])!;

        Assert.Equal(7U, edited.Id);
        Assert.Equal([110f, 120f], edited.Minimum);
        Assert.Equal([310f, 420f], edited.Maximum);
        Assert.Equal(0xa1b2c3d4U, edited.Color);
        Assert.Equal(0.35f, edited.Opacity);
        Assert.Equal([-3.5f, 4.5f], edited.Drift);
        Assert.Equal("see_through", edited.VisionPolicy);
        Assert.Equal("pass_through", edited.ShotPolicy);

        var confirmDelete = policyType.GetMethod("ConfirmDelete", BindingFlags.Public | BindingFlags.Static);
        Assert.NotNull(confirmDelete);
        Assert.False((bool)confirmDelete!.Invoke(null, [false])!);
        Assert.True((bool)confirmDelete.Invoke(null, [true])!);
    }
}
