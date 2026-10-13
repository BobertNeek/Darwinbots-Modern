using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.ViewModels;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class LegacySettingsNativeTests
{
    [Fact]
    public void ConfigurationRestoresNonDefaultCostMutationAndMainRuleEditorsWithoutTicking()
    {
        var costs = new GlobalSimulationCostSettings
        {
            DnaNumber = 0.11, DnaDereference = 0.12, DnaBasic = 0.13,
            DnaAdvanced = 0.14, DnaBitwise = 0.15, DnaCondition = 0.16,
            DnaLogic = 0.17, DnaStore = 0.18, DnaFlow = 0.19,
            DnaUpkeep = 0.21, DnaCopy = 0.22, BodyUpkeep = 0.23,
            ChloroplastConstruction = 0.24, GlobalMultiplier = 1.5,
            DynamicMultiplier = 0.5, DynamicAllRobots = false,
            DynamicTargetPopulation = 20, DynamicPopulationRange = 10,
            AgeBase = 2, AgeStart = 2.5, AgeLinear = 0.25, AgeLogarithmic = 1,
        };
        var advanced = new GlobalAdvancedMutationSettings
        {
            Point2Enabled = true, CopyError2Enabled = true,
            AmplificationEnabled = true, TranslocationEnabled = true,
            Delta2Enabled = true, Delta2Interval = 7, Delta2RateStep = 1.25,
            Delta2DistributionStep = 0.5, Delta2PreferenceStepPercent = 3,
            NormalizationEnabled = true, NormalizationDnaLength = 128,
            OscillationMode = GlobalMutationOscillationMode.Sine,
            OscillationPeriod = 21, OscillationAmplitude = 2,
            EpigeneticReset = true, EpigeneticAmplification = 1.25,
            EpigeneticOverloadThreshold = 12, SaveAsTemporaryGene = true,
        };
        using var engine = new NativeEngineClient(new WorldSetupOptions
        {
            Backend = "Cpu",
            GlobalSettings = new GlobalSettings
            {
                Costs = costs, AdvancedMutation = advanced,
                Main = new GlobalMainSettings { BodyCutoff = 12345, GreedyEnforcement = true },
            },
        });
        var before = engine.Save();
        var configuration = engine.ReadConfiguration();
        Assert.Equal(costs, configuration.Costs);
        Assert.Equal(advanced, configuration.AdvancedMutation);
        Assert.Equal(12345, configuration.BodyCutoff);
        Assert.True(configuration.GreedyReproduction);
        Assert.Equal(before, engine.Save());
        Assert.Equal(0ul, engine.Snapshot().Tick);
    }

    [Fact]
    public void OriginalExecutableRetainsExportedAnimalCellsAndTheirNativeTie()
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        const string relative = "docs/parity/evidence/native-organism-original-roundtrip.zip";
        while (directory is not null && !File.Exists(Path.Combine(directory.FullName, relative))) directory = directory.Parent;
        Assert.NotNull(directory);
        using var archive = System.IO.Compression.ZipFile.OpenRead(Path.Combine(directory.FullName, relative));
        var entry = archive.GetEntry("original-after-animal-dbo.sim");
        Assert.NotNull(entry);
        using var stream = entry.Open();
        using var bytes = new MemoryStream();
        stream.CopyTo(bytes);
        using var engine = new NativeEngineClient(backend: "Cpu");
        engine.Load(bytes.ToArray());
        var snapshot = engine.Snapshot();
        Assert.Equal(1185ul, snapshot.Tick);
        Assert.Equal(922, snapshot.Population);
        var first = Assert.Single(snapshot.Organisms, bot => bot.UniqueId == 250 && bot.Age == 44);
        var second = Assert.Single(snapshot.Organisms, bot => bot.UniqueId == 294 && bot.Age == 0);
        Assert.Equal(1899, first.Energy);
        Assert.Equal(1899, second.Energy);
        Assert.Equal(150, first.Body);
        Assert.Equal(149, second.Body);
        Assert.Equal(378u, first.DnaLength);
        Assert.Equal(378u, second.DnaLength);
        var tie = Assert.Single(snapshot.Ties, value => value.First.Slot == first.Slot && value.Second.Slot == second.Slot);
        Assert.Equal(109f, tie.RestLength);
        using var saved = System.Text.Json.JsonDocument.Parse(engine.Save().AsMemory(10));
        var nativeTie = Assert.Single(saved.RootElement.GetProperty("ties").EnumerateArray(), value =>
            value.GetProperty("first").GetProperty("slot").GetUInt32() == first.Slot
            && value.GetProperty("second").GetProperty("slot").GetUInt32() == second.Slot);
        Assert.Equal(0, nativeTie.GetProperty("first_port").GetInt32());
        Assert.Equal(1, nativeTie.GetProperty("second_port").GetInt32());
        Assert.True(snapshot.Organisms.Count(bot => bot.UniqueId == 250) > 1);
    }

    [Fact]
    public void OriginalExecutableContinuationOfThePortExportLoadsAndSteps()
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        const string relative = "docs/parity/evidence/native-sim-original-roundtrip.zip";
        while (directory is not null && !File.Exists(Path.Combine(directory.FullName, relative))) directory = directory.Parent;
        Assert.NotNull(directory);
        using var archive = System.IO.Compression.ZipFile.OpenRead(Path.Combine(directory.FullName, relative));
        var entry = archive.GetEntry("original-after-port654.sim");
        Assert.NotNull(entry);
        using var stream = entry.Open();
        using var bytes = new MemoryStream();
        stream.CopyTo(bytes);
        using var engine = new NativeEngineClient(backend: "Cpu");
        engine.Load(bytes.ToArray());
        var loaded = engine.Snapshot();
        Assert.Equal(654ul, loaded.Tick);
        Assert.Equal(543, loaded.Population);
        engine.Tick();
        Assert.Equal(655ul, engine.Snapshot().Tick);
        var advanced = engine.Snapshot();
        // Source-correct body feeding kills one robot while a birth adds one.
        // The fixture proves native exchange, not matching future trajectories.
        Assert.Equal(1ul, advanced.Stats.Births - loaded.Stats.Births);
        Assert.Equal(1ul, advanced.Stats.Deaths - loaded.Stats.Deaths);
        Assert.Equal(543, advanced.Population);
        Assert.Equal(advanced.Population, advanced.Organisms.Count);
    }

    [Fact]
    public void NativeSimulationExportThroughDesktopClientReflectsTheAdvancedWorld()
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        const string relative = "modern/engine/tests/fixtures/db24832-original-user.sim";
        while (directory is not null && !File.Exists(Path.Combine(directory.FullName, relative))) directory = directory.Parent;
        Assert.NotNull(directory);
        using var engine = new NativeEngineClient(backend: "Cpu");
        engine.Load(File.ReadAllBytes(Path.Combine(directory.FullName, relative)));
        engine.Tick();
        var before = engine.Snapshot();
        var first = before.Organisms[0];
        engine.SetOrganismMemory(first.Slot, first.Generation, 1000, -712);
        var bytes = engine.SaveLegacy();
        Assert.Equal(before.Population, BitConverter.ToInt16(bytes, 0));
        using var imported = new NativeEngineClient(backend: "Cpu");
        imported.Load(bytes);
        var after = imported.Snapshot();
        Assert.Equal(282ul, after.Tick);
        Assert.Equal(before.Population, after.Population);
        Assert.Equal(before.Species, after.Species);
        Assert.Equal(before.Organisms.Select(bot => bot.Energy), after.Organisms.Select(bot => bot.Energy));
        Assert.Equal(before.Organisms.Select(bot => bot.Body), after.Organisms.Select(bot => bot.Body));
        imported.Tick();
        Assert.Equal(283ul, imported.Snapshot().Tick);
    }

    [Fact]
    public void Genuine24832SimulationLoadsThroughTheDesktopClientAndResumes()
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        const string relative = "modern/engine/tests/fixtures/db24832-original-user.sim";
        while (directory is not null && !File.Exists(Path.Combine(directory.FullName, relative))) directory = directory.Parent;
        Assert.NotNull(directory);
        var native = File.ReadAllBytes(Path.Combine(directory.FullName, relative));
        using var engine = new NativeEngineClient(backend: "Cpu", worldWidth: 1000, worldHeight: 1000);
        engine.Load(native);
        var beforeConfigurationRead = engine.Save();
        var configuration = engine.ReadConfiguration();
        Assert.False(configuration.Environment.ToroidalWorld);
        Assert.Equal(2, configuration.Species.Count);
        Assert.All(configuration.Species, species => Assert.False(string.IsNullOrWhiteSpace(species.Dna)));
        Assert.Equal(beforeConfigurationRead, engine.Save());
        var snapshot = engine.Snapshot();
        Assert.Equal(281ul, snapshot.Tick);
        Assert.Equal(196, snapshot.Population);
        Assert.Equal(2, snapshot.Species.Count);
        var viewModel = new MainWindowViewModel();
        viewModel.Update(snapshot);
        Assert.Equal(2, viewModel.SpeciesCount);
        Assert.Equal(37, snapshot.Organisms.Count(robot => robot.Vegetable));
        Assert.Equal(new float[] { 16_000, 12_000 }, snapshot.WorldSize);
        Assert.Equal(587, snapshot.Organisms[0].Energy);
        var modern = engine.Save();
        Assert.ThrowsAny<Exception>(() => engine.Load(native[..^1]));
        Assert.Equal(modern, engine.Save());
        engine.Load(modern);
        engine.Tick();
        Assert.Equal(282ul, engine.Snapshot().Tick);
        engine.ImportSpecies(new SpeciesImport("Post-load center", "start stop", false,
            0xff886644, 1, 1000, 0, false, 0)
        {
            Placement = new SpeciesPlacement(SpeciesPlacementMode.Center),
        });
        var inserted = Assert.Single(engine.Snapshot().Organisms, robot => robot.Species == 2);
        Assert.Equal(new float[] { 8000, 6000 }, inserted.Position);
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
