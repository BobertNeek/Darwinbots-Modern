using System.ComponentModel;
using System.Globalization;
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
    private readonly bool _nativeSkin;
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
        SpeciesRestrictionSettings? restrictions = null,
        MutationProfileUpdate? mutationProfile = null,
        string? dnaPath = null,
        uint? speciesId = null,
        bool nativeSkin = false)
    {
        SpeciesId = speciesId;
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
        _nativeSkin = nativeSkin;
        _restrictions = restrictions ?? new SpeciesRestrictionSettings();
        MutationProfile = mutationProfile;
        DnaPath = dnaPath;
    }

    public event PropertyChangedEventHandler? PropertyChanged;
    public uint? SpeciesId { get; }
    public string Name { get => _name; set => Set(ref _name, value); }
    public string Comment { get => _comment; set => Set(ref _comment, value); }
    public string Dna { get; }
    public string? DnaPath { get; }
    public MutationProfileUpdate? MutationProfile { get; set; }
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

    public decimal SkinRadiusMinimum => _nativeSkin ? 0m : 0.15m;
    public decimal SkinRadiusMaximum => _nativeSkin ? 1m : 0.82m;
    // Native angles are 314 - 2 * the signed VB6 Integer in the file.
    public int SkinAngleMinimum => _nativeSkin ? 314 - 2 * short.MaxValue : 0;
    public int SkinAngleMaximum => _nativeSkin ? 314 - 2 * short.MinValue : 1256;
    public decimal SkinRadius0 { get => GetSkinRadius(0); set => SetSkinRadius(0, value); }
    public decimal SkinRadius1 { get => GetSkinRadius(1); set => SetSkinRadius(1, value); }
    public decimal SkinRadius2 { get => GetSkinRadius(2); set => SetSkinRadius(2, value); }
    public decimal SkinRadius3 { get => GetSkinRadius(3); set => SetSkinRadius(3, value); }
    public int SkinAngle0 { get => _skin[0].Angle; set => SetSkinAngle(0, value); }
    public int SkinAngle1 { get => _skin[1].Angle; set => SetSkinAngle(1, value); }
    public int SkinAngle2 { get => _skin[2].Angle; set => SetSkinAngle(2, value); }
    public int SkinAngle3 { get => _skin[3].Angle; set => SetSkinAngle(3, value); }
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
        SpeciesId = SpeciesId,
        Comment = Comment,
        Placement = _placement,
        Skin = _skin.ToArray(),
        Restrictions = _restrictions,
        MutationProfile = MutationProfile,
        DnaPath = DnaPath,
    };

    public SetupSpeciesRow Duplicate() => new(
        $"{Name} Copy", Dna, Vegetable, Color, InitialCount, InitialEnergy,
        MutationRate, Reseed, MinimumPopulation, Comment, _placement, _skin, _restrictions,
        MutationProfile, DnaPath, nativeSkin: _nativeSkin);

    private void SetPlacement(SpeciesPlacement value)
    {
        if (_placement == value) return;
        _placement = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(PlacementIndex)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(PlacementX)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(PlacementY)));
    }

    private decimal GetSkinRadius(int index) =>
        decimal.Parse(_skin[index].Radius.ToString("R", CultureInfo.InvariantCulture),
            NumberStyles.Float, CultureInfo.InvariantCulture);

    private void SetSkinRadius(int index, decimal radius)
    {
        // NumericUpDown uses decimal. A direct conversion from float can round
        // away digits when the binding echoes an untouched native value.
        if (radius == GetSkinRadius(index)) return;
        SetSkin(index, _skin[index] with { Radius = (float)Math.Clamp(radius, SkinRadiusMinimum, SkinRadiusMaximum) });
    }

    private void SetSkinAngle(int index, int angle) =>
        SetSkin(index, _skin[index] with { Angle = Math.Clamp(angle, SkinAngleMinimum, SkinAngleMaximum) });

    private void SetSkin(int index, SkinPointSnapshot value)
    {
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
