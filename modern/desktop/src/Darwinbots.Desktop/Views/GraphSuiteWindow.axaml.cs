using Avalonia.Controls;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class GraphSuiteWindow : Window, IPersistentSnapshotToolWindow
{
    private readonly List<GraphSeriesWindow> _graphWindows = [];
    private GraphSamplingSettings _lastSampling = new();
    private readonly LegacyGraphRecordingRun? _recording;
    private readonly Func<bool>? _autoFork;
    private readonly Func<Task<NativeGraphRecordingSample?>>? _readNativeSample;
    private static readonly IReadOnlyDictionary<GraphMetric, double> EmptyMetrics = new Dictionary<GraphMetric, double>();
    private readonly Dictionary<GraphMetric, RecordingChart> _recordingCharts = [];
    private EngineSnapshot _snapshot = EngineSnapshot.Empty;

    public GraphSuiteWindow() : this(EngineSnapshot.Empty)
    {
    }

    public GraphSuiteWindow(EngineSnapshot snapshot, LegacyGraphRecordingRun? recording = null, Func<bool>? autoFork = null,
        Func<Task<NativeGraphRecordingSample?>>? readNativeSample = null)
    {
        _recording = recording;
        _autoFork = autoFork;
        _readNativeSample = readNativeSample;
        InitializeComponent();
        GraphView.GraphWindowRequested += OpenGraphWindow;
        SetSnapshot(snapshot);
        ViewModel.GraphsReset += ResetRecordingCharts;
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

    private bool UsesNativeSamples => _readNativeSample is not null && _snapshot.NativeCycle.HasValue;

    internal bool WantsNativeGraphSamples => UsesNativeSamples && IsVisible && _recording?.IsSuspended == false
        && _recordingCharts.Values.Any(chart => chart.Windows.Any(window => window.IsVisible));

    public void SetSnapshot(EngineSnapshot snapshot)
    {
        _snapshot = snapshot;
        GraphView.SetSnapshot(snapshot);
        if (_recording is null || _recording.IsSuspended) return;
        var reset = _recordingCharts
            .Where(pair => pair.Value.Generation != _recording.Generation
                || (!UsesNativeSamples && snapshot.Tick < pair.Value.MostRecentTick))
            .ToArray();
        if (reset.Length > 0) ResetRecordingCharts(reset, feedImmediately: true);
        if (UsesNativeSamples || snapshot.History.Count == 0) return;
        foreach (var (metric, chart) in _recordingCharts)
        {
            foreach (var sample in ViewModel.HistorySamples)
            {
                if (sample.Tick <= chart.MostRecentTick) continue;
                RecordPoint(metric, chart, sample);
                chart.MostRecentTick = sample.Tick;
            }
        }
    }

    public void RecordNativeSamples(IReadOnlyList<NativeGraphRecordingSample> samples, int generation)
    {
        if (!WantsNativeGraphSamples || _recording?.Generation != generation) return;
        foreach (var (metric, chart) in _recordingCharts)
        {
            if (chart.Generation != generation || chart.AwaitingNativeRead || !chart.Windows.Any(window => window.IsVisible))
                continue;
            foreach (var sample in samples)
            {
                if (sample.Tick <= chart.MostRecentTick) continue;
                RecordPoint(metric, chart, ProjectNativePoint(sample));
                chart.MostRecentTick = sample.Tick;
            }
        }
    }

    private void ResetRecordingCharts() => ResetRecordingCharts(_recordingCharts, feedImmediately: true);

    private void ResetRecordingCharts(IEnumerable<KeyValuePair<GraphMetric, RecordingChart>> charts, bool feedImmediately)
    {
        if (_recording is null || _recording.IsSuspended) return;
        var pendingReads = new List<(GraphMetric Metric, RecordingChart Chart, int Version)>();
        foreach (var (metric, chart) in charts)
        {
            chart.Save.ResetPoints();
            chart.Generation = _recording.Generation;
            chart.MostRecentTick = _snapshot.Tick;
            chart.Version++;
            chart.AwaitingNativeRead = UsesNativeSamples;
            if (chart.AwaitingNativeRead)
                pendingReads.Add((metric, chart, chart.Version));
            else if (feedImmediately && ViewModel.HistorySamples.LastOrDefault() is { } sample)
                RecordPoint(metric, chart, sample);
        }
        if (pendingReads.Count > 0)
            _ = FinishNativeReadAsync(pendingReads, _recording.Generation, feedImmediately);
    }

    private async Task FinishNativeReadAsync(List<(GraphMetric Metric, RecordingChart Chart, int Version)> pending,
        int generation, bool feedImmediately)
    {
        if (_readNativeSample is not { } read) return;
        var sample = await read();
        if (_recording is not { } recording || recording.Generation != generation || recording.IsSuspended) return;

        foreach (var (metric, chart, version) in pending)
        {
            if (chart.Version != version || !_recordingCharts.TryGetValue(metric, out var active) || active != chart)
                continue;

            chart.AwaitingNativeRead = false;
            if (sample is null) continue;
            chart.MostRecentTick = sample.Tick;
            if (feedImmediately) RecordPoint(metric, chart, ProjectNativePoint(sample));
        }
    }

    private static GraphMetricSample ProjectNativePoint(NativeGraphRecordingSample sample) =>
        new(sample.Tick, EmptyMetrics, sample.SpeciesGraphs, sample.DynamicCostSeries);

    private void RecordPoint(GraphMetric metric, RecordingChart chart, GraphMetricSample sample) =>
        chart.Save.AddPoint(ViewModel.LegacyPoint(metric, sample), ViewModel.LegacyHeader(metric),
            _autoFork?.Invoke() == true);

    public void OpenEvolutionGraphs()
    {
        // MDIForm1.frm 1724-1727: keep existing charts and open the three
        // original evolution defaults without changing the manual chooser.
        var selected = ViewModel.SelectedChoice;
        try
        {
            foreach (var metric in new[] { GraphMetric.Population, GraphMetric.AverageMutations, GraphMetric.TotalEnergy })
            {
                if (ViewModel.RunningGraphs.Any(graph => graph.Metric == metric)) continue;
                ViewModel.SelectedChoice = GraphCatalog.For(metric);
                if (ViewModel.OpenSelectedGraph() is { } graph)
                {
                    _recording?.Graph(metric).SetEnabled(true);
                    OpenGraphWindow(graph);
                }
            }
        }
        finally { ViewModel.SelectedChoice = selected; }
    }

    private void OpenGraphWindow(GraphSeriesViewModel graph)
    {
        var series = ViewModel.RunningGraphs.Where(value => value.Metric == graph.Metric).ToArray();
        RecordingChart? recordingChart = null;
        if (_recording is not null && !graph.Id.StartsWith("legacy-", StringComparison.Ordinal))
        {
            if (!_recordingCharts.TryGetValue(graph.Metric, out recordingChart))
            {
                recordingChart = new RecordingChart(_recording.Graph(graph.Metric));
                _recordingCharts.Add(graph.Metric, recordingChart);
                ResetRecordingCharts([new(graph.Metric, recordingChart)], feedImmediately: true);
            }
        }
        var window = new GraphSeriesWindow(graph, series, recordingChart?.Save, recordingChart is null ? null : () =>
        {
            ViewModel.ResetGraph(graph.Metric);
            ResetRecordingCharts([new(graph.Metric, recordingChart)], feedImmediately: false);
        });
        recordingChart?.Windows.Add(window);
        _graphWindows.Add(window);
        window.Closed += (_, _) =>
        {
            _graphWindows.Remove(window);
            if (recordingChart is not null)
            {
                recordingChart.Windows.Remove(window);
                if (recordingChart.Windows.Count == 0) _recordingCharts.Remove(graph.Metric);
            }
            foreach (var item in series) ViewModel.RunningGraphs.Remove(item);
        };
        window.Show(this);
    }

    private sealed class RecordingChart(LegacyGraphAutoSave save)
    {
        public LegacyGraphAutoSave Save { get; } = save;
        public ulong MostRecentTick { get; set; }
        public int Generation { get; set; }
        public int Version { get; set; }
        public bool AwaitingNativeRead { get; set; }
        public List<GraphSeriesWindow> Windows { get; } = [];
    }

    protected override void OnClosed(EventArgs e)
    {
        foreach (var window in _graphWindows.ToArray()) window.Close();
        _graphWindows.Clear();
        NotifySamplingChanged(new GraphSamplingSettings());
        base.OnClosed(e);
    }
}
