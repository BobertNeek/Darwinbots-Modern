using System.Collections;
using System.Reflection;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class MazePresetBuilderTests
{
    [Fact]
    public void BuildsAllSixHistoricalMazePresetsWithRustObstacleGeometry()
    {
        var assembly = typeof(ObstacleSnapshot).Assembly;
        var builderType = assembly.GetType("Darwinbots.Desktop.Core.MazePresetBuilder");
        Assert.NotNull(builderType);
        var presetType = assembly.GetType("Darwinbots.Desktop.Core.MazePreset");
        Assert.NotNull(presetType);
        var build = builderType!.GetMethod("Build", BindingFlags.Public | BindingFlags.Static);
        Assert.NotNull(build);

        var expected = new Dictionary<string, int>
        {
            ["Horizontal"] = 56,
            ["Vertical"] = 42,
            ["Spiral"] = 40,
            ["Checkerboard"] = 35,
            ["PolarIce"] = 9,
            ["TrashCompactor"] = 2,
        };

        foreach (var (name, count) in expected)
        {
            var preset = Enum.Parse(presetType!, name);
            var shapes = ((IEnumerable)build!.Invoke(null, [preset, 16_000f, 12_000f])!).Cast<object>().ToArray();
            Assert.Equal(count, shapes.Length);
            Assert.All(shapes, shape =>
            {
                var minimum = (float[])shape.GetType().GetProperty("Minimum")!.GetValue(shape)!;
                var maximum = (float[])shape.GetType().GetProperty("Maximum")!.GetValue(shape)!;
                Assert.True(minimum[0] < maximum[0]);
                Assert.True(minimum[1] < maximum[1]);
            });

            if (name is "PolarIce" or "TrashCompactor")
            {
                Assert.Contains(shapes, shape =>
                {
                    var drift = (float[])shape.GetType().GetProperty("Drift")!.GetValue(shape)!;
                    return drift.Any(value => value != 0f);
                });
            }
        }
    }
}
