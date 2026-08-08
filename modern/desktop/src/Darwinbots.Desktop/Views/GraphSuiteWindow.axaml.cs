using Avalonia.Controls;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class GraphSuiteWindow : Window
{
    private readonly List<GraphSeriesWindow> _graphWindows = [];

    public GraphSuiteWindow() : this(EngineSnapshot.Empty)
    {
    }

    public GraphSuiteWindow(EngineSnapshot snapshot)
    {
        InitializeComponent();
        GraphView.GraphWindowRequested += OpenGraphWindow;
        GraphView.SetSnapshot(snapshot);
    }

    public GraphSuiteViewModel ViewModel => GraphView.ViewModel;

    public void SetSnapshot(EngineSnapshot snapshot) => GraphView.SetSnapshot(snapshot);

    private void OpenGraphWindow(GraphSeriesViewModel graph)
    {
        var window = new GraphSeriesWindow(graph);
        _graphWindows.Add(window);
        window.Closed += (_, _) => _graphWindows.Remove(window);
        window.Show(this);
    }

    protected override void OnClosed(EventArgs e)
    {
        foreach (var window in _graphWindows.ToArray()) window.Close();
        _graphWindows.Clear();
        base.OnClosed(e);
    }
}
