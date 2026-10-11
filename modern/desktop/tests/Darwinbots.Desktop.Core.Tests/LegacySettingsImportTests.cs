using System.Globalization;
using System.Text;
using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class LegacySettingsImportTests
{
    [Theory]
    [InlineData("Chloroplast Ecosystem.set", 1, 25)]
    [InlineData("Complex Simulation.set", 6, 300)]
    [InlineData("Large Sex Simulation.set", 2, 5000)]
    [InlineData("Ledge Island.set", 2, 100)]
    [InlineData("Simple Simulation.set", 2, 100)]
    [InlineData("Small Sex Simulation.set", 2, 25)]
    [InlineData("Walls and Waste.set", 2, 500)]
    public void ShippedSettingsRetainSpeciesAndUseThePostScriptPhysicsOffsets(string name, int count, int cap)
    {
        var path = Path.Combine(RepositoryRoot(), "Installer", "sets", name);
        var lines = File.ReadAllLines(path);
        var settings = Db2SettingsCodec.Decode(File.ReadAllBytes(path));
        int global = 2 + 40 * count;
        int extra = global + 90 + 8 * count;
        int tail = global + 169 + 56 * count;
        double Number(int i) => double.Parse(lines[i], CultureInfo.InvariantCulture);
        Assert.Equal(count, settings.Species.Count);
        Assert.Equal(32000, settings.OrganismCapacity);
        Assert.Equal(cap, settings.VegetablePopulationCap);
        Assert.Equal((float)Number(global + 18), settings.Gravity[1]);
        Assert.Equal((float)Number(global + 19), settings.Physics.SurfaceGravity);
        Assert.Equal((float)Number(extra + 2), settings.Physics.StaticFriction);
        Assert.Equal((float)Number(tail + 6), settings.Physics.MaxVelocity);
        Assert.Equal((int)Number(tail + 2), settings.Recording.SamplingInterval);
        Assert.Equal((ulong)Number(global + 89), settings.Seed);
        for (int i = 0; i < count; i++)
        {
            Assert.Equal((int)Number(2 + 40 * i + 6), settings.Species[i].InitialCount);
            Assert.Equal(Number(2 + 40 * i + 13), settings.Species[i].MutationProfile!.Point.OneIn);
            Assert.EndsWith(settings.Species[i].Name, settings.Species[i].DnaPath);
        }
    }

    [Fact]
    public void ExportedSettingsKeepNativeFieldsReadableWithoutTheModernExtension()
    {
        var expected = new DesktopSettings
        {
            Seed = 12345, VegetablePopulationCap = 25, Gravity = [0, 3.77f],
            Physics = Db2PhysicsOptions.Default with { MaxVelocity = 180, SurfaceGravity = 2, StaticFriction = 0.6f },
            Recording = new(SamplingInterval: 17),
            Species = [new("Hunter.txt", "start stop", false, 0xff123456, 5, 900, 0, false, 0)
            {
                DnaPath = "Robots/Hunter.txt", MutationProfile = MutationProfileUpdate.Default with { Point = new(0, 3, 1), Delta = new(25, 500, 150) },
            }],
        };
        string native = Encoding.UTF8.GetString(Db2SettingsCodec.Encode(expected)).Split("#DARWINBOTS_MODERN_SETTINGS_V1#")[0];
        var actual = Db2SettingsCodec.Decode(Encoding.UTF8.GetBytes(native));
        Assert.Equal(expected.Seed, actual.Seed);
        Assert.Equal(expected.Physics, actual.Physics);
        Assert.Equal(25, actual.VegetablePopulationCap);
        Assert.Equal(17, actual.Recording.SamplingInterval);
        Assert.Equal(5, Assert.Single(actual.Species).InitialCount);
        Assert.Equal(0, actual.Species[0].MutationProfile!.Point.OneIn);
        Assert.Equal(25, actual.Species[0].MutationProfile!.Delta.OneIn);
        Assert.Equal(expected.Species[0].Color, actual.Species[0].Color);
    }

    [Fact]
    public void NativeWallFooterMovesWhenImportedWallsAreEdited()
    {
        var settings = Db2SettingsCodec.Decode(File.ReadAllBytes(Path.Combine(RepositoryRoot(), "Installer", "sets", "Walls and Waste.set")));
        var edited = settings with
        {
            RobotFixingAllowed = false,
            Obstacles = [new(1, [25, 35], [250, 450])
            {
                Color = 0xff123456, Drift = [2, 3], VisionPolicy = "see_through", ShotPolicy = "absorb",
            }],
        };
        string native = Encoding.UTF8.GetString(Db2SettingsCodec.Encode(edited)).Split("#DARWINBOTS_MODERN_SETTINGS_V1#")[0];
        var actual = Db2SettingsCodec.Decode(Encoding.UTF8.GetBytes(native));
        var wall = Assert.Single(actual.Obstacles);
        Assert.Equal(new float[] { 25, 35 }, wall.Minimum);
        Assert.Equal(new float[] { 250, 450 }, wall.Maximum);
        Assert.Equal(0xff123456u, wall.Color);
        Assert.Equal(new float[] { 2, 3 }, wall.Drift);
        Assert.Equal("see_through", wall.VisionPolicy);
        Assert.False(actual.RobotFixingAllowed);
        Assert.Equal(settings.Vegetation.Tides, actual.Vegetation.Tides);
        Assert.Equal(settings.Species.Count, actual.Species.Count);
    }

    internal static string RepositoryRoot()
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        while (directory is not null && !File.Exists(Path.Combine(directory.FullName, "Installer", "sets", "Simple Simulation.set"))) directory = directory.Parent;
        return directory?.FullName ?? throw new DirectoryNotFoundException("Repository fixture directory was not found.");
    }
}
