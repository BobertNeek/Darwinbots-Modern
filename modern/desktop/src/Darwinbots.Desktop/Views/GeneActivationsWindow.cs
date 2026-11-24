using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed class GeneActivationsWindow : Window, IPersistentSnapshotToolWindow
{
    private readonly TextBlock _focusLabel = new();
    private readonly GeneActivationBands _bands = new() { Name = "ActivationBands", Height = 85, Margin = new Thickness(0, 0, 0, 10) };
    private readonly TextBlock _activity = new()
    {
        TextWrapping = TextWrapping.Wrap,
    };
    private OrganismKey? _focus;

    public GeneActivationsWindow(ulong watchId = 0)
    {
        WatchId = watchId;
        Title = "Gene Activations";
        Width = 560;
        Height = 230;
        MinWidth = 260;
        MinHeight = 180;
        Topmost = true;
        Content = new DockPanel
        {
            Margin = new Thickness(16),
            Children = { _focusLabel, _bands, new ScrollViewer { Content = _activity } },
        };
        DockPanel.SetDock(_focusLabel, Dock.Top);
        DockPanel.SetDock(_bands, Dock.Top);
        _focusLabel.Margin = new Thickness(0, 0, 0, 10);
    }

    internal ulong WatchId { get; private set; }

    internal void RestartWatch(ulong watchId)
    {
        WatchId = watchId;
        _bands.SetMeter(null, false);
    }

    public void SetFocus(OrganismKey? focus, EngineSnapshot snapshot)
    {
        _focus = focus;
        SetSnapshot(snapshot);
    }

    public void SetSnapshot(EngineSnapshot snapshot)
    {
        var meter = snapshot.GeneActivationMeter is { } observed && observed.WatchId == WatchId ? observed : null;
        var robot = _focus is { } focus
            ? snapshot.Organisms.FirstOrDefault(value =>
                value.Slot == focus.Slot && value.Generation == focus.Generation)
            : null;
        var current = robot is not null && meter?.Focus == _focus ? meter?.Current : null;
        _bands.SetMeter(meter, current is not null);
        if (robot is null)
        {
            _focusLabel.Text = "No robot focused";
            _activity.Text = string.Empty;
            return;
        }

        var cycle = snapshot.NativeCycle?.ToString("N0") ?? snapshot.Tick.ToString("N0");
        _focusLabel.Text = $"Robot {robot.Slot}:{robot.Generation} · Cycle {cycle}";
        _activity.Text = current is { Count: > 0 } ? FormatGenes(current) : FormatActivations(robot.ActivationBitmap);
    }

    internal static string FormatActivations(IReadOnlyList<bool> activations) =>
        activations.Count <= 1
            ? "NO GENE DATA"
            // The VM bitmap is one-based. Index zero is a sentinel.
            : FormatGenes(activations.Skip(1));

    private static string FormatGenes(IEnumerable<bool> genes) => string.Join("  ", genes.Select((active, index) =>
        $"G{index + 1}:{(active ? "ON" : "off")}"));
}
