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
    private bool _updatingTopology;

    public bool Accepted { get; private set; }
    public MutationSettingsUpdate MutationUpdate => new(
        MutationsEnabled.IsChecked == true,
        (float)(GlobalMutationRate.Value ?? 1m),
        _speciesRates.Select(value => new SpeciesMutationRateUpdate(value.Species, (float)value.Slider.Value)).ToArray())
    {
        Profile = ReadMutationProfile(),
    };
    public EnvironmentUpdate Update => new(
        (int)(Metabolism.Value ?? 1), (int)(VegetableEnergy.Value ?? 0), (int)(Sunlight.Value ?? 100), [(float)(GravityX.Value ?? 0), (float)(GravityY.Value ?? 0)], (float)(Drag.Value ?? 0), Db2BrownianScale.FromPercent(Brownian.Value ?? 0),
        new Db2PhysicsOptions((float)(MaxVelocity.Value ?? 180), (float)(MovementEfficiency.Value ?? 0.66m), (float)(SurfaceGravity.Value ?? 2), (float)(StaticFriction.Value ?? 0.6m), (float)(KineticFriction.Value ?? 0.4m), (double)(Density.Value ?? 0), (double)(Viscosity.Value ?? 0), (float)(Elasticity.Value ?? 0m), PlanetEaters.IsChecked == true, (float)(PlanetEatersG.Value ?? 0m)),
        new Db2ShotOptions((float)(ShotSpeed.Value ?? 40), (float)(ShotRangeMultiplier.Value ?? 1), (float)(ShotDecay.Value ?? 40), EnergyShotsNoDecay.IsChecked == true, WasteShotsNoDecay.IsChecked == true),
        new Db2VegetationOptions((int)(StartChloroplasts.Value ?? 16_000), (int)(MaxPlantEnergy.Value ?? 40), (int)(MinimumChloroplastEquivalents.Value ?? 10), (int)(RepopulationAmount.Value ?? 10), (ulong)(RepopulationCooldown.Value ?? 25), (float)(FeedingToBody.Value ?? 0.5m), Daytime.IsChecked == true, DayNightEnabled.IsChecked == true, (ulong)(CycleLength.Value ?? 10_000), PondMode.IsChecked == true, (float)(PondLightIntensity.Value ?? 100m), (float)(PondGradient.Value ?? 1m), (uint)(TidePeriod.Value ?? 0), (int)(TideOffset.Value ?? 0), WeatherEnabled.IsChecked == true, SunUpEnabled.IsChecked == true, (int)(SunUpThreshold.Value ?? 500_000), SunDownEnabled.IsChecked == true, (int)(SunDownThreshold.Value ?? 1_000_000), SunThresholdMode.SelectedIndex),
        AutoSpeciation.IsChecked == true,
        (float)(SpeciationDistance.Value ?? 20),
        ToroidalWorld.IsChecked == true,
        WrapX.IsChecked == true,
        WrapY.IsChecked == true,
        new Db2CorpseDecayOptions(
            CorpseEnabled.IsChecked == true,
            (int)(CorpseDeathEnergyThreshold.Value ?? 15),
            (Db2CorpseDecayMode)Math.Max(0, CorpseMode.SelectedIndex),
            (int)(CorpsePacketSize.Value ?? 1),
            (ulong)(CorpsePeriod.Value ?? 10)),
        new Db2WasteOptions(
            (int)(WasteThreshold.Value ?? 400),
            (int)(DefecationThreshold.Value ?? 32_000)),
        new Db2RecordingOptions(
            SafeModeBackup.IsChecked == true,
            DeadSnapshotEnabled.IsChecked == true,
            ExcludeRepopulatingRobots.IsChecked == true,
            (ulong)(AutosaveInterval.Value ?? 0),
            (int)(SamplingInterval.Value ?? 10)));

    public AdvancedSettingsWindow() : this(EnvironmentUpdate.Default, EngineSnapshot.Empty) { }
    public AdvancedSettingsWindow(EnvironmentUpdate update, EngineSnapshot? snapshot = null)
    {
        InitializeComponent();
        ToroidalWorld.IsCheckedChanged += (_, _) =>
        {
            if (_initializing || _updatingTopology || ToroidalWorld.IsChecked is null) return;
            _updatingTopology = true;
            WrapX.IsChecked = ToroidalWorld.IsChecked;
            WrapY.IsChecked = ToroidalWorld.IsChecked;
            _updatingTopology = false;
        };
        WrapX.IsCheckedChanged += (_, _) =>
        {
            if (_initializing || _updatingTopology) return;
            _updatingTopology = true;
            ToroidalWorld.IsChecked = WrapX.IsChecked == true && WrapY.IsChecked == true;
            _updatingTopology = false;
        };
        WrapY.IsCheckedChanged += (_, _) =>
        {
            if (_initializing || _updatingTopology) return;
            _updatingTopology = true;
            ToroidalWorld.IsChecked = WrapX.IsChecked == true && WrapY.IsChecked == true;
            _updatingTopology = false;
        };
        PhysicsPreset.SelectionChanged += (_, _) => ApplyPhysicsPreset();
        Density.ValueChanged += (_, _) => UpdateReynoldsDisplay();
        Viscosity.ValueChanged += (_, _) => UpdateReynoldsDisplay();
        PopulateEnvironment(update);
        PopulateMutations(snapshot ?? EngineSnapshot.Empty);
        WireDirtyTracking();
        _initializing = false;
    }

    private void PopulateEnvironment(EnvironmentUpdate update)
    {
        Metabolism.Value = update.MetabolismCost; VegetableEnergy.Value = update.VegetableEnergyPerTick; Sunlight.Value = update.SunlightEnergy;
        GravityX.Value = (decimal)update.Gravity[0]; GravityY.Value = (decimal)update.Gravity[1]; Drag.Value = (decimal)update.Drag; Brownian.Value = Db2BrownianScale.ToPercent(update.BrownianMotion);
        PopulatePhysics(update.Physics);
        ShotSpeed.Value = (decimal)update.Shots.Speed; ShotRangeMultiplier.Value = (decimal)update.Shots.RangeMultiplier; ShotDecay.Value = (decimal)update.Shots.Decay; EnergyShotsNoDecay.IsChecked = update.Shots.EnergyShotsDoNotDecay; WasteShotsNoDecay.IsChecked = update.Shots.WasteShotsDoNotDecay;
        StartChloroplasts.Value = update.Vegetation.StartChloroplasts; MaxPlantEnergy.Value = update.Vegetation.MaxEnergyPerTick; MinimumChloroplastEquivalents.Value = update.Vegetation.MinimumChloroplastEquivalents; RepopulationAmount.Value = update.Vegetation.RepopulationAmount; RepopulationCooldown.Value = update.Vegetation.RepopulationCooldown; FeedingToBody.Value = (decimal)update.Vegetation.FeedingToBody; Daytime.IsChecked = update.Vegetation.Daytime; DayNightEnabled.IsChecked = update.Vegetation.DayNightEnabled; CycleLength.Value = update.Vegetation.CycleLength; PondMode.IsChecked = update.Vegetation.PondMode; PondLightIntensity.Value = (decimal)update.Vegetation.LightIntensity; PondGradient.Value = (decimal)update.Vegetation.Gradient; TidePeriod.Value = update.Vegetation.Tides; TideOffset.Value = update.Vegetation.TidesOffset; WeatherEnabled.IsChecked = update.Vegetation.WeatherEnabled; SunUpEnabled.IsChecked = update.Vegetation.SunUpEnabled; SunUpThreshold.Value = update.Vegetation.SunUpThreshold; SunDownEnabled.IsChecked = update.Vegetation.SunDownEnabled; SunDownThreshold.Value = update.Vegetation.SunDownThreshold; SunThresholdMode.SelectedIndex = update.Vegetation.SunThresholdMode;
        var corpse = update.CorpseDecay ?? new Db2CorpseDecayOptions();
        CorpseEnabled.IsChecked = corpse.Enabled; CorpseDeathEnergyThreshold.Value = corpse.DeathEnergyThreshold; CorpseMode.SelectedIndex = (int)corpse.Mode; CorpsePacketSize.Value = corpse.PacketSize; CorpsePeriod.Value = corpse.Period;
        var waste = update.Waste ?? new Db2WasteOptions();
        WasteThreshold.Value = waste.AlzheimerThreshold; DefecationThreshold.Value = waste.DefecationThreshold;
        var recording = update.Recording ?? new Db2RecordingOptions();
        SafeModeBackup.IsChecked = recording.SafeModeBackup; DeadSnapshotEnabled.IsChecked = recording.DeadSnapshotEnabled; ExcludeRepopulatingRobots.IsChecked = recording.ExcludeRepopulatingRobots; AutosaveInterval.Value = recording.AutosaveInterval; SamplingInterval.Value = recording.SamplingInterval;
        AutoSpeciation.IsChecked = update.AutoSpeciation; SpeciationDistance.Value = (decimal)update.SpeciationGeneticDistancePercent;
        WrapX.IsChecked = update.WrapX; WrapY.IsChecked = update.WrapY; ToroidalWorld.IsChecked = update.ToroidalWorld;
        UpdateReynoldsDisplay();
    }

    private void PopulatePhysics(Db2PhysicsOptions physics)
    {
        MaxVelocity.Value = (decimal)physics.MaxVelocity;
        MovementEfficiency.Value = (decimal)physics.MovementEfficiency;
        SurfaceGravity.Value = (decimal)physics.SurfaceGravity;
        StaticFriction.Value = (decimal)physics.StaticFriction;
        KineticFriction.Value = (decimal)physics.KineticFriction;
        Density.Value = (decimal)physics.Density;
        Viscosity.Value = (decimal)physics.Viscosity;
        Elasticity.Value = (decimal)physics.Elasticity;
        PlanetEaters.IsChecked = physics.PlanetEaters;
        PlanetEatersG.Value = (decimal)physics.PlanetEatersG;
        PhysicsPreset.SelectedIndex = physics == Db2PhysicsOptions.ForPreset(Db2PhysicsPreset.Fluid)
            ? (int)Db2PhysicsPreset.Fluid
            : physics == Db2PhysicsOptions.ForPreset(Db2PhysicsPreset.Solid)
                ? (int)Db2PhysicsPreset.Solid
                : (int)Db2PhysicsPreset.Custom;
    }

    private void ApplyPhysicsPreset()
    {
        if (_initializing || PhysicsPreset.SelectedIndex is < 0 or > 1) return;
        PopulatePhysics(Db2PhysicsOptions.ForPreset((Db2PhysicsPreset)PhysicsPreset.SelectedIndex));
    }

    private void UpdateReynoldsDisplay()
    {
        var physics = Db2PhysicsOptions.Default with
        {
            Density = (double)(Density.Value ?? 0m),
            Viscosity = (double)(Viscosity.Value ?? 0m),
        };
        ReynoldsDisplay.Text = $"{physics.ReynoldsNumber:0.###E+0} · {physics.ReynoldsDescription}";
    }

    private void PhysicsHelp_Click(object? sender, RoutedEventArgs e)
    {
        PhysicsHelpText.Text = $"{Db2PhysicsOptions.Default.ContextualHelp} " +
            "Fluid viscosity affects laminar drag. Density affects added mass and turbulent drag. " +
            "Solid Z gravity and friction act on the floor plane.";
    }

    private void PopulateMutations(EngineSnapshot snapshot)
    {
        MutationsEnabled.IsChecked = snapshot.MutationSettings.Enabled;
        GlobalMutationSlider.Value = snapshot.MutationSettings.GlobalMultiplier;
        GlobalMutationRate.Value = (decimal)snapshot.MutationSettings.GlobalMultiplier;
        PopulateMutationProfile(snapshot.MutationSettings.Profile);
        GlobalMutationSlider.PropertyChanged += (_, e) => { if (e.Property == Slider.ValueProperty) GlobalMutationRate.Value = (decimal)GlobalMutationSlider.Value; };
        GlobalMutationRate.ValueChanged += (_, _) => GlobalMutationSlider.Value = (double)(GlobalMutationRate.Value ?? 1m);
        foreach (var entry in snapshot.Species.Select((species, index) => (species, index)).Where(value => value.index > 0)) AddSpeciesRow((uint)entry.index, entry.species);
        if (_speciesRates.Count == 0) SpeciesMutationRows.Children.Add(new TextBlock { Text = "No imported species yet.", Foreground = Brushes.DimGray });
    }

    private void PopulateMutationProfile(MutationProfileUpdate profile)
    {
        PointOneIn.Value = (decimal)profile.Point.OneIn;
        Point2OneIn.Value = (decimal)profile.Point2.OneIn;
        DeltaOneIn.Value = (decimal)profile.Delta.OneIn;
        CopyErrorOneIn.Value = (decimal)profile.CopyError.OneIn;
        CopyError2OneIn.Value = (decimal)profile.CopyError2.OneIn;
        InsertionOneIn.Value = (decimal)profile.Insertion.OneIn;
        ReversalOneIn.Value = (decimal)profile.Reversal.OneIn;
        MinorDeletionOneIn.Value = (decimal)profile.MinorDeletion.OneIn;
        MajorDeletionOneIn.Value = (decimal)profile.MajorDeletion.OneIn;
        AmplificationOneIn.Value = (decimal)profile.Amplification.OneIn;
        TranslocationOneIn.Value = (decimal)profile.Translocation.OneIn;
    }

    private MutationProfileUpdate ReadMutationProfile()
    {
        var profile = MutationProfileUpdate.Default;
        return profile with
        {
            Point = ReadRate(PointOneIn, profile.Point),
            Point2 = ReadRate(Point2OneIn, profile.Point2),
            Delta = ReadRate(DeltaOneIn, profile.Delta),
            CopyError = ReadRate(CopyErrorOneIn, profile.CopyError),
            CopyError2 = ReadRate(CopyError2OneIn, profile.CopyError2),
            Insertion = ReadRate(InsertionOneIn, profile.Insertion),
            Reversal = ReadRate(ReversalOneIn, profile.Reversal),
            MinorDeletion = ReadRate(MinorDeletionOneIn, profile.MinorDeletion),
            MajorDeletion = ReadRate(MajorDeletionOneIn, profile.MajorDeletion),
            Amplification = ReadRate(AmplificationOneIn, profile.Amplification),
            Translocation = ReadRate(TranslocationOneIn, profile.Translocation),
        };
    }

    private static MutationRateUpdate ReadRate(NumericUpDown control, MutationRateUpdate current) =>
        current with { OneIn = (double)(control.Value ?? (decimal)current.OneIn) };

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
