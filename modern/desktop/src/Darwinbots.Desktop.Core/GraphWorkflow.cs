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
        new("Average Age", GraphMetric.AverageAge),
        new("Average Descendents", GraphMetric.AverageDescendents),
        new("Average Energy", GraphMetric.AverageEnergy),
        new("Average DNA length", GraphMetric.AverageDnaLength),
        new("Average DNA conditions", GraphMetric.AverageDnaConditions),
        new("Average mutations/DNA len", GraphMetric.AverageMutationsPerDnaLength),
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
    IReadOnlyDictionary<GraphMetric, double> Values,
    IReadOnlyList<SpeciesGraphSampleSnapshot>? SpeciesGraphs = null,
    IReadOnlyList<double>? DynamicCostSeries = null);

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
    private string _title;
    private string? _customExpression;
    private bool _isSelected;

    public GraphSeriesViewModel(
        string id,
        GraphMetric metric,
        string title,
        IReadOnlyList<GraphSample> samples,
        string? customExpression = null,
        uint color = 0xff62a844u,
        string? speciesName = null,
        int? dynamicCostIndex = null)
    {
        Id = id;
        Metric = metric;
        _title = title;
        _customExpression = customExpression;
        _samples = samples;
        Color = color;
        SpeciesName = speciesName;
        DynamicCostIndex = dynamicCostIndex;
    }

    public string Id { get; }
    public GraphMetric Metric { get; }
    public uint Color { get; }
    public string? SpeciesName { get; }
    public int? DynamicCostIndex { get; }
    public string Title { get => _title; private set => Set(ref _title, value); }
    public string? CustomExpression { get => _customExpression; private set => Set(ref _customExpression, value); }
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

    public void UpdateDefinition(string title, string? customExpression)
    {
        Title = title;
        CustomExpression = customExpression;
    }

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
    private IReadOnlyList<SpeciesGraphSampleSnapshot> _currentSpecies = [];
    private GraphChoice _selectedChoice = GraphCatalog.Metrics[0];
    private string _status = "SELECT A GRAPH";
    private string _comparisonText = "NO COMPARISON";
    private ulong? _historyCutoffTick;

    public GraphSuiteViewModel(IReadOnlyList<GraphMetricSample>? samples = null)
    {
        _samples = samples ?? [];
        CustomMetrics =
        [
            new CustomMetricSlotViewModel(1, "Custom graph 1", "population"),
            new CustomMetricSlotViewModel(2, "Custom graph 2", "total_energy"),
            new CustomMetricSlotViewModel(3, "Custom graph 3", "average_energy"),
        ];
        foreach (var custom in CustomMetrics)
            custom.PropertyChanged += CustomMetric_PropertyChanged;
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
        _currentSpecies = snapshot.Species.Select((species, index) => (species, index))
            .Where(value => snapshot.Organisms.Any(robot => robot.Species == value.index))
            .Select(value => new SpeciesGraphSampleSnapshot(value.species.Name, value.species.Color, new Dictionary<string, double>()))
            .ToArray();
        foreach (var graph in RunningGraphs)
            graph.Update(ProjectGraph(graph));
    }

    public void SetSamples(IReadOnlyList<GraphMetricSample> samples)
    {
        _samples = samples ?? throw new ArgumentNullException(nameof(samples));
        foreach (var graph in RunningGraphs)
            graph.Update(ProjectGraph(graph));
    }

    public void ConfigureCustomMetric(int slot, string name, string expression)
    {
        var custom = CustomMetrics.SingleOrDefault(value => value.Slot == slot)
            ?? throw new ArgumentOutOfRangeException(nameof(slot), "Custom graph slots are 1, 2, and 3.");
        custom.Name = string.IsNullOrWhiteSpace(name) ? $"Custom graph {slot}" : name.Trim();
        custom.Expression = string.IsNullOrWhiteSpace(expression) ? "0" : expression.Trim();
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
        var graphs = new List<GraphSeriesViewModel>();
        var latest = _samples.LastOrDefault();
        var speciesGraphs = latest?.SpeciesGraphs?.Count > 0 ? latest.SpeciesGraphs : _currentSpecies;
        if (metric == GraphMetric.DynamicCostStats && latest?.DynamicCostSeries?.Count == 6)
        {
            string[] labels = ["Cost Multiplier", "Population / Target", "Upper Range", "Lower Range", "Zero Level", "Reinstatement Level"];
            uint[] colors = [0xff2d826f, 0xffd88724, 0xff4169aa, 0xffa64d79, 0xff888536, 0xff7956b5];
            for (var index = 0; index < labels.Length; index++)
                graphs.Add(new GraphSeriesViewModel($"graph-{RunningGraphs.Count + graphs.Count + 1}", metric, labels[index], [], color: colors[index], dynamicCostIndex: index));
        }
        else if (metric != GraphMetric.DynamicCostStats && speciesGraphs.Count > 0)
        {
            foreach (var species in speciesGraphs)
                graphs.Add(new GraphSeriesViewModel($"graph-{RunningGraphs.Count + graphs.Count + 1}", metric,
                    $"{title} · {species.Name}", [], custom?.Expression, species.Color, species.Name));
        }
        else
            graphs.Add(new GraphSeriesViewModel($"graph-{RunningGraphs.Count + 1}", metric, title, [], custom?.Expression));
        foreach (var opened in graphs)
        {
            opened.Update(ProjectGraph(opened));
            RunningGraphs.Add(opened);
        }
        var graph = graphs[0];
        Status = $"RUNNING · {title.ToUpperInvariant()}";
        return graph;
    }

    public void ResetAllGraphs()
    {
        _historyCutoffTick = _samples.LastOrDefault()?.Tick;
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

    public string ExportDatabase(EngineSnapshot snapshot) => DatabaseExport.Serialize(snapshot);

    public IReadOnlyList<GraphSeriesViewModel> ImportLegacyGsave(IEnumerable<string> segmentFiles)
    {
        var imported = LegacyGraphJoin.ToSeries(LegacyGraphJoin.Parse(segmentFiles));
        foreach (var graph in imported) RunningGraphs.Add(graph);
        Status = imported.Count == 0
            ? "NO LEGACY GRAPHS IMPORTED"
            : $"IMPORTED · {imported.Count:N0} LEGACY GRAPH SERIES";
        return imported;
    }

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
        var differences = leftValues.Keys.Intersect(rightValues.Keys).Order()
            .Select(tick => new GraphDifference(
                tick,
                leftValues[tick] - rightValues[tick]))
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

    private IReadOnlyList<GraphSample> ProjectGraph(GraphSeriesViewModel graph) =>
        ProjectGraph(graph.Metric, CustomMetric(graph.Metric), graph.SpeciesName, graph.DynamicCostIndex);

    private IReadOnlyList<GraphSample> ProjectGraph(GraphMetric metric, CustomMetricSlotViewModel? custom, string? speciesName = null, int? dynamicCostIndex = null)
    {
        var samples = _historyCutoffTick is { } cutoff
            ? _samples.Where(sample => sample.Tick > cutoff).ToArray()
            : _samples;
        return GraphMetricSampleProjector.Project(samples, metric, custom, speciesName, dynamicCostIndex);
    }

    private void CustomMetric_PropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (sender is not CustomMetricSlotViewModel custom) return;
        var metric = CustomMetric(custom.Slot);
        foreach (var graph in RunningGraphs.Where(value => value.Metric == metric))
        {
            graph.UpdateDefinition(custom.Name, custom.Expression);
            graph.Update(ProjectGraph(graph));
        }
    }

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
        var history = snapshot.History.Count == 0
            ? [new HistorySampleSnapshot(snapshot.Tick, snapshot.Population, snapshot.Stats.TotalEnergy, snapshot.Stats.Births, snapshot.Stats.Deaths, snapshot.Stats.Mutations, snapshot.Stats.ShotsFired)]
            : snapshot.History;
        return history.Select(sample => new GraphMetricSample(sample.Tick, new Dictionary<GraphMetric, double>
        {
            [GraphMetric.Population] = sample.Population,
            [GraphMetric.AverageMutations] = sample.AverageMutations,
            [GraphMetric.AverageMutationsPerDnaLength] = sample.AverageMutationsPerDnaLength,
            [GraphMetric.AverageAge] = sample.AverageAge,
            [GraphMetric.AverageDescendents] = sample.AverageDescendents,
            [GraphMetric.AverageEnergy] = sample.AverageEnergy,
            [GraphMetric.AverageDnaLength] = sample.AverageDnaLength,
            [GraphMetric.AverageDnaConditions] = sample.AverageDnaConditions,
            [GraphMetric.TotalEnergy] = sample.TotalEnergy,
            [GraphMetric.DynamicCostStats] = sample.DynamicCostStats,
            [GraphMetric.SpeciesDiversity] = sample.SpeciesDiversity,
            [GraphMetric.AverageChloroplasts] = sample.AverageChloroplasts,
            [GraphMetric.SelectiveGeneticDistance] = sample.SelectiveGeneticDistance,
            [GraphMetric.GenerationalDistance] = sample.GenerationalDistance,
            [GraphMetric.SlowGeneticDistance] = sample.SlowGeneticDistance,
            [GraphMetric.CustomizableGraph1] = sample.AverageEnergy,
            [GraphMetric.CustomizableGraph2] = sample.TotalEnergy,
            [GraphMetric.CustomizableGraph3] = sample.AverageChloroplasts,
        }, sample.SpeciesGraphs, sample.DynamicCostSeries)).ToArray();
    }

    public static IReadOnlyList<GraphSample> Project(
        IReadOnlyList<GraphMetricSample> samples,
        GraphMetric metric,
        CustomMetricSlotViewModel? custom,
        string? speciesName = null,
        int? dynamicCostIndex = null)
    {
        return samples.Where(sample => speciesName is null || sample.SpeciesGraphs?.Count > 0).Select(sample =>
        {
            if (dynamicCostIndex is { } index)
                return new GraphSample(sample.Tick, sample.DynamicCostSeries?.ElementAtOrDefault(index) ?? 0);
            if (speciesName is not null)
            {
                var species = sample.SpeciesGraphs?.FirstOrDefault(value => value.Name == speciesName);
                sample = sample with { Values = SpeciesValues(species) };
            }
            return new GraphSample(
            sample.Tick,
            custom is null
                ? sample.Values.GetValueOrDefault(metric)
                : GraphExpressionEvaluator.Evaluate(custom.Expression, sample));
        }).ToArray();
    }
    private static IReadOnlyDictionary<GraphMetric, double> SpeciesValues(SpeciesGraphSampleSnapshot? species)
    {
        var names = new Dictionary<GraphMetric, string>
        {
            [GraphMetric.Population] = "population",
            [GraphMetric.AverageMutations] = "average_mutations",
            [GraphMetric.AverageMutationsPerDnaLength] = "average_mutations_per_dna_length",
            [GraphMetric.AverageAge] = "average_age",
            [GraphMetric.AverageDescendents] = "average_descendents",
            [GraphMetric.AverageEnergy] = "average_energy",
            [GraphMetric.AverageDnaLength] = "average_dna_length",
            [GraphMetric.AverageDnaConditions] = "average_dna_conditions",
            [GraphMetric.TotalEnergy] = "total_energy",
            [GraphMetric.SpeciesDiversity] = "species_diversity",
            [GraphMetric.AverageChloroplasts] = "average_chloroplasts",
            [GraphMetric.SelectiveGeneticDistance] = "selective_genetic_distance",
            [GraphMetric.GenerationalDistance] = "generational_distance",
            [GraphMetric.SlowGeneticDistance] = "slow_genetic_distance",
        };
        return names.ToDictionary(value => value.Key, value => species?.Values.GetValueOrDefault(value.Value) ?? 0);
    }

}

