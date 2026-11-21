using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using System.Globalization;
using System.Text;
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
    private const string NativeDeadMemory = "This robot is dead";
    private static long _nextPostDnaMemoryRequestId;
    private readonly SimulationSession? _session;
    private readonly Func<OrganismKey?>? _readFocus;
    private OrganismKey? _focus;
    private ulong _followedTick;
    private long _dnaReadVersion;
    private ulong _postDnaMemoryRequestId;
    private OrganismKey? _postDnaMemoryFocus;
    private OrganismKey? _nativeMemoryRemovedFocus;
    private bool _closed;
    private readonly RobotDiagnosticModel _diagnostic = new();
    private readonly TextBlock _subtitle = new();
    private readonly TextBlock _facts = new();
    private readonly TextBlock _factsTab = new();
    private readonly TextBox _dna = new();
    private readonly TextBox _memory = new();
    private readonly CheckBox _postDnaMemory = new()
    {
        Name = "PostDnaMemory",
        Content = "Display memory after DNA execution, before physics",
        IsVisible = false,
    };
    private readonly TextBlock _activationSummary = new();
    private readonly StackPanel _activationTimeline = new();
    private readonly TextBlock _status = new();
    private readonly TabControl _tabs = new();

    public RobotDetailsWindow(EngineSnapshot snapshot, OrganismKey focus, string dna = "")
        : this(null, focus, RobotDetailsTab.Facts, dna)
    {
        SetSnapshot(snapshot);
    }

    public RobotDetailsWindow(
        SimulationSession session,
        OrganismKey focus,
        RobotDetailsTab initialTab = RobotDetailsTab.Facts,
        Func<OrganismKey?>? readFocus = null)
        : this(session, focus, initialTab, string.Empty, readFocus)
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
                Child = new DockPanel
                {
                    Children = { _postDnaMemory, _memory },
                },
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
        DockPanel.SetDock(_postDnaMemory, Dock.Top);
        _postDnaMemory.IsCheckedChanged += (_, _) => SynchronizePostDnaMemoryWatch();
        _tabs.SelectionChanged += (_, _) => SynchronizePostDnaMemoryWatch();
        PropertyChanged += (_, change) =>
        {
            if (change.Property != IsVisibleProperty) return;
            if (!IsVisible) ClearNativeMemoryDeath();
            SynchronizePostDnaMemoryWatch();
        };

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

    private async Task RefreshDnaAsync()
    {
        var version = ++_dnaReadVersion;
        if (_session is null || _focus is not { } focus) return;
        try
        {
            var dna = await _session.ExportDnaAsync(focus.Slot, focus.Generation);
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
        _closed = true;
        SynchronizePostDnaMemoryWatch();
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
            SynchronizePostDnaMemoryWatch();
            if (_readFocus is not null && snapshot.Tick != _followedTick)
                SetFollowedSnapshot(snapshot);
            else
                SetSnapshot(snapshot);
        });
    }

    internal void SynchronizePostDnaMemoryWatch(bool reset = false)
    {
        if (_session is null || _readFocus is null) return;
        var snapshot = _session.LatestSnapshot;
        _postDnaMemory.IsVisible = snapshot.NativeCycle.HasValue
            && string.Equals(snapshot.Backend, "CPU", StringComparison.OrdinalIgnoreCase);
        var focus = !_closed && IsVisible && _postDnaMemory.IsVisible && _postDnaMemory.IsChecked == true
            && _tabs.SelectedIndex == (int)RobotDetailsTab.Memory ? _readFocus() : null;
        if (focus == _postDnaMemoryFocus && (!reset || focus is null)) return;
        _postDnaMemoryFocus = focus;
        var requestId = (ulong)Interlocked.Increment(ref _nextPostDnaMemoryRequestId);
        _postDnaMemoryRequestId = requestId;
        _ = ConfigurePostDnaMemoryWatchAsync(_session, requestId, focus);
        if (!_closed) SetSnapshot(snapshot);
    }

    private async Task ConfigurePostDnaMemoryWatchAsync(SimulationSession session, ulong requestId, OrganismKey? focus)
    {
        try { await session.ConfigureNativePostDnaMemoryWatchAsync(requestId, focus); }
        catch (Exception error)
        {
            if (IsVisible && requestId == _postDnaMemoryRequestId && focus.HasValue)
                _memory.Text = $"POST-DNA MEMORY UNAVAILABLE · {error.Message}";
        }
    }

    internal void SetFollowedSnapshot(EngineSnapshot snapshot)
    {
        _followedTick = snapshot.Tick;
        var focus = _readFocus?.Invoke();
        if (focus != _focus)
        {
            _focus = focus;
            if (focus.HasValue && focus != _nativeMemoryRemovedFocus)
                _nativeMemoryRemovedFocus = null;
            Title = focus is { } key ? $"Darwinbots — Robot Details {key.Slot}:{key.Generation}" : "Darwinbots — Robot Details";
            _dna.Text = string.Empty;
            _status.Text = "Engine-owned values refresh each published cycle.";
            _ = RefreshDnaAsync();
        }
        SetSnapshot(snapshot);
    }

    internal void ShowNativeMemoryDeath(OrganismKey focus)
    {
        if (!IsVisible || _focus != focus || _tabs.SelectedIndex != (int)RobotDetailsTab.Memory)
            return;

        _nativeMemoryRemovedFocus = focus;
        _memory.Text = NativeDeadMemory;
    }

    internal void ClearNativeMemoryDeath()
    {
        if (_nativeMemoryRemovedFocus is null) return;

        _nativeMemoryRemovedFocus = null;
        _memory.Text = string.Empty;
    }

    public void SetSnapshot(EngineSnapshot snapshot)
    {
        _diagnostic.Update(snapshot, _focus);
        // An older queued snapshot can still contain the robot after the menu reported its removal.
        var removedFocus = snapshot.NativeCycle.HasValue ? _nativeMemoryRemovedFocus : null;
        var organism = _focus is { } focus && focus != removedFocus ? snapshot.Organisms.FirstOrDefault(value =>
            value.Slot == focus.Slot && value.Generation == focus.Generation) : null;
        if (organism is null)
        {
            _subtitle.Text = _focus is { } removed ? $"ROBOT {removed.Slot}:{removed.Generation} · REMOVED" : "NO ROBOT SELECTED";
            _facts.Text = _focus is null ? "Select a robot in the simulation." : "The focused robot no longer exists in the live snapshot.";
            _factsTab.Text = _facts.Text;
            _memory.Text = removedFocus.HasValue ? NativeDeadMemory : string.Empty;
            _activationTimeline.Children.Clear();
            _activationSummary.Text = _diagnostic.ActivationSummary;
            return;
        }

        var species = snapshot.Species.ElementAtOrDefault((int)organism.Species)?.Name ?? "UNASSIGNED";
        var dnaSummary = string.IsNullOrWhiteSpace(_dna.Text)
            ? "UNAVAILABLE"
            : _dna.Text.Replace("\r\n", " / ", StringComparison.Ordinal).Replace('\n', ' ');
        var memorySummary = organism.Memory.Count == 0
            ? "NONE"
            : string.Join(", ", organism.Memory.Select(cell => cell.Address.ToString(CultureInfo.InvariantCulture)));
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
            $"TIES           {snapshot.Ties.Count(tie => tie.First == _focus || tie.Second == _focus):N0}",
            $"SHOTS          {snapshot.Shots.Count(shot => shot.Alive && shot.Owner == _focus):N0}",
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
        if (snapshot.NativeCycle.HasValue)
        {
            var memory = _postDnaMemory.IsVisible && _postDnaMemory.IsChecked == true
                && snapshot.NativePostDnaMemory is { } sample && sample.RequestId == _postDnaMemoryRequestId
                && sample.Tick == snapshot.Tick && sample.Focus == _focus ? sample.Memory : organism.Memory;
            _memory.Text = FormatNativeMemory(memory);
        }
        else
        {
            _memory.Text = FormatMemory(organism.Memory);
            if (!string.IsNullOrWhiteSpace(organism.DebugText))
                _memory.Text += $"{Environment.NewLine}{Environment.NewLine}DEBUG{Environment.NewLine}{organism.DebugText}";
        }

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

    private static string FormatMemory(IReadOnlyList<MemoryCellSnapshot> memory) =>
        memory.Count == 0 ? "No non-zero public memory cells were published."
            : string.Join(Environment.NewLine, memory.Select(cell => $"{cell.Address,4}  {cell.Value,10}"));

    private static string FormatNativeMemory(IReadOnlyList<MemoryCellSnapshot> memory)
    {
        const int cellCount = 1_000;
        const int rowWidth = 10;
        Span<int> values = stackalloc int[cellCount];
        values.Clear();
        foreach (var cell in memory)
        {
            if (cell.Address is >= 1 and <= cellCount)
                values[cell.Address - 1] = cell.Value;
        }

        var text = new StringBuilder(cellCount * 4);
        for (var row = 0; row < cellCount; row += rowWidth)
        {
            text.Append(CultureInfo.InvariantCulture, $" {row + 1}:");
            for (var index = row; index < row + rowWidth; index++)
            {
                text.Append('\t');
                var value = values[index];
                if (value >= 0) text.Append(' '); // VB Str$ reserves a place for the sign.
                text.Append(CultureInfo.InvariantCulture, $"{value}");
            }
            text.Append("\r\n");
        }
        return text.ToString();
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
