using Avalonia.Controls;
using Avalonia.Interactivity;

namespace Darwinbots.Desktop.Views;

public sealed partial class AdvancedSettingsWindow : Window
{
    public bool Accepted { get; private set; }
    public int MetabolismCost => (int)(Metabolism.Value ?? 1);
    public int VegetableEnergyPerTick => (int)(VegetableEnergy.Value ?? 4);
    public int SunlightEnergy => (int)(Sunlight.Value ?? 100);
    public float[] GravityVector => [(float)(GravityX.Value ?? 0), (float)(GravityY.Value ?? 0)];
    public float DragValue => (float)(Drag.Value ?? 0);
    public float BrownianValue => (float)(Brownian.Value ?? 0);

    public AdvancedSettingsWindow() => InitializeComponent();

    public AdvancedSettingsWindow(int metabolism, int vegetableEnergy, int sunlight, float[] gravity, float drag, float brownian) : this()
    {
        Metabolism.Value = metabolism;
        VegetableEnergy.Value = vegetableEnergy;
        Sunlight.Value = sunlight;
        GravityX.Value = (decimal)gravity[0];
        GravityY.Value = (decimal)gravity[1];
        Drag.Value = (decimal)drag;
        Brownian.Value = (decimal)brownian;
    }

    private void Apply_Click(object? sender, RoutedEventArgs e) { Accepted = true; Close(); }
    private void Cancel_Click(object? sender, RoutedEventArgs e) => Close();
}

