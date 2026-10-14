using Avalonia.Controls;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class GlobalSettingsWindow : Window
{
    public bool Accepted { get; private set; }
    public GlobalSettings Settings { get; private set; }

    public GlobalSettingsWindow() : this(new GlobalSettings()) { }

    public GlobalSettingsWindow(GlobalSettings settings)
    {
        Settings = settings ?? throw new ArgumentNullException(nameof(settings));
        InitializeComponent();
        Populate(settings);
    }

    private void Populate(GlobalSettings settings)
    {
        var main = settings.Main;
        FixedRatio.IsChecked = main.FixedRatio;
        OldColors.IsChecked = main.OldColors;
        SuppressBuoyancy.IsChecked = main.SuppressBuoyancy;
        VideoOff.IsChecked = main.VideoOff;
        HideTray.IsChecked = main.HideTray;
        GraphUpdates.Value = main.GraphUpdatesPerTick;
        BodyCutoff.Value = main.BodyCutoff;
        GreedyEnforcement.IsChecked = main.GreedyEnforcement;
        NewSeed.IsChecked = main.NewSeed;
        SafeMode.IsChecked = main.SafeMode;
        FindBestWeight.Value = main.EffectiveFindBestWeight;
        MainDirectory.Text = main.MainDirectory;

        var mutation = settings.AdvancedMutation;
        Point2Enabled.IsChecked = mutation.Point2Enabled;
        CopyError2Enabled.IsChecked = mutation.CopyError2Enabled;
        AmplificationEnabled.IsChecked = mutation.AmplificationEnabled;
        TranslocationEnabled.IsChecked = mutation.TranslocationEnabled;
        Delta2Enabled.IsChecked = mutation.Delta2Enabled;
        Delta2Interval.Value = mutation.Delta2Interval;
        var native = mutation.NativeDelta2;
        NativeRateExponential.Value = (decimal)native.RateExponential;
        NativeRateLinear.Value = (decimal)native.RateLinear;
        NativeDistributionExponential.Value = (decimal)native.DistributionExponential;
        NativeDistributionLinear.Value = (decimal)native.DistributionLinear;
        NativeRateChance.Value = native.RateChancePercent;
        NativeDistributionChance.Value = native.DistributionChancePercent;
        NativePreferenceStep.Value = native.PreferenceStep;
        NativeMaximumRate.Value = native.MaximumRatePerBase;
        NormalizationEnabled.IsChecked = mutation.NormalizationEnabled;
        NormalizationDnaLength.Value = mutation.NormalizationDnaLength;
        OscillationMode.ItemsSource = Enum.GetNames<GlobalMutationOscillationMode>();
        OscillationMode.SelectedIndex = (int)mutation.OscillationMode;
        OscillationPeriod.Value = mutation.OscillationPeriod;
        OscillationAmplitude.Value = (decimal)mutation.OscillationAmplitude;
        EpigeneticReset.IsChecked = mutation.EpigeneticReset;
        EpigeneticAmplification.Value = (decimal)mutation.EpigeneticAmplification;
        EpigeneticOverloadThreshold.Value = mutation.EpigeneticOverloadThreshold;
        SaveAsTemporaryGene.IsChecked = mutation.SaveAsTemporaryGene;
        var costs = settings.Costs;
        GlobalCostMultiplier.Value = (decimal)costs.GlobalMultiplier;
        DnaNumber.Value = (decimal)costs.DnaNumber;
        DnaDereference.Value = (decimal)costs.DnaDereference;
        DnaBasic.Value = (decimal)costs.DnaBasic;
        DnaAdvanced.Value = (decimal)costs.DnaAdvanced;
        DnaBitwise.Value = (decimal)costs.DnaBitwise;
        DnaCondition.Value = (decimal)costs.DnaCondition;
        DnaLogic.Value = (decimal)costs.DnaLogic;
        DnaStore.Value = (decimal)costs.DnaStore;
        DnaFlow.Value = (decimal)costs.DnaFlow;
        DnaUpkeep.Value = (decimal)costs.DnaUpkeep;
        DnaCopy.Value = (decimal)costs.DnaCopy;
        MovementCost.Value = (decimal)costs.Movement;
        RotationCost.Value = (decimal)costs.Rotation;
        TieFormationCost.Value = (decimal)costs.TieFormation;
        ShotFormationCost.Value = (decimal)costs.ShotFormation;
        VenomCost.Value = (decimal)costs.Venom;
        PoisonCost.Value = (decimal)costs.Poison;
        SlimeCost.Value = (decimal)costs.Slime;
        ShellCost.Value = (decimal)costs.Shell;
        BodyUpkeep.Value = (decimal)costs.BodyUpkeep;
        ChloroplastConstructionCost.Value = (decimal)costs.ChloroplastConstruction;
        VirusCost.Value = (decimal)costs.Virus;
        AgeBase.Value = (decimal)costs.AgeBase;
        AgeStart.Value = (decimal)costs.AgeStart;
        AgeLinear.Value = (decimal)costs.AgeLinear;
        AgeLogarithmic.Value = (decimal)costs.AgeLogarithmic;
        DynamicMultiplier.Value = (decimal)costs.DynamicMultiplier;
        DynamicAllRobots.IsChecked = costs.DynamicAllRobots;
        DynamicTargetPopulation.Value = costs.DynamicTargetPopulation;
        DynamicPopulationRange.Value = costs.DynamicPopulationRange;
        DynamicSensitivity.Value = (decimal)costs.DynamicSensitivity;
        DynamicAllowNegative.IsChecked = costs.DynamicAllowNegative;
        DynamicLowerZeroThreshold.Value = (decimal)costs.DynamicLowerZeroThreshold;
        DynamicUpperZeroThreshold.Value = (decimal)costs.DynamicUpperZeroThreshold;
        DynamicLowerReinstateThreshold.Value = (decimal)costs.DynamicLowerReinstateThreshold;
        DynamicUpperReinstateThreshold.Value = (decimal)costs.DynamicUpperReinstateThreshold;
        LeagueOfflineOwned.IsChecked = settings.League.OwnedByOfflineModes;
        DisqualificationRule.SelectedIndex = settings.League.DisqualificationRule;

        var evolution = settings.Evolution;
        EnableStagedEvolution.IsChecked = evolution.Enabled;
        EvolutionMode.ItemsSource = Enum.GetNames<EvolutionMode>();
        EvolutionMode.SelectedIndex = (int)evolution.Mode;
        SourceBot.Text = evolution.SourceBot;
        StageLength.Value = (decimal)evolution.StageLength;
        CycleLength.Value = evolution.CycleLength;
        AutoGraphs.IsChecked = evolution.AutoGraphs;
        OptionalInternetStartup.IsChecked = evolution.OptionalInternetStartup;
        NormalizeDnaSize.IsChecked = evolution.NormalizeDnaSize;
        ZerobotInitialDnaLength.Value = evolution.ZerobotInitialDnaLength;
    }

    private GlobalSettings ReadSettings() => new()
    {
        Main = new GlobalMainSettings
        {
            FixedRatio = FixedRatio.IsChecked == true,
            OldColors = OldColors.IsChecked == true,
            SuppressBuoyancy = SuppressBuoyancy.IsChecked == true,
            VideoOff = VideoOff.IsChecked == true,
            GraphUpdatesPerTick = (int)(GraphUpdates.Value ?? 1),
            HideTray = HideTray.IsChecked == true,
            BodyCutoff = (int)(BodyCutoff.Value ?? 0),
            GreedyEnforcement = GreedyEnforcement.IsChecked == true,
            NewSeed = NewSeed.IsChecked == true,
            MainDirectory = MainDirectory.Text ?? string.Empty,
            SafeMode = SafeMode.IsChecked == true,
            FindBestEnergyWeighting = (FindBestWeight.Value ?? 0) > 0,
            FindBestWeight = (int)(FindBestWeight.Value ?? 0),
        },
        AdvancedMutation = new GlobalAdvancedMutationSettings
        {
            Point2Enabled = Point2Enabled.IsChecked == true,
            CopyError2Enabled = CopyError2Enabled.IsChecked == true,
            AmplificationEnabled = AmplificationEnabled.IsChecked == true,
            TranslocationEnabled = TranslocationEnabled.IsChecked == true,
            Delta2Enabled = Delta2Enabled.IsChecked == true,
            Delta2Interval = (ulong)(Delta2Interval.Value ?? 3000),
            Delta2RateStep = Settings.AdvancedMutation.Delta2RateStep,
            Delta2DistributionStep = Settings.AdvancedMutation.Delta2DistributionStep,
            Delta2PreferenceStepPercent = Settings.AdvancedMutation.Delta2PreferenceStepPercent,
            NativeDelta2 = new NativeDelta2Settings {
                RateExponential = (double)(NativeRateExponential.Value ?? 1), RateLinear = (double)(NativeRateLinear.Value ?? 0),
                DistributionExponential = (double)(NativeDistributionExponential.Value ?? 7),
                DistributionLinear = (double)(NativeDistributionLinear.Value ?? 1),
                RateChancePercent = (byte)(NativeRateChance.Value ?? 100),
                DistributionChancePercent = (byte)(NativeDistributionChance.Value ?? 30),
                PreferenceStep = (byte)(NativePreferenceStep.Value ?? 15),
                MaximumRatePerBase = (uint)(NativeMaximumRate.Value ?? 1071),
            },
            NormalizationEnabled = NormalizationEnabled.IsChecked == true,
            NormalizationDnaLength = (int)(NormalizationDnaLength.Value ?? 1071),
            OscillationMode = (GlobalMutationOscillationMode)(OscillationMode.SelectedIndex < 0 ? 0 : OscillationMode.SelectedIndex),
            OscillationPeriod = (ulong)(OscillationPeriod.Value ?? 0),
            OscillationAmplitude = (double)(OscillationAmplitude.Value ?? 0),
            EpigeneticReset = EpigeneticReset.IsChecked == true,
            EpigeneticAmplification = (double)(EpigeneticAmplification.Value ?? 1),
            EpigeneticOverloadThreshold = (ulong)(EpigeneticOverloadThreshold.Value ?? 32),
            SaveAsTemporaryGene = SaveAsTemporaryGene.IsChecked == true,
        },
        Costs = new GlobalSimulationCostSettings
        {
            GlobalMultiplier = (double)(GlobalCostMultiplier.Value ?? 1),
            DnaNumber = (double)(DnaNumber.Value ?? 0),
            DnaDereference = (double)(DnaDereference.Value ?? 0),
            DnaBasic = (double)(DnaBasic.Value ?? 0),
            DnaAdvanced = (double)(DnaAdvanced.Value ?? 0),
            DnaBitwise = (double)(DnaBitwise.Value ?? 0),
            DnaCondition = (double)(DnaCondition.Value ?? 0),
            DnaLogic = (double)(DnaLogic.Value ?? 0),
            DnaStore = (double)(DnaStore.Value ?? 0),
            DnaFlow = (double)(DnaFlow.Value ?? 0),
            DnaUpkeep = (double)(DnaUpkeep.Value ?? 0),
            DnaCopy = (double)(DnaCopy.Value ?? 0),
            Movement = (double)(MovementCost.Value ?? 0),
            Rotation = (double)(RotationCost.Value ?? 0),
            TieFormation = (double)(TieFormationCost.Value ?? 0),
            ShotFormation = (double)(ShotFormationCost.Value ?? 0),
            Venom = (double)(VenomCost.Value ?? 0),
            Poison = (double)(PoisonCost.Value ?? 0),
            Slime = (double)(SlimeCost.Value ?? 0),
            Shell = (double)(ShellCost.Value ?? 0),
            BodyUpkeep = (double)(BodyUpkeep.Value ?? 0),
            ChloroplastConstruction = (double)(ChloroplastConstructionCost.Value ?? 0),
            Virus = (double)(VirusCost.Value ?? 0),
            AgeBase = (double)(AgeBase.Value ?? 0),
            AgeStart = (double)(AgeStart.Value ?? 0),
            AgeLinear = (double)(AgeLinear.Value ?? 0),
            AgeLogarithmic = (double)(AgeLogarithmic.Value ?? 0),
            DynamicMultiplier = (double)(DynamicMultiplier.Value ?? 0),
            DynamicAllRobots = DynamicAllRobots.IsChecked == true,
            DynamicTargetPopulation = (int)(DynamicTargetPopulation.Value ?? 0),
            DynamicPopulationRange = (int)(DynamicPopulationRange.Value ?? 0),
            DynamicSensitivity = (double)(DynamicSensitivity.Value ?? 1),
            DynamicAllowNegative = DynamicAllowNegative.IsChecked == true,
            DynamicLowerZeroThreshold = (double)(DynamicLowerZeroThreshold.Value ?? 0),
            DynamicUpperZeroThreshold = (double)(DynamicUpperZeroThreshold.Value ?? 0),
            DynamicLowerReinstateThreshold = (double)(DynamicLowerReinstateThreshold.Value ?? 0),
            DynamicUpperReinstateThreshold = (double)(DynamicUpperReinstateThreshold.Value ?? 0),
        },
        League = new GlobalLeagueSettings { OwnedByOfflineModes = true, DisqualificationRule = Math.Max(0, DisqualificationRule.SelectedIndex) },
        Evolution = new GlobalEvolutionSettings
        {
            Enabled = EnableStagedEvolution.IsChecked == true,
            Mode = (EvolutionMode)(EvolutionMode.SelectedIndex < 0 ? 0 : EvolutionMode.SelectedIndex),
            SourceBot = SourceBot.Text ?? string.Empty,
            StageLength = (double)(StageLength.Value ?? 10),
            CycleLength = (int)(CycleLength.Value ?? 100),
            ReducePopulationOscillation = Settings.Evolution.ReducePopulationOscillation,
            AutoGraphs = AutoGraphs.IsChecked == true,
            OptionalInternetStartup = OptionalInternetStartup.IsChecked == true,
            NormalizeDnaSize = NormalizeDnaSize.IsChecked == true,
            ZerobotInitialDnaLength = (int)(ZerobotInitialDnaLength.Value ?? 32_000),
            RestrictMutation = Settings.Evolution.RestrictMutation,
            RestrictSpeciesMixing = Settings.Evolution.RestrictSpeciesMixing,
        },
    };

    private void Apply_Click(object? sender, RoutedEventArgs e)
    {
        Settings = ReadSettings();
        Accepted = true;
        Close();
    }

    private async void CustomSimulationCosts_Click(object? sender, RoutedEventArgs e)
    {
        var dialog = new CustomSimulationCostsWindow(Settings);
        await dialog.ShowDialog(this);
        if (!dialog.Accepted) return;

        Settings = Settings with { Costs = dialog.Costs };
        Populate(Settings);
    }

    private void Cancel_Click(object? sender, RoutedEventArgs e) => Close();
}
