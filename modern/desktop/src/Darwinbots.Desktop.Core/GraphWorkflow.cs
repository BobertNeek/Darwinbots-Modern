using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Globalization;
using System.Runtime.CompilerServices;
using System.Text;

namespace Darwinbots.Desktop.Core;

public enum GraphMetric
{
    Population,
    AverageMutations,
    AverageMutationsPerDnaLength,
    AverageAge,
    AverageDescendents,
    AverageEnergy,
    AverageDnaLength,
    AverageDnaConditions,
    TotalEnergy,
    DynamicCostStats,
    SpeciesDiversity,
    AverageChloroplasts,
    SelectiveGeneticDistance,
    GenerationalDistance,
    SlowGeneticDistance,
    CustomizableGraph1,
    CustomizableGraph2,
    CustomizableGraph3,
}

public enum GraphMenuAction
{
    None,
    ResetAllGraphs,
    ListAllRunningGraphs,
}

public sealed record GraphChoice(
    string Label,
    GraphMetric? Metric = null,
    GraphMenuAction Action = GraphMenuAction.None,
    bool IsSeparatorBefore = false);

public static class GraphCatalog
{
    public static IReadOnlyList<GraphChoice> Items { get; } =
    [
        new("Population", GraphMetric.Population),
        new("Average mutations", GraphMetric.AverageMutations),
        new("Average mutations/DNA len", GraphMetric.AverageMutationsPerDnaLength),
        new("Average Age", GraphMetric.AverageAge),
        new("Average Descendents", GraphMetric.AverageDescendents),
        new("Average Energy", GraphMetric.AverageEnergy),
        new("Average DNA length", GraphMetric.AverageDnaLength),
        new("Average DNA conditions", GraphMetric.AverageDnaConditions),
        new("Total Energy", GraphMetric.TotalEnergy),
        new("Dynamic Cost Stats", GraphMetric.DynamicCostStats),
        new("Species Diversity", GraphMetric.SpeciesDiversity),
        new("Average Chloroplasts", GraphMetric.AverageChloroplasts),
        new("Selective Genetic Distance", GraphMetric.SelectiveGeneticDistance, IsSeparatorBefore: true),
        new("Generational Distance", GraphMetric.GenerationalDistance),
        new("Slow Genetic Distance", GraphMetric.SlowGeneticDistance),
        new("Customizable Graph 1", GraphMetric.CustomizableGraph1, IsSeparatorBefore: true),
        new("Customizable Graph 2", GraphMetric.CustomizableGraph2),
        new("Customizable Graph 3", GraphMetric.CustomizableGraph3),
        new("Reset all graphs", Action: GraphMenuAction.ResetAllGraphs, IsSeparatorBefore: true),
        new("List all running graphs", Action: GraphMenuAction.ListAllRunningGraphs),
    ];

    public static IReadOnlyList<GraphChoice> Metrics { get; } = Items.Where(value => value.Metric is not null).ToArray();

    public static GraphChoice For(GraphMetric metric) =>
        Items.Single(value => value.Metric == metric);
}

public sealed record GraphMetricSample(
    ulong Tick,
    IReadOnlyDictionary<GraphMetric, double> Values);

public sealed record GraphSample(ulong Tick, double Value);

public sealed record GraphDifference(ulong Tick, double Difference);

public sealed record GraphComparison(
    string LeftTitle,
    string RightTitle,
    IReadOnlyList<GraphDifference> Differences)
{
    public string Summary =>
        Differences.Count == 0
            ? "NO OVERLAPPING SAMPLES"
            : $"{LeftTitle} − {RightTitle} · {Differences.Count:N0} SAMPLES";
}

public sealed class CustomMetricSlotViewModel : INotifyPropertyChanged
{
    private string _name;
    private string _expression;

    public CustomMetricSlotViewModel(int slot, string name, string expression)
    {
        Slot = slot;
        _name = name;
        _expression = expression;
    }

    public int Slot { get; }
    public string Name { get => _name; set => Set(ref _name, value); }
    public string Expression { get => _expression; set => Set(ref _expression, value); }

    public event PropertyChangedEventHandler? PropertyChanged;

    private void Set<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value)) return;
        field = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
}

public sealed class GraphSeriesViewModel : INotifyPropertyChanged
{
    private IReadOnlyList<GraphSample> _samples;
    private bool _isSelected;

    public GraphSeriesViewModel(string id, GraphMetric metric, string title, IReadOnlyList<GraphSample> samples)
    {
        Id = id;
        Metric = metric;
        Title = title;
        _samples = samples;
    }

