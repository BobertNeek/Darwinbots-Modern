using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using System.Globalization;
using Darwinbots.Desktop.Core;
using Microsoft.VisualBasic;

namespace Darwinbots.Desktop.Views;

public enum RobotDetailsTab
{
    Facts,
    Dna,
    Memory,
    GeneActivations,
    MutationDetails,
}

public sealed class RobotDetailsWindow : Window
{
    private readonly SimulationSession? _session;
    private readonly Func<OrganismKey?>? _readFocus;
    private OrganismKey? _focus;
    private ulong _followedTick;
    private OrganismKey? _consoleDnaFacts;
    private long _dnaReadVersion;
    private readonly RobotDiagnosticModel _diagnostic = new();
    private readonly TextBlock _subtitle = new();
    private readonly TextBlock _facts = new();
    private readonly TextBlock _factsTab = new();
    private readonly TextBox _dna = new() { Name = "RobotDnaText" };
    private readonly Button? _markDnaLocation;
    private readonly TextBox _memory = new();
    private readonly TextBox? _mutationDetails;
    private IReadOnlyList<string>? _mutationLog;
    private IReadOnlyList<string>? _displayedMutationLog;
    private readonly TextBlock _activationSummary = new();
    private readonly StackPanel _activationTimeline = new();
    private readonly TextBlock _status = new();
    private readonly TabControl _tabs = new();

    public RobotDetailsWindow(EngineSnapshot snapshot, OrganismKey focus, string dna = "")
        : this(null, focus, RobotDetailsTab.Facts, dna, snapshot.NativeCycle.HasValue)
    {
        SetSnapshot(snapshot);
    }

    public RobotDetailsWindow(
        SimulationSession session,
        OrganismKey focus,
        RobotDetailsTab initialTab = RobotDetailsTab.Facts,
        Func<OrganismKey?>? readFocus = null)
        : this(session, focus, initialTab, string.Empty, session.LatestSnapshot.NativeCycle.HasValue, readFocus)
    {
        SetSnapshot(session.LatestSnapshot);
    }

    public string ActivationSummary => _diagnostic.ActivationSummary;

    public void SelectTab(RobotDetailsTab tab) => _tabs.SelectedIndex = (int)tab;

