using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed class PhylogenyWindow : Window, IPersistentSnapshotToolWindow
{
    private readonly OrganismKey _focus;
    private readonly NumericUpDown _depth = new() { Name = "Depth", Minimum = 1, Maximum = 1_000, Value = 1, Width = 100 };
    private readonly TextBlock _status = new() { Name = "DescendantCount" };
    private readonly Button _highlight = new() { Name = "HighlightDescendants", Content = "Highlight Descendants" };
    private readonly Button _wholeLineage = new() { Name = "ShowWholeLineage", Content = "Show Whole Lineage" };
    private readonly StackPanel _nodes = new() { Spacing = 5 };
    private readonly Action<int>? _highlightDescendants;
    private readonly Action? _showWholeLineage;
    private EngineSnapshot _snapshot = EngineSnapshot.Empty;

    public PhylogenyWindow(EngineSnapshot snapshot, OrganismKey focus, Action<int>? highlightDescendants = null,
        Action? showWholeLineage = null)
    {
        _focus = focus;
        _highlightDescendants = highlightDescendants;
        _showWholeLineage = showWholeLineage;
        Title = $"Darwinbots — Philogeny {focus.Slot}:{focus.Generation}";
        Width = 620;
        Height = 560;
        MinWidth = 500;
        MinHeight = 420;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;
        Background = Brush.Parse("#F4F0E6");
        _status.Foreground = Brush.Parse("#537B43");
        _status.FontFamily = DesktopFonts.Heading;
        _status.FontSize = 11;
        _depth.ValueChanged += (_, _) => RenderGraph();

        var calculate = new Button { Name = "CalculateDescendants", Content = "Calculate" };
        calculate.Click += (_, _) => RenderGraph();
        _highlight.Click += (_, _) => _highlightDescendants?.Invoke(Depth);
        _wholeLineage.Click += (_, _) => _showWholeLineage?.Invoke();
        var controls = new WrapPanel
        {
            ItemSpacing = 8,
            Children =
            {
                new TextBlock { Text = "DEPTH", FontWeight = FontWeight.Bold, VerticalAlignment = VerticalAlignment.Center },
                _depth, calculate, _highlight, _wholeLineage,
            },
        };
        var body = new StackPanel { Spacing = 10, Margin = new Thickness(18), Children = { controls, _status, new ScrollViewer { Content = _nodes } } };
        Content = new Border { Background = Brush.Parse("#FBF9F2"), Padding = new Thickness(0), Child = new Grid { RowDefinitions = new RowDefinitions("62,*"), Children = { new Border { Background = Brush.Parse("#FBF9F2"), BorderBrush = Brush.Parse("#C8C3B6"), BorderThickness = new Thickness(0, 0, 0, 1), Padding = new Thickness(20, 12), Child = new StackPanel { Spacing = 2, Children = { new TextBlock { Text = "PHILOGENY", FontFamily = DesktopFonts.Heading, FontSize = 21, FontWeight = FontWeight.Bold, LetterSpacing = 1.3 }, new TextBlock { Text = "AUTHORITATIVE PARENT / CHILD IDENTITIES", Foreground = Brush.Parse("#537B43"), FontSize = 10, FontWeight = FontWeight.Bold } } } }, body.With(Grid.RowProperty, 1) } } };
        SetSnapshot(snapshot);
    }

    public int Depth => (int)(_depth.Value ?? 1);

    public void SetSnapshot(EngineSnapshot snapshot)
    {
        _snapshot = snapshot;
        RenderGraph();
    }

    private void RenderGraph()
    {
        var graph = PhylogenyModel.Build(_snapshot, _focus, Depth);
        _nodes.Children.Clear();
        _highlight.IsEnabled = graph.Nodes.Count > 0 && _highlightDescendants is not null;
        _wholeLineage.IsEnabled = graph.Nodes.Count > 0 && _showWholeLineage is not null;
        if (graph.Nodes.Count == 0)
        {
            _status.Text = $"FOCUS LOST · {_focus.Slot}:{_focus.Generation} IS NOT BOUND TO A REPLACEMENT";
            return;
        }
        var descendantCount = PhylogenyModel.DescendantLinks(_snapshot, _focus, Depth).Count;
        _status.Text = $"DESCENDANTS {descendantCount:N0} · DEPTH {Depth}";
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
                Child = new TextBlock { Text = text, FontFamily = DesktopFonts.Inspector, FontSize = 12 },
            });
        }
    }
}
