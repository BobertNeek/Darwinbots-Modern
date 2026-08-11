namespace Darwinbots.Desktop.Core;

public sealed class RobotDiagnosticModel
{
    public OrganismKey? Focus { get; private set; }
    public ulong Tick { get; private set; }
    public IReadOnlyList<ActivationFrameSnapshot> ActivationTimeline { get; private set; } = [];

    public string ActivationSummary => Focus is null
        ? "NO ROBOT FOCUSED"
        : $"LIVE · TICK {Tick:N0} · {ActivationTimeline.Count:N0} GENE FRAMES";

    public void Update(EngineSnapshot snapshot, OrganismKey focus)
    {
        ArgumentNullException.ThrowIfNull(snapshot);
        var organism = snapshot.Organisms.FirstOrDefault(value =>
            value.Slot == focus.Slot && value.Generation == focus.Generation);
        Focus = organism is null ? null : focus;
        Tick = snapshot.Tick;
        ActivationTimeline = organism?.ActivationHistory ?? [];
    }
}