    private RobotDetailsWindow(
        SimulationSession? session,
        OrganismKey focus,
        RobotDetailsTab initialTab,
        string dna,
        bool native,
        Func<OrganismKey?>? readFocus = null)
    {
        _session = session;
        _readFocus = readFocus;
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
        if (native && session is not null)
        {
            _markDnaLocation = new Button
            {
                Name = "MarkRobotDnaLocation",
                Content = "Mark a location",
                Margin = new Thickness(0, 8, 0, 0),
            };
            _markDnaLocation.Click += async (_, _) => await MarkDnaLocationAsync();
            DockPanel.SetDock(_markDnaLocation, Dock.Bottom);
        }

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
                Child = _markDnaLocation is { } marker ? new DockPanel { Children = { marker, _dna } } : _dna,
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
        if (native)
        {
            _mutationDetails = new TextBox
            {
                Name = "RobotMutationDetails",
                IsReadOnly = true,
                AcceptsReturn = true,
                TextWrapping = TextWrapping.Wrap,
                Background = _dna.Background,
                FontFamily = _dna.FontFamily,
                FontSize = _dna.FontSize,
            };
            _tabs.Items.Add(new TabItem
            {
                Header = "MUTATION DETAILS",
                Content = new Border { Padding = new Thickness(12), Child = _mutationDetails },
            });
        }
        _tabs.SelectedIndex = (int)initialTab;
        _tabs.SelectionChanged += (_, _) => RefreshMutationDetails();

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

    private void Window_Opened(object? sender, EventArgs e)
    {
        if (_session is null)
        {
            return;
        }

        _session.SnapshotPublished -= Session_SnapshotPublished;
        _session.SnapshotPublished += Session_SnapshotPublished;
        SetSnapshot(_session.LatestSnapshot);
        _ = RefreshDnaAsync();
    }

    private async Task MarkDnaLocationAsync()
    {
        if (_focus is not { } focus || _markDnaLocation is not { IsEnabled: true }) return;
        var version = _dnaReadVersion;
        var dialog = new LegacyValueWindow("Mark a location", "DNA position", string.Empty,
            "Enter a one-based DNA position. A blank entry clears the marker.", useDefaultButtons: true);
        await dialog.ShowDialog(this);
        if (IsVisible && _focus == focus && version == _dnaReadVersion)
            await RefreshDnaAsync(dialog.Accepted ? ParseDnaMarkerPosition(dialog.RawValue) : null);
    }

    private static uint? ParseDnaMarkerPosition(string input)
    {
        const uint maximum = 32_000;
        double value;
        try { value = Conversion.Val(input); }
        catch (OverflowException) { return maximum; }
        catch (InvalidCastException) { return null; }

        var position = (uint)Math.Round(Math.Min(Math.Abs(value), maximum), MidpointRounding.ToEven);
        return position == 0 ? null : position;
    }

    private async Task RefreshDnaAsync(uint? markerPosition = null)
    {
        var version = ++_dnaReadVersion;
        if (_session is null || _focus is not { } focus) return;
        try
        {
            var dna = _session.LatestSnapshot.NativeCycle.HasValue
                ? await _session.ReadRobotDnaAsync(focus, markerPosition)
                : await _session.ExportDnaAsync(focus.Slot, focus.Generation);
            if (version == _dnaReadVersion && IsVisible)
            {
                _dna.Text = dna;
                SetSnapshot(_session.LatestSnapshot);
            }
        }
        catch (Exception error)
        {
            if (version == _dnaReadVersion && IsVisible)
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
        Avalonia.Threading.Dispatcher.UIThread.Post(() =>
        {
            if (!IsVisible) return;
            if (_readFocus is not null && snapshot.Tick != _followedTick)
                SetFollowedSnapshot(snapshot);
            else
                SetSnapshot(snapshot);
        });
    }

    internal void SetFollowedSnapshot(EngineSnapshot snapshot, OrganismKey? consoleDnaFacts = null, bool refreshDna = false)
    {
        _followedTick = snapshot.Tick;
        _consoleDnaFacts = consoleDnaFacts;
        var focus = _readFocus?.Invoke();
        var focusChanged = focus != _focus;
        if (focusChanged)
        {
            _focus = focus;
            Title = focus is { } key ? $"Darwinbots — Robot Details {key.Slot}:{key.Generation}" : "Darwinbots — Robot Details";
        }
        if (focusChanged || consoleDnaFacts.HasValue || refreshDna)
        {
            _dna.Text = consoleDnaFacts.HasValue && !focus.HasValue
                ? "This Robot is dead.  No DNA available."
                : string.Empty;
            _status.Text = "Engine-owned values refresh each published cycle.";
            _ = RefreshDnaAsync();
        }
        SetSnapshot(snapshot);
    }

    public void SetSnapshot(EngineSnapshot snapshot)
    {
        _diagnostic.Update(snapshot, _focus);
        var selected = FindActor(_focus);
        if (_markDnaLocation is not null)
        {
            _markDnaLocation.IsVisible = snapshot.NativeCycle.HasValue;
            _markDnaLocation.IsEnabled = selected is not null;
        }
        _mutationLog = selected?.MutationLog;
        RefreshMutationDetails();
        // Console showdna initially displays its actor's facts alongside the selected actor's DNA.
        var consoleActor = snapshot.Tick == _followedTick ? FindActor(_consoleDnaFacts) : null;
        var organism = consoleActor ?? selected;
        if (organism is null)
        {
            _subtitle.Text = _focus is { } removed ? $"ROBOT {removed.Slot}:{removed.Generation} · REMOVED" : "NO ROBOT SELECTED";
            _facts.Text = _focus is null ? "Select a robot in the simulation." : "The focused robot no longer exists in the live snapshot.";
            _factsTab.Text = _facts.Text;
            _memory.Text = string.Empty;
            _activationTimeline.Children.Clear();
            _activationSummary.Text = _diagnostic.ActivationSummary;
            return;
        }

        var factFocus = new OrganismKey(organism.Slot, organism.Generation);
        var species = snapshot.Species.ElementAtOrDefault((int)organism.Species)?.Name ?? "UNASSIGNED";
        var dnaSummary = string.IsNullOrWhiteSpace(_dna.Text)
            ? "UNAVAILABLE"
            : _dna.Text.Replace("\r\n", " / ", StringComparison.Ordinal).Replace('\n', ' ');
        var memory = selected?.Memory ?? [];
        var memorySummary = memory.Count == 0
            ? "NONE"
            : string.Join(", ", memory.Select(cell => cell.Address.ToString(CultureInfo.InvariantCulture)));
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
            $"TOUCH / TASTE  {organism.Touch:N0} / {organism.Taste:N0}",
            $"SKIN           {organism.Phenotype.Skin.Length:N0} points · COLOR #{organism.Phenotype.Color & 0x00ffffff:X6}",
            $"EYES           {organism.Vision.Eyes.Length:N0} · FOCUS {organism.Vision.FocusEye:N0}",
            $"TIES           {snapshot.Ties.Count(tie => tie.First == factFocus || tie.Second == factFocus):N0}",
            $"SHOTS          {snapshot.Shots.Count(shot => shot.Alive && shot.Owner == factFocus):N0}",
            $"NEW / TOTAL M.  {organism.NewMutations:N0} / {organism.TotalMutations:N0}",
            $"SUBSPECIES     {organism.Subspecies:N0}",
            $"LAST MUTATION  {organism.LastMutationKind ?? "NONE"} @ {organism.LastMutationCycle?.ToString("N0") ?? "-"}",
            $"MUTATION LOG   {(organism.MutationLog.Count == 0 ? "NONE" : string.Join(" | ", organism.MutationLog.Take(4)))}",
            $"DNA / GENES    {organism.DnaLength:N0} / {organism.GeneCount:N0}",
            $"CURRENT GENE   {organism.CurrentGene:N0} ({organism.ConditionCount:N0} conditions)",
            $"POSITION       {FormatVector(organism.Position)}",
            $"DNA VIEW       {dnaSummary}",
            $"MEMORY VIEW    {memorySummary}");
        _factsTab.Text = _facts.Text;
        _memory.Text = selected is null ? string.Empty : memory.Count == 0
            ? "No non-zero public memory cells were published."
            : string.Join(Environment.NewLine,
                memory.Select(cell => $"{cell.Address,4}  {cell.Value,10}"));
        if (selected is { DebugText: { } debug } && !string.IsNullOrWhiteSpace(debug))
            _memory.Text += $"{Environment.NewLine}{Environment.NewLine}DEBUG{Environment.NewLine}{debug}";

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

        OrganismSnapshot? FindActor(OrganismKey? focus) => focus is { } key
            ? snapshot.Organisms.FirstOrDefault(value => value.Slot == key.Slot && value.Generation == key.Generation)
            : null;
    }

    private void RefreshMutationDetails()
    {
        if (_mutationDetails is null || _tabs.SelectedIndex != (int)RobotDetailsTab.MutationDetails) return;
        var log = _mutationLog;
        if (ReferenceEquals(log, _displayedMutationLog)) return;
        var previous = _displayedMutationLog;
        _displayedMutationLog = log;
        if (log is not null && previous is not null && log.SequenceEqual(previous, StringComparer.Ordinal)) return;

        _mutationDetails.Text = log switch
        {
            null => string.Empty,
            { Count: 0 } => "No mutations",
            _ => string.Join(Environment.NewLine, log),
        };
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
