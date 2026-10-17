using System.ComponentModel;
using System.Globalization;
using System.Runtime.CompilerServices;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.ViewModels;

public sealed class MainWindowViewModel : INotifyPropertyChanged
{
    private EngineSnapshot _snapshot = EngineSnapshot.Empty;
    private OrganismKey? _selectedKey;
    private bool _selectionWasLost;
    private ulong _tick;
    private int? _nativeCycle;
    private string _cycleLabel = "0";
    private int _population;
    private string _backend = "STARTING";
    private string _status = "INITIALIZING ENGINE";
    private string _selectedId = "NONE";
    private string _selectedColor = "#858982";
    private int _selectedEnergy;
    private ulong _selectedAge;
    private string _selectedSpecies = "NONE";
    private int _speciesCount;
    private ulong _births;
    private ulong _deaths;
    private ulong _mutations;
    private ulong _shots;
    private ulong _projectileImpacts;
    private ulong _plantEnergyGenerated;
    private long _totalEnergy;
    private ulong _energyHarvested;
    private int _selectedBody;
    private int _selectedWaste;
    private int _selectedShell;
    private int _selectedSlime;
    private int _selectedPoison;
    private int _selectedVenom;
    private int _selectedChloroplasts;
    private int _selectedAim;
    private int _selectedParalyzed;
    private int _selectedPoisoned;
    private string _selectedVelocity = "0.0, 0.0";
    private string _selectedParents = "NONE / NONE";
    private string _selectedActivity = "NONE";
    private double _ticksPerSecond;
    private double _snapshotsPerSecond;
    private string _limitingPhase = "WAITING";
    private long? _previousTotalEnergy;
    private ulong? _previousShots;
    private ulong? _previousTick;
    private double _costMultiplier = 1.0;

    private readonly StatusPanelViewModel[] _statusPanels =
    [
        new("CPS"),
        new("Tot"),
        new("Bots"),
        new("Chlr"),
        new("Born"),
        new("Cycles"),
        new("Time"),
        new("Mut"),
        new("Restarts"),
        new("Shots"),
        new("Nrg"),
        new("Delta"),
        new("CostX"),
        new("Feed"),
        new("Move"),
        new("Events"),
    ];

