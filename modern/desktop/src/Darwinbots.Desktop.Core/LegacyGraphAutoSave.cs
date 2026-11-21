using System.Globalization;

namespace Darwinbots.Desktop.Core;

public sealed class LegacyGraphRecordingRun
{
    private readonly Func<string> _root;
    private readonly Dictionary<GraphMetric, LegacyGraphAutoSave> _graphs = [];
    private int _generation;
    private static readonly GraphMetric[] NativeGraphOrder =
    [
        GraphMetric.Population, GraphMetric.AverageMutations, GraphMetric.AverageAge,
        GraphMetric.AverageDescendents, GraphMetric.AverageEnergy, GraphMetric.AverageDnaLength,
        GraphMetric.AverageDnaConditions, GraphMetric.AverageMutationsPerDnaLength, GraphMetric.TotalEnergy,
        GraphMetric.DynamicCostStats, GraphMetric.SpeciesDiversity, GraphMetric.AverageChloroplasts,
        GraphMetric.SlowGeneticDistance, GraphMetric.GenerationalDistance, GraphMetric.SelectiveGeneticDistance,
        GraphMetric.CustomizableGraph1, GraphMetric.CustomizableGraph2, GraphMetric.CustomizableGraph3,
    ];

    public LegacyGraphRecordingRun(string root, string runStamp) : this(() => root, runStamp) { }

    public LegacyGraphRecordingRun(Func<string> root, string runStamp)
    {
        _root = root;
        RunStamp = ValidateRunStamp(runStamp);
    }

    public string RunStamp { get; private set; }
    public int Generation => Volatile.Read(ref _generation);
    public bool IsSuspended { get; set; }
    public event Action<int, NativeGraphRecordingRow>? NativeSettingsChanged;

    public static string NewRunStamp() => DateTime.Now.ToString("yyyy-MM-dd HH-mm-ss.fff", CultureInfo.InvariantCulture);

    public static int NativeIndexOf(GraphMetric metric) => Array.IndexOf(NativeGraphOrder, metric);

    public static GraphMetric? NativeMetricAt(int index) =>
        index >= 0 && index < NativeGraphOrder.Length ? NativeGraphOrder[index] : null;

    public LegacyGraphAutoSave Graph(GraphMetric metric)
    {
        if (!_graphs.TryGetValue(metric, out var graph))
        {
            graph = new LegacyGraphAutoSave(() => Path.Combine(_root(), RunStamp), Caption(metric));
            _graphs.Add(metric, graph);
            var index = NativeIndexOf(metric);
            graph.NativeSettingsChanged += () => NativeSettingsChanged?.Invoke(
                Generation, new(index, graph.Enabled, graph.Counter));
        }
        return graph;
    }

    public void BeginRun(string runStamp)
    {
        RunStamp = ValidateRunStamp(runStamp);
        Interlocked.Increment(ref _generation);
        foreach (var graph in _graphs.Values) graph.BeginRun();
    }

    public void RestoreNativeState(NativeGraphRecordingState? state)
    {
        BeginRun(state is not null && IsLocalRunStamp(state.RunStamp) ? state.RunStamp : NewRunStamp());
        if (state is null) return;
        foreach (var row in state.Graphs)
            if (NativeMetricAt(row.GraphIndex) is { } metric)
                Graph(metric).RestoreNativeSettings(row.Enabled, Math.Max(0, row.Counter));
    }

    private static string ValidateRunStamp(string value) => IsLocalRunStamp(value) ? value
        : throw new ArgumentException("A graph run stamp must be one local folder name.", nameof(value));

    private static bool IsLocalRunStamp(string value) => !string.IsNullOrWhiteSpace(value)
        && value is not "." and not ".." && value.IndexOfAny(['/', '\\', ':']) == -1
        && value.IndexOfAny(Path.GetInvalidFileNameChars()) == -1;

    public static string Caption(GraphMetric metric) => metric switch
    {
        GraphMetric.Population => "Populations",
        GraphMetric.AverageMutations => "Average_Mutations",
        GraphMetric.AverageAge => "Average_Age",
        GraphMetric.AverageDescendents => "Average_Offspring",
        GraphMetric.AverageEnergy => "Average_Energy",
        GraphMetric.AverageDnaLength => "Average_DNA_length",
        GraphMetric.AverageDnaConditions => "Average_DNA_Cond_statements",
        GraphMetric.AverageMutationsPerDnaLength => "Average_Mutations_per_DNA_length_x1000-",
        GraphMetric.TotalEnergy => "Total_Energy_per_Species_x1000-",
        GraphMetric.DynamicCostStats => "Dynamic_Costs",
        GraphMetric.SpeciesDiversity => "Species_Diversity",
        GraphMetric.AverageChloroplasts => "Average_Chloroplasts",
        GraphMetric.SlowGeneticDistance => "Genetic_Distance_x1000-",
        GraphMetric.GenerationalDistance => "Max_Generational_Distance",
        GraphMetric.SelectiveGeneticDistance => "Simple_Genetic_Distance_x1000-",
        GraphMetric.CustomizableGraph1 => "Customizable_Graph_1-",
        GraphMetric.CustomizableGraph2 => "Customizable_Graph_2-",
        GraphMetric.CustomizableGraph3 => "Customizable_Graph_3-",
        _ => throw new ArgumentOutOfRangeException(nameof(metric)),
    };
}

public sealed record LegacyGraphPointValue(string Name, uint Color, double Value);

