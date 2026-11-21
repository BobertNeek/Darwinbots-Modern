using Avalonia.Controls;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class GraphSuiteWindow : Window, IPersistentSnapshotToolWindow
{
    private readonly List<GraphSeriesWindow> _graphWindows = [];
    private GraphSamplingSettings _lastSampling = new();
    private readonly LegacyGraphRecordingRun? _recording;
    private readonly bool _keepUpdatingOnClose;
    private readonly bool _nativeOldColors;
    private readonly Func<bool>? _autoFork;
    private readonly Func<GraphSamplingSettings?, Action<NativeGraphRecordingSample?>, Task>? _consumeNativeSample;
    private readonly Func<Action<NativeGraphRecordingSample?>, Task>? _consumeNativeResetSample;
    private static readonly IReadOnlyDictionary<GraphMetric, double> EmptyMetrics = new Dictionary<GraphMetric, double>();
    private readonly Dictionary<GraphMetric, RecordingChart> _recordingCharts = [];
    private EngineSnapshot _snapshot = EngineSnapshot.Empty;
    private uint? _nativeChartArgb;
    private NativeGraphLayoutState? _nativeLayout;
    private Task _resetAllFeed = Task.CompletedTask;

    public GraphSuiteWindow() : this(EngineSnapshot.Empty)
    {
    }

    public GraphSuiteWindow(EngineSnapshot snapshot, LegacyGraphRecordingRun? recording = null, Func<bool>? autoFork = null,
        Func<Task<NativeGraphRecordingSample?>>? readNativeSample = null,
        Func<GraphSamplingSettings?, Action<NativeGraphRecordingSample?>, Task>? consumeNativeSample = null,
        bool keepUpdatingOnClose = false,
        Func<Action<NativeGraphRecordingSample?>, Task>? consumeNativeResetSample = null,
        Func<Func<Task<bool>>, Func<Task>, Task>? runNativeResetModal = null,
        bool nativeOldColors = false)
    {
        _recording = recording;
        _keepUpdatingOnClose = keepUpdatingOnClose;
        _nativeOldColors = nativeOldColors;
        _autoFork = autoFork;
        _consumeNativeSample = consumeNativeSample ?? (readNativeSample is null ? null
            : async (_, consume) => consume(await readNativeSample()));
        _consumeNativeResetSample = consumeNativeResetSample;
        InitializeComponent();
        GraphView.GraphWindowRequested += OpenGraphWindow;
        GraphView.ReopenGraphRequested = ReopenGraphWindow;
        GraphView.ResetAllRequestedAsync = confirm => ResetAllAsync(confirm, runNativeResetModal);
        SetSnapshot(snapshot);
        ViewModel.GraphsReset += () => _resetAllFeed = ResetRecordingChartsAsync(_recordingCharts, feedImmediately: true, resetSweep: true);
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

    private bool UsesNativeSamples => _recording is not null && _consumeNativeSample is not null && _snapshot.NativeCycle.HasValue;

    internal bool WantsNativeGraphSamples => UsesNativeSamples && IsVisible && _recording?.IsSuspended == false
        && _recordingCharts.Values.Any(chart => chart.Windows.Any(window => window.IsCollecting));

    public void SetSnapshot(EngineSnapshot snapshot)
    {
        var usedNativeSamples = UsesNativeSamples;
        _snapshot = snapshot;
        SetScenePalette(snapshot);
        var usesNativeSamples = UsesNativeSamples;
        ViewModel.SetNativeFeed(usesNativeSamples);
        GraphView.SetSnapshot(snapshot);
        if (usedNativeSamples != usesNativeSamples)
        {
            foreach (var chart in _recordingCharts.Values) RefreshChartSeries(chart);
        }
        if (_recording is null || _recording.IsSuspended) return;
        var reset = _recordingCharts
            .Where(pair => pair.Value.Generation != _recording.Generation || usedNativeSamples != usesNativeSamples
                || (!usesNativeSamples && snapshot.Tick < pair.Value.MostRecentTick))
            .ToArray();
        if (reset.Length > 0)
        {
            if (usesNativeSamples)
                foreach (var (metric, _) in reset) ViewModel.ResetGraph(metric, preserveImported: true);
            _ = ResetRecordingChartsAsync(reset, feedImmediately: true);
        }
        if (usesNativeSamples || snapshot.History.Count == 0) return;
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

    internal void SetScenePalette(EngineSnapshot snapshot)
    {
        var chartArgb = snapshot.NativeCycle.HasValue
            ? _nativeOldColors ? 0xffffffffu : snapshot.NativeScenePalette?.ChartArgb
            : null;
        if (_nativeChartArgb == chartArgb) return;
        _nativeChartArgb = chartArgb;
        foreach (var window in _graphWindows)
            window.SetNativeChartBackground(chartArgb);
    }

    public void RecordNativeSamples(IReadOnlyList<NativeGraphRecordingSample> samples, int generation)
    {
        if (!WantsNativeGraphSamples || _recording?.Generation != generation) return;
        var points = samples.Select(ProjectNativePoint).ToArray();
        foreach (var (metric, chart) in _recordingCharts)
        {
            if (chart.Generation != generation || chart.AwaitingNativeRead || !chart.Windows.Any(window => window.IsCollecting))
                continue;
            List<GraphMetricSample> accepted = [];
            foreach (var sample in points)
            {
                if (sample.Tick <= chart.MostRecentTick) continue;
                RecordPoint(metric, chart, sample);
                chart.MostRecentTick = sample.Tick;
                accepted.Add(sample);
            }
            AppendChartPoints(metric, chart, accepted);
        }
    }

    private async Task ResetAllAsync(Func<Task<bool>> confirm, Func<Func<Task<bool>>, Func<Task>, Task>? runNativeResetModal)
    {
        if (UsesNativeSamples && runNativeResetModal is not null)
            await runNativeResetModal(confirm, ResetAndFeedAsync);
        else if (await confirm())
            await ResetAndFeedAsync();

        Task ResetAndFeedAsync()
        {
            ViewModel.ResetAllGraphs();
            return _resetAllFeed;
        }
    }

    private Task ResetRecordingChartsAsync(IEnumerable<KeyValuePair<GraphMetric, RecordingChart>> charts, bool feedImmediately,
        bool resetSweep = false)
    {
        if (_recording is null || _recording.IsSuspended) return Task.CompletedTask;
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
            {
                if (resetSweep) AppendImmediatePoint(metric, chart, sample);
                else RecordPoint(metric, chart, sample);
            }
        }
        if (pendingReads.Count > 0 && _consumeNativeSample is { } capture)
        {
            var generation = _recording.Generation;
            void Complete(NativeGraphRecordingSample? sample) =>
                FinishNativeRead(pendingReads, generation, feedImmediately, resetSweep, sample);
            if (resetSweep && _consumeNativeResetSample is { } captureReset)
                return captureReset(Complete);

            GraphSamplingSettings? sampling = !feedImmediately ? null : resetSweep ? new(Generational: true)
                : SamplingFor(pendingReads.Select(entry => entry.Metric));
            return capture(sampling, Complete);
        }
        return Task.CompletedTask;
    }

    private void FinishNativeRead(List<(GraphMetric Metric, RecordingChart Chart, int Version)> pending,
        int generation, bool feedImmediately, bool resetSweep, NativeGraphRecordingSample? sample)
    {
        if (_recording is not { } recording || recording.Generation != generation || recording.IsSuspended) return;

        foreach (var (metric, chart, version) in pending)
        {
            if (chart.Version != version || !_recordingCharts.TryGetValue(metric, out var active) || active != chart)
                continue;

            chart.AwaitingNativeRead = false;
            if (sample is null) continue;
            chart.MostRecentTick = sample.Tick;
            if (feedImmediately)
                AppendImmediatePoint(metric, chart, resetSweep ? ProjectResetSweepPoint(sample, metric) : ProjectNativePoint(sample));
        }
    }

    private static GraphMetricSample ProjectNativePoint(NativeGraphRecordingSample sample) =>
        new(sample.Tick, EmptyMetrics, sample.SpeciesGraphs, sample.DynamicCostSeries);

    private static GraphMetricSample ProjectResetSweepPoint(NativeGraphRecordingSample sample, GraphMetric metric)
    {
        var point = ProjectNativePoint(sample) with { NativeResetSweep = true };
        // FeedGraph(0) carries the dynamic chart's one-row limit into the remaining graphs.
        return LegacyGraphRecordingRun.NativeIndexOf(metric) > LegacyGraphRecordingRun.NativeIndexOf(GraphMetric.DynamicCostStats)
            ? point with { SpeciesGraphs = sample.SpeciesGraphs.Take(1).ToArray() }
            : point;
    }

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
        AppendChartPoints(metric, chart, [sample]);
    }

    private void AppendChartPoints(GraphMetric metric, RecordingChart chart, IReadOnlyList<GraphMetricSample> samples)
    {
        var before = ViewModel.RunningGraphs.Count;
        ViewModel.AppendChartPoints(metric, samples, chart.Save);
        if (ViewModel.RunningGraphs.Count == before) return;
        RefreshChartSeries(chart);
        GraphView.RefreshChart();
    }

    private void RefreshChartSeries(RecordingChart chart)
    {
        foreach (var window in chart.Windows) window.SetSeries(GraphView.SeriesFor(window.Graph));
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
        RecordingChart? recordingChart = null;
        if (_recording is not null && !graph.Id.StartsWith("legacy-", StringComparison.Ordinal))
        {
            if (!_recordingCharts.TryGetValue(graph.Metric, out recordingChart))
            {
                recordingChart = new RecordingChart(_recording.Graph(graph.Metric));
                _recordingCharts.Add(graph.Metric, recordingChart);
                _ = ResetRecordingChartsAsync([new(graph.Metric, recordingChart)], feedImmediately: true);
            }
        }
        var series = GraphView.SeriesFor(graph);
        var window = new GraphSeriesWindow(graph, series, recordingChart?.Save, recordingChart is null ? null : () =>
        {
            ViewModel.ResetGraph(graph.Metric);
            _ = ResetRecordingChartsAsync([new(graph.Metric, recordingChart)], feedImmediately: false);
        }, recordingChart is null ? null : () => UpdateGraphNow(graph.Metric, recordingChart), _keepUpdatingOnClose);
        window.SetNativeChartBackground(_nativeChartArgb);
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
            foreach (var item in window.Series) ViewModel.RunningGraphs.Remove(item);
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