    public LocalModeViewModel LocalMode { get; } = new();
    public ulong Tick { get => _tick; private set => Set(ref _tick, value); }
    public int? NativeCycle { get => _nativeCycle; private set => Set(ref _nativeCycle, value); }
    public string CycleLabel { get => _cycleLabel; private set => Set(ref _cycleLabel, value); }
    public int Population { get => _population; private set => Set(ref _population, value); }
    public string Backend { get => _backend; private set => Set(ref _backend, value); }
    public string Status { get => _status; set => Set(ref _status, value); }
    public string SelectedId { get => _selectedId; private set => Set(ref _selectedId, value); }
    public string SelectedColor { get => _selectedColor; private set => Set(ref _selectedColor, value); }
    public int SelectedEnergy { get => _selectedEnergy; private set => Set(ref _selectedEnergy, value); }
    public ulong SelectedAge { get => _selectedAge; private set => Set(ref _selectedAge, value); }
    public string SelectedSpecies { get => _selectedSpecies; private set => Set(ref _selectedSpecies, value); }
    public int SpeciesCount { get => _speciesCount; private set => Set(ref _speciesCount, value); }
    public ulong Births { get => _births; private set => Set(ref _births, value); }
    public ulong Deaths { get => _deaths; private set => Set(ref _deaths, value); }
    public ulong Mutations { get => _mutations; private set => Set(ref _mutations, value); }
    public ulong Shots { get => _shots; private set => Set(ref _shots, value); }
    public ulong ProjectileImpacts { get => _projectileImpacts; private set => Set(ref _projectileImpacts, value); }
    public ulong PlantEnergyGenerated { get => _plantEnergyGenerated; private set => Set(ref _plantEnergyGenerated, value); }
    public long TotalEnergy { get => _totalEnergy; private set => Set(ref _totalEnergy, value); }
    public ulong EnergyHarvested { get => _energyHarvested; private set => Set(ref _energyHarvested, value); }
    public int SelectedBody { get => _selectedBody; private set => Set(ref _selectedBody, value); }
    public int SelectedWaste { get => _selectedWaste; private set => Set(ref _selectedWaste, value); }
    public int SelectedShell { get => _selectedShell; private set => Set(ref _selectedShell, value); }
    public int SelectedSlime { get => _selectedSlime; private set => Set(ref _selectedSlime, value); }
    public int SelectedPoison { get => _selectedPoison; private set => Set(ref _selectedPoison, value); }
    public int SelectedVenom { get => _selectedVenom; private set => Set(ref _selectedVenom, value); }
    public int SelectedChloroplasts { get => _selectedChloroplasts; private set => Set(ref _selectedChloroplasts, value); }
    public int SelectedAim { get => _selectedAim; private set => Set(ref _selectedAim, value); }
    public int SelectedParalyzed { get => _selectedParalyzed; private set => Set(ref _selectedParalyzed, value); }
    public int SelectedPoisoned { get => _selectedPoisoned; private set => Set(ref _selectedPoisoned, value); }
    public string SelectedVelocity { get => _selectedVelocity; private set => Set(ref _selectedVelocity, value); }
    public string SelectedParents { get => _selectedParents; private set => Set(ref _selectedParents, value); }
    public string SelectedActivity { get => _selectedActivity; private set => Set(ref _selectedActivity, value); }
    public double TicksPerSecond { get => _ticksPerSecond; private set => Set(ref _ticksPerSecond, value); }
    public double SnapshotsPerSecond { get => _snapshotsPerSecond; private set => Set(ref _snapshotsPerSecond, value); }
    public string LimitingPhase { get => _limitingPhase; private set => Set(ref _limitingPhase, value); }
    public string PopulationSummary => $"{Population:N0} organisms · {SpeciesCount:N0} species";
    public string LifecycleSummary => $"BIRTH {Births:N0}    DEATH {Deaths:N0}";
    public string CombatSummary => $"FIRED {Shots:N0}    IMPACT {ProjectileImpacts:N0}    HARVEST {EnergyHarvested:N0}    DONATED {_snapshot.Stats.EnergyDonated:N0}    FEED {_snapshot.Stats.FeedingEvents:N0}    MOVE {_snapshot.Stats.IntentionalMovementEvents:N0}    EFFECT {_snapshot.Stats.ProjectileEffects:N0}";
    public string EnergyActivitySummary => $"FIRED {Shots:N0}    IMPACT {ProjectileImpacts:N0}    PLANT {PlantEnergyGenerated:N0}";
    public string PlantSummary => $"PLANT ENERGY {PlantEnergyGenerated:N0}";
    public string MutationSummary => $"MUTATIONS {Mutations:N0}";
    public IReadOnlyList<StatusPanelViewModel> StatusPanels => _statusPanels;

    public event PropertyChangedEventHandler? PropertyChanged;

    public OrganismSnapshot? SelectedOrganism => _selectedKey is { } key
        ? _snapshot.Organisms.FirstOrDefault(organism => organism.Slot == key.Slot && organism.Generation == key.Generation)
        : null;
    public uint? SelectedSlot => _selectedKey?.Slot;
    public OrganismKey? SelectedKey => _selectedKey;

    public void Select(uint slot)
    {
        var organism = _snapshot.Organisms.FirstOrDefault(value => value.Slot == slot);
        _selectedKey = organism is null ? null : new OrganismKey(organism.Slot, organism.Generation);
        _selectionWasLost = _selectedKey is null;
        ApplySelection();
    }

    public void Select(OrganismKey key)
    {
        _selectedKey = _snapshot.Organisms.Any(value => value.Slot == key.Slot && value.Generation == key.Generation)
            ? key
            : null;
        _selectionWasLost = _selectedKey is null;
        ApplySelection();
    }

    public void ClearSelection()
    {
        _selectedKey = null;
        _selectionWasLost = true;
        ApplySelection();
    }

