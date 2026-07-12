namespace Darwinbots.Desktop.Core;

public enum ZerobotProgressionStage { AwaitingSelfReproduction, AwaitingFeeding, AwaitingMovement, Complete }
public enum ZerobotProgressionAction { SwitchToEnergyOnlyFeeder, RemoveFeederAssistance, DisableBrownianMotion }
public sealed record ZerobotProgressionTransition(ZerobotProgressionStage Stage, ZerobotProgressionAction Action, string Message);

public sealed class ZerobotProgressionController(bool automatic)
{
    public bool Automatic { get; } = automatic;
    public ZerobotProgressionStage Stage { get; private set; } = ZerobotProgressionStage.AwaitingSelfReproduction;

    public ZerobotProgressionTransition? Observe(EngineSnapshot snapshot)
    {
        if (!Automatic) return null;
        return Stage switch
        {
            ZerobotProgressionStage.AwaitingSelfReproduction when snapshot.Stats.SelfReproductions > 0 => Advance(),
            ZerobotProgressionStage.AwaitingFeeding when snapshot.Stats.FeedingEvents > 0 => Advance(),
            ZerobotProgressionStage.AwaitingMovement when snapshot.Stats.IntentionalMovementEvents > 0 => Advance(),
            _ => null,
        };
    }

    public ZerobotProgressionTransition? AdvanceManually() => Advance();

    private ZerobotProgressionTransition? Advance()
    {
        var transition = Stage switch
        {
            ZerobotProgressionStage.AwaitingSelfReproduction => new ZerobotProgressionTransition(
                ZerobotProgressionStage.AwaitingFeeding, ZerobotProgressionAction.SwitchToEnergyOnlyFeeder,
                "Self-reproduction evolved; feeder assistance reduced to energy only."),
            ZerobotProgressionStage.AwaitingFeeding => new ZerobotProgressionTransition(
                ZerobotProgressionStage.AwaitingMovement, ZerobotProgressionAction.RemoveFeederAssistance,
                "Independent feeding evolved; feeder assistance removed."),
            ZerobotProgressionStage.AwaitingMovement => new ZerobotProgressionTransition(
                ZerobotProgressionStage.Complete, ZerobotProgressionAction.DisableBrownianMotion,
                "Intentional movement evolved; Brownian assistance disabled."),
            _ => null,
        };
        if (transition is not null) Stage = transition.Stage;
        return transition;
    }
}
