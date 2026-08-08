namespace Darwinbots.Desktop.Core;

public enum LocalModeKind
{
    League,
    Contest,
    SimpleSurvival,
    EcoSurvival,
}

public enum LeaguePreset
{
    F1,
    F2,
    F3,
    Multibot,
    ShortBot,
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
    LocalModeRestrictions Restrictions)
{
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