public static class GraphExport
{
    public static string Serialize(IEnumerable<GraphSeriesViewModel> graphs)
    {
        var builder = new StringBuilder("series_id,metric,title,expression,tick,value\r\n");
        foreach (var graph in graphs)
        foreach (var sample in graph.Samples)
            builder.Append(Escape(graph.Id))
                .Append(',')
                .Append(Escape(graph.Metric.ToString()))
                .Append(',')
                .Append(Escape(graph.Title))
                .Append(',')
                .Append(Escape(graph.CustomExpression ?? string.Empty))
                .Append(',')
                .Append(sample.Tick.ToString(CultureInfo.InvariantCulture))
                .Append(',')
                .Append(sample.Value.ToString("R", CultureInfo.InvariantCulture))
                .Append("\r\n");
        return builder.ToString();
    }

    private static string Escape(string value) =>
        value.IndexOfAny([',', '"', '\r', '\n']) < 0
            ? value
            : $"\"{value.Replace("\"", "\"\"", StringComparison.Ordinal)}\"";
}

internal static class GraphExpressionEvaluator
{
    public static double Evaluate(string expression, GraphMetricSample sample)
    {
        try
        {
            var tokens = expression.ToLowerInvariant().Split(' ', StringSplitOptions.RemoveEmptyEntries);
            if (tokens.Any(token => NativeNames.ContainsKey(token) || token is "add" or "sub" or "mult" or "div" or "pow"))
                return EvaluateNativeQuery(tokens, sample);
            var parser = new Parser(expression, sample);
            var value = parser.ParseExpression();
            return parser.AtEnd ? value : 0;
        }
        catch (FormatException)
        {
            return 0;
        }
    }

