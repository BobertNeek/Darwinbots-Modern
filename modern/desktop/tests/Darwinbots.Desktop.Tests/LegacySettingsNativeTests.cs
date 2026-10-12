using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class LegacySettingsNativeTests
{
    [Fact]
    public void Genuine24832SimulationLoadsThroughTheDesktopClientAndResumes()
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        const string relative = "modern/engine/tests/fixtures/db24832-original-user.sim";
        while (directory is not null && !File.Exists(Path.Combine(directory.FullName, relative))) directory = directory.Parent;
        Assert.NotNull(directory);
        var native = File.ReadAllBytes(Path.Combine(directory.FullName, relative));
        using var engine = new NativeEngineClient(backend: "Cpu");
        engine.Load(native);
        var snapshot = engine.Snapshot();
        Assert.Equal(281ul, snapshot.Tick);
        Assert.Equal(196, snapshot.Population);
        Assert.Equal(2, snapshot.Species.Count);
        Assert.Equal(37, snapshot.Organisms.Count(robot => robot.Vegetable));
        Assert.Equal(new float[] { 16_000, 12_000 }, snapshot.WorldSize);
        Assert.Equal(587, snapshot.Organisms[0].Energy);
        var modern = engine.Save();
        Assert.ThrowsAny<Exception>(() => engine.Load(native[..^1]));
        Assert.Equal(modern, engine.Save());
        engine.Load(modern);
        engine.Tick();
        Assert.Equal(282ul, engine.Snapshot().Tick);
    }

    [Theory]
    [InlineData("Chloroplast Ecosystem.set")]
    [InlineData("Complex Simulation.set")]
    [InlineData("Large Sex Simulation.set")]
    [InlineData("Ledge Island.set")]
    [InlineData("Simple Simulation.set")]
    [InlineData("Small Sex Simulation.set")]
    [InlineData("Walls and Waste.set")]
    public void ShippedSettingsResolveDnaAndConfigureTheProductionEngine(string name)
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        while (directory is not null && !File.Exists(Path.Combine(directory.FullName, "Installer", "sets", name))) directory = directory.Parent;
        var path = Path.Combine(directory!.FullName, "Installer", "sets", name);
        var settings = SetupStorageService.DecodeSettings(path, File.ReadAllBytes(path));
        var setup = settings.ToWorldSetup() with { Backend = "Cpu" };
        using var engine = new NativeEngineClient(setup);
        foreach (var species in setup.Species) engine.ImportSpecies(species);
        var snapshot = engine.Snapshot();
        Assert.Equal(setup.Species.Count + 1, snapshot.Species.Count);
        Assert.All(setup.Species, species => Assert.False(string.IsNullOrWhiteSpace(species.Dna)));
        Assert.Equal(setup.Obstacles.Count, snapshot.Obstacles.Count);
        Assert.Equal(setup.TiesEnabled, snapshot.TiesEnabled);
        for (int i = 0; i < setup.Species.Count; i++)
        {
            var robots = snapshot.Organisms.Where(robot => robot.Species == (uint)(i + 1)).ToArray();
            Assert.All(robots, robot => Assert.Equal(setup.Species[i].MutationProfile, robot.MutationProfile));
        }
        var save = engine.Save();
        Assert.Equal("DB3S", System.Text.Encoding.ASCII.GetString(save, 0, 4));
        engine.Load(save);
        Assert.Equal(snapshot.Population, engine.Snapshot().Population);
    }
}
