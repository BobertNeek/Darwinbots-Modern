using System.ComponentModel;
using System.Runtime.CompilerServices;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.ViewModels;

public sealed class LocalModeViewModel : INotifyPropertyChanged
{
    private LocalModeKind _kind = LocalModeKind.League;
    private LeaguePreset _preset = LeaguePreset.F1;
    private LeagueWorkflow _workflow;
    private string _entrantOrderText = string.Empty;
    private string _initialLadderText = string.Empty;
    private uint? _seedingOpponent;
    private bool _finishWithLadder;
    private LeagueSeriesSnapshot? _series;
    private IReadOnlyList<SpeciesSnapshot> _species = [];
    private int? _nativeCycle;
    private StagedEvolutionSnapshot? _evolution;
    public StagedEvolutionConfiguration? StagedEvolution { get; set; }
    public StagedEvolutionSnapshot? Evolution { get => _evolution; private set => Set(ref _evolution, value); }
    public string EvolutionLabel => Evolution is { } state
        ? $"STAGE {state.Stage} · {state.Phase.Replace('_', ' ').ToUpperInvariant()} · LFOR {state.ReductionUnit:0.###}"
        : string.Empty;
    private uint _minimumRounds = 5;
    private uint _maximumRounds = 1;
    private ulong _maximumCycles = 15_000;
    private int _maximumPopulation = 700;
    private bool _restartOnExtinction = true;
    private int _energyHandicap;
    private bool _prohibitInformationShots;
    private bool _prohibitViruses;
    private bool _prohibitSexualReproduction;
    private bool _prohibitCrossSpeciesTieTransfer;
    private bool _robotFixingAllowed = true;
    private bool _asexualReproductionAllowed = true;
    private uint? _sourceSpecies;
    private float _selectionPopulationWeight = 0.5f;
    private float _selectionEnergyWeight = 0.5f;
    private LocalModeStatus _status = LocalModeStatus.Idle;
    private uint _currentRound;
    private ulong _cycleInRound;
    private uint _restartCount;
    private IReadOnlyList<LocalModeResult> _results = [];
    private LocalModeAdjudication? _adjudication;

    public LocalModeKind Kind { get => _kind; private set => Set(ref _kind, value); }
    public LeagueWorkflow Workflow { get => _workflow; set => Set(ref _workflow, value); }
    public string EntrantOrderText { get => _entrantOrderText; set => Set(ref _entrantOrderText, value); }
    public string InitialLadderText { get => _initialLadderText; set => Set(ref _initialLadderText, value); }
    public uint? SeedingOpponent { get => _seedingOpponent; set => Set(ref _seedingOpponent, value); }
    public bool FinishWithLadder { get => _finishWithLadder; set => Set(ref _finishWithLadder, value); }
    public LeagueSeriesSnapshot? Series { get => _series; private set => Set(ref _series, value); }
    public string SeriesLabel => Evolution is { } evolution
        ? evolution.Phase == "testing" ? $"TEST AGAINST STAGE {evolution.OpponentStage}"
            : evolution.Hidden ? "BASELINE HIDDEN" : "BASELINE ACTIVE"
        : Series is { Workflow: LeagueWorkflow.PopulationSeeding } population
        ? population.PopulationSeedingPhase switch
        {
            ClassicTournamentPhase.Tournament => $"CLASSIC TOURNAMENT · ROUND {population.TournamentRound} · {ActivePair(population)}",
            ClassicTournamentPhase.Ladder => $"CLASSIC STEP LADDER · {ActivePair(population)}",
            ClassicTournamentPhase.Complete => population.EnteredLadder ? "CLASSIC STEP LADDER COMPLETE" : "CLASSIC TOURNAMENT COMPLETE",
            _ => population.ActiveSeed is { } seed ? $"POPULATION SEED · {SpeciesName(seed)}" : "POPULATION SEEDING COMPLETE",
        }
        : Series?.ActiveMatch is { Count: 2 } pair
        ? $"MATCH {Series.MatchNumber} · SPECIES {pair[0]} vs {pair[1]}"
        : Series is { Workflow: not LeagueWorkflow.Direct } ? "SCHEDULE COMPLETE" : "DIRECT MATCH";
    public string RankingLabel => Series is { Workflow: LeagueWorkflow.PopulationSeeding } population
        ? "POPULATION SCORES · " + string.Join(", ", population.PopulationSeeds.Select(seed => $"{SpeciesName(seed.Species)}: {seed.Population}"))
        : Series is { Workflow: LeagueWorkflow.Seeding } series
        ? $"SEEDED · {string.Join(", ", series.Seeded)}"
        : Series is { Ladder.Count: > 0 } ranked ? $"RANKING · {string.Join(", ", ranked.Ladder)}" : string.Empty;
    public bool IsPopulationSeeding => Workflow == LeagueWorkflow.PopulationSeeding;
    // Classic match settings apply after its fixed population census.
    public bool UsesCompetitiveSettings => true;
    public bool UsesInitialLadder => Workflow == LeagueWorkflow.Ladder;
    public bool UsesSeedFilter => Workflow == LeagueWorkflow.Seeding;
    public bool CanAdjudicate => !IsPopulationSeeding
        || Status == LocalModeStatus.Running && Series?.IsClassicCompetitive == true;
    public bool CanExportPopulationSeeds => Series is { Workflow: LeagueWorkflow.PopulationSeeding, PopulationSeeds.Count: > 0 };
    public bool CanExportClassicTournament => IsPopulationSeeding && Status == LocalModeStatus.Completed
        && Series is { Workflow: LeagueWorkflow.PopulationSeeding, PopulationSeedingPhase: ClassicTournamentPhase.Complete,
            Ladder.Count: > 0 } series && (series.EnteredLadder || series.Ladder.Count == 1);
    public string ScheduleLabel => Series is not { Workflow: LeagueWorkflow.PopulationSeeding } series ? string.Empty
        : series.PopulationSeedingPhase == ClassicTournamentPhase.Complete
            ? "FINAL RANKING · " + Names(series.Ladder)
        : series.PopulationSeedingPhase == ClassicTournamentPhase.Ladder
            ? "RANKING · " + Names(series.Ladder) + " · QUEUED · " + Names(series.Pending)
        : series.PopulationSeedingPhase == ClassicTournamentPhase.Tournament
            ? "CURRENT · " + Names(series.Pending) + " · NEXT · " + Names(series.RoundWinners) + " · RESERVE · " + Names(series.Seeded)
        : string.Empty;

