using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Controls;

namespace Darwinbots.Desktop.Views;

public sealed partial class GraphSuiteView : UserControl
{
    private GraphSuiteViewModel _viewModel;
    private GraphSeriesViewModel? _displayedSeries;

    public GraphSuiteView() : this(new GraphSuiteViewModel())
    {
    }

    public GraphSuiteView(GraphSuiteViewModel viewModel)
    {
        _viewModel = viewModel ?? throw new ArgumentNullException(nameof(viewModel));
        InitializeComponent();
        DataContext = _viewModel;
        RefreshChart();
    }

    public GraphSuiteViewModel ViewModel => _viewModel;
    public event Action<GraphSeriesViewModel>? GraphWindowRequested;

    public void SetSnapshot(EngineSnapshot snapshot)
    {
        _viewModel.Update(snapshot);
        RefreshChart();
    }

    private void OpenGraph_Click(object? sender, RoutedEventArgs e)
    {
        var opened = _viewModel.OpenSelectedGraph();
        if (opened is not null)
        {
            _displayedSeries = opened;
            GraphWindowRequested?.Invoke(opened);
        }
        RefreshChart();
    }

    private void ResetGraphs_Click(object? sender, RoutedEventArgs e)
    {
        _viewModel.ResetAllGraphs();
        RefreshChart();
    }

    private void ListGraphs_Click(object? sender, RoutedEventArgs e) => _viewModel.ListAllRunningGraphs();

    private async void ExportGraphs_Click(object? sender, RoutedEventArgs e)
    {
        if (TopLevel.GetTopLevel(this) is not { } owner) return;
        var file = await owner.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Export Darwinbots graph data",
            SuggestedFileName = "darwinbots-graphs.csv",
            FileTypeChoices = [new FilePickerFileType("CSV graph data") { Patterns = ["*.csv"] }],
        });
        if (file is null) return;
        await using var stream = await file.OpenWriteAsync();
        stream.SetLength(0);
        await using var writer = new StreamWriter(stream);
        await writer.WriteAsync(_viewModel.ExportCsv());
    }

    private void JoinCompare_Click(object? sender, RoutedEventArgs e)
    {
        var selected = _viewModel.RunningGraphs.Where(value => value.IsSelected).ToArray();
        _viewModel.JoinAndCompare(selected);
        RefreshChart();
    }

    private void ViewGraph_Click(object? sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: GraphSeriesViewModel graph })
        {
            _displayedSeries = graph;
            RefreshChart();
            GraphWindowRequested?.Invoke(graph);
        }
    }

    private void Close_Click(object? sender, RoutedEventArgs e)
    {
        if (TopLevel.GetTopLevel(this) is Window window) window.Close();
    }

    private void RefreshChart()
    {
        SeriesChart.SetSeries(_displayedSeries ?? _viewModel.RunningGraphs.LastOrDefault());
    }
}