    public string Id { get; }
    public GraphMetric Metric { get; }
    public string Title { get; }
    public IReadOnlyList<GraphSample> Samples { get => _samples; private set => Set(ref _samples, value); }
    public bool IsSelected { get => _isSelected; set => Set(ref _isSelected, value); }
    public double CurrentValue => Samples.Count == 0 ? 0 : Samples[^1].Value;
    public string CurrentValueText => CurrentValue.ToString("N1", CultureInfo.InvariantCulture);

    public event PropertyChangedEventHandler? PropertyChanged;

    public void Update(IReadOnlyList<GraphSample> samples)
    {
        Samples = samples;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(CurrentValue)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(CurrentValueText)));
    }

    public void Reset() => Update([]);

    private void Set<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value)) return;
        field = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
}

public sealed class GraphSuiteViewModel : INotifyPropertyChanged
{
    private IReadOnlyList<GraphMetricSample> _samples;
    private GraphChoice _selectedChoice = GraphCatalog.Metrics[0];
    private string _status = "SELECT A GRAPH";
    private string _comparisonText = "NO COMPARISON";

    public GraphSuiteViewModel(IReadOnlyList<GraphMetricSample>? samples = null)
    {
        _samples = samples ?? [];
        CustomMetrics =
        [
            new CustomMetricSlotViewModel(1, "Custom graph 1", "population"),
            new CustomMetricSlotViewModel(2, "Custom graph 2", "total_energy"),
            new CustomMetricSlotViewModel(3, "Custom graph 3", "average_energy"),
        ];
    }

    public IReadOnlyList<GraphChoice> Choices => GraphCatalog.Items;
    public ObservableCollection<GraphSeriesViewModel> RunningGraphs { get; } = [];
    public ObservableCollection<CustomMetricSlotViewModel> CustomMetrics { get; }
    public GraphChoice SelectedChoice
    {
        get => _selectedChoice;
        set => Set(ref _selectedChoice, value);
    }

    public string Status { get => _status; private set => Set(ref _status, value); }
    public string ComparisonText { get => _comparisonText; private set => Set(ref _comparisonText, value); }

    public event PropertyChangedEventHandler? PropertyChanged;

    public void Update(EngineSnapshot snapshot)
    {
        ArgumentNullException.ThrowIfNull(snapshot);
        _samples = GraphMetricSampleProjector.FromSnapshot(snapshot);
        foreach (var graph in RunningGraphs)
            graph.Update(GraphMetricSampleProjector.Project(_samples, graph.Metric, CustomMetric(graph.Metric)));
    }

    public void SetSamples(IReadOnlyList<GraphMetricSample> samples)
    {
        _samples = samples ?? throw new ArgumentNullException(nameof(samples));
        foreach (var graph in RunningGraphs)
            graph.Update(GraphMetricSampleProjector.Project(_samples, graph.Metric, CustomMetric(graph.Metric)));
    }

    public void ConfigureCustomMetric(int slot, string name, string expression)
    {
        var custom = CustomMetrics.SingleOrDefault(value => value.Slot == slot)
            ?? throw new ArgumentOutOfRangeException(nameof(slot), "Custom graph slots are 1, 2, and 3.");
        custom.Name = string.IsNullOrWhiteSpace(name) ? $"Custom graph {slot}" : name.Trim();
        custom.Expression = string.IsNullOrWhiteSpace(expression) ? "0" : expression.Trim();
        foreach (var graph in RunningGraphs.Where(value => value.Metric == CustomMetric(slot)))
            graph.Update(GraphMetricSampleProjector.Project(_samples, graph.Metric, custom));
    }

    public GraphSeriesViewModel? OpenSelectedGraph()
    {
        if (SelectedChoice.Action == GraphMenuAction.ResetAllGraphs)
        {
            ResetAllGraphs();
            return null;
        }
        if (SelectedChoice.Action == GraphMenuAction.ListAllRunningGraphs)
        {
            ListAllRunningGraphs();
            return null;
        }
        if (SelectedChoice.Metric is not { } metric) return null;

        var custom = CustomMetric(metric);
        var title = custom?.Name ?? SelectedChoice.Label;
        var graph = new GraphSeriesViewModel(
            $"graph-{RunningGraphs.Count + 1}",
            metric,
            title,
            GraphMetricSampleProjector.Project(_samples, metric, custom));
        RunningGraphs.Add(graph);
        Status = $"RUNNING · {title.ToUpperInvariant()}";
        return graph;
    }

    public void ResetAllGraphs()
    {
        foreach (var graph in RunningGraphs) graph.Reset();
        ComparisonText = "NO COMPARISON";
        Status = "ALL GRAPHS RESET";
    }

    public string ListAllRunningGraphs()
    {
        var list = RunningGraphs.Count == 0
            ? "NO RUNNING GRAPHS"
            : string.Join(" · ", RunningGraphs.Select(value => value.Title));
        Status = list;
        return list;
    }