    public LeaguePreset Preset { get => _preset; set => Set(ref _preset, value); }
    public uint MinimumRounds { get => _minimumRounds; set => Set(ref _minimumRounds, value); }
    public uint MaximumRounds { get => _maximumRounds; set => Set(ref _maximumRounds, value); }
    public ulong MaximumCycles { get => _maximumCycles; set => Set(ref _maximumCycles, value); }
    public int MaximumPopulation { get => _maximumPopulation; set => Set(ref _maximumPopulation, value); }
    public bool RestartOnExtinction { get => _restartOnExtinction; set => Set(ref _restartOnExtinction, value); }
    public int EnergyHandicap { get => _energyHandicap; set => Set(ref _energyHandicap, value); }
    public bool ProhibitInformationShots { get => _prohibitInformationShots; set => Set(ref _prohibitInformationShots, value); }
    public bool ProhibitViruses { get => _prohibitViruses; set => Set(ref _prohibitViruses, value); }
    public bool ProhibitSexualReproduction { get => _prohibitSexualReproduction; set => Set(ref _prohibitSexualReproduction, value); }
    public bool ProhibitCrossSpeciesTieTransfer { get => _prohibitCrossSpeciesTieTransfer; set => Set(ref _prohibitCrossSpeciesTieTransfer, value); }
    public bool RobotFixingAllowed { get => _robotFixingAllowed; set => Set(ref _robotFixingAllowed, value); }
    public bool AsexualReproductionAllowed { get => _asexualReproductionAllowed; set => Set(ref _asexualReproductionAllowed, value); }
    public uint? SourceSpecies { get => _sourceSpecies; set => Set(ref _sourceSpecies, value); }
    public float SelectionPopulationWeight { get => _selectionPopulationWeight; set => Set(ref _selectionPopulationWeight, value); }
    public float SelectionEnergyWeight { get => _selectionEnergyWeight; set => Set(ref _selectionEnergyWeight, value); }
    public LocalModeStatus Status { get => _status; private set => Set(ref _status, value); }
    public uint CurrentRound { get => _currentRound; private set => Set(ref _currentRound, value); }
    public ulong CycleInRound { get => _cycleInRound; private set => Set(ref _cycleInRound, value); }
    public uint RestartCount { get => _restartCount; private set => Set(ref _restartCount, value); }
    public IReadOnlyList<LocalModeResult> Results { get => _results; private set => Set(ref _results, value); }
    public LocalModeAdjudication? Adjudication { get => _adjudication; private set => Set(ref _adjudication, value); }

