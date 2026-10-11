using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.ViewModels;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class MainWindowViewModelTests
{
    [Fact]
    public void SnapshotUpdateExposesAutomaticSelectionAndPhenotypeColor()
    {
        var organism = new OrganismSnapshot(
            7,
            3,
            [100f, 200f],
            [0f, 0f],
            875,
            19,
            Species: 1)
        {
            Phenotype = new VisualPhenotypeSnapshot(
                4,
                0xff239ac0,
                [new(0.2f, 0), new(0.4f, 314), new(0.6f, 628), new(0.8f, 942)],
                0),
        };
        var snapshot = new EngineSnapshot(91, 1, "CPU", [organism])
        {
            Species =
            [
                new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false),
                new SpeciesSnapshot("Animal Minimalis", false, 0xff239ac0, 0, false),
            ],
        };
        var viewModel = new MainWindowViewModel();

        viewModel.Update(snapshot);

        Assert.Equal(7U, viewModel.SelectedSlot);
        Assert.Equal("#239AC0", viewModel.SelectedColor);
        Assert.Equal("Animal Minimalis", viewModel.SelectedSpecies);
    }

    [Fact]
    public void AutomaticSelectionPrefersTheCentralAnimalOverAnEdgeSlot()
    {
        var edgeAnimal = new OrganismSnapshot(1, 0, [0f, 0f], [0f, 0f], 1000, 0, Species: 1);
        var centralAnimal = new OrganismSnapshot(2, 0, [8_000f, 6_000f], [0f, 0f], 1000, 0, Species: 1);
        var centralVegetable = new OrganismSnapshot(
            3,
            0,
            [8_000f, 6_000f],
            [0f, 0f],
            1000,
            0,
            Species: 2,
            Vegetable: true);
        var snapshot = new EngineSnapshot(0, 3, "CPU", [edgeAnimal, centralVegetable, centralAnimal])
        {
            Species =
            [
                new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false),
                new SpeciesSnapshot("Animal Minimalis", false, 0xff239ac0, 0, false),
                new SpeciesSnapshot("Alga Minimalis", true, 0xff62a844, 0, false),
            ],
        };
        var viewModel = new MainWindowViewModel();

        viewModel.Update(snapshot);

        Assert.Equal(2U, viewModel.SelectedSlot);
    }

    [Fact]
    public void SnapshotUpdatesDoNotEraseTheLastOperationStatus()
    {
        var viewModel = new MainWindowViewModel
        {
            Status = "BOT MOVED",
        };

        viewModel.Update(new EngineSnapshot(1, 0, "CPU", []));

        Assert.Equal("BOT MOVED", viewModel.Status);
    }

    [Fact]
    public void SnapshotReplacementDoesNotRebindSelectedFocusToAReusedSlot()
    {
        var original = new OrganismSnapshot(7, 2, [100f, 100f], [0f, 0f], 100, 1);
        var replacement = original with { Generation = 3, Energy = 900 };
        var viewModel = new MainWindowViewModel();

        viewModel.Update(new EngineSnapshot(1, 1, "CPU", [original]));
        viewModel.Select(new OrganismKey(7, 2));
        viewModel.Update(new EngineSnapshot(2, 1, "CPU", []));
        viewModel.Update(new EngineSnapshot(3, 1, "CPU", [replacement]));

        Assert.Null(viewModel.SelectedKey);
        Assert.Null(viewModel.SelectedOrganism);
    }

    [Fact]
    public void InspectorReportsPublishedMotionParentageAndActivityInsteadOfStaticLabels()
    {
        var organism = new OrganismSnapshot(
            9,
            2,
            [100f, 200f],
            [3f, 4f],
            500,
            12,
            Species: 1,
            Parents: [new OrganismKey(4, 1), null]);
        var snapshot = new EngineSnapshot(12, 1, "CPU", [organism])
        {
            Species =
            [
                new SpeciesSnapshot("Unassigned", false, 0xff858982, 0, false),
                new SpeciesSnapshot("Animal Minimalis", false, 0xff239ac0, 0, false),
            ],
        };
        var viewModel = new MainWindowViewModel();

        viewModel.Update(snapshot);

        Assert.Equal("3.0, 4.0", viewModel.SelectedVelocity);
        Assert.Equal("4:1 / NONE", viewModel.SelectedParents);
        Assert.Equal("MOVING", viewModel.SelectedActivity);
    }

    [Fact]
    public void StatusInstrumentMapsPublishedSnapshotAndSettings()
    {
        var vegetable = new OrganismSnapshot(
            2,
            0,
            [1f, 1f],
            [0f, 0f],
            100,
            0,
            Vegetable: true,
            Chloroplasts: 4);
        var animal = new OrganismSnapshot(
            7,
            3,
            [100f, 200f],
            [0f, 0f],
            800,
            12,
            Chloroplasts: 1);
        var first = new EngineSnapshot(20, 2, "CPU", [vegetable, animal])
        {
            Stats = new SimulationStatsSnapshot(4, 1, 10, 0, 0, 0, 900),
            MutationSettings = new MutationSettingsSnapshot(true, 1.5f),
            LocalMode = LocalModeSnapshot.Idle with { RestartCount = 2 },
            TicksPerSecond = 12.3456,
        };
        var viewModel = new MainWindowViewModel();

        viewModel.Update(first, TimeSpan.FromSeconds(3_723), 1.75);

        Assert.Equal(
            [
                "CPS 12.346",
                "Tot 2",
                "Bots 1",
                "Chlr 5",
                "Born 4",
                "Cycles 20",
                "Time 1h2m3s",
                "Mut 1.5x",
                "Restarts 2",
                "Shots 0",
                "Nrg 900",
                "Delta 0",
                "CostX 1.75",
                "Feed 0",
                "Move 0",
                "Events 0",
            ],
            viewModel.StatusPanels.Select(panel => panel.Display));

        var second = first with
        {
            Tick = 21,
            Stats = first.Stats with { ShotsFired = 13, TotalEnergy = 1_150 },
        };

        viewModel.Update(second, TimeSpan.FromSeconds(3_724), 1.75);

        Assert.Equal("Shots 3", viewModel.StatusPanels[9].Display);
        Assert.Equal("Nrg 1150", viewModel.StatusPanels[10].Display);
        Assert.Equal("Delta 250", viewModel.StatusPanels[11].Display);
    }
}
