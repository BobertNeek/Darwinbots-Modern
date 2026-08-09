namespace Darwinbots.Desktop.Core;

public enum SimulationSettingsTab
{
    Species,
    General,
    PhysicsAndCosts,
    Mutations,
    RestartAndLeague,
    Internet,
}

public sealed record SimulationSettingsTabDefinition(
    SimulationSettingsTab Key,
    string Header);

public sealed class SimulationSettingsWorkflow
{
    public static IReadOnlyList<SimulationSettingsTabDefinition> Tabs { get; } =
    [
        new(SimulationSettingsTab.Species, "SPECIES"),
        new(SimulationSettingsTab.General, "GENERAL"),
        new(SimulationSettingsTab.PhysicsAndCosts, "PHYSICS AND COSTS"),
        new(SimulationSettingsTab.Mutations, "MUTATIONS"),
        new(SimulationSettingsTab.RestartAndLeague, "RESTART AND LEAGUE"),
        new(SimulationSettingsTab.Internet, "INTERNET"),
    ];

    public WorldSetupOptions Live { get; private set; }
    public WorldSetupOptions WorkingCopy { get; private set; }

    public SimulationSettingsWorkflow(WorldSetupOptions live)
    {
        ArgumentNullException.ThrowIfNull(live);
        Live = Clone(live);
        WorkingCopy = Clone(live);
    }

    public void UpdateWorkingCopy(Func<WorldSetupOptions, WorldSetupOptions> update)
    {
        ArgumentNullException.ThrowIfNull(update);
        WorkingCopy = Clone(update(WorkingCopy));
    }

    public void LoadWorkingCopy(WorldSetupOptions settings) => WorkingCopy = Clone(settings);

    public void Cancel() => WorkingCopy = Clone(Live);

    public WorldSetupOptions Change()
    {
        Live = Clone(WorkingCopy);
        WorkingCopy = Clone(Live);
        return Live;
    }

    public WorldSetupOptions StartNew() => Change();

    public void Reset(WorldSetupOptions settings)
    {
        Live = Clone(settings);
        WorkingCopy = Clone(settings);
    }

    private static WorldSetupOptions Clone(WorldSetupOptions settings) => settings with
    {
        Gravity = settings.Gravity.ToArray(),
        Species = settings.Species.ToArray(),
    };
}
