using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed class PhylogenyWindow : Window, IPersistentSnapshotToolWindow
{
    private readonly OrganismKey _focus;
    private readonly NumericUpDown _depth = new() { Minimum = 1, Maximum = 1_000, Value = 4, Width = 100 };
    private readonly TextBlock _status = new();
    private readonly StackPanel _nodes = new() { Spacing = 5 };
    private EngineSnapshot _snapshot = EngineSnapshot.Empty;

    public PhylogenyWindow(EngineSnapshot snapshot, OrganismKey focus)
    {
        _focus = focus;
        Title = $"Darwinbots — Philogeny {focus.Slot}:{focus.Generation}";
        Width = 620;
        Height = 560;
        MinWidth = 500;
        MinHeight = 420;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;
        Background = Brush.Parse("#F4F0E6");
        _status.Foreground = Brush.Parse("#537B43");
        _status.FontFamily = new FontFamily("Bahnschrift");
        _status.FontSize = 11;
        _depth.ValueChanged += (_, _) => RenderGraph();

        var controls = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, Children = { new TextBlock { Text = "DEPTH", FontWeight = FontWeight.Bold, VerticalAlignment = VerticalAlignment.Center }, _depth } };
        var body = new StackPanel { Spacing = 10, Margin = new Thickness(18), Children = { controls, _status, new ScrollViewer { Content = _nodes } } };
        Content = new Border { Background = Brush.Parse("#FBF9F2"), Padding = new Thickness(0), Child = new Grid { RowDefinitions = new RowDefinitions("62,*"), Children = { new Border { Background = Brush.Parse("#FBF9F2"), BorderBrush = Brush.Parse("#C8C3B6"), BorderThickness = new Thickness(0, 0, 0, 1), Padding = new Thickness(20, 12), Child = new StackPanel { Spacing = 2, Children = { new TextBlock { Text = "PHILOGENY", FontFamily = new FontFamily("Bahnschrift"), FontSize = 21, FontWeight = FontWeight.Bold, LetterSpacing = 1.3 }, new TextBlock { Text = "AUTHORITATIVE PARENT / CHILD IDENTITIES", Foreground = Brush.Parse("#537B43"), FontSize = 10, FontWeight = FontWeight.Bold } } } }, body.With(Grid.RowProperty, 1) } } };
        SetSnapshot(snapshot);
    }

    public int Depth => (int)(_depth.Value ?? 4);

    public void SetSnapshot(EngineSnapshot snapshot)
    {
        _snapshot = snapshot;
        RenderGraph();
    }

    private void RenderGraph()
    {
        var graph = PhylogenyModel.Build(_snapshot, _focus, Depth);
        _nodes.Children.Clear();
        if (graph.Nodes.Count == 0)
        {
            _status.Text = $"FOCUS LOST · {_focus.Slot}:{_focus.Generation} IS NOT BOUND TO A REPLACEMENT";
            return;
        }
        _status.Text = $"{graph.Nodes.Count:N0} HIGHLIGHTED LINEAGE MEMBERS · DEPTH ±{graph.MaxDepth}";
        foreach (var node in graph.Nodes)
        {
            var label = node.IsFocused ? "FOCUS" : node.Depth < 0 ? "ANCESTOR" : "DESCENDANT";
            var text = $"{new string(' ', Math.Max(0, node.Depth + graph.MaxDepth) * 2)}{label,-10} {node.Key.Slot}:{node.Key.Generation}  depth {node.Depth,3}  lineage {node.LineageId:N0}";
            _nodes.Children.Add(new Border
            {
                Margin = new Thickness(Math.Max(0, node.Depth + graph.MaxDepth) * 4, 0, 0, 0),
                Padding = new Thickness(10, 7),
                Background = node.IsFocused ? Brush.Parse("#D9E8C6") : Brush.Parse("#FFFCF5"),
                BorderBrush = node.IsHighlighted ? Brush.Parse("#7BA25F") : Brush.Parse("#C8C3B6"),
                BorderThickness = new Thickness(1),
                Child = new TextBlock { Text = text, FontFamily = new FontFamily("Consolas"), FontSize = 12 },
            });
        }
    }
}
