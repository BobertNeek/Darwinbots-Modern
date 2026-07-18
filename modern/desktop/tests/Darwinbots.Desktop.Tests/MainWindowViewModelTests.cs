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
}
