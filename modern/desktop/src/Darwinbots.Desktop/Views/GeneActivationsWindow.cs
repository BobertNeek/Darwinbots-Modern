using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed class GeneActivationsWindow : Window, IPersistentSnapshotToolWindow
{
    private readonly TextBlock _focusLabel = new();
    private readonly TextBlock _activity = new()
    {
        TextWrapping = TextWrapping.Wrap,
    };
    private OrganismKey? _focus;

    public GeneActivationsWindow()
    {
        Title = "Gene Activations";
        Width = 560;
        Height = 230;
        MinWidth = 260;
        MinHeight = 150;
        Topmost = true;
        Content = new DockPanel
        {
            Margin = new Thickness(16),
            Children = { _focusLabel, new ScrollViewer { Content = _activity } },
        };
        DockPanel.SetDock(_focusLabel, Dock.Top);
        _focusLabel.Margin = new Thickness(0, 0, 0, 10);
    }

    public void SetFocus(OrganismKey? focus, EngineSnapshot snapshot)
    {
        _focus = focus;
        SetSnapshot(snapshot);
    }

    public void SetSnapshot(EngineSnapshot snapshot)
    {
        var robot = _focus is { } focus
            ? snapshot.Organisms.FirstOrDefault(value =>
                value.Slot == focus.Slot && value.Generation == focus.Generation)
            : null;
        if (robot is null)
        {
            _focusLabel.Text = "No robot focused";
            _activity.Text = string.Empty;
            return;
        }

        var cycle = snapshot.NativeCycle?.ToString("N0") ?? snapshot.Tick.ToString("N0");
        _focusLabel.Text = $"Robot {robot.Slot}:{robot.Generation} · Cycle {cycle}";
        _activity.Text = FormatActivations(robot.ActivationBitmap);
    }

    internal static string FormatActivations(IReadOnlyList<bool> activations) =>
        activations.Count <= 1
            ? "NO GENE DATA"
            // The VM bitmap is one-based. Index zero is a sentinel.
            : string.Join("  ", activations.Skip(1).Select((active, index) =>
                $"G{index + 1}:{(active ? "ON" : "off")}"));
}
