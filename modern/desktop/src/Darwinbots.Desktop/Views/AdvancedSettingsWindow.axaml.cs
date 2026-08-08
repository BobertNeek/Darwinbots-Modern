using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.LogicalTree;
using Avalonia.Media;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class AdvancedSettingsWindow : Window
{
    private readonly List<(uint Species, Slider Slider)> _speciesRates = [];
    private bool _initializing = true;

    public bool Accepted { get; private set; }
    public MutationSettingsUpdate MutationUpdate => new(MutationsEnabled.IsChecked == true, (float)(GlobalMutationRate.Value ?? 1m), _speciesRates.Select(value => new SpeciesMutationRateUpdate(value.Species, (float)value.Slider.Value)).ToArray());
    public EnvironmentUpdate Update => new(
        (int)(Metabolism.Value ?? 1), (int)(VegetableEnergy.Value ?? 0), (int)(Sunlight.Value ?? 100), [(float)(GravityX.Value ?? 0), (float)(GravityY.Value ?? 0)], (float)(Drag.Value ?? 0), Db2BrownianScale.FromPercent(Brownian.Value ?? 0),
        new Db2PhysicsOptions((float)(MaxVelocity.Value ?? 180), (float)(MovementEfficiency.Value ?? 0.66m), (float)(SurfaceGravity.Value ?? 2), (float)(StaticFriction.Value ?? 0.6m), (float)(KineticFriction.Value ?? 0.4m), (double)(Density.Value ?? 0), (double)(Viscosity.Value ?? 0), (float)(Elasticity.Value ?? 0m)),
        new Db2ShotOptions((float)(ShotSpeed.Value ?? 40), (float)(ShotRangeMultiplier.Value ?? 1), (float)(ShotDecay.Value ?? 40), EnergyShotsNoDecay.IsChecked == true, WasteShotsNoDecay.IsChecked == true),
        new Db2VegetationOptions((int)(StartChloroplasts.Value ?? 16_000), (int)(MaxPlantEnergy.Value ?? 40), (int)(MinimumChloroplastEquivalents.Value ?? 10), (int)(RepopulationAmount.Value ?? 10), (ulong)(RepopulationCooldown.Value ?? 25), (float)(FeedingToBody.Value ?? 0.5m), Daytime.IsChecked == true, DayNightEnabled.IsChecked == true, (ulong)(CycleLength.Value ?? 10_000), PondMode.IsChecked == true, (float)(PondLightIntensity.Value ?? 100m), (float)(PondGradient.Value ?? 1m), (uint)(TidePeriod.Value ?? 0), (int)(TideOffset.Value ?? 0)),
        AutoSpeciation.IsChecked == true, (float)(SpeciationDistance.Value ?? 20), ToroidalWorld.IsChecked == true);

    public AdvancedSettingsWindow() : this(EnvironmentUpdate.Default, EngineSnapshot.Empty) { }
    public AdvancedSettingsWindow(EnvironmentUpdate update, EngineSnapshot? snapshot = null)
    {
        InitializeComponent();
        PopulateEnvironment(update);
        PopulateMutations(snapshot ?? EngineSnapshot.Empty);
        WireDirtyTracking();
        _initializing = false;
    }

    private void PopulateEnvironment(EnvironmentUpdate update)
    {
        Metabolism.Value = update.MetabolismCost; VegetableEnergy.Value = update.VegetableEnergyPerTick; Sunlight.Value = update.SunlightEnergy;
        GravityX.Value = (decimal)update.Gravity[0]; GravityY.Value = (decimal)update.Gravity[1]; Drag.Value = (decimal)update.Drag; Brownian.Value = Db2BrownianScale.ToPercent(update.BrownianMotion);
        MaxVelocity.Value = (decimal)update.Physics.MaxVelocity; MovementEfficiency.Value = (decimal)update.Physics.MovementEfficiency; SurfaceGravity.Value = (decimal)update.Physics.SurfaceGravity; StaticFriction.Value = (decimal)update.Physics.StaticFriction; KineticFriction.Value = (decimal)update.Physics.KineticFriction; Density.Value = (decimal)update.Physics.Density; Viscosity.Value = (decimal)update.Physics.Viscosity; Elasticity.Value = (decimal)update.Physics.Elasticity;
        ShotSpeed.Value = (decimal)update.Shots.Speed; ShotRangeMultiplier.Value = (decimal)update.Shots.RangeMultiplier; ShotDecay.Value = (decimal)update.Shots.Decay; EnergyShotsNoDecay.IsChecked = update.Shots.EnergyShotsDoNotDecay; WasteShotsNoDecay.IsChecked = update.Shots.WasteShotsDoNotDecay;
        StartChloroplasts.Value = update.Vegetation.StartChloroplasts; MaxPlantEnergy.Value = update.Vegetation.MaxEnergyPerTick; MinimumChloroplastEquivalents.Value = update.Vegetation.MinimumChloroplastEquivalents; RepopulationAmount.Value = update.Vegetation.RepopulationAmount; RepopulationCooldown.Value = update.Vegetation.RepopulationCooldown; FeedingToBody.Value = (decimal)update.Vegetation.FeedingToBody; Daytime.IsChecked = update.Vegetation.Daytime; DayNightEnabled.IsChecked = update.Vegetation.DayNightEnabled; CycleLength.Value = update.Vegetation.CycleLength; PondMode.IsChecked = update.Vegetation.PondMode; PondLightIntensity.Value = (decimal)update.Vegetation.LightIntensity; PondGradient.Value = (decimal)update.Vegetation.Gradient; TidePeriod.Value = update.Vegetation.Tides; TideOffset.Value = update.Vegetation.TidesOffset;
        AutoSpeciation.IsChecked = update.AutoSpeciation; SpeciationDistance.Value = (decimal)update.SpeciationGeneticDistancePercent; ToroidalWorld.IsChecked = update.ToroidalWorld;
    }

    private void PopulateMutations(EngineSnapshot snapshot)
    {
        MutationsEnabled.IsChecked = snapshot.MutationSettings.Enabled;
        GlobalMutationSlider.Value = snapshot.MutationSettings.GlobalMultiplier;
        GlobalMutationRate.Value = (decimal)snapshot.MutationSettings.GlobalMultiplier;
        GlobalMutationSlider.PropertyChanged += (_, e) => { if (e.Property == Slider.ValueProperty) GlobalMutationRate.Value = (decimal)GlobalMutationSlider.Value; };
        GlobalMutationRate.ValueChanged += (_, _) => GlobalMutationSlider.Value = (double)(GlobalMutationRate.Value ?? 1m);
        foreach (var entry in snapshot.Species.Select((species, index) => (species, index)).Where(value => value.index > 0)) AddSpeciesRow((uint)entry.index, entry.species);
        if (_speciesRates.Count == 0) SpeciesMutationRows.Children.Add(new TextBlock { Text = "No imported species yet.", Foreground = Brushes.DimGray });
    }

    private void AddSpeciesRow(uint index, SpeciesSnapshot species)
    {
        var slider = new Slider { Minimum = 0, Maximum = 100, TickFrequency = 0.1, Value = species.MutationRate };
        var numeric = new NumericUpDown { Minimum = 0, Maximum = 100, Increment = 0.1m, FormatString = "0.0'%'", Value = (decimal)species.MutationRate };
        slider.PropertyChanged += (_, e) => { if (e.Property == Slider.ValueProperty) numeric.Value = (decimal)slider.Value; };
        numeric.ValueChanged += (_, _) => slider.Value = (double)(numeric.Value ?? 0m);
        var label = new StackPanel(); label.Children.Add(new TextBlock { Text = species.Name, FontWeight = FontWeight.SemiBold }); label.Children.Add(new TextBlock { Text = species.Vegetable ? "AUTOTROPH" : "HETEROTROPH", FontSize = 10, Foreground = Brushes.DimGray });
        var grid = new Grid { ColumnDefinitions = new ColumnDefinitions("180,*,118"), ColumnSpacing = 12 }; grid.Children.Add(label); Grid.SetColumn(slider, 1); grid.Children.Add(slider); Grid.SetColumn(numeric, 2); grid.Children.Add(numeric);
        SpeciesMutationRows.Children.Add(grid); _speciesRates.Add((index, slider));
    }

    private void WireDirtyTracking()
    {
        foreach (var numeric in this.GetLogicalDescendants().OfType<NumericUpDown>()) numeric.ValueChanged += (_, _) => MarkDirty();
        foreach (var checkBox in this.GetLogicalDescendants().OfType<CheckBox>()) checkBox.IsCheckedChanged += (_, _) => MarkDirty();
        foreach (var slider in this.GetLogicalDescendants().OfType<Slider>()) slider.PropertyChanged += (_, e) => { if (e.Property == Slider.ValueProperty) MarkDirty(); };
    }

    private void MarkDirty() { if (!_initializing) DirtyBadge.IsVisible = true; }
    private void Apply_Click(object? sender, RoutedEventArgs e) { Accepted = true; Close(); }
    private void Cancel_Click(object? sender, RoutedEventArgs e) => Close();
}