    public string StatusLabel => Status switch
    {
        LocalModeStatus.Configured => "CONFIGURED",
        LocalModeStatus.Running => "RUNNING",
        LocalModeStatus.RoundComplete => "ROUND COMPLETE",
        LocalModeStatus.Completed => "COMPLETE",
        _ => "IDLE",
    };

    public string RoundLabel => Evolution is { } evolution
        ? evolution.Phase == "testing" ? $"CHAMPION TEST WINS {evolution.TestWins}" : $"ATTEMPT {RestartCount}"
        : Series is { IsPopulationCensus: true } population
        ? $"CANDIDATES SCORED · {population.PopulationSeeds.Count}"
        : CurrentRound == 0
        ? "NOT STARTED"
        : $"ROUND {CurrentRound} / {MaximumRounds}";

    public string CycleLabel => Series is { IsPopulationCensus: true }
        ? $"CYCLE {(_nativeCycle is { } cycle ? cycle.ToString("N0") : CycleInRound.ToString("N0"))} / 2,000"
        : Evolution is { Phase: not "testing" }
        ? $"CYCLE {CycleInRound:N0}"
        : MaximumCycles == 0
        ? "CYCLE —"
        : $"CYCLE {CycleInRound:N0} / {MaximumCycles:N0}";

    public string AdjudicationLabel => IsPopulationSeeding && (Series is null || Series.IsPopulationCensus)
        ? "SCORE = NON-PLANT POPULATION AT CYCLE 2,000"
        : Adjudication?.Winner is { } winner
        ? $"WINNER · SPECIES {winner}"
        : Adjudication is not null
            ? "NO WINNER"
            : "ADJUDICATION PENDING";

    public event PropertyChangedEventHandler? PropertyChanged;

    public void SelectKind(LocalModeKind kind)
    {
        Kind = kind;
        MarkConfigured();
    }

    public LocalModeConfig BuildConfiguration() => new(
        Kind,
        Preset,
        MinimumRounds,
        MaximumRounds,
        MaximumCycles,
        MaximumPopulation,
        RestartOnExtinction,
        SourceSpecies,
        SelectionPopulationWeight,
        SelectionEnergyWeight,
        new LocalModeRestrictions(
            ProhibitInformationShots,
            ProhibitViruses,
            ProhibitSexualReproduction,
            ProhibitCrossSpeciesTieTransfer,
            RobotFixingAllowed,
            AsexualReproductionAllowed),
        EnergyHandicap)
    {
        StagedEvolution = StagedEvolution,
        Workflow = Workflow,
        EntrantOrder = ParseSpeciesOrder(EntrantOrderText),
        InitialLadder = IsPopulationSeeding ? [] : ParseSpeciesOrder(InitialLadderText),
        SeedingOpponent = Workflow == LeagueWorkflow.PopulationSeeding ? null : SeedingOpponent,
        FinishWithLadder = IsPopulationSeeding && FinishWithLadder,
    };

    public static IReadOnlyList<uint> ParseSpeciesOrder(string text)
    {
        return text.Split([',', ';', ' ', '\t', '\r', '\n'], StringSplitOptions.RemoveEmptyEntries)
            .Select(value => uint.TryParse(value, System.Globalization.NumberStyles.None,
                System.Globalization.CultureInfo.InvariantCulture, out var id)
                ? id : throw new ArgumentException("League entries must be non-negative species IDs.")).ToArray();
    }

    public void MarkConfigured() => Status = LocalModeStatus.Configured;

