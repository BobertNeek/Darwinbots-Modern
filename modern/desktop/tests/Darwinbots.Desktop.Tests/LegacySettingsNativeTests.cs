using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class LegacySettingsNativeTests
{
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
