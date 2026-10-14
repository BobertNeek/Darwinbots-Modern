using Avalonia.Controls;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class GraphSuiteWindow : Window, IPersistentSnapshotToolWindow
{
    private readonly List<GraphSeriesWindow> _graphWindows = [];
    private GraphSamplingSettings _lastSampling = new();

    public GraphSuiteWindow() : this(EngineSnapshot.Empty)
    {
    }

    public GraphSuiteWindow(EngineSnapshot snapshot)
    {
        InitializeComponent();
        GraphView.GraphWindowRequested += OpenGraphWindow;
        GraphView.SetSnapshot(snapshot);
        ViewModel.RunningGraphs.CollectionChanged += (_, _) => NotifySamplingChanged(SamplingSettings);

    }

    public GraphSuiteViewModel ViewModel => GraphView.ViewModel;
    public event Action<GraphSamplingSettings>? SamplingChanged;
    public GraphSamplingSettings SamplingSettings => new(
        ViewModel.RunningGraphs.Any(graph => graph.Metric is GraphMetric.SelectiveGeneticDistance or GraphMetric.CustomizableGraph1 or GraphMetric.CustomizableGraph2 or GraphMetric.CustomizableGraph3),
        ViewModel.RunningGraphs.Any(graph => graph.Metric is GraphMetric.GenerationalDistance or GraphMetric.CustomizableGraph1 or GraphMetric.CustomizableGraph2 or GraphMetric.CustomizableGraph3),
        ViewModel.RunningGraphs.Any(graph => graph.Metric == GraphMetric.SlowGeneticDistance));

    private void NotifySamplingChanged(GraphSamplingSettings settings)
    {
        if (settings == _lastSampling) return;
        _lastSampling = settings;
        SamplingChanged?.Invoke(settings);
    }

    public void SetSnapshot(EngineSnapshot snapshot) => GraphView.SetSnapshot(snapshot);

    private void OpenGraphWindow(GraphSeriesViewModel graph)
    {
        var series = ViewModel.RunningGraphs.Where(value => value.Metric == graph.Metric).ToArray();
        var window = new GraphSeriesWindow(graph, series);
        _graphWindows.Add(window);
        window.Closed += (_, _) =>
        {
            _graphWindows.Remove(window);
            foreach (var item in series) ViewModel.RunningGraphs.Remove(item);
        };
        window.Show(this);
    }

    protected override void OnClosed(EventArgs e)
    {
        foreach (var window in _graphWindows.ToArray()) window.Close();
        _graphWindows.Clear();
        NotifySamplingChanged(new GraphSamplingSettings());
        base.OnClosed(e);
    }
}
