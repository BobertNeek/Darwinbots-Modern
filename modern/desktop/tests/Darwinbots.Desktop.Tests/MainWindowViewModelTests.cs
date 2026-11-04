using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.ViewModels;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class MainWindowViewModelTests
{
    [Fact]
    public void NativeEnergyStatusUsesPublishedRingValuesIncludingZeroAndLoadedTicks()
    {
        var viewModel = new MainWindowViewModel();
        var first = new EngineSnapshot(20, 0, "CPU", [])
        {
            Stats = SimulationStatsSnapshot.Empty with
            {
                TotalEnergy = 900, NativeEnergyDisplayed = 2_000, NativeEnergyDelta = 200,
            },
        };
        viewModel.Update(first);
        Assert.Equal("Nrg 2000", viewModel.StatusPanels[10].Display);
        Assert.Equal("Delta 200", viewModel.StatusPanels[11].Display);
        Assert.Equal(900L, viewModel.TotalEnergy);
        var zero = first with
        {
            Tick = 21,
            Stats = first.Stats with { TotalEnergy = 950, NativeEnergyDisplayed = 0, NativeEnergyDelta = 0 },
        };
        viewModel.Update(zero);
        viewModel.Update(zero);
        Assert.Equal("Nrg 0", viewModel.StatusPanels[10].Display);
        Assert.Equal("Delta 0", viewModel.StatusPanels[11].Display);
        var loaded = first with
        {
            Tick = 1,
            Stats = first.Stats with { TotalEnergy = 700, NativeEnergyDisplayed = 123, NativeEnergyDelta = -45 },
        };
        viewModel.Update(loaded);
        Assert.Equal("Nrg 123", viewModel.StatusPanels[10].Display);
        Assert.Equal("Delta -45", viewModel.StatusPanels[11].Display);
        viewModel.Update(loaded with
        {
            Tick = 2,
            Stats = loaded.Stats with { TotalEnergy = 750, NativeEnergyDisplayed = null, NativeEnergyDelta = null },
        });
        Assert.Equal("Nrg 750", viewModel.StatusPanels[10].Display);
        Assert.Equal("Delta 50", viewModel.StatusPanels[11].Display);
        Assert.Equal(750L, viewModel.TotalEnergy);
    }

    [Fact]
    public void NativeShotStatusUsesVisitedSlotsInsteadOfFiredDeltaOrVisibleTrails()
    {
        var trail = new ShotSnapshot(new OrganismKey(1, 0), [0f, 0f], [1f, 0f],
            [1f, 0f], -1, 10, 1, 40, 10f, false);
        var first = new EngineSnapshot(20, 0, "CPU", [])
        {
            Stats = SimulationStatsSnapshot.Empty with { ShotsFired = 10, NativeShotsVisited = 2 },
            Shots = [trail],
        };
        var viewModel = new MainWindowViewModel();

        // A visited slot can expire before publication; neither final trails nor firing total is the count.
        viewModel.Update(first);
        Assert.Equal("Shots 2", viewModel.StatusPanels[9].Display);
        Assert.Single(first.Shots);
        Assert.Equal(10UL, viewModel.Shots);

        var next = first with
        {
            Tick = 21,
            Stats = first.Stats with { ShotsFired = 13, NativeShotsVisited = 1 },
            Shots = [trail, trail with { Kind = -2 }],
        };
        // A slot appended beyond the original loop bound is visible before it is visited.
        viewModel.Update(next);
        Assert.Equal("Shots 1", viewModel.StatusPanels[9].Display);
        Assert.Equal(2, next.Shots.Count);
        Assert.Equal(13UL, viewModel.Shots);
        Assert.Contains("FIRED 13", viewModel.CombatSummary);
    }

    [Fact]
    public void NativeShotStatusRetainsPublishedZeroAndCountAcrossPausedRepeatedAndLoadedTicks()
    {
        var snapshot = new EngineSnapshot(20, 0, "CPU", [])
        {
            Stats = SimulationStatsSnapshot.Empty with { ShotsFired = 10, NativeShotsVisited = 4 },
        };
        var viewModel = new MainWindowViewModel();

        viewModel.Update(snapshot);
        Assert.Equal("Shots 4", viewModel.StatusPanels[9].Display);
        viewModel.Update(snapshot);
        Assert.Equal("Shots 4", viewModel.StatusPanels[9].Display);

        snapshot = snapshot with { Stats = snapshot.Stats with { NativeShotsVisited = 0 } };
        viewModel.Update(snapshot);
        Assert.Equal("Shots 0", viewModel.StatusPanels[9].Display);

        // A newly loaded world may publish a lower scheduling tick and firing total.
        var loaded = snapshot with
        {
            Tick = 3,
            Stats = snapshot.Stats with { ShotsFired = 2, NativeShotsVisited = 17 },
        };
        viewModel.Update(loaded);
        Assert.Equal("Shots 17", viewModel.StatusPanels[9].Display);
        viewModel.Update(loaded);
        Assert.Equal("Shots 17", viewModel.StatusPanels[9].Display);

        viewModel.Update(loaded with
        {
            Tick = 4,
            Stats = loaded.Stats with { ShotsFired = 5, NativeShotsVisited = 0 },
        });
        Assert.Equal("Shots 0", viewModel.StatusPanels[9].Display);
        Assert.Equal(5UL, viewModel.Shots);
    }

    [Fact]
    public void OlderAndExplicitLegacyPayloadsKeepTheShotDeltaFallback()
    {
        var first = NativeSnapshotParser.Parse(
            "{\"tick\":10,\"organisms\":[],\"stats\":{\"shots_fired\":8}}", "CPU");
        var next = NativeSnapshotParser.Parse(
            "{\"tick\":11,\"organisms\":[],\"stats\":{\"shots_fired\":11,\"native_shots_visited\":null}}", "CPU");
        var loaded = NativeSnapshotParser.Parse(
            "{\"tick\":0,\"organisms\":[],\"stats\":{\"shots_fired\":1}}", "CPU");
        var viewModel = new MainWindowViewModel();

        Assert.Null(first.Stats.NativeShotsVisited);
        Assert.Null(next.Stats.NativeShotsVisited);
        viewModel.Update(first);
        Assert.Equal("Shots 0", viewModel.StatusPanels[9].Display);
        viewModel.Update(next);
        Assert.Equal("Shots 3", viewModel.StatusPanels[9].Display);
        viewModel.Update(next);
        Assert.Equal("Shots 0", viewModel.StatusPanels[9].Display);
        viewModel.Update(loaded);
        Assert.Equal("Shots 0", viewModel.StatusPanels[9].Display);
        viewModel.Update(loaded with
        {
            Tick = 1,
            Stats = loaded.Stats with { ShotsFired = 3 },
        });
        Assert.Equal("Shots 2", viewModel.StatusPanels[9].Display);
        Assert.Equal(3UL, viewModel.Shots);
    }

    [Fact]
    public void OriginalCycleDisplayCanResetWhileSchedulingTicksAndDeltasKeepAdvancing()
    {
        var viewModel = new MainWindowViewModel();
        var notifications = new List<string?>();
        viewModel.PropertyChanged += (_, args) => notifications.Add(args.PropertyName);
        var snapshot = new EngineSnapshot(20, 0, "CPU", [])
        {
            NativeCycle = -1,
            Stats = SimulationStatsSnapshot.Empty with { TotalEnergy = 100, ShotsFired = 2 },
            TicksPerSecond = 12.5,
        };
        viewModel.Update(snapshot);
        Assert.Equal(20ul, viewModel.Tick);
        Assert.Equal(-1, viewModel.NativeCycle);
        Assert.Equal("-1", viewModel.CycleLabel);
        Assert.Equal("Cycles -1", viewModel.StatusPanels[5].Display);
        Assert.Contains(nameof(MainWindowViewModel.CycleLabel), notifications);

        viewModel.Update(snapshot with
        {
            Tick = 21, NativeCycle = 0,
            Stats = snapshot.Stats with { TotalEnergy = 150, ShotsFired = 5 },
        });
        Assert.Equal(21ul, viewModel.Tick);
        Assert.Equal("0", viewModel.CycleLabel);
        Assert.Equal("Cycles 0", viewModel.StatusPanels[5].Display);
        Assert.Equal("Shots 3", viewModel.StatusPanels[9].Display);
        Assert.Equal("Delta 50", viewModel.StatusPanels[11].Display);
        Assert.Equal(12.5, viewModel.TicksPerSecond);

        viewModel.Update(snapshot with { Tick = ulong.MaxValue, NativeCycle = null });
        Assert.Null(viewModel.NativeCycle);
        Assert.Equal(ulong.MaxValue.ToString(System.Globalization.CultureInfo.InvariantCulture), viewModel.CycleLabel);
        Assert.Equal($"Cycles {viewModel.CycleLabel}", viewModel.StatusPanels[5].Display);
    }

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
