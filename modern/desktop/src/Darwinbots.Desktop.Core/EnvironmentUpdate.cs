namespace Darwinbots.Desktop.Core;

public sealed record EnvironmentUpdate(
    int MetabolismCost,
    int VegetableEnergyPerTick,
    int SunlightEnergy,
    float[] Gravity,
    float Drag,
    float BrownianMotion);
