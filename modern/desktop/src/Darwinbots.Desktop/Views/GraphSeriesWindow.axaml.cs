using Avalonia.Controls;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class GraphSeriesWindow : Window
{
    private readonly GraphSeriesViewModel _graph;
    private readonly IReadOnlyList<GraphSeriesViewModel> _series;

    public GraphSeriesWindow() : this(new GraphSeriesViewModel("empty", GraphMetric.Population, "Population", []))
    {
    }

    public GraphSeriesWindow(GraphSeriesViewModel graph, IReadOnlyList<GraphSeriesViewModel>? series = null)
    {
        _graph = graph ?? throw new ArgumentNullException(nameof(graph));
        _series = series ?? new[] { graph };
        InitializeComponent();
        foreach (var item in _series) item.PropertyChanged += Graph_PropertyChanged;
        Refresh();
        Closed += (_, _) => { foreach (var item in _series) item.PropertyChanged -= Graph_PropertyChanged; };
    }

    private void Graph_PropertyChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e) => Refresh();

    private void Refresh()
    {
        TitleText.Text = _graph.Title.ToUpperInvariant();
        ValueText.Text = _graph.CurrentValueText;
        SeriesChart.SetSeries(_series);
    }
}
