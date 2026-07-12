using System.ComponentModel;
using System.Runtime.CompilerServices;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.ViewModels;

public sealed class MainWindowViewModel : INotifyPropertyChanged
{
    private EngineSnapshot _snapshot = EngineSnapshot.Empty;
    private uint? _selectedSlot;
    private ulong _tick;
    private int _population;
    private string _backend = "STARTING";
    private string _status = "INITIALIZING ENGINE";
    private string _selectedId = "NONE";
    private int _selectedEnergy;
    private ulong _selectedAge;
    private string _selectedSpecies = "NONE";
    private int _speciesCount;
    private ulong _births;
    private ulong _deaths;
    private ulong _mutations;
    private ulong _shots;
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
    private double _ticksPerSecond;
    private double _snapshotsPerSecond;
    private string _limitingPhase = "WAITING";

    public ulong Tick { get => _tick; private set => Set(ref _tick, value); }
    public int Population { get => _population; private set => Set(ref _population, value); }
    public string Backend { get => _backend; private set => Set(ref _backend, value); }
    public string Status { get => _status; set => Set(ref _status, value); }
    public string SelectedId { get => _selectedId; private set => Set(ref _selectedId, value); }
    public int SelectedEnergy { get => _selectedEnergy; private set => Set(ref _selectedEnergy, value); }
    public ulong SelectedAge { get => _selectedAge; private set => Set(ref _selectedAge, value); }
    public string SelectedSpecies { get => _selectedSpecies; private set => Set(ref _selectedSpecies, value); }
    public int SpeciesCount { get => _speciesCount; private set => Set(ref _speciesCount, value); }
    public ulong Births { get => _births; private set => Set(ref _births, value); }
    public ulong Deaths { get => _deaths; private set => Set(ref _deaths, value); }
    public ulong Mutations { get => _mutations; private set => Set(ref _mutations, value); }
    public ulong Shots { get => _shots; private set => Set(ref _shots, value); }
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
    public double TicksPerSecond { get => _ticksPerSecond; private set => Set(ref _ticksPerSecond, value); }
    public double SnapshotsPerSecond { get => _snapshotsPerSecond; private set => Set(ref _snapshotsPerSecond, value); }
    public string LimitingPhase { get => _limitingPhase; private set => Set(ref _limitingPhase, value); }
    public string PopulationSummary => $"{Population:N0} organisms · {SpeciesCount:N0} species";
    public string LifecycleSummary => $"BIRTH {Births:N0}    DEATH {Deaths:N0}";
    public string CombatSummary => $"SHOTS {Shots:N0}    HARVEST {EnergyHarvested:N0}    DONATED {_snapshot.Stats.EnergyDonated:N0}";
    public string MutationSummary => $"MUTATIONS {Mutations:N0}";

    public event PropertyChangedEventHandler? PropertyChanged;

    public OrganismSnapshot? SelectedOrganism => _selectedSlot is { } slot
        ? _snapshot.Organisms.FirstOrDefault(organism => organism.Slot == slot)
        : null;

    public void Select(uint slot)
    {
        _selectedSlot = slot;
        ApplySelection();
    }

    public void Update(EngineSnapshot snapshot)
    {
        _snapshot = snapshot;
        Tick = snapshot.Tick;
        Population = snapshot.Population;
        Backend = snapshot.Backend;
        Status = $"READY · {snapshot.Backend} BACKEND";
        if (_selectedSlot is null || snapshot.Organisms.All(organism => organism.Slot != _selectedSlot))
            _selectedSlot = snapshot.Organisms.FirstOrDefault()?.Slot;
        var selected = SelectedOrganism;
        SelectedId = selected is null ? "NONE" : $"{selected.Slot}:{selected.Generation}";
        SelectedEnergy = selected?.Energy ?? 0;
        SelectedAge = selected?.Age ?? 0;
        SpeciesCount = Math.Max(0, snapshot.Species.Count - 1);
        SelectedSpecies = selected is null || selected.Species >= snapshot.Species.Count
            ? "NONE"
            : snapshot.Species[(int)selected.Species].Name;
        Births = snapshot.Stats.Births;
        Deaths = snapshot.Stats.Deaths;
        Mutations = snapshot.Stats.Mutations;
        Shots = snapshot.Stats.ShotsFired;
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
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(PopulationSummary)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(LifecycleSummary)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(CombatSummary)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(MutationSummary)));
    }

    private void ApplySelection()
    {
        var selected = SelectedOrganism;
        SelectedId = selected is null ? "NONE" : $"{selected.Slot}:{selected.Generation}";
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
    }

    private void Set<T>(ref T field, T value, [CallerMemberName] string? name = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value)) return;
        field = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
    }
}