    private static readonly IReadOnlyDictionary<string, GraphMetric> NativeNames = new Dictionary<string, GraphMetric>
    {
        ["pop"] = GraphMetric.Population, ["avgmut"] = GraphMetric.AverageMutations,
        ["avgage"] = GraphMetric.AverageAge, ["avgsons"] = GraphMetric.AverageDescendents,
        ["avgnrg"] = GraphMetric.AverageEnergy, ["avglen"] = GraphMetric.AverageDnaLength,
        ["avgcond"] = GraphMetric.AverageDnaConditions, ["simnrg"] = GraphMetric.TotalEnergy,
        ["specidiv"] = GraphMetric.SpeciesDiversity, ["maxgd"] = GraphMetric.GenerationalDistance,
        ["simpgenetic"] = GraphMetric.SelectiveGeneticDistance,
    };

    private static double EvaluateNativeQuery(string[] tokens, GraphMetricSample sample)
    {
        var stack = new List<double>();
        double Pop()
        {
            if (stack.Count == 0) return 0;
            var value = stack[^1];
            stack.RemoveAt(stack.Count - 1);
            return value;
        }
        void Push(double value)
        {
            if (stack.Count == 101) stack.RemoveAt(0);
            stack.Add(value);
        }
        const double limit = 2_000_000_000;
        foreach (var token in tokens)
        {
            if (double.TryParse(token, NumberStyles.Float, CultureInfo.InvariantCulture, out var number)) Push(number);
            else if (NativeNames.TryGetValue(token, out var metric)) Push(sample.Values.GetValueOrDefault(metric));
            else if (token is "add" or "sub" or "mult" or "div" or "pow")
            {
                var right = Pop();
                var left = Pop();
                if (token is "add" or "sub")
                {
                    if (left > limit) left = Math.Round(left, MidpointRounding.ToEven) % limit;
                    if (right > limit) right = Math.Round(right, MidpointRounding.ToEven) % limit;
                    var value = token == "add" ? left + right : left - right;
                    if (Math.Abs(value) > limit) value -= Math.Sign(value) * limit;
                    Push(value);
                }
                else if (token == "div") Push(right == 0 ? 0 : left / right);
                else
                {
                    var value = token == "mult" ? left * right : left == 0 ? 0 : Math.Pow(left, Math.Clamp(right, -10, 10));
                    Push(double.IsNaN(value) ? 0 : Math.Clamp(value, -limit, limit));
                }
            }
        }
        return Math.Max(0, Pop());
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
