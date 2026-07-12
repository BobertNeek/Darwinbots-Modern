using Darwinbots.Desktop.Core;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class ZerobotProgressionControllerTests
{
    [Fact]
    public void AutomaticProgressionRequiresSelfReproductionFeedingThenMovement()
    {
        var controller = new ZerobotProgressionController(true);

        Assert.Null(controller.Observe(Snapshot(selfReproductions: 0, feeding: 0, movement: 0)));
        Assert.Equal(ZerobotProgressionAction.SwitchToEnergyOnlyFeeder,
            controller.Observe(Snapshot(selfReproductions: 1, feeding: 0, movement: 0))!.Action);
        Assert.Equal(ZerobotProgressionAction.RemoveFeederAssistance,
            controller.Observe(Snapshot(selfReproductions: 1, feeding: 1, movement: 0))!.Action);
        Assert.Equal(ZerobotProgressionAction.DisableBrownianMotion,
            controller.Observe(Snapshot(selfReproductions: 1, feeding: 1, movement: 1))!.Action);
        Assert.Equal(ZerobotProgressionStage.Complete, controller.Stage);
    }

    [Fact]
    public void ManualProgressionAdvancesWithoutBehaviorThresholds()
    {
        var controller = new ZerobotProgressionController(false);
        Assert.Equal(ZerobotProgressionAction.SwitchToEnergyOnlyFeeder, controller.AdvanceManually()!.Action);
        Assert.Equal(ZerobotProgressionAction.RemoveFeederAssistance, controller.AdvanceManually()!.Action);
        Assert.Equal(ZerobotProgressionAction.DisableBrownianMotion, controller.AdvanceManually()!.Action);
        Assert.Null(controller.AdvanceManually());
    }

    private static EngineSnapshot Snapshot(ulong selfReproductions, ulong feeding, ulong movement) =>
        EngineSnapshot.Empty with
        {
            Stats = new SimulationStatsSnapshot(0, 0, 0, 0, 0, 0, 0,
                SelfReproductions: selfReproductions, FeedingEvents: feeding, IntentionalMovementEvents: movement),
        };
}
