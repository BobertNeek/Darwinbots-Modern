using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.ViewModels;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class LocalModeWorkflowTests
{
    [Fact]
    public void ClassicPhasesKeepPopulationArchivesAndExposeOnlyActualCompetitiveActions()
    {
        var config = LocalModeConfig.Default with { Workflow = LeagueWorkflow.PopulationSeeding,
            EntrantOrder = [0, 1, 2], FinishWithLadder = true };
        var series = new LeagueSeriesSnapshot(LeagueWorkflow.PopulationSeeding, [], 0, [], [], []) {
            ActiveSeed = 2, PopulationSeedingPhase = ClassicTournamentPhase.Seeding,
            PopulationSeeds = [new(0, 0, 2000) { FileName = "0.txt" }, new(1, 0, 2000) { FileName = "0a.txt" }],
        };
        var snapshot = new LocalModeSnapshot(LocalModeStatus.Running, LocalModeKind.League, LeaguePreset.F1,
            1, 3, 7, 15000, 0, [], null, new()) { Configuration = config, Series = series };
        var names = new SpeciesSnapshot[] { new("Zero.txt", false, 0, 0, false), new("Tie.txt", false, 0, 0, false), new("Next.txt", false, 0, 0, false) };
        var model = new LocalModeViewModel();
        var notifications = new List<string?>();
        model.PropertyChanged += (_, args) => notifications.Add(args.PropertyName);
        model.Observe(snapshot, 1999, names);
        Assert.True(model.UsesCompetitiveSettings);
        Assert.True(model.FinishWithLadder);
        Assert.False(model.CanAdjudicate);
        Assert.False(model.CanExportClassicTournament);
        Assert.Equal("CYCLE 1,999 / 2,000", model.CycleLabel);
        Assert.Equal("POPULATION SCORES · Zero.txt: 0, Tie.txt: 0", model.RankingLabel);
        Assert.True(model.BuildConfiguration().FinishWithLadder);

        series = series with { ActiveSeed = null, PopulationSeedingPhase = ClassicTournamentPhase.Tournament,
            TournamentRound = 0, ActiveMatch = [2, 0], Pending = [1], Seeded = [0], RoundWinners = [2] };
        model.Observe(snapshot with { Series = series }, -1, names);
        Assert.True(model.CanAdjudicate);
        Assert.Contains("CLASSIC TOURNAMENT · ROUND 0", model.SeriesLabel);
        Assert.Contains("Next.txt vs Zero.txt", model.SeriesLabel);
        Assert.Equal("CURRENT · Tie.txt · NEXT · Next.txt · RESERVE · Zero.txt", model.ScheduleLabel);
        Assert.Equal("CYCLE 7 / 15,000", model.CycleLabel);
        Assert.Equal("ADJUDICATION PENDING", model.AdjudicationLabel);
        Assert.True(model.CanExportPopulationSeeds);

        series = series with { PopulationSeedingPhase = ClassicTournamentPhase.Ladder, EnteredLadder = true, Ladder = [2, 1] };
        model.Observe(snapshot with { Series = series }, 0, names);
        Assert.Contains("CLASSIC STEP LADDER", model.SeriesLabel);
        Assert.True(model.CanAdjudicate);
        series = series with { PopulationSeedingPhase = ClassicTournamentPhase.Complete, ActiveMatch = [] };
        model.Observe(snapshot with { Status = LocalModeStatus.Completed, Series = series }, 10, names);
        Assert.Equal("CLASSIC STEP LADDER COMPLETE", model.SeriesLabel);
        Assert.Equal("FINAL RANKING · Next.txt, Tie.txt", model.ScheduleLabel);
        Assert.True(model.CanExportClassicTournament);
        Assert.True(model.CanExportPopulationSeeds);
        Assert.False(model.CanAdjudicate);
        Assert.Contains(nameof(LocalModeViewModel.CanAdjudicate), notifications);
        Assert.Contains(nameof(LocalModeViewModel.CanExportClassicTournament), notifications);
        Assert.Contains(nameof(LocalModeViewModel.ScheduleLabel), notifications);
    }

    [Fact]
    public void PopulationSeedingShowsNamedZeroScoresWithoutWinsAndIgnoresTheFilterReference()
    {
        var config = LocalModeConfig.Default with
        {
            Workflow = LeagueWorkflow.PopulationSeeding, EntrantOrder = [0, 1], SeedingOpponent = 9,
        };
        var mode = new LocalModeSnapshot(LocalModeStatus.Running, LocalModeKind.League, LeaguePreset.F1,
            0, 1, 2000, 15000, 1, [], null, new())
        {
            Configuration = config,
            Series = new(LeagueWorkflow.PopulationSeeding, [], 0, [], [], [])
            {
                ActiveSeed = 1, PopulationSeeds = [new(0, 0, 2000)],
            },
        };
        var model = new LocalModeViewModel();
        model.Observe(mode, 1999,
            [new("Gone.txt", false, 0, 0, false), new("Next.txt", false, 0, 0, false)]);
        Assert.Equal("POPULATION SEED · Next.txt", model.SeriesLabel);
        Assert.Equal("POPULATION SCORES · Gone.txt: 0", model.RankingLabel);
        Assert.Equal("CANDIDATES SCORED · 1", model.RoundLabel);
        Assert.Equal("CYCLE 1,999 / 2,000", model.CycleLabel);
        Assert.Equal("SCORE = NON-PLANT POPULATION AT CYCLE 2,000", model.AdjudicationLabel);
        Assert.False(model.CanAdjudicate);
        Assert.True(model.CanExportPopulationSeeds);
        Assert.Null(model.BuildConfiguration().SeedingOpponent);
        Assert.Equal(new uint[] { 0, 1 }, model.BuildConfiguration().EntrantOrder);
        model.Observe(mode with { Status = LocalModeStatus.Completed, Series = mode.Series! with { ActiveSeed = null } }, 2000);
        Assert.Equal("POPULATION SEEDING COMPLETE", model.SeriesLabel);
        Assert.Empty(model.Results);
    }

    [Fact]
    public void LeagueScheduleControlsRetainExplicitOrderAndReference()
    {
        var model = new LocalModeViewModel {
            Workflow = LeagueWorkflow.Seeding, EntrantOrderText = "4, 2; 1",
            InitialLadderText = "4 2", SeedingOpponent = 0,
        };
        var config = model.BuildConfiguration();
        Assert.Equal(LeagueWorkflow.Seeding, config.Workflow);
        Assert.Equal(new uint[] { 4, 2, 1 }, config.EntrantOrder);
        Assert.Equal(new uint[] { 4, 2 }, config.InitialLadder);
        Assert.Equal(0U, config.SeedingOpponent);
        Assert.Throws<ArgumentException>(() => LocalModeViewModel.ParseSpeciesOrder("1, rubbish"));
    }

    [Fact]
    public void LoadedModeEditsRetainItsPopulationSourceAndSelectionPolicy()
    {
        var configuration = LocalModeConfig.Default with
        {
            Kind = LocalModeKind.Contest, Preset = LeaguePreset.F2,
            MinimumRounds = 2, MaximumRounds = 7, MaximumCycles = 123,
            MaximumPopulation = 37, RestartOnExtinction = false,
            SourceSpecies = 4, SelectionPopulationWeight = 0.25f,
            SelectionEnergyWeight = 0.75f, EnergyHandicap = 25,
        };
        var viewModel = new LocalModeViewModel();
        viewModel.Observe(new LocalModeSnapshot(
            LocalModeStatus.Configured, configuration.Kind, configuration.Preset,
            0, configuration.MaximumRounds, 0, configuration.MaximumCycles,
            0, [], null, configuration.Restrictions)
        {
            Configuration = configuration, MinimumRounds = configuration.MinimumRounds,
            EnergyHandicap = configuration.EnergyHandicap,
        });
        Assert.Equal(configuration, viewModel.BuildConfiguration());
        viewModel.MaximumPopulation = 38;
        Assert.Equal(configuration with { MaximumPopulation = 38 }, viewModel.BuildConfiguration());
    }

    [Fact]
    public void LocalModeOpensWithDb24832ContestDefaults()
    {
        var localMode = new MainWindowViewModel().LocalMode;

        Assert.Equal(5U, localMode.MinimumRounds);
        Assert.Equal(1U, localMode.MaximumRounds);
        Assert.Equal(15_000UL, localMode.MaximumCycles);
        Assert.Equal(700, localMode.MaximumPopulation);
    }

    [Fact]
    public void LeagueTabWorkflowBuildsConfigStartsObservesAndCompletesLocalMode()
    {
        var viewModel = new MainWindowViewModel();

        viewModel.LocalMode.SelectKind(LocalModeKind.Contest);
        viewModel.LocalMode.Preset = LeaguePreset.F1;
        viewModel.LocalMode.MinimumRounds = 1;
        viewModel.LocalMode.MaximumRounds = 2;
        viewModel.LocalMode.MaximumCycles = 120;
        viewModel.LocalMode.MaximumPopulation = 16;
        viewModel.LocalMode.RestartOnExtinction = true;
        viewModel.LocalMode.EnergyHandicap = 25;
        viewModel.LocalMode.ProhibitViruses = true;

        var config = viewModel.LocalMode.BuildConfiguration();
        Assert.Equal(LocalModeKind.Contest, config.Kind);
        Assert.Equal(LeaguePreset.F1, config.Preset);
        Assert.Equal(25, config.EnergyHandicap);
        Assert.True(config.Restrictions.ProhibitViruses);
        Assert.Equal("CONFIGURED", viewModel.LocalMode.StatusLabel);

        viewModel.LocalMode.Observe(new LocalModeSnapshot(
            LocalModeStatus.Running,
            LocalModeKind.Contest,
            LeaguePreset.F1,
            1,
            2,
            18,
            120,
            0,
            [],
            null,
            config.Restrictions));
        Assert.Equal("ROUND 1 / 2", viewModel.LocalMode.RoundLabel);
        Assert.Equal("RUNNING", viewModel.LocalMode.StatusLabel);

        viewModel.LocalMode.Observe(new LocalModeSnapshot(
            LocalModeStatus.Completed,
            LocalModeKind.Contest,
            LeaguePreset.F1,
            2,
            2,
            120,
            120,
            0,
            [new LocalModeResult(2, 1, 90, 12_400, "operator decision")],
            new LocalModeAdjudication(1, "operator decision", false),
            config.Restrictions));
        Assert.Equal("COMPLETE", viewModel.LocalMode.StatusLabel);
        Assert.Equal("WINNER · SPECIES 1", viewModel.LocalMode.AdjudicationLabel);
    }
}
