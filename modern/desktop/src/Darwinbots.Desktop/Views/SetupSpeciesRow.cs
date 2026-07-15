using System.ComponentModel;
using System.Runtime.CompilerServices;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed class SetupSpeciesRow : INotifyPropertyChanged
{
    private string _name;
    private bool _vegetable;
    private int _initialCount;
    private int _initialEnergy;
    private double _mutationRate;
    private bool _reseed;

    public SetupSpeciesRow(string name, string dna, bool vegetable, uint color, int initialCount,
        int initialEnergy, double mutationRate, bool reseed)
    {
        _name = name;
        Dna = dna;
        _vegetable = vegetable;
        Color = color;
        _initialCount = initialCount;
        _initialEnergy = initialEnergy;
        _mutationRate = mutationRate;
        _reseed = reseed;
    }

    public event PropertyChangedEventHandler? PropertyChanged;
    public string Name { get => _name; set => Set(ref _name, value); }
    public string Dna { get; }
    public bool Vegetable { get => _vegetable; set => Set(ref _vegetable, value); }
    public uint Color { get; set; }
    public int InitialCount { get => _initialCount; set => Set(ref _initialCount, value); }
    public int InitialEnergy { get => _initialEnergy; set => Set(ref _initialEnergy, value); }
    public double MutationRate { get => _mutationRate; set => Set(ref _mutationRate, Math.Clamp(value, 0, 100)); }
    public bool Reseed { get => _reseed; set => Set(ref _reseed, value); }

    public SpeciesImport ToImport(int index) => new(
        Name, Dna, Vegetable, Color, InitialCount, InitialEnergy,
        Reseed ? Math.Min(InitialCount, Math.Max(1, InitialCount / 5)) : 0,
        Reseed, (float)MutationRate);

    private void Set<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value)) return;
        field = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
}