    public void Update(EngineSnapshot snapshot, TimeSpan? elapsed = null, double? costMultiplier = null)
    {
        var (energyDelta, shotsThisCycle) = CalculateDeltas(snapshot);
        if (costMultiplier is { } value)
            _costMultiplier = value;
        _snapshot = snapshot;
        LocalMode.Observe(snapshot.LocalMode, snapshot.NativeCycle, snapshot.Species);
        Tick = snapshot.Tick;
        NativeCycle = snapshot.NativeCycle;
        CycleLabel = snapshot.NativeCycle?.ToString(CultureInfo.InvariantCulture)
            ?? snapshot.Tick.ToString(CultureInfo.InvariantCulture);
        Population = snapshot.Population;
        Backend = snapshot.Backend;
        if (_selectedKey is null)
        {
            if (!_selectionWasLost) _selectedKey = SelectDefaultOrganism(snapshot);
        }
        else if (snapshot.Organisms.All(organism =>
                     organism.Slot != _selectedKey.Value.Slot || organism.Generation != _selectedKey.Value.Generation))
        {
            _selectedKey = null;
            _selectionWasLost = true;
        }
        // Modern worlds have an empty Unassigned sentinel; native DB2 starts with a real species at zero.
        var emptySentinel = snapshot.Species.FirstOrDefault()?.Name == "Unassigned"
            && snapshot.Organisms.All(robot => robot.Species != 0);
        SpeciesCount = Math.Max(0, snapshot.Species.Count - (emptySentinel ? 1 : 0));
        ApplySelection();
        Births = snapshot.Stats.Births;
        Deaths = snapshot.Stats.Deaths;
        Mutations = snapshot.Stats.Mutations;
        Shots = snapshot.Stats.ShotsFired;
        ProjectileImpacts = snapshot.Stats.ProjectileImpacts;
        PlantEnergyGenerated = snapshot.Stats.PlantEnergyGenerated;
        TotalEnergy = snapshot.Stats.TotalEnergy;
        EnergyHarvested = snapshot.Stats.EnergyHarvested;
        TicksPerSecond = snapshot.TicksPerSecond;
        SnapshotsPerSecond = snapshot.SnapshotsPerSecond;
        var phases = new Dictionary<string, double>
        {
            ["DNA"] = snapshot.PhaseTimings.Dna,
            ["SPATIAL"] = snapshot.PhaseTimings.Spatial,
            ["SENSING"] = snapshot.PhaseTimings.Sensing,
            ["INTERACTIONS"] = snapshot.PhaseTimings.Interactions,
            ["PHYSICS"] = snapshot.PhaseTimings.Physics,
            ["LIFECYCLE"] = snapshot.PhaseTimings.Lifecycle,
            ["SNAPSHOT"] = snapshot.PhaseTimings.Snapshot,
        };
        var limiting = phases.MaxBy(value => value.Value);
        LimitingPhase = $"{limiting.Key} {limiting.Value:0.0} ms";
        UpdateStatusPanels(snapshot, elapsed ?? TimeSpan.Zero, energyDelta, shotsThisCycle);
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(PopulationSummary)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(LifecycleSummary)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(CombatSummary)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(EnergyActivitySummary)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(PlantSummary)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(MutationSummary)));
    }

    private (long EnergyDelta, ulong ShotsThisCycle) CalculateDeltas(EngineSnapshot snapshot)
    {
        var advancing = _previousTick is { } previousTick
            && snapshot.Tick > previousTick
            && _previousTotalEnergy is not null
            && _previousShots is not null;
        var energyDelta = advancing
            ? snapshot.Stats.TotalEnergy - _previousTotalEnergy!.Value
            : 0;
        var shotsThisCycle = advancing && snapshot.Stats.ShotsFired >= _previousShots!.Value
            ? snapshot.Stats.ShotsFired - _previousShots.Value
            : 0;
        _previousTick = snapshot.Tick;
        _previousTotalEnergy = snapshot.Stats.TotalEnergy;
        _previousShots = snapshot.Stats.ShotsFired;
        return (energyDelta, shotsThisCycle);
    }

    private void UpdateStatusPanels(
        EngineSnapshot snapshot,
        TimeSpan elapsed,
        long energyDelta,
        ulong shotsThisCycle)
    {
        SetPanel(0, snapshot.TicksPerSecond.ToString("0.###", CultureInfo.InvariantCulture));
        SetPanel(1, snapshot.Population.ToString(CultureInfo.InvariantCulture));
        SetPanel(2, snapshot.Organisms.Count(organism => !organism.Vegetable).ToString(CultureInfo.InvariantCulture));
        SetPanel(3, snapshot.Organisms.Sum(organism => organism.Chloroplasts).ToString(CultureInfo.InvariantCulture));
        SetPanel(4, snapshot.Stats.Births.ToString(CultureInfo.InvariantCulture));
        SetPanel(5, CycleLabel);
        SetPanel(6, FormatElapsed(elapsed));
        SetPanel(7, $"{snapshot.MutationSettings.GlobalMultiplier.ToString("0.###", CultureInfo.InvariantCulture)}x");
        SetPanel(8, snapshot.LocalMode.RestartCount.ToString(CultureInfo.InvariantCulture));
        SetPanel(9, (snapshot.Stats.NativeShotsVisited ?? shotsThisCycle).ToString(CultureInfo.InvariantCulture));
        SetPanel(10, (snapshot.Stats.NativeEnergyDisplayed is { } nativeEnergy
            ? nativeEnergy : snapshot.Stats.TotalEnergy).ToString(CultureInfo.InvariantCulture));
        SetPanel(11, (snapshot.Stats.NativeEnergyDelta ?? energyDelta).ToString(CultureInfo.InvariantCulture));
        SetPanel(12, _costMultiplier.ToString("0.#####", CultureInfo.InvariantCulture));
        SetPanel(13, snapshot.Stats.FeedingEvents.ToString(CultureInfo.InvariantCulture));
        SetPanel(14, snapshot.Stats.IntentionalMovementEvents.ToString(CultureInfo.InvariantCulture));
        SetPanel(15, snapshot.Stats.ProjectileEffects.ToString(CultureInfo.InvariantCulture));
    }

    private void SetPanel(int index, string value)
    {
        _statusPanels[index].SetValue(value);
    }

    private static string FormatElapsed(TimeSpan elapsed)
    {
        var hours = (long)Math.Max(0, elapsed.TotalHours);
        return $"{hours}h{elapsed.Minutes}m{elapsed.Seconds}s";
    }

    private void ApplySelection()
    {
        var selected = SelectedOrganism;
        SelectedId = selected is null ? "NONE" : $"{selected.Slot}:{selected.Generation}";
        SelectedColor = selected is null ? "#858982" : $"#{selected.Phenotype.Color & 0x00ffffff:X6}";
        SelectedEnergy = selected?.Energy ?? 0;
        SelectedAge = selected?.Age ?? 0;
        SelectedSpecies = selected is null || selected.Species >= _snapshot.Species.Count
            ? "NONE"
            : _snapshot.Species[(int)selected.Species].Name;
        SelectedBody = selected?.Body ?? 0;
        SelectedWaste = selected?.Waste ?? 0;
        SelectedShell = selected?.Shell ?? 0;
        SelectedSlime = selected?.Slime ?? 0;
        SelectedPoison = selected?.Poison ?? 0;
        SelectedVenom = selected?.Venom ?? 0;
        SelectedChloroplasts = selected?.Chloroplasts ?? 0;
        SelectedAim = selected?.Aim ?? 0;
        SelectedParalyzed = selected?.Paralyzed ?? 0;
        SelectedPoisoned = selected?.Poisoned ?? 0;
        var velocityX = selected?.Velocity.ElementAtOrDefault(0) ?? 0f;
        var velocityY = selected?.Velocity.ElementAtOrDefault(1) ?? 0f;
        SelectedVelocity = $"{velocityX:0.0}, {velocityY:0.0}";
        SelectedParents = selected?.Parents is { Length: > 0 } parents
            ? string.Join(" / ", parents.Take(2).Select(parent => parent is { } value ? $"{value.Slot}:{value.Generation}" : "NONE"))
            : "NONE / NONE";
        SelectedActivity = selected is null
            ? "NONE"
            : selected.Paralyzed > 0
                ? "PARALYZED"
                : velocityX * velocityX + velocityY * velocityY > 0.0025f
                    ? "MOVING"
                    : "IDLE";
    }

    private static OrganismKey? SelectDefaultOrganism(EngineSnapshot snapshot)
    {
        var animals = snapshot.Organisms.Where(organism => !organism.Vegetable).ToArray();
        var candidates = animals.Length > 0 ? animals : snapshot.Organisms;
        var centerX = snapshot.WorldSize.ElementAtOrDefault(0) * 0.5f;
        var centerY = snapshot.WorldSize.ElementAtOrDefault(1) * 0.5f;
        return candidates.MinBy(organism =>
        {
            var offsetX = organism.Position.ElementAtOrDefault(0) - centerX;
            var offsetY = organism.Position.ElementAtOrDefault(1) - centerY;
            return offsetX * offsetX + offsetY * offsetY;
        }) is { } selected
            ? new OrganismKey(selected.Slot, selected.Generation)
            : null;
    }

    private void Set<T>(ref T field, T value, [CallerMemberName] string? name = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value)) return;
        field = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
    }
}