    public string ExportCsv() => GraphExport.Serialize(RunningGraphs);

    public GraphComparison? JoinAndCompare(IReadOnlyList<GraphSeriesViewModel> graphs)
    {
        ArgumentNullException.ThrowIfNull(graphs);
        if (graphs.Count != 2)
        {
            Status = "SELECT EXACTLY TWO GRAPHS TO JOIN / COMPARE";
            return null;
        }

        var left = graphs[0];
        var right = graphs[1];
        var leftValues = left.Samples.ToDictionary(value => value.Tick, value => value.Value);
        var rightValues = right.Samples.ToDictionary(value => value.Tick, value => value.Value);
        var differences = leftValues.Keys.Union(rightValues.Keys).Order()
            .Select(tick => new GraphDifference(
                tick,
                leftValues.GetValueOrDefault(tick) - rightValues.GetValueOrDefault(tick)))
            .ToArray();
        var comparison = new GraphComparison(left.Title, right.Title, differences);
        ComparisonText = comparison.Summary;
        Status = $"JOINED · {comparison.Summary}";
        return comparison;
    }

    private CustomMetricSlotViewModel? CustomMetric(GraphMetric metric) =>
        metric switch
        {
            GraphMetric.CustomizableGraph1 => CustomMetrics[0],
            GraphMetric.CustomizableGraph2 => CustomMetrics[1],
            GraphMetric.CustomizableGraph3 => CustomMetrics[2],
            _ => null,
        };

    private static GraphMetric CustomMetric(int slot) => slot switch
    {
        1 => GraphMetric.CustomizableGraph1,
        2 => GraphMetric.CustomizableGraph2,
        3 => GraphMetric.CustomizableGraph3,
        _ => throw new ArgumentOutOfRangeException(nameof(slot)),
    };

    private void Set<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value)) return;
        field = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
}

public static class GraphMetricSampleProjector
{
    public static IReadOnlyList<GraphMetricSample> FromSnapshot(EngineSnapshot snapshot)
    {
        var currentAverageAge = snapshot.Organisms.Count == 0 ? 0 : snapshot.Organisms.Average(value => (double)value.Age);
        var currentAverageEnergy = snapshot.Organisms.Count == 0 ? 0 : snapshot.Organisms.Average(value => value.Energy);
        var currentAverageChloroplasts = snapshot.Organisms.Count == 0 ? 0 : snapshot.Organisms.Average(value => value.Chloroplasts);
        var history = snapshot.History.Count == 0
            ? [new HistorySampleSnapshot(snapshot.Tick, snapshot.Population, snapshot.Stats.TotalEnergy, snapshot.Stats.Births, snapshot.Stats.Deaths, snapshot.Stats.Mutations, snapshot.Stats.ShotsFired)]
            : snapshot.History;
        return history.Select(sample => new GraphMetricSample(sample.Tick, new Dictionary<GraphMetric, double>
        {
            [GraphMetric.Population] = sample.Population,
            [GraphMetric.AverageMutations] = sample.Population == 0 ? 0 : (double)sample.Mutations / sample.Population,
            [GraphMetric.AverageMutationsPerDnaLength] = sample.Population == 0 ? 0 : (double)sample.Mutations / sample.Population,
            [GraphMetric.AverageAge] = currentAverageAge,
            [GraphMetric.AverageDescendents] = sample.Population == 0 ? 0 : (double)sample.Births / sample.Population,
            [GraphMetric.AverageEnergy] = sample.Population == 0 ? 0 : (double)sample.TotalEnergy / sample.Population,
            [GraphMetric.AverageDnaLength] = 0,
            [GraphMetric.AverageDnaConditions] = 0,
            [GraphMetric.TotalEnergy] = sample.TotalEnergy,
            [GraphMetric.DynamicCostStats] = 0,
            [GraphMetric.SpeciesDiversity] = snapshot.Species.Count,
            [GraphMetric.AverageChloroplasts] = currentAverageChloroplasts,
            [GraphMetric.SelectiveGeneticDistance] = 0,
            [GraphMetric.GenerationalDistance] = 0,
            [GraphMetric.SlowGeneticDistance] = 0,
            [GraphMetric.CustomizableGraph1] = currentAverageEnergy,
            [GraphMetric.CustomizableGraph2] = sample.TotalEnergy,
            [GraphMetric.CustomizableGraph3] = currentAverageChloroplasts,
        })).ToArray();
    }