    public void Observe(LocalModeSnapshot snapshot, int? nativeCycle = null, IReadOnlyList<SpeciesSnapshot>? species = null)
    {
        ArgumentNullException.ThrowIfNull(snapshot);
        _nativeCycle = nativeCycle;
        _species = species ?? [];
        Kind = snapshot.Kind;
        Preset = snapshot.Preset;
        Status = snapshot.Status;
        CurrentRound = snapshot.CurrentRound;
        MaximumRounds = snapshot.MaximumRounds;
        CycleInRound = snapshot.CycleInRound;
        MaximumCycles = snapshot.MaximumCycles;
        RestartCount = snapshot.RestartCount;
        EnergyHandicap = snapshot.EnergyHandicap;
        if (snapshot.Configuration is { } configuration)
        {
            StagedEvolution = configuration.StagedEvolution;
            Workflow = configuration.Workflow;
            EntrantOrderText = string.Join(", ", configuration.EntrantOrder);
            InitialLadderText = string.Join(", ", configuration.InitialLadder);
            SeedingOpponent = configuration.SeedingOpponent;
            FinishWithLadder = configuration.FinishWithLadder;
            MinimumRounds = configuration.MinimumRounds;
            MaximumPopulation = configuration.MaximumPopulation;
            RestartOnExtinction = configuration.RestartOnExtinction;
            SourceSpecies = configuration.SourceSpecies;
            SelectionPopulationWeight = configuration.SelectionPopulationWeight;
            SelectionEnergyWeight = configuration.SelectionEnergyWeight;
        }
        Evolution = snapshot.Evolution;
        Series = snapshot.Series;
        Results = snapshot.Results;
        Adjudication = snapshot.Adjudication;
        ProhibitInformationShots = snapshot.Restrictions.ProhibitInformationShots;
        ProhibitViruses = snapshot.Restrictions.ProhibitViruses;
        ProhibitSexualReproduction = snapshot.Restrictions.ProhibitSexualReproduction;
        ProhibitCrossSpeciesTieTransfer = snapshot.Restrictions.ProhibitCrossSpeciesTieTransfer;
        RobotFixingAllowed = snapshot.Restrictions.RobotFixingAllowed;
        AsexualReproductionAllowed = snapshot.Restrictions.AsexualReproductionAllowed;
        Raise(nameof(StatusLabel));
        Raise(nameof(RoundLabel));
        Raise(nameof(CycleLabel));
        Raise(nameof(AdjudicationLabel));
        Raise(nameof(SeriesLabel));
        Raise(nameof(RankingLabel));
    }

    private string SpeciesName(uint species) => species < _species.Count
        ? _species[(int)species].Name : $"SPECIES {species}";
    private string Names(IReadOnlyList<uint> species) => species.Count == 0 ? "—" : string.Join(", ", species.Select(SpeciesName));
    private string ActivePair(LeagueSeriesSnapshot series) => series.ActiveMatch.Count == 2
        ? $"{SpeciesName(series.ActiveMatch[0])} vs {SpeciesName(series.ActiveMatch[1])}" : "NO ACTIVE MATCH";

    private void Set<T>(ref T field, T value, [CallerMemberName] string? name = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value)) return;
        field = value;
        Raise(name);
        if (name == nameof(Evolution)) {
            Raise(nameof(EvolutionLabel)); Raise(nameof(SeriesLabel)); Raise(nameof(RoundLabel)); Raise(nameof(CycleLabel));
        }
        if (name == nameof(Series)) {
            Raise(nameof(SeriesLabel)); Raise(nameof(RankingLabel)); Raise(nameof(RoundLabel));
            Raise(nameof(CycleLabel)); Raise(nameof(CanExportPopulationSeeds));
            Raise(nameof(CanAdjudicate)); Raise(nameof(CanExportClassicTournament)); Raise(nameof(ScheduleLabel)); Raise(nameof(AdjudicationLabel));
        }
        if (name == nameof(Workflow)) {
            Raise(nameof(CanAdjudicate)); Raise(nameof(AdjudicationLabel)); Raise(nameof(IsPopulationSeeding));
            Raise(nameof(UsesCompetitiveSettings)); Raise(nameof(UsesSeedFilter));
            Raise(nameof(UsesInitialLadder));
            Raise(nameof(CanExportClassicTournament));
        }
        if (name is nameof(Status) or nameof(CurrentRound) or nameof(MaximumRounds)
            or nameof(CycleInRound) or nameof(MaximumCycles) or nameof(Adjudication))
        {
            Raise(nameof(StatusLabel));
            Raise(nameof(RoundLabel));
            Raise(nameof(CycleLabel));
            Raise(nameof(AdjudicationLabel));
            Raise(nameof(CanExportClassicTournament));
            Raise(nameof(CanAdjudicate));
        }
    }

    private void Raise(string? name) => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}
