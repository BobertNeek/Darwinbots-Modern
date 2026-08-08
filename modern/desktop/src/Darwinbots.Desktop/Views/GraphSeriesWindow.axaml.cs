using Avalonia.Controls;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class GraphSeriesWindow : Window
{
    private readonly GraphSeriesViewModel _graph;

    public GraphSeriesWindow() : this(new GraphSeriesViewModel("empty", GraphMetric.Population, "Population", []))
    {
    }

    public GraphSeriesWindow(GraphSeriesViewModel graph)
    {
        _graph = graph ?? throw new ArgumentNullException(nameof(graph));
        InitializeComponent();
        _graph.PropertyChanged += Graph_PropertyChanged;
        Refresh();
        Closed += (_, _) => _graph.PropertyChanged -= Graph_PropertyChanged;
    }

    private void Graph_PropertyChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e) => Refresh();

    private void Refresh()
    {
        TitleText.Text = _graph.Title.ToUpperInvariant();
        ValueText.Text = _graph.CurrentValueText;
        SeriesChart.SetSeries(_graph);
    }
}