    public static IReadOnlyList<GraphSample> Project(
        IReadOnlyList<GraphMetricSample> samples,
        GraphMetric metric,
        CustomMetricSlotViewModel? custom)
    {
        return samples.Select(sample => new GraphSample(
            sample.Tick,
            custom is null
                ? sample.Values.GetValueOrDefault(metric)
                : GraphExpressionEvaluator.Evaluate(custom.Expression, sample))).ToArray();
    }
}

public static class GraphExport
{
    public static string Serialize(IEnumerable<GraphSeriesViewModel> graphs)
    {
        var builder = new StringBuilder("series,tick,value\n");
        foreach (var graph in graphs)
        foreach (var sample in graph.Samples)
            builder.Append(graph.Title.Replace(',', ';'))
                .Append(',')
                .Append(sample.Tick.ToString(CultureInfo.InvariantCulture))
                .Append(',')
                .AppendLine(sample.Value.ToString("R", CultureInfo.InvariantCulture));
        return builder.ToString();
    }
}

internal static class GraphExpressionEvaluator
{
    public static double Evaluate(string expression, GraphMetricSample sample)
    {
        try
        {
            var parser = new Parser(expression, sample);
            var value = parser.ParseExpression();
            return parser.AtEnd ? value : 0;
        }
        catch (FormatException)
        {
            return 0;
        }
    }

    private sealed class Parser
    {
        private readonly string _text;
        private readonly GraphMetricSample _sample;
        private int _position;

        public Parser(string text, GraphMetricSample sample)
        {
            _text = text;
            _sample = sample;
        }

        public bool AtEnd
        {
            get
            {
                SkipWhitespace();
                return _position == _text.Length;
            }
        }

        public double ParseExpression()
        {
            var value = ParseProduct();
            while (true)
            {
                SkipWhitespace();
                if (!Match('+') && !Match('-')) return value;
                var operation = _text[_position - 1];
                var right = ParseProduct();
                value = operation == '+' ? value + right : value - right;
            }
        }

        private double ParseProduct()
        {
            var value = ParsePrimary();
            while (true)
            {
                SkipWhitespace();
                if (!Match('*') && !Match('/')) return value;
                var operation = _text[_position - 1];
                var right = ParsePrimary();
                value = operation == '*'
                    ? value * right
                    : right == 0 ? 0 : value / right;
            }
        }

        private double ParsePrimary()
        {
            SkipWhitespace();
            if (Match('('))
            {
                var value = ParseExpression();
                if (!Match(')')) throw new FormatException();
                return value;
            }
            if (Match('-')) return -ParsePrimary();
            if (_position < _text.Length && (char.IsDigit(_text[_position]) || _text[_position] == '.'))
            {
                var start = _position;
                while (_position < _text.Length && (char.IsDigit(_text[_position]) || _text[_position] == '.')) _position++;
                return double.Parse(_text[start.._position], CultureInfo.InvariantCulture);
            }
            var identifierStart = _position;
            while (_position < _text.Length && (char.IsLetterOrDigit(_text[_position]) || _text[_position] == '_')) _position++;
            if (identifierStart == _position) throw new FormatException();
            var identifier = _text[identifierStart.._position].ToLowerInvariant();
            return Resolve(identifier, _sample);
        }

        private bool Match(char value)
        {
            if (_position >= _text.Length || _text[_position] != value) return false;
            _position++;
            return true;
        }

        private void SkipWhitespace()
        {
            while (_position < _text.Length && char.IsWhiteSpace(_text[_position])) _position++;
        }

        private static double Resolve(string identifier, GraphMetricSample sample)
        {
            var metric = identifier switch
            {
                "population" => GraphMetric.Population,
                "average_mutations" or "mutations" => GraphMetric.AverageMutations,
                "average_mutations_dna_len" => GraphMetric.AverageMutationsPerDnaLength,
                "average_age" or "age" => GraphMetric.AverageAge,
                "average_descendents" or "descendents" => GraphMetric.AverageDescendents,
                "average_energy" or "energy" => GraphMetric.AverageEnergy,
                "average_dna_length" => GraphMetric.AverageDnaLength,
                "average_dna_conditions" => GraphMetric.AverageDnaConditions,
                "total_energy" => GraphMetric.TotalEnergy,
                "dynamic_cost_stats" => GraphMetric.DynamicCostStats,
                "species_diversity" => GraphMetric.SpeciesDiversity,
                "average_chloroplasts" or "chloroplasts" => GraphMetric.AverageChloroplasts,
                "selective_genetic_distance" => GraphMetric.SelectiveGeneticDistance,
                "generational_distance" => GraphMetric.GenerationalDistance,
                "slow_genetic_distance" => GraphMetric.SlowGeneticDistance,
                _ => throw new FormatException(),
            };
            return sample.Values.GetValueOrDefault(metric);
        }
    }
}
