using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public enum RobotDetailsTab
{
    Facts,
    Dna,
    Memory,
    GeneActivations,
}

public sealed class RobotDetailsWindow : Window
{
    private readonly SimulationSession? _session;
    private readonly OrganismKey _focus;
    private readonly RobotDiagnosticModel _diagnostic = new();
    private readonly TextBlock _subtitle = new();
    private readonly TextBlock _facts = new();
    private readonly TextBlock _factsTab = new();
    private readonly TextBox _dna = new();
    private readonly TextBox _memory = new();
    private readonly TextBlock _activationSummary = new();
    private readonly StackPanel _activationTimeline = new();
    private readonly TextBlock _status = new();
    private readonly TabControl _tabs = new();

    public RobotDetailsWindow(EngineSnapshot snapshot, OrganismKey focus, string dna = "")
        : this(null, focus, RobotDetailsTab.Facts, dna)
    {
        Update(snapshot);
    }

    public RobotDetailsWindow(
        SimulationSession session,
        OrganismKey focus,
        RobotDetailsTab initialTab = RobotDetailsTab.Facts)
        : this(session, focus, initialTab, string.Empty)
    {
        Update(session.LatestSnapshot);
    }

    public string ActivationSummary => _diagnostic.ActivationSummary;

    public void SelectTab(RobotDetailsTab tab) => _tabs.SelectedIndex = (int)tab;

    private RobotDetailsWindow(
        SimulationSession? session,
        OrganismKey focus,
        RobotDetailsTab initialTab,
        string dna)
    {
        _session = session;
        _focus = focus;
        Title = $"Darwinbots — Robot Details {focus.Slot}:{focus.Generation}";
        Width = 1_080;
        Height = 680;
        MinWidth = 820;
        MinHeight = 520;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;
        Background = Brush.Parse("#F4F0E6");

        _subtitle.Foreground = Brush.Parse("#537B43");
        _subtitle.FontFamily = new FontFamily("Bahnschrift");
        _subtitle.FontSize = 11;
        _subtitle.FontWeight = FontWeight.Bold;
        _subtitle.Text = $"LIVE ROBOT {focus.Slot}:{focus.Generation}";

        _facts.TextWrapping = TextWrapping.Wrap;
        _facts.FontFamily = new FontFamily("Consolas");
        _facts.FontSize = 12;
        _facts.Foreground = Brush.Parse("#1D2824");
        _factsTab.TextWrapping = TextWrapping.Wrap;
        _factsTab.FontFamily = new FontFamily("Consolas");
        _factsTab.FontSize = 12;
        _factsTab.Foreground = Brush.Parse("#1D2824");

        _dna.IsReadOnly = true;
        _dna.AcceptsReturn = true;
        _dna.TextWrapping = TextWrapping.Wrap;
        _dna.Background = Brush.Parse("#FFFCF5");
        _dna.FontFamily = new FontFamily("Consolas");
        _dna.FontSize = 12;
        _dna.Text = dna;

        _memory.IsReadOnly = true;
        _memory.AcceptsReturn = true;
        _memory.TextWrapping = TextWrapping.NoWrap;
        _memory.Background = Brush.Parse("#FFFCF5");
        _memory.FontFamily = new FontFamily("Consolas");
        _memory.FontSize = 12;

        _activationSummary.FontFamily = new FontFamily("Bahnschrift");
        _activationSummary.FontWeight = FontWeight.Bold;
        _activationSummary.Foreground = Brush.Parse("#2D826F");
        _activationTimeline.Spacing = 5;

        _tabs.Margin = new Thickness(18, 0, 18, 18);
        _tabs.Items.Add(new TabItem
        {
            Header = "FACTS",
            Content = new ScrollViewer
            {
                Padding = new Thickness(16),
                Content = _factsTab,
            },
        });
        _tabs.Items.Add(new TabItem
        {
            Header = "DNA",
            Content = new Border
            {
                Padding = new Thickness(12),
                Child = _dna,
            },
        });
        _tabs.Items.Add(new TabItem
        {
            Header = "MEMORY",
            Content = new Border
            {
                Padding = new Thickness(12),
                Child = _memory,
            },
        });
        _tabs.Items.Add(new TabItem
        {
            Header = "GENE ACTIVATIONS",
            Content = new ScrollViewer
            {
                Padding = new Thickness(16),
                Content = new StackPanel
                {
                    Spacing = 10,
                    Children =
                    {
                        _activationSummary,
                        _activationTimeline,
                    },
                },
            },
        });
        _tabs.SelectedIndex = (int)initialTab;

        var root = new Grid { RowDefinitions = new RowDefinitions("76,*,42") };
        root.Children.Add(new Border
        {
            Background = Brush.Parse("#FBF9F2"),
            BorderBrush = Brush.Parse("#C8C3B6"),
            BorderThickness = new Thickness(0, 0, 0, 1),
            Padding = new Thickness(22, 12),
            Child = new StackPanel
            {
                Spacing = 2,
                Children =
                {
                    new TextBlock
                    {
                        Text = "ROBOT DETAILS",
                        FontFamily = new FontFamily("Bahnschrift"),
                        FontSize = 22,
                        FontWeight = FontWeight.Bold,
                        LetterSpacing = 1.4,
                    },
                    _subtitle,
                },
            },
        });

        var body = new Grid { ColumnDefinitions = new ColumnDefinitions("330,*") };
        body.Children.Add(new Border
        {
            Margin = new Thickness(18, 18, 0, 0),
            Padding = new Thickness(14),
            Background = Brush.Parse("#E5ECD9"),
            BorderBrush = Brush.Parse("#C8C3B6"),
            BorderThickness = new Thickness(1),
            Child = new ScrollViewer { Content = _facts },
        });
        Grid.SetColumn(_tabs, 1);
        body.Children.Add(_tabs);
        Grid.SetRow(body, 1);
        root.Children.Add(body);

        _status.Text = "Engine-owned values refresh each published cycle.";
        _status.Foreground = Brush.Parse("#69716D");
        _status.FontSize = 11;
        _status.Margin = new Thickness(18, 0);
        Grid.SetRow(_status, 2);
        root.Children.Add(_status);
        Content = root;

        Opened += Window_Opened;
        Closed += Window_Closed;
    }

