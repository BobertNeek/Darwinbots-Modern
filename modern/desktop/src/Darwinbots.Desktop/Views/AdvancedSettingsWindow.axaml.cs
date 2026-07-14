using Avalonia.Controls;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class AdvancedSettingsWindow : Window
{
    public bool Accepted { get; private set; }

    public EnvironmentUpdate Update => new(
        (int)(Metabolism.Value ?? 1),
        (int)(VegetableEnergy.Value ?? 0),
        (int)(Sunlight.Value ?? 100),
        [(float)(GravityX.Value ?? 0), (float)(GravityY.Value ?? 0)],
        (float)(Drag.Value ?? 0),
        (float)(Brownian.Value ?? 0),
        new Db2PhysicsOptions(
            (float)(MaxVelocity.Value ?? 60),
            (float)(MovementEfficiency.Value ?? 0.66m),
            (float)(SurfaceGravity.Value ?? 0),
            (float)(StaticFriction.Value ?? 0),
            (float)(KineticFriction.Value ?? 0),
            (double)(Density.Value ?? 0),
            (double)(Viscosity.Value ?? 0),
            (float)(Elasticity.Value ?? 0.8m)),
        new Db2ShotOptions(
            (float)(ShotSpeed.Value ?? 40),
            (float)(ShotRangeMultiplier.Value ?? 1),
            (float)(ShotDecay.Value ?? 20),
            EnergyShotsNoDecay.IsChecked == true,
            WasteShotsNoDecay.IsChecked == true),
        new Db2VegetationOptions(
            (int)(StartChloroplasts.Value ?? 16_000),
            (int)(MaxPlantEnergy.Value ?? 100),
            (int)(MinimumChloroplastEquivalents.Value ?? 0),
            (int)(RepopulationAmount.Value ?? 10),
            (ulong)(RepopulationCooldown.Value ?? 1_000),
            (float)(FeedingToBody.Value ?? 0),
            Daytime.IsChecked == true,
            DayNightEnabled.IsChecked == true,
            (ulong)(CycleLength.Value ?? 10_000)));

    public AdvancedSettingsWindow() : this(EnvironmentUpdate.Default)
    {
    }

    public AdvancedSettingsWindow(EnvironmentUpdate update)
    {
        InitializeComponent();
        Metabolism.Value = update.MetabolismCost;
        VegetableEnergy.Value = update.VegetableEnergyPerTick;
        Sunlight.Value = update.SunlightEnergy;
        GravityX.Value = (decimal)update.Gravity[0];
        GravityY.Value = (decimal)update.Gravity[1];
        Drag.Value = (decimal)update.Drag;
        Brownian.Value = (decimal)update.BrownianMotion;
        MaxVelocity.Value = (decimal)update.Physics.MaxVelocity;
        MovementEfficiency.Value = (decimal)update.Physics.MovementEfficiency;
        SurfaceGravity.Value = (decimal)update.Physics.SurfaceGravity;
        StaticFriction.Value = (decimal)update.Physics.StaticFriction;
        KineticFriction.Value = (decimal)update.Physics.KineticFriction;
        Density.Value = (decimal)update.Physics.Density;
        Viscosity.Value = (decimal)update.Physics.Viscosity;
        Elasticity.Value = (decimal)update.Physics.Elasticity;
        ShotSpeed.Value = (decimal)update.Shots.Speed;
        ShotRangeMultiplier.Value = (decimal)update.Shots.RangeMultiplier;
        ShotDecay.Value = (decimal)update.Shots.Decay;
        EnergyShotsNoDecay.IsChecked = update.Shots.EnergyShotsDoNotDecay;
        WasteShotsNoDecay.IsChecked = update.Shots.WasteShotsDoNotDecay;
        StartChloroplasts.Value = update.Vegetation.StartChloroplasts;
        MaxPlantEnergy.Value = update.Vegetation.MaxEnergyPerTick;
        MinimumChloroplastEquivalents.Value = update.Vegetation.MinimumChloroplastEquivalents;
        RepopulationAmount.Value = update.Vegetation.RepopulationAmount;
        RepopulationCooldown.Value = update.Vegetation.RepopulationCooldown;
        FeedingToBody.Value = (decimal)update.Vegetation.FeedingToBody;
        Daytime.IsChecked = update.Vegetation.Daytime;
        DayNightEnabled.IsChecked = update.Vegetation.DayNightEnabled;
        CycleLength.Value = update.Vegetation.CycleLength;
    }

    private void Apply_Click(object? sender, RoutedEventArgs e) { Accepted = true; Close(); }
    private void Cancel_Click(object? sender, RoutedEventArgs e) => Close();
}
