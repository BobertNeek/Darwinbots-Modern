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
    {
        ArgumentNullException.ThrowIfNull(settings);
        Costs = settings.Costs;
        InitializeComponent();
        Populate(Costs);
    }

    private void Populate(GlobalSimulationCostSettings costs)
    {
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
    }

    private GlobalSimulationCostSettings ReadCosts() => new()
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
    };

    private void Apply_Click(object? sender, RoutedEventArgs e)
    {
        Costs = ReadCosts();
        Accepted = true;
        Close();
    }

    private void Cancel_Click(object? sender, RoutedEventArgs e) => Close();
}