    private async void Window_Opened(object? sender, EventArgs e)
    {
        if (_session is null)
        {
            return;
        }

        _session.SnapshotPublished += Session_SnapshotPublished;
        Update(_session.LatestSnapshot);
        try
        {
            _dna.Text = await _session.ExportDnaAsync(_focus.Slot, _focus.Generation);
        }
        catch (Exception error)
        {
            _status.Text = $"DNA READ FAILED · {error.Message}";
        }
    }

    private void Window_Closed(object? sender, EventArgs e)
    {
        if (_session is not null)
        {
            _session.SnapshotPublished -= Session_SnapshotPublished;
        }
    }

    private void Session_SnapshotPublished(EngineSnapshot snapshot)
    {
        Avalonia.Threading.Dispatcher.UIThread.Post(() => Update(snapshot));
    }

    private void Update(EngineSnapshot snapshot) => SetSnapshot(snapshot);

    public void SetSnapshot(EngineSnapshot snapshot)
    {
        _diagnostic.Update(snapshot, _focus);
        var organism = snapshot.Organisms.FirstOrDefault(value =>
            value.Slot == _focus.Slot && value.Generation == _focus.Generation);
        if (organism is null)
        {
            _subtitle.Text = $"ROBOT {_focus.Slot}:{_focus.Generation} · REMOVED";
            _facts.Text = "The focused robot no longer exists in the live snapshot.";
            _factsTab.Text = _facts.Text;
            _memory.Text = string.Empty;
            _activationTimeline.Children.Clear();
            _activationSummary.Text = _diagnostic.ActivationSummary;
            return;
        }

        var species = snapshot.Species.ElementAtOrDefault((int)organism.Species)?.Name ?? "UNASSIGNED";
        _subtitle.Text = $"LIVE ROBOT {organism.Slot}:{organism.Generation} · {species.ToUpperInvariant()} · TICK {snapshot.Tick:N0}";
        _facts.Text = string.Join(Environment.NewLine,
            $"ARRAY ID       {organism.Slot:N0}",
            $"UNIQUE ID      {organism.UniqueId:N0}",
            $"SIMULATION     {organism.SimulationId:N0}",
            $"PARENT 1       {FormatParent(organism.Parents, 0)}",
            $"PARENT 2       {FormatParent(organism.Parents, 1)}",
            $"AGE / GENER.   {organism.Age:N0} / {organism.Generation:N0}",
            $"OFFSPRING      {organism.Offspring:N0}",
            $"DESCENDANTS    {organism.Descendants:N0}",
            $"ENERGY         {organism.Energy:N0}",
            $"BODY           {organism.Body:N0}",
            $"MASS / RADIUS  {organism.Mass:0.0} / {organism.Radius:0.0}",
            $"VELOCITY       {FormatVector(organism.Velocity)}",
            $"WASTE / PERM.  {organism.Waste:N0} / {organism.PermanentWaste:N0}",
            $"VENOM          {organism.Venom:N0}",
            $"SHELL / SLIME  {organism.Shell:N0} / {organism.Slime:N0}",
            $"POISON         {organism.Poison:N0}",
            $"VIRUS TIMER    {organism.VirusTimer:N0}",
            $"CHLOROPLASTS   {organism.Chloroplasts:N0}",
            $"NEW / TOTAL M.  {organism.NewMutations:N0} / {organism.TotalMutations:N0}",
            $"DNA / GENES    {organism.DnaLength:N0} / {organism.GeneCount:N0}",
            $"CURRENT GENE   {organism.CurrentGene:N0} ({organism.ConditionCount:N0} conditions)",
            $"POSITION       {FormatVector(organism.Position)}");
        _factsTab.Text = _facts.Text;
        _memory.Text = organism.Memory.Count == 0
            ? "No non-zero public memory cells were published."
            : string.Join(Environment.NewLine,
                organism.Memory.Select(cell => $"{cell.Address,4}  {cell.Value,10}"));
        if (!string.IsNullOrWhiteSpace(organism.DebugText))
            _memory.Text += $"{Environment.NewLine}{Environment.NewLine}DEBUG{Environment.NewLine}{organism.DebugText}";

        _activationSummary.Text = _diagnostic.ActivationSummary;
        _activationTimeline.Children.Clear();
        foreach (var frame in _diagnostic.ActivationTimeline)
        {
            _activationTimeline.Children.Add(new Border
            {
                Padding = new Thickness(10, 6),
                Background = Brush.Parse("#E5ECD9"),
                BorderBrush = Brush.Parse("#C8C3B6"),
                BorderThickness = new Thickness(1),
                Child = new TextBlock
                {
                    Text = $"TICK {frame.Tick:N0}    {FormatActivations(frame.Activations)}    ACTIVE {frame.ActiveGeneCount:N0}",
                    FontFamily = new FontFamily("Consolas"),
                    FontSize = 12,
                },
            });
        }
    }

    private static string FormatParent(OrganismKey?[]? parents, int index) =>
        parents is { Length: > 0 } && parents.ElementAtOrDefault(index) is { } parent
            ? $"{parent.Slot}:{parent.Generation}"
            : "NONE";

    private static string FormatVector(float[] values) =>
        values.Length >= 2 ? $"{values[0]:0.00}, {values[1]:0.00}" : "0.00, 0.00";

    private static string FormatActivations(IReadOnlyList<bool> activations) =>
        activations.Count == 0
            ? "NO GENE DATA"
            : string.Join("  ", activations.Select((active, index) => $"G{index + 1}:{(active ? "ON" : "off")}"));
}
