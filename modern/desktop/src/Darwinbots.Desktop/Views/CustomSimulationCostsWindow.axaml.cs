using Avalonia.Controls;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class CustomSimulationCostsWindow : Window
{
    public bool Accepted { get; private set; }
    public GlobalSimulationCostSettings Costs { get; private set; }

    public CustomSimulationCostsWindow() : this(new GlobalSettings()) { }

    public CustomSimulationCostsWindow(GlobalSettings settings)
        : this(settings?.Costs ?? throw new ArgumentNullException(nameof(settings)), stageForLiveSettings: false) { }

    public CustomSimulationCostsWindow(GlobalSimulationCostSettings costs, bool stageForLiveSettings)
    {
        Costs = costs ?? throw new ArgumentNullException(nameof(costs));
        InitializeComponent();
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
        var ageLinear = Read(AgeLinear, Costs.AgeLinear);
        return Costs with
        {
            GlobalMultiplier = Read(GlobalCostMultiplier, Costs.GlobalMultiplier),
            DnaNumber = Read(DnaNumber, Costs.DnaNumber),
            DnaDereference = Read(DnaDereference, Costs.DnaDereference),
            DnaBasic = Read(DnaBasic, Costs.DnaBasic),
            DnaAdvanced = Read(DnaAdvanced, Costs.DnaAdvanced),
            DnaBitwise = Read(DnaBitwise, Costs.DnaBitwise),
            DnaCondition = Read(DnaCondition, Costs.DnaCondition),
            DnaLogic = Read(DnaLogic, Costs.DnaLogic),
            DnaStore = Read(DnaStore, Costs.DnaStore),
            DnaFlow = Read(DnaFlow, Costs.DnaFlow),
            DnaUpkeep = Read(DnaUpkeep, Costs.DnaUpkeep),
            DnaCopy = Read(DnaCopy, Costs.DnaCopy),
            Movement = Read(MovementCost, Costs.Movement),
            Rotation = Read(RotationCost, Costs.Rotation),
            TieFormation = Read(TieFormationCost, Costs.TieFormation),
            ShotFormation = Read(ShotFormationCost, Costs.ShotFormation),
            Venom = Read(VenomCost, Costs.Venom),
            Poison = Read(PoisonCost, Costs.Poison),
            Slime = Read(SlimeCost, Costs.Slime),
            Shell = Read(ShellCost, Costs.Shell),
            BodyUpkeep = Read(BodyUpkeep, Costs.BodyUpkeep),
            ChloroplastConstruction = Read(ChloroplastConstructionCost, Costs.ChloroplastConstruction),
            Virus = Read(VirusCost, Costs.Virus),
            AgeBase = Read(AgeBase, Costs.AgeBase),
            AgeStart = Read(AgeStart, Costs.AgeStart),
            AgeLinear = ageLinear,
            AgeLinearEnabled = ReadAgeLinearEnabled(AgeLinearEnabled, Costs.AgeLinearEnabled, ageLinear),
            AgeLogarithmic = Read(AgeLogarithmic, Costs.AgeLogarithmic),
            DynamicMultiplier = Read(DynamicMultiplier, Costs.DynamicMultiplier),
            DynamicAllRobots = DynamicAllRobots.IsChecked == true,
            DynamicTargetPopulation = (int)Read(DynamicTargetPopulation, Costs.DynamicTargetPopulation),
            DynamicUpperRangePercent = ReadRangePercent(DynamicUpperRangePercent,
                Costs.DynamicUpperRangePercent, Costs.EffectiveDynamicUpperRangePercent),
            DynamicLowerRangePercent = ReadRangePercent(DynamicLowerRangePercent,
                Costs.DynamicLowerRangePercent, Costs.EffectiveDynamicLowerRangePercent),
            DynamicSensitivity = Read(DynamicSensitivity, Costs.DynamicSensitivity),
            DynamicAllowNegative = DynamicAllowNegative.IsChecked == true,
            DynamicLowerZeroThreshold = Read(DynamicLowerZeroThreshold, Costs.DynamicLowerZeroThreshold),
            DynamicLowerReinstateThreshold = Read(DynamicLowerReinstateThreshold, Costs.DynamicLowerReinstateThreshold),
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

    private void Apply_Click(object? sender, RoutedEventArgs e)
    {
        Costs = ReadCosts();
        Accepted = true;
        Close();
    }

    private void Cancel_Click(object? sender, RoutedEventArgs e) => Close();
}
