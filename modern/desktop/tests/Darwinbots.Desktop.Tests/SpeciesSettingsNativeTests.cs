using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class SpeciesSettingsNativeTests
{
    [Fact]
    public void NativeRosterUpdateRetainsRobotsAndMetadataAndRejectsInvalidIdentitiesAtomically()
    {
        var options = new WorldSetupOptions
        {
            Backend = "Cpu", PopulationCapacity = 64,
            GlobalSettings = new GlobalSettings { Main = new GlobalMainSettings { NewSeed = false } },
        };
        using var engine = new NativeEngineClient(options);
        engine.ImportSpecies(new("First", "' retained metadata\r\nstart stop", false, 0xff239ac0, 1, 1000, 0, false, 0) { Comment = "A retained description", Placement = new SpeciesPlacement { Left = 0.1f, Right = 0.8f, Top = 0.2f, Bottom = 0.9f } });
        engine.ImportSpecies(new("Second", "start stop", false, 0xff239ac0, 1, 900, 0, false, 0));
        var original = engine.ReadConfiguration()!.Species;
        var first = Assert.Single(original, species => species.Name == "First");
        var second = Assert.Single(original, species => species.Name == "Second");
        Assert.NotNull(first.SpeciesId);
        Assert.Equal("A retained description", first.Comment);
        Assert.Equal(0.1f, first.Placement.Left);
        var robots = engine.Snapshot().Organisms.Select(robot => (robot.Slot, robot.Generation, robot.Energy, robot.Species)).ToArray();
        var updated = first with { Name = "Renamed", InitialCount = 4, InitialEnergy = 777 };
        var added = new SpeciesImport("Added", "start stop", true, 0xff4c963b, 2, 800, 0, false, 0);
        engine.UpdateSpeciesSettings([updated, added]);
        var configured = engine.ReadConfiguration()!.Species;
        Assert.Equal(first.SpeciesId, Assert.Single(configured, species => species.Name == "Renamed").SpeciesId);
        Assert.DoesNotContain(configured, species => species.Name == "Second");
        Assert.Equal(robots, engine.Snapshot().Organisms.Select(robot => (robot.Slot, robot.Generation, robot.Energy, robot.Species)));
        Assert.Equal(first.Dna, engine.ExportSpeciesTemplateDna(first.SpeciesId!.Value));
        Assert.Equal(second.Dna, engine.ExportSpeciesTemplateDna(second.SpeciesId!.Value));
        var saved = engine.Save();
        using var resumed = new NativeEngineClient(options);
        resumed.Load(saved);
        Assert.Equal(2, resumed.Snapshot().Population);
        var restored = resumed.ReadConfiguration()!.Species;
        var restoredFirst = Assert.Single(restored, species => species.Name == "Renamed");
        Assert.Equal(777, restoredFirst.InitialEnergy);
        Assert.Equal(first.Comment, restoredFirst.Comment);
        Assert.Equal(first.Placement, restoredFirst.Placement);
        Assert.DoesNotContain(restored, species => species.Name == "Second");
        var beforeFailure = resumed.Save();
        var failure = Record.Exception(() => resumed.UpdateSpeciesSettings([updated with { SpeciesId = uint.MaxValue }]));
        Assert.NotNull(failure);
        Assert.Contains("identity", failure.Message, StringComparison.OrdinalIgnoreCase);
        Assert.Equal(beforeFailure, resumed.Save());
    }
}
