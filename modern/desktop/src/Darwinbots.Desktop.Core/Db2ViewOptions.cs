namespace Darwinbots.Desktop.Core;

public sealed record Db2ViewOptions(
    bool ResourceGauges = true,
    bool MovementVectors = true,
    bool ShotImpacts = true,
    bool VisionGrid = true);
