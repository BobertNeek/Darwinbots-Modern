using Avalonia.Controls;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class GraphSeriesWindow : Window
{
    private readonly GraphSeriesViewModel _graph;
    private readonly IReadOnlyList<GraphSeriesViewModel> _series;
    private readonly LegacyGraphAutoSave? _recording;
    private readonly Action? _resetGraph;

    public GraphSeriesWindow() : this(new GraphSeriesViewModel("empty", GraphMetric.Population, "Population", []))
    {
    }

    public GraphSeriesWindow(GraphSeriesViewModel graph, IReadOnlyList<GraphSeriesViewModel>? series = null,
        LegacyGraphAutoSave? recording = null, Action? resetGraph = null)
    {
        _graph = graph ?? throw new ArgumentNullException(nameof(graph));
        _series = series ?? new[] { graph };
        _recording = recording;
        _resetGraph = resetGraph;
        InitializeComponent();
        ResetGraphButton.IsVisible = _resetGraph is not null;
        foreach (var item in _series) item.PropertyChanged += Graph_PropertyChanged;
        if (_recording is not null)
        {
            GraphRecordingControls.IsVisible = true;
            _recording.Changed += UpdateRecordingStatus;
            UpdateRecordingStatus();
        }
        Refresh();
        Closed += (_, _) =>
        {
            foreach (var item in _series) item.PropertyChanged -= Graph_PropertyChanged;
            if (_recording is not null) _recording.Changed -= UpdateRecordingStatus;
        };
    }

    private void ResetGraph_Click(object? sender, RoutedEventArgs e) => _resetGraph?.Invoke();

    private void SaveGraphData_Click(object? sender, RoutedEventArgs e) =>
        _recording?.SetEnabled(SaveGraphData.IsChecked == true);

    private void UpdateRecordingStatus()
    {
        if (_recording is not { } recording) return;
        SaveGraphData.IsChecked = recording.Enabled;
        SaveGraphStatus.Text = recording.Status;
    }

    private void Graph_PropertyChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e) => Refresh();

    private void Refresh()
    {
        TitleText.Text = _graph.Title.ToUpperInvariant();
        ValueText.Text = _graph.CurrentValueText;
        SeriesChart.SetSeries(_series);
    }
}
