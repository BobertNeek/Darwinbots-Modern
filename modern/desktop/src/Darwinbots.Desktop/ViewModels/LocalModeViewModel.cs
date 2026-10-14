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
    private LeagueSeriesSnapshot? _series;
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
    public LeagueSeriesSnapshot? Series { get => _series; private set => Set(ref _series, value); }
    public string SeriesLabel => Series?.ActiveMatch is { Count: 2 } pair
        ? $"MATCH {Series.MatchNumber} · SPECIES {pair[0]} vs {pair[1]}"
        : Series is { Workflow: not LeagueWorkflow.Direct } ? "SCHEDULE COMPLETE" : "DIRECT MATCH";
    public string RankingLabel => Series is { Workflow: LeagueWorkflow.Seeding } series
        ? $"SEEDED · {string.Join(", ", series.Seeded)}"
        : Series is { Ladder.Count: > 0 } ranked ? $"RANKING · {string.Join(", ", ranked.Ladder)}" : string.Empty;

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

    public string RoundLabel => CurrentRound == 0
        ? "NOT STARTED"
        : $"ROUND {CurrentRound} / {MaximumRounds}";

    public string CycleLabel => MaximumCycles == 0
        ? "CYCLE —"
        : $"CYCLE {CycleInRound:N0} / {MaximumCycles:N0}";

    public string AdjudicationLabel => Adjudication?.Winner is { } winner
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
        Workflow = Workflow,
        EntrantOrder = ParseSpeciesOrder(EntrantOrderText),
        InitialLadder = ParseSpeciesOrder(InitialLadderText),
        SeedingOpponent = SeedingOpponent,
    };

    public static IReadOnlyList<uint> ParseSpeciesOrder(string text)
    {
        return text.Split([',', ';', ' ', '\t', '\r', '\n'], StringSplitOptions.RemoveEmptyEntries)
            .Select(value => uint.TryParse(value, System.Globalization.NumberStyles.None,
                System.Globalization.CultureInfo.InvariantCulture, out var id)
                ? id : throw new ArgumentException("League entries must be non-negative species IDs.")).ToArray();
    }

    public void MarkConfigured() => Status = LocalModeStatus.Configured;

    public void Observe(LocalModeSnapshot snapshot)
    {
        ArgumentNullException.ThrowIfNull(snapshot);
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
            Workflow = configuration.Workflow;
            EntrantOrderText = string.Join(", ", configuration.EntrantOrder);
            InitialLadderText = string.Join(", ", configuration.InitialLadder);
            SeedingOpponent = configuration.SeedingOpponent;
            MinimumRounds = configuration.MinimumRounds;
            MaximumPopulation = configuration.MaximumPopulation;
            RestartOnExtinction = configuration.RestartOnExtinction;
            SourceSpecies = configuration.SourceSpecies;
            SelectionPopulationWeight = configuration.SelectionPopulationWeight;
            SelectionEnergyWeight = configuration.SelectionEnergyWeight;
        }
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
    }

    private void Set<T>(ref T field, T value, [CallerMemberName] string? name = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value)) return;
        field = value;
        Raise(name);
        if (name == nameof(Series)) { Raise(nameof(SeriesLabel)); Raise(nameof(RankingLabel)); }
        if (name is nameof(Status) or nameof(CurrentRound) or nameof(MaximumRounds)
            or nameof(CycleInRound) or nameof(MaximumCycles) or nameof(Adjudication))
        {
            Raise(nameof(StatusLabel));
            Raise(nameof(RoundLabel));
            Raise(nameof(CycleLabel));
            Raise(nameof(AdjudicationLabel));
        }
    }

    private void Raise(string? name) => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}
