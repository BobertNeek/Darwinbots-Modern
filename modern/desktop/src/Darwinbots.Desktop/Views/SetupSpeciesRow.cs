using System.ComponentModel;
using System.Runtime.CompilerServices;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed class SetupSpeciesRow : INotifyPropertyChanged
{
    private string _name;
    private string _comment;
    private bool _vegetable;
    private int _initialCount;
    private int _initialEnergy;
    private int _minimumPopulation;
    private double _mutationRate;
    private bool _reseed;
    private SpeciesPlacement _placement;
    private SkinPointSnapshot[] _skin;
    private SpeciesRestrictionSettings _restrictions;

    public SetupSpeciesRow(
        string name,
        string dna,
        bool vegetable,
        uint color,
        int initialCount,
        int initialEnergy,
        double mutationRate,
        bool reseed,
        int? minimumPopulation = null,
        string? comment = null,
        SpeciesPlacement? placement = null,
        SkinPointSnapshot[]? skin = null,
        SpeciesRestrictionSettings? restrictions = null)
    {
        _name = name;
        Dna = dna;
        _vegetable = vegetable;
        Color = color;
        _initialCount = initialCount;
        _initialEnergy = initialEnergy;
        _minimumPopulation = minimumPopulation ?? (reseed ? Math.Min(initialCount, Math.Max(1, initialCount / 5)) : 0);
        _mutationRate = mutationRate;
        _reseed = reseed;
        _comment = comment ?? string.Empty;
        _placement = placement ?? new SpeciesPlacement();
        _skin = skin?.ToArray() ?? DefaultSkin();
        _restrictions = restrictions ?? new SpeciesRestrictionSettings();
    }

    public event PropertyChangedEventHandler? PropertyChanged;
    public string Name { get => _name; set => Set(ref _name, value); }
    public string Comment { get => _comment; set => Set(ref _comment, value); }
    public string Dna { get; }
    public bool Vegetable { get => _vegetable; set => Set(ref _vegetable, value); }
    public uint Color { get; set; }
    public int InitialCount { get => _initialCount; set => Set(ref _initialCount, value); }
    public int InitialEnergy { get => _initialEnergy; set => Set(ref _initialEnergy, value); }
    public int MinimumPopulation { get => _minimumPopulation; set => Set(ref _minimumPopulation, Math.Max(0, value)); }
    public double MutationRate { get => _mutationRate; set => Set(ref _mutationRate, Math.Clamp(value, 0, 100)); }
    public bool Reseed { get => _reseed; set => Set(ref _reseed, value); }

    public int PlacementIndex
    {
        get => (int)_placement.Mode;
        set
        {
            if (!Enum.IsDefined((SpeciesPlacementMode)value)) return;
            SetPlacement(_placement with { Mode = (SpeciesPlacementMode)value });
        }
    }

    public float PlacementX { get => _placement.X; set => SetPlacement(_placement with { X = value }); }
    public float PlacementY { get => _placement.Y; set => SetPlacement(_placement with { Y = value }); }

    public float SkinRadius0 { get => _skin[0].Radius; set => SetSkin(0, _skin[0] with { Radius = value }); }
    public float SkinRadius1 { get => _skin[1].Radius; set => SetSkin(1, _skin[1] with { Radius = value }); }
    public float SkinRadius2 { get => _skin[2].Radius; set => SetSkin(2, _skin[2] with { Radius = value }); }
    public float SkinRadius3 { get => _skin[3].Radius; set => SetSkin(3, _skin[3] with { Radius = value }); }
    public int SkinAngle0 { get => _skin[0].Angle; set => SetSkin(0, _skin[0] with { Angle = value }); }
    public int SkinAngle1 { get => _skin[1].Angle; set => SetSkin(1, _skin[1] with { Angle = value }); }
    public int SkinAngle2 { get => _skin[2].Angle; set => SetSkin(2, _skin[2] with { Angle = value }); }
    public int SkinAngle3 { get => _skin[3].Angle; set => SetSkin(3, _skin[3] with { Angle = value }); }
    public string SkinPreview => string.Join(" · ", _skin.Select(point => $"{point.Radius:0.00}/{point.Angle}"));

    public bool KillMultibot { get => _restrictions.KillMultibot; set => SetRestrictions(_restrictions with { KillMultibot = value }, nameof(KillMultibot)); }
    public bool KillDisqualified { get => _restrictions.KillDisqualified; set => SetRestrictions(_restrictions with { KillDisqualified = value }, nameof(KillDisqualified)); }
    public bool FixedInPlace { get => _restrictions.FixedInPlace; set => SetRestrictions(_restrictions with { FixedInPlace = value }, nameof(FixedInPlace)); }
    public bool DisableVision { get => _restrictions.DisableVision; set => SetRestrictions(_restrictions with { DisableVision = value }, nameof(DisableVision)); }
    public bool DisableDna { get => _restrictions.DisableDna; set => SetRestrictions(_restrictions with { DisableDna = value }, nameof(DisableDna)); }
    public bool DisableMovement { get => _restrictions.DisableMovement; set => SetRestrictions(_restrictions with { DisableMovement = value }, nameof(DisableMovement)); }
    public bool DisableReproduction { get => _restrictions.DisableReproduction; set => SetRestrictions(_restrictions with { DisableReproduction = value }, nameof(DisableReproduction)); }
    public bool VirusImmune { get => _restrictions.VirusImmune; set => SetRestrictions(_restrictions with { VirusImmune = value }, nameof(VirusImmune)); }
    public bool DisableMutations { get => _restrictions.DisableMutations; set => SetRestrictions(_restrictions with { DisableMutations = value }, nameof(DisableMutations)); }
    public bool DisableChloroplasts { get => _restrictions.DisableChloroplasts; set => SetRestrictions(_restrictions with { DisableChloroplasts = value }, nameof(DisableChloroplasts)); }

    public SpeciesImport ToImport(int index) => new(
        Name, Dna, Vegetable, Color, InitialCount, InitialEnergy,
        MinimumPopulation, Reseed, (float)MutationRate)
    {
        Comment = Comment,
        Placement = _placement,
        Skin = _skin.ToArray(),
        Restrictions = _restrictions,
    };

    public SetupSpeciesRow Duplicate() => new(
        $"{Name} Copy", Dna, Vegetable, Color, InitialCount, InitialEnergy,
        MutationRate, Reseed, MinimumPopulation, Comment, _placement, _skin, _restrictions);

    private void SetPlacement(SpeciesPlacement value)
    {
        if (_placement == value) return;
        _placement = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(PlacementIndex)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(PlacementX)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(PlacementY)));
    }

    private void SetSkin(int index, SkinPointSnapshot value)
    {
        value = value with { Radius = Math.Clamp(value.Radius, 0.15f, 0.82f), Angle = Math.Clamp(value.Angle, 0, 1256) };
        if (_skin[index] == value) return;
        _skin[index] = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs($"SkinRadius{index}"));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs($"SkinAngle{index}"));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(SkinPreview)));
    }

    private void SetRestrictions(SpeciesRestrictionSettings value, string propertyName)
    {
        if (_restrictions == value) return;
        _restrictions = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }

    private static SkinPointSnapshot[] DefaultSkin() =>
    [
        new(0.45f, 0),
        new(0.55f, 314),
        new(0.45f, 628),
        new(0.55f, 942),
    ];

    private void Set<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value)) return;
        field = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
}
