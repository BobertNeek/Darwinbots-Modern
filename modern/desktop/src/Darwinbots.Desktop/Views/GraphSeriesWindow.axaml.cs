using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Threading;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class GraphSeriesWindow : Window
{
    private readonly GraphSeriesViewModel _graph;
    private readonly IReadOnlyList<GraphSeriesViewModel> _series;
    private readonly LegacyGraphAutoSave? _recording;
    private readonly Action? _resetGraph;
    private readonly Action? _updateNow;
    private LegacyConfirmationWindow? _closePrompt;
    private bool _closePromptPending;
    private bool _keepUpdating;
    private bool _closed;

    public GraphSeriesWindow() : this(new GraphSeriesViewModel("empty", GraphMetric.Population, "Population", []))
    {
    }

    public GraphSeriesWindow(GraphSeriesViewModel graph, IReadOnlyList<GraphSeriesViewModel>? series = null,
        LegacyGraphAutoSave? recording = null, Action? resetGraph = null, Action? updateNow = null)
    {
        _graph = graph ?? throw new ArgumentNullException(nameof(graph));
        _series = series ?? new[] { graph };
        _recording = recording;
        _resetGraph = resetGraph;
        _updateNow = updateNow;
        InitializeComponent();
        ResetGraphButton.IsVisible = _resetGraph is not null;
        UpdateGraphNowButton.IsVisible = _updateNow is not null;
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
            _closed = true;
            CancelClosePrompt();
            foreach (var item in _series) item.PropertyChanged -= Graph_PropertyChanged;
            if (_recording is not null) _recording.Changed -= UpdateRecordingStatus;
        };
    }

    internal GraphSeriesViewModel Graph => _graph;
    internal bool IsCollecting => IsVisible || _keepUpdating;
    internal bool NeedsCleanupOnLoad => _keepUpdating || _closePromptPending;

    internal void Reopen(Window owner)
    {
        _keepUpdating = false;
        if (!IsVisible) Show(owner);
        Activate();
    }

    internal void CloseForCleanup()
    {
        if (_closed) return;
        CancelClosePrompt();
        Close();
    }

    protected override void OnClosing(WindowClosingEventArgs e)
    {
        base.OnClosing(e);
        if (_recording is null || e.Cancel || e.IsProgrammatic || !IsVisible
            || e.CloseReason is not (WindowCloseReason.WindowClosing or WindowCloseReason.Undefined))
            return;
        e.Cancel = true;
        if (_closePromptPending) return;
        _closePromptPending = true;
        Dispatcher.UIThread.Post(() => _ = ConfirmCloseAsync());
    }

    private async Task ConfirmCloseAsync()
    {
        if (!_closePromptPending || _closed || !IsVisible)
        {
            CancelClosePrompt();
            return;
        }
        var dialog = new LegacyConfirmationWindow("Graph", "Would you like to keep updating this graph?");
        _closePrompt = dialog;
        var keep = await dialog.ShowDialog<bool>(this);
        if (_closed || _closePrompt != dialog) return;
        _closePrompt = null;
        _closePromptPending = false;
        if (keep)
        {
            _keepUpdating = true;
            Hide();
        }
        else Close();
    }

    private void CancelClosePrompt()
    {
        _closePromptPending = false;
        var dialog = _closePrompt;
        _closePrompt = null;
        if (dialog?.IsVisible == true) dialog.Close(false);
    }

    private void UpdateGraphNow_Click(object? sender, RoutedEventArgs e) => _updateNow?.Invoke();

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
