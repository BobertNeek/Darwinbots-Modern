namespace Darwinbots.Desktop.Core;

public enum LocalModeKind
{
    League,
    Contest,
    SimpleSurvival,
    EcoSurvival,
    Zerobot,
}

public enum LeaguePreset
{
    F1,
    F2,
    F3,
    Multibot,
    ShortBot,
    // Retained save/API name for SG, the short bot alias with a six-gene limit.
    SingleGene,
    SingleStore,
}

public enum LocalModeStatus
{
    Idle,
    Configured,
    Running,
    RoundComplete,
    Completed,
}

public enum LeagueWorkflow { Direct, Ladder, Tournament, Seeding, PopulationSeeding }
public enum ClassicTournamentPhase { Seeding, Tournament, Ladder, Complete }

public sealed record StagedEvolutionConfiguration(EvolutionMode Mode, string SourceDna,
    float ReductionUnit, uint CycleLength, bool NormalizeDnaSize, int ZerobotInitialDnaLength,
    bool AutoGraphs)
{
    public ushort FitnessWeight { get; init; } = 100;
    public MutationProfileUpdate? SourceMutationProfile { get; init; }
}

public sealed record StagedEvolutionSnapshot(uint Stage, string Phase, bool Hidden,
    float ReductionUnit, uint HideCycles, uint TestWins, uint OpponentStage, double Handicap)
{
    public IReadOnlyList<uint> BaselineSpecies { get; init; } = [];
    public IReadOnlyList<uint> MutatorSpecies { get; init; } = [];
    public int TestResultCursor { get; init; }
}

public sealed record LeagueMatchSnapshot(uint Number, IReadOnlyList<uint> Entrants, uint Winner,
    int FirstResult, int ResultCount);

public sealed record PopulationSeedSnapshot(uint Species, int Population, int NativeCycle)
{
    public string FileName { get; init; } = string.Empty;
}

public sealed record LeagueSeriesSnapshot(LeagueWorkflow Workflow, IReadOnlyList<uint> ActiveMatch,
    uint MatchNumber, IReadOnlyList<uint> Ladder, IReadOnlyList<uint> Seeded, IReadOnlyList<LeagueMatchSnapshot> Matches)
{
    public int MatchStartResult { get; init; }
    public uint? ActiveSeed { get; init; }
    public IReadOnlyList<PopulationSeedSnapshot> PopulationSeeds { get; init; } = [];
    public ClassicTournamentPhase? PopulationSeedingPhase { get; init; }
    public uint TournamentRound { get; init; }
    public bool EnteredLadder { get; init; }
    public IReadOnlyList<uint> Pending { get; init; } = [];
    public IReadOnlyList<uint> RoundWinners { get; init; } = [];
    public IReadOnlyList<uint> SeedFileOrder { get; init; } = [];
    public bool IsPopulationCensus => Workflow == LeagueWorkflow.PopulationSeeding
        && PopulationSeedingPhase is null or ClassicTournamentPhase.Seeding;
    public bool IsClassicCompetitive => Workflow == LeagueWorkflow.PopulationSeeding
        && PopulationSeedingPhase is ClassicTournamentPhase.Tournament or ClassicTournamentPhase.Ladder
        && ActiveMatch.Count == 2;
}

public sealed record LocalModeRestrictions(
    bool ProhibitInformationShots = false,
    bool ProhibitViruses = false,
    bool ProhibitSexualReproduction = false,
    bool ProhibitCrossSpeciesTieTransfer = false,
    bool RobotFixingAllowed = true,
    bool AsexualReproductionAllowed = true);

public sealed record LocalModeConfig(
    LocalModeKind Kind,
    LeaguePreset Preset,
    uint MinimumRounds,
    uint MaximumRounds,
    ulong MaximumCycles,
    int MaximumPopulation,
    bool RestartOnExtinction,
    uint? SourceSpecies,
    float SelectionPopulationWeight,
    float SelectionEnergyWeight,
    LocalModeRestrictions Restrictions,
    int EnergyHandicap = 0)
{
    public LeagueWorkflow Workflow { get; init; }
    public IReadOnlyList<uint> EntrantOrder { get; init; } = [];
    public IReadOnlyList<uint> InitialLadder { get; init; } = [];
    public uint? SeedingOpponent { get; init; }
    public bool FinishWithLadder { get; init; }
    public StagedEvolutionConfiguration? StagedEvolution { get; init; }
    public static LocalModeConfig Default { get; } = new(
        LocalModeKind.League,
        LeaguePreset.F1,
        1,
        3,
        10_000,
        25_000,
        true,
        null,
        0.5f,
        0.5f,
        new());
}

public sealed record LocalModeResult(
    uint Round,
    uint? Winner,
    int WinningPopulation,
    long WinningEnergy,
    string Reason,
    bool Automatic = false);

public sealed record LocalModeAdjudication(
    uint? Winner,
    string Reason,
    bool Automatic = false);

public sealed record DisqualificationRecord(
    ulong Tick, uint Round, uint Species, string FileName, string Tag, string Reason);

public sealed record LocalModeSnapshot(
    LocalModeStatus Status,
    LocalModeKind Kind,
    LeaguePreset Preset,
    uint CurrentRound,
    uint MaximumRounds,
    ulong CycleInRound,
    ulong MaximumCycles,
    uint RestartCount,
    IReadOnlyList<LocalModeResult> Results,
    LocalModeAdjudication? Adjudication,
    LocalModeRestrictions Restrictions)
{
    public uint MinimumRounds { get; init; } = 1;
    public int EnergyHandicap { get; init; }
    public LocalModeConfig? Configuration { get; init; }
    public IReadOnlyList<DisqualificationRecord> Disqualifications { get; init; } = [];
    public LeagueSeriesSnapshot? Series { get; init; }
    public StagedEvolutionSnapshot? Evolution { get; init; }

    public static LocalModeSnapshot Idle { get; } = new(
        LocalModeStatus.Idle,
        LocalModeKind.League,
        LeaguePreset.F1,
        0,
        0,
        0,
        0,
        0,
        [],
        null,
        new())
    {
        MinimumRounds = 0,
    };
}
