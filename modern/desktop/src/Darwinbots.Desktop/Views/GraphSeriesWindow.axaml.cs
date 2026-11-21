using Avalonia;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Threading;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class GraphSeriesWindow : Window
{
    private readonly GraphSeriesViewModel _graph;
    private IReadOnlyList<GraphSeriesViewModel> _series;
    private readonly LegacyGraphAutoSave? _recording;
    private readonly bool _keepUpdatingOnClose;
    private readonly Action? _resetGraph;
    private readonly Action? _updateNow;
    private LegacyConfirmationWindow? _closePrompt;
    private bool _closePromptPending;
    private bool _keepUpdating;
    private bool _closed;
    private int _positioningVersion;
    private bool _placingFromLayout;
    private PixelPoint? _restoredPosition;

    public GraphSeriesWindow() : this(new GraphSeriesViewModel("empty", GraphMetric.Population, "Population", []))
    {
    }

    public GraphSeriesWindow(GraphSeriesViewModel graph, IReadOnlyList<GraphSeriesViewModel>? series = null,
        LegacyGraphAutoSave? recording = null, Action? resetGraph = null, Action? updateNow = null,
        bool keepUpdatingOnClose = false)
    {
        _graph = graph ?? throw new ArgumentNullException(nameof(graph));
        _series = series ?? new[] { graph };
        _recording = recording;
        _keepUpdatingOnClose = keepUpdatingOnClose;
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
    internal IReadOnlyList<GraphSeriesViewModel> Series => _series;
    internal bool IsCollecting => IsVisible || _keepUpdating;
    internal bool NeedsCleanupOnLoad => _keepUpdating || _closePromptPending;
    internal event Action? DiscardedByUser;

    internal void SetSeries(IReadOnlyList<GraphSeriesViewModel> series)
    {
        if (_series.SequenceEqual(series)) return;
        foreach (var item in _series) item.PropertyChanged -= Graph_PropertyChanged;
        _series = series;
        foreach (var item in _series) item.PropertyChanged += Graph_PropertyChanged;
        Refresh();
    }

    internal void PlaceFromNativeLayout(Window owner, NativeGraphLayoutRow layout)
    {
        if (layout.Left is null && layout.Top is null) return;
        var screen = owner.Screens.ScreenFromWindow(owner) ?? owner.Screens.Primary;
        if (screen is null) return;

        var workingArea = screen.WorkingArea;
        var scale = screen.Scaling;
        var left = layout.Left is { } x ? Math.Round(x * scale / 15) : workingArea.X;
        var top = layout.Top is { } y ? Math.Round(y * scale / 15) : workingArea.Y;
        var rightmost = workingArea.X + Math.Max(0, workingArea.Width - Math.Round(Width * scale));
        var bottommost = workingArea.Y + Math.Max(0, workingArea.Height - Math.Round(Height * scale));
        var version = ++_positioningVersion;
        _placingFromLayout = true;
        WindowStartupLocation = WindowStartupLocation.Manual;
        var position = new PixelPoint((int)Math.Clamp(left, workingArea.X, rightmost),
            (int)Math.Clamp(top, workingArea.Y, bottommost));
        _restoredPosition = position;
        Position = position;
        Dispatcher.UIThread.Post(() =>
        {
            if (_positioningVersion == version) _placingFromLayout = false;
        }, DispatcherPriority.Background);
    }

    internal (int Left, int Top)? ReadOnscreenNativePosition()
    {
        if (_placingFromLayout || !IsVisible || WindowState != WindowState.Normal) return null;
        var point = Position;
        if (point == _restoredPosition) return null;
        var screen = Screens.ScreenFromPoint(point);
        if (screen is null || !screen.WorkingArea.Contains(point)) return null;

        _restoredPosition = null;
        return (ToTwips(point.X, screen.Scaling), ToTwips(point.Y, screen.Scaling));

        static int ToTwips(int pixel, double scale) => (int)Math.Clamp(Math.Round(pixel * 15d / scale),
            int.MinValue, int.MaxValue);
    }

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
        if (_keepUpdatingOnClose)
        {
            HideAndKeepUpdating();
            return;
        }
        var dialog = new LegacyConfirmationWindow("Graph", "Would you like to keep updating this graph?");
        _closePrompt = dialog;
        var keep = await dialog.ShowDialog<bool>(this);
        if (_closed || _closePrompt != dialog) return;
        _closePrompt = null;
        if (keep)
            HideAndKeepUpdating();
        else
        {
            _closePromptPending = false;
            DiscardedByUser?.Invoke();
            Close();
        }
    }

    private void HideAndKeepUpdating()
    {
        _closePromptPending = false;
        _keepUpdating = true;
        Hide();
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