public sealed class LegacyGraphAutoSave(Func<string> directory, string caption)
{
    private const int RingLength = 1001;
    private const int MaximumColumns = 65;
    private readonly List<Column> _columns = [];
    private readonly Queue<string> _pending = [];
    private int _pivot;

    public bool Enabled { get; private set; }
    public int Counter { get; private set; }
    public string Status { get; private set; } = "";
    public event Action? Changed;
    internal event Action? NativeSettingsChanged;

    public void SetEnabled(bool enabled)
    {
        if (Enabled == enabled) return;
        Enabled = enabled;
        NativeSettingsChanged?.Invoke();
        if (enabled) FlushPending();
        Changed?.Invoke();
    }

    public void AddPoint(IEnumerable<LegacyGraphPointValue> point, string header, bool autoFork = false)
    {
        foreach (var value in point)
        {
            var sample = (float)value.Value;
            if (sample == 0 || !float.IsFinite(sample)) continue;
            var column = _columns.Find(column => column.Name == value.Name);
            if (column is null)
            {
                column = new(value.Name, value.Color);
                if (_columns.Count < MaximumColumns) _columns.Add(column);
                else
                {
                    // VB6 reassigns the final slot when the series table is full.
                    column = column with { Values = _columns[^1].Values };
                    _columns[^1] = column;
                }
            }
            column.Values[_pivot] = sample;
        }

        var latest = _pivot;
        _pivot = (_pivot + 1) % RingLength;
        foreach (var column in _columns) column.Values[_pivot] = 0;
        if (_pivot != 0)
        {
            // grafico.ReorderSeries swaps on strict comparisons, including indirect ties.
            for (var first = 0; first < _columns.Count; first++)
            for (var next = first + 1; next < _columns.Count; next++)
                if (_columns[first].Values[latest] < _columns[next].Values[latest])
                    (_columns[first], _columns[next]) = (_columns[next], _columns[first]);
            var seriesCount = _columns.Count;
            if (seriesCount > 10) _columns.RemoveAll(column => SumInRingOrder(column) == 0);
            if (seriesCount > 50 || autoFork) _columns.RemoveAll(column => column.Values[latest] == 0);
        }

        if (!Enabled) return;
        if (_pivot == LegacyGraphJoin.SamplesPerSegment)
        {
            var series = _columns.Select(column => new LegacyGraphRobot(column.Name, column.Color,
                column.Values.Take(LegacyGraphJoin.SamplesPerSegment).Select(value => (double)value).ToArray()))
                .ToArray();
            _pending.Enqueue(LegacyGraphJoin.SerializeNativeSegment(header, series));
        }
        FlushPending();
    }

    private float SumInRingOrder(Column column)
    {
        float sum = 0;
        for (var offset = 1; offset < RingLength; offset++)
            sum += column.Values[(_pivot + offset) % RingLength];
        return sum;
    }

    public void ResetPoints()
    {
        _pivot = 0;
        _columns.Clear();
    }

    internal void BeginRun()
    {
        ResetPoints();
        _pending.Clear();
        Enabled = false;
        Counter = 0;
        Status = "";
        Changed?.Invoke();
    }

    internal void RestoreNativeSettings(bool enabled, int counter)
    {
        Enabled = enabled;
        Counter = counter;
        Changed?.Invoke();
    }

    private static void RequireMatchingSegment(string path, byte[] segment)
    {
        if (new FileInfo(path).Length == segment.Length && File.ReadAllBytes(path).AsSpan().SequenceEqual(segment))
            return;
        throw new IOException($"COLLISION · {Path.GetFileName(path)} already contains different graph data.");
    }

    private void FlushPending()
    {
        while (_pending.TryPeek(out var segment))
        {
            string? temporary = null;
            try
            {
                var folder = directory();
                Directory.CreateDirectory(folder);
                if (Counter == int.MaxValue) throw new IOException("The original graph segment counter is full.");
                var next = Counter + 1;
                var fileName = caption + next.ToString(CultureInfo.InvariantCulture) + ".gsave";
                var destination = Path.Combine(folder, fileName);
                var bytes = LegacyDnaText.Encode(segment);
                if (File.Exists(destination)) RequireMatchingSegment(destination, bytes);
                else
                {
                    temporary = Path.Combine(folder, $".{Guid.NewGuid():N}.tmp");
                    File.WriteAllBytes(temporary, bytes);
                    try
                    {
                        File.Move(temporary, destination);
                        temporary = null;
                    }
                    catch (IOException) when (File.Exists(destination))
                    {
                        RequireMatchingSegment(destination, bytes);
                    }
                }
                _pending.Dequeue();
                Counter = next;
                Status = $"SAVED {fileName}";
                NativeSettingsChanged?.Invoke();
                Changed?.Invoke();
            }
            catch (Exception error) when (error is IOException or UnauthorizedAccessException or ArgumentException)
            {
                var failure = $"GRAPH SAVE WAITING · {error.Message}";
                if (Status != failure)
                {
                    Status = failure;
                    Changed?.Invoke();
                }
                return;
            }
            finally
            {
                if (temporary is not null)
                {
                    try { File.Delete(temporary); }
                    catch (Exception error) when (error is IOException or UnauthorizedAccessException) { }
                }
            }
        }
    }

    private sealed record Column(string Name, uint Color)
    {
        public float[] Values { get; init; } = new float[RingLength];
    }
}
