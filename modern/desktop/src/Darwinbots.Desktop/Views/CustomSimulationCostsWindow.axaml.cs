using Avalonia.Controls;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class CustomSimulationCostsWindow : Window
{
    public bool Accepted { get; private set; }
    public GlobalSimulationCostSettings Costs { get; private set; }

    private GlobalSimulationCostSettings _draftCosts;

    public CustomSimulationCostsWindow() : this(new GlobalSettings()) { }

    public CustomSimulationCostsWindow(GlobalSettings settings)
        : this(settings?.Costs ?? throw new ArgumentNullException(nameof(settings)), stageForLiveSettings: false) { }

    public CustomSimulationCostsWindow(GlobalSimulationCostSettings costs, bool stageForLiveSettings)
        : this(costs, stageForLiveSettings, nativeCosts: false) { }

    internal CustomSimulationCostsWindow(GlobalSimulationCostSettings costs, bool stageForLiveSettings, bool nativeCosts)
    {
        Costs = costs ?? throw new ArgumentNullException(nameof(costs));
        _draftCosts = Costs;
        InitializeComponent();
        if (nativeCosts) GlobalCostMultiplier.Minimum = -GlobalCostMultiplier.Maximum;
        Populate(Costs);
        if (stageForLiveSettings)
        {
            CostScope.Text = "Edit costs for the running simulation";
            CostDescription.Text = "Zero disables a cost.";
            CostApplyNotice.Text = "Apply Physics and Costs to use these values.";
            ApplyButton.Content = "STAGE COSTS";
        }
    }

    private void Populate(GlobalSimulationCostSettings costs)
    {
        SetDisplayValue(GlobalCostMultiplier, costs.GlobalMultiplier);
        SetDisplayValue(DnaNumber, costs.DnaNumber);
        SetDisplayValue(DnaDereference, costs.DnaDereference);
        SetDisplayValue(DnaBasic, costs.DnaBasic);
        SetDisplayValue(DnaAdvanced, costs.DnaAdvanced);
        SetDisplayValue(DnaBitwise, costs.DnaBitwise);
        SetDisplayValue(DnaCondition, costs.DnaCondition);
        SetDisplayValue(DnaLogic, costs.DnaLogic);
        SetDisplayValue(DnaStore, costs.DnaStore);
        SetDisplayValue(DnaFlow, costs.DnaFlow);
        SetDisplayValue(DnaUpkeep, costs.DnaUpkeep);
        SetDisplayValue(DnaCopy, costs.DnaCopy);
        SetDisplayValue(MovementCost, costs.Movement);
        SetDisplayValue(RotationCost, costs.Rotation);
        SetDisplayValue(TieFormationCost, costs.TieFormation);
        SetDisplayValue(ShotFormationCost, costs.ShotFormation);
        SetDisplayValue(VenomCost, costs.Venom);
        SetDisplayValue(PoisonCost, costs.Poison);
        SetDisplayValue(SlimeCost, costs.Slime);
        SetDisplayValue(ShellCost, costs.Shell);
        SetDisplayValue(BodyUpkeep, costs.BodyUpkeep);
        SetDisplayValue(ChloroplastConstructionCost, costs.ChloroplastConstruction);
        SetDisplayValue(VirusCost, costs.Virus);
        SetDisplayValue(AgeBase, costs.AgeBase);
        SetDisplayValue(AgeStart, costs.AgeStart);
        SetDisplayValue(AgeLinear, costs.AgeLinear);
        AgeLinearEnabled.IsChecked = costs.EffectiveAgeLinearEnabled;
        SetDisplayValue(AgeLogarithmic, costs.AgeLogarithmic);
        SetDisplayValue(DynamicMultiplier, costs.DynamicMultiplier);
        DynamicAllRobots.IsChecked = costs.DynamicAllRobots;
        SetDisplayValue(DynamicTargetPopulation, costs.DynamicTargetPopulation);
        SetDisplayValue(DynamicUpperRangePercent, costs.EffectiveDynamicUpperRangePercent);
        SetDisplayValue(DynamicLowerRangePercent, costs.EffectiveDynamicLowerRangePercent);
        SetDisplayValue(DynamicSensitivity, costs.DynamicSensitivity);
        DynamicAllowNegative.IsChecked = costs.DynamicAllowNegative;
        SetDisplayValue(DynamicLowerZeroThreshold, costs.DynamicLowerZeroThreshold);
        SetDisplayValue(DynamicLowerReinstateThreshold, costs.DynamicLowerReinstateThreshold);
    }

    private GlobalSimulationCostSettings ReadCosts()
    {
        var costs = _draftCosts;
        var ageLinear = Read(AgeLinear, costs.AgeLinear);
        return costs with
        {
            GlobalMultiplier = Read(GlobalCostMultiplier, costs.GlobalMultiplier),
            DnaNumber = Read(DnaNumber, costs.DnaNumber),
            DnaDereference = Read(DnaDereference, costs.DnaDereference),
            DnaBasic = Read(DnaBasic, costs.DnaBasic),
            DnaAdvanced = Read(DnaAdvanced, costs.DnaAdvanced),
            DnaBitwise = Read(DnaBitwise, costs.DnaBitwise),
            DnaCondition = Read(DnaCondition, costs.DnaCondition),
            DnaLogic = Read(DnaLogic, costs.DnaLogic),
            DnaStore = Read(DnaStore, costs.DnaStore),
            DnaFlow = Read(DnaFlow, costs.DnaFlow),
            DnaUpkeep = Read(DnaUpkeep, costs.DnaUpkeep),
            DnaCopy = Read(DnaCopy, costs.DnaCopy),
            Movement = Read(MovementCost, costs.Movement),
            Rotation = Read(RotationCost, costs.Rotation),
            TieFormation = Read(TieFormationCost, costs.TieFormation),
            ShotFormation = Read(ShotFormationCost, costs.ShotFormation),
            Venom = Read(VenomCost, costs.Venom),
            Poison = Read(PoisonCost, costs.Poison),
            Slime = Read(SlimeCost, costs.Slime),
            Shell = Read(ShellCost, costs.Shell),
            BodyUpkeep = Read(BodyUpkeep, costs.BodyUpkeep),
            ChloroplastConstruction = Read(ChloroplastConstructionCost, costs.ChloroplastConstruction),
            Virus = Read(VirusCost, costs.Virus),
            AgeBase = Read(AgeBase, costs.AgeBase),
            AgeStart = Read(AgeStart, costs.AgeStart),
            AgeLinear = ageLinear,
            AgeLinearEnabled = ReadAgeLinearEnabled(AgeLinearEnabled, costs.AgeLinearEnabled, ageLinear),
            AgeLogarithmic = Read(AgeLogarithmic, costs.AgeLogarithmic),
            DynamicMultiplier = Read(DynamicMultiplier, costs.DynamicMultiplier),
            DynamicAllRobots = DynamicAllRobots.IsChecked == true,
            DynamicTargetPopulation = (int)Read(DynamicTargetPopulation, costs.DynamicTargetPopulation),
            DynamicUpperRangePercent = ReadRangePercent(DynamicUpperRangePercent,
                costs.DynamicUpperRangePercent, costs.EffectiveDynamicUpperRangePercent),
            DynamicLowerRangePercent = ReadRangePercent(DynamicLowerRangePercent,
                costs.DynamicLowerRangePercent, costs.EffectiveDynamicLowerRangePercent),
            DynamicSensitivity = Read(DynamicSensitivity, costs.DynamicSensitivity),
            DynamicAllowNegative = DynamicAllowNegative.IsChecked == true,
            DynamicLowerZeroThreshold = Read(DynamicLowerZeroThreshold, costs.DynamicLowerZeroThreshold),
            DynamicLowerReinstateThreshold = Read(DynamicLowerReinstateThreshold, costs.DynamicLowerReinstateThreshold),
        };
    }

    private static decimal DisplayValue(NumericUpDown control, double value) =>
        (decimal)Math.Clamp(value, (double)control.Minimum, (double)control.Maximum);

    internal static void SetDisplayValue(NumericUpDown control, double value) =>
        control.Value = DisplayValue(control, value);

    private static double Read(NumericUpDown control, double previous) =>
        control.Value is { } value && value != DisplayValue(control, previous) ? (double)value : previous;

    internal static double? ReadRangePercent(NumericUpDown control, double? explicitValue, double effectiveValue) =>
        control.Value is { } value && value != DisplayValue(control, effectiveValue) ? (double)value : explicitValue;

    internal static bool? ReadAgeLinearEnabled(CheckBox control, bool? explicitValue, double coefficient)
    {
        var enabled = control.IsChecked == true;
        return explicitValue is null && enabled == (coefficient != 0) ? null : enabled;
    }

    private void F1Defaults_Click(object? sender, RoutedEventArgs e)
    {
        // The original Custom dialog's Default button sets a different
        // chloroplast charge and only resets fields that CostsForm writes.
        var f1 = GlobalSimulationCostSettings.F1Default;
        _draftCosts = ReadCosts() with
        {
            GlobalMultiplier = f1.GlobalMultiplier,
            DnaNumber = f1.DnaNumber,
            DnaDereference = f1.DnaDereference,
            DnaBasic = f1.DnaBasic,
            DnaAdvanced = f1.DnaAdvanced,
            DnaBitwise = f1.DnaBitwise,
            DnaCondition = f1.DnaCondition,
            DnaLogic = f1.DnaLogic,
            DnaStore = f1.DnaStore,
            DnaFlow = f1.DnaFlow,
            DnaUpkeep = f1.DnaUpkeep,
            DnaCopy = f1.DnaCopy,
            Movement = f1.Movement,
            Rotation = f1.Rotation,
            TieFormation = f1.TieFormation,
            ShotFormation = f1.ShotFormation,
            Venom = f1.Venom,
            Poison = f1.Poison,
            Slime = f1.Slime,
            Shell = f1.Shell,
            BodyUpkeep = f1.BodyUpkeep,
            ChloroplastConstruction = 0.2f,
            AgeBase = f1.AgeBase,
            AgeStart = f1.AgeStart,
            AgeLogarithmic = f1.AgeLogarithmic,
            DynamicLowerZeroThreshold = f1.DynamicLowerZeroThreshold,
            DynamicLowerReinstateThreshold = f1.DynamicLowerReinstateThreshold,
        };
        Populate(_draftCosts);
        DynamicMultiplier.IsEnabled = false;
        DynamicTargetPopulation.IsEnabled = false;
    }

    private void Apply_Click(object? sender, RoutedEventArgs e)
    {
        Costs = ReadCosts();
        Accepted = true;
        Close();
    }

    private void Cancel_Click(object? sender, RoutedEventArgs e) => Close();
}
