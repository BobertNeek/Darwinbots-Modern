using Avalonia.Controls;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class GraphSuiteWindow : Window, IPersistentSnapshotToolWindow
{
    private readonly List<GraphSeriesWindow> _graphWindows = [];
    private GraphSamplingSettings _lastSampling = new();
    private readonly LegacyGraphRecordingRun? _recording;
    private readonly bool _keepUpdatingOnClose;
    private readonly Func<bool>? _autoFork;
    private readonly Func<GraphSamplingSettings?, Action<NativeGraphRecordingSample?>, Task>? _consumeNativeSample;
    private static readonly IReadOnlyDictionary<GraphMetric, double> EmptyMetrics = new Dictionary<GraphMetric, double>();
    private readonly Dictionary<GraphMetric, RecordingChart> _recordingCharts = [];
    private EngineSnapshot _snapshot = EngineSnapshot.Empty;
    private NativeGraphLayoutState? _nativeLayout;

    public GraphSuiteWindow() : this(EngineSnapshot.Empty)
    {
    }

    public GraphSuiteWindow(EngineSnapshot snapshot, LegacyGraphRecordingRun? recording = null, Func<bool>? autoFork = null,
        Func<Task<NativeGraphRecordingSample?>>? readNativeSample = null,
        Func<GraphSamplingSettings?, Action<NativeGraphRecordingSample?>, Task>? consumeNativeSample = null,
        bool keepUpdatingOnClose = false)
    {
        _recording = recording;
        _keepUpdatingOnClose = keepUpdatingOnClose;
        _autoFork = autoFork;
        _consumeNativeSample = consumeNativeSample ?? (readNativeSample is null ? null
            : async (_, consume) => consume(await readNativeSample()));
        InitializeComponent();
        GraphView.GraphWindowRequested += OpenGraphWindow;
        GraphView.ReopenGraphRequested = ReopenGraphWindow;
        SetSnapshot(snapshot);
        ViewModel.GraphsReset += ResetRecordingCharts;
        ViewModel.RunningGraphs.CollectionChanged += (_, _) => NotifySamplingChanged(SamplingSettings);
        foreach (var slot in ViewModel.CustomMetrics)
            slot.PropertyChanged += (_, e) =>
            {
                if (e.PropertyName == nameof(CustomMetricSlotViewModel.Expression))
                    NotifySamplingChanged(SamplingSettings);
            };
    }

    public GraphSuiteViewModel ViewModel => GraphView.ViewModel;
    public event Action<GraphSamplingSettings>? SamplingChanged;
    internal event Action<NativeGraphLayoutState, NativeGraphLayoutRow, NativeGraphCustomQuery?>? NativeLayoutEdited;
    public GraphSamplingSettings SamplingSettings => SamplingFor(ViewModel.RunningGraphs.Select(graph => graph.Metric));

    private GraphSamplingSettings SamplingFor(IEnumerable<GraphMetric> metrics) => metrics.Distinct()
        .Select(ViewModel.SamplingSettingsFor)
        .Aggregate(new GraphSamplingSettings(), (combined, settings) => new(
            combined.Selective || settings.Selective,
            combined.Generational || settings.Generational,
            combined.Slow || settings.Slow));

    private void NotifySamplingChanged(GraphSamplingSettings settings)
    {
        if (settings == _lastSampling) return;
        _lastSampling = settings;
        SamplingChanged?.Invoke(settings);
    }

    private bool UsesNativeSamples => _consumeNativeSample is not null && _snapshot.NativeCycle.HasValue;

    internal bool WantsNativeGraphSamples => UsesNativeSamples && IsVisible && _recording?.IsSuspended == false
        && _recordingCharts.Values.Any(chart => chart.Windows.Any(window => window.IsCollecting));

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
            if (chart.Generation != generation || chart.AwaitingNativeRead || !chart.Windows.Any(window => window.IsCollecting))
                continue;
            foreach (var sample in samples)
            {
                if (sample.Tick <= chart.MostRecentTick) continue;
                RecordPoint(metric, chart, ProjectNativePoint(sample));
                chart.MostRecentTick = sample.Tick;
            }
        }
    }

    private void ResetRecordingCharts() => ResetRecordingCharts(_recordingCharts, feedImmediately: false);

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
        if (pendingReads.Count > 0 && _consumeNativeSample is { } capture)
        {
            var generation = _recording.Generation;
            var sampling = feedImmediately ? SamplingFor(pendingReads.Select(entry => entry.Metric)) : null;
            _ = capture(sampling, sample => FinishNativeRead(pendingReads, generation, feedImmediately, sample));
        }
    }

    private void FinishNativeRead(List<(GraphMetric Metric, RecordingChart Chart, int Version)> pending,
        int generation, bool feedImmediately, NativeGraphRecordingSample? sample)
    {
        if (_recording is not { } recording || recording.Generation != generation || recording.IsSuspended) return;

        foreach (var (metric, chart, version) in pending)
        {
            if (chart.Version != version || !_recordingCharts.TryGetValue(metric, out var active) || active != chart)
                continue;

            chart.AwaitingNativeRead = false;
            if (sample is null) continue;
            chart.MostRecentTick = sample.Tick;
            if (feedImmediately) AppendImmediatePoint(metric, chart, ProjectNativePoint(sample));
        }
    }

    private static GraphMetricSample ProjectNativePoint(NativeGraphRecordingSample sample) =>
        new(sample.Tick, EmptyMetrics, sample.SpeciesGraphs, sample.DynamicCostSeries);

    private void RecordPoint(GraphMetric metric, RecordingChart chart, GraphMetricSample sample) =>
        chart.Save.AddPoint(ViewModel.LegacyPoint(metric, sample), ViewModel.LegacyHeader(metric),
            _autoFork?.Invoke() == true);

    internal void SetNativeLayout(NativeGraphLayoutState? layout)
    {
        _nativeLayout = layout;
        if (layout is null) return;
        var firstCustom = LegacyGraphRecordingRun.NativeIndexOf(GraphMetric.CustomizableGraph1);
        foreach (var slot in ViewModel.CustomMetrics)
        {
            var expression = layout.CustomQueries[slot.Slot - 1];
            var row = layout.Graphs.FirstOrDefault(candidate => candidate.GraphIndex == firstCustom + slot.Slot - 1);
            if (expression.Length > 0 || row is { Visible: true } or { Left: not null } or { Top: not null })
                slot.Expression = expression;
        }
    }

    internal void LoadGraphWorld(EngineSnapshot snapshot, NativeGraphLayoutState? layout)
    {
        foreach (var window in _graphWindows.Where(window => window.NeedsCleanupOnLoad).ToArray())
            window.CloseForCleanup();
        ViewModel.ClearPreviousGraphWorld();
        SetNativeLayout(layout);
        SetSnapshot(snapshot);
        if (layout is null) return;

        var selected = ViewModel.SelectedChoice;
        try
        {
            foreach (var row in layout.Graphs.Where(row => row.Visible == true))
            {
                if (LegacyGraphRecordingRun.NativeMetricAt(row.GraphIndex) is not { } metric) continue;
                if (_recordingCharts.TryGetValue(metric, out var existing)
                    && existing.Windows.FirstOrDefault(window => window.IsVisible) is { } visible)
                {
                    // NewGraph repositions already-visible charts at the display origin.
                    visible.PlaceFromNativeLayout(this, row with { Left = 0, Top = 0 });
                    continue;
                }

                ViewModel.SelectedChoice = GraphCatalog.For(metric);
                if (ViewModel.OpenSelectedGraph() is { } graph) OpenGraphWindow(graph, restoring: true);
            }
        }
        finally { ViewModel.SelectedChoice = selected; }
    }

    private void UpdateGraphNow(GraphMetric metric, RecordingChart chart)
    {
        if (_recording is not { } recording || recording.IsSuspended) return;
        var generation = recording.Generation;
        var version = chart.Version;
        if (UsesNativeSamples && _consumeNativeSample is { } capture)
            _ = capture(ViewModel.SamplingSettingsFor(metric), sample =>
            {
                if (sample is null || recording.IsSuspended || recording.Generation != generation
                    || chart.Version != version || !_recordingCharts.TryGetValue(metric, out var active) || active != chart)
                    return;
                AppendImmediatePoint(metric, chart, ProjectNativePoint(sample));
            });
        else if (ViewModel.HistorySamples.LastOrDefault() is { } sample)
            AppendImmediatePoint(metric, chart, sample);
    }

    private void AppendImmediatePoint(GraphMetric metric, RecordingChart chart, GraphMetricSample sample)
    {
        RecordPoint(metric, chart, sample);
        chart.MostRecentTick = Math.Max(chart.MostRecentTick, sample.Tick);
        ViewModel.AppendImmediatePoint(metric, sample);
    }

    private GraphSeriesViewModel? ReopenGraphWindow(GraphMetric metric)
    {
        if (!_recordingCharts.TryGetValue(metric, out var chart) || chart.Windows.FirstOrDefault() is not { } window)
            return null;
        window.Reopen(this);
        RememberOpenedLayout(window);
        UpdateGraphNow(metric, chart); // Original NewGraph feeds once when shown again.
        return window.Graph;
    }

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

    private void OpenGraphWindow(GraphSeriesViewModel graph) => OpenGraphWindow(graph, restoring: false);

    private void OpenGraphWindow(GraphSeriesViewModel graph, bool restoring)
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
        }, recordingChart is null ? null : () => UpdateGraphNow(graph.Metric, recordingChart), _keepUpdatingOnClose);
        recordingChart?.Windows.Add(window);
        _graphWindows.Add(window);
        if (recordingChart is not null)
        {
            var graphIndex = LegacyGraphRecordingRun.NativeIndexOf(graph.Metric);
            if (_nativeLayout?.Graphs.FirstOrDefault(row => row.GraphIndex == graphIndex) is { } layout)
                window.PlaceFromNativeLayout(this, layout);
            window.PositionChanged += (_, _) =>
            {
                if (window.ReadOnscreenNativePosition() is { } position)
                    RememberNativeLayout(new(graphIndex, Left: position.Left, Top: position.Top));
            };
            window.DiscardedByUser += () => RememberNativeLayout(new(graphIndex, Visible: false));
        }
        window.Closed += (_, _) =>
        {
            _graphWindows.Remove(window);
            if (recordingChart is not null)
            {
                recordingChart.Windows.Remove(window);
                if (recordingChart.Windows.Count == 0)
                {
                    _recordingCharts.Remove(graph.Metric);
                    recordingChart.Save.ResetPoints();
                    ViewModel.CloseGraph(graph.Metric);
                }
            }
            foreach (var item in series) ViewModel.RunningGraphs.Remove(item);
        };
        window.Show(this);
        if (!restoring && recordingChart is not null) RememberOpenedLayout(window);
    }

    private void RememberOpenedLayout(GraphSeriesWindow window)
    {
        var index = LegacyGraphRecordingRun.NativeIndexOf(window.Graph.Metric);
        var slot = index - LegacyGraphRecordingRun.NativeIndexOf(GraphMetric.CustomizableGraph1) + 1;
        var expression = window.Graph.CustomExpression;
        var position = window.ReadOnscreenNativePosition();
        RememberNativeLayout(new(index, true, position?.Left, position?.Top),
            slot is >= 1 and <= 3 && expression is not null ? new(slot, expression) : null);
    }

    private void RememberNativeLayout(NativeGraphLayoutRow change, NativeGraphCustomQuery? query = null)
    {
        if (_nativeLayout is not { } layout || _recording?.IsSuspended != false) return;
        var prior = layout.Graphs.FirstOrDefault(row => row.GraphIndex == change.GraphIndex);
        if (prior is null) return;
        var updated = new NativeGraphLayoutRow(prior.GraphIndex, change.Visible ?? prior.Visible,
            change.Left ?? prior.Left, change.Top ?? prior.Top);
        var queries = layout.CustomQueries;
        if (query is not null)
        {
            if (queries[query.Slot - 1] == query.Expression) query = null;
            else
            {
                var nextQueries = queries.ToArray();
                nextQueries[query.Slot - 1] = query.Expression;
                queries = nextQueries;
            }
        }
        if (updated == prior && query is null) return;
        _nativeLayout = new(layout.Graphs.Select(row => row == prior ? updated : row).ToArray(), queries);
        NativeLayoutEdited?.Invoke(_nativeLayout, change, query);
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
        foreach (var window in _graphWindows.ToArray()) window.CloseForCleanup();
        _graphWindows.Clear();
        NotifySamplingChanged(new GraphSamplingSettings());
        base.OnClosed(e);
    }
}
