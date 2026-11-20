using Avalonia;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Media;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed class RgbMemoryMonitorWindow : Window, IPersistentSnapshotToolWindow
{
    private readonly OrganismKey _focus;
    private readonly Action<RgbMemoryMonitorSettings> _applySettings;
    private readonly NumericUpDown[] _addresses = new NumericUpDown[3];
    private readonly NumericUpDown[] _floors = new NumericUpDown[3];
    private readonly NumericUpDown[] _ceilings = new NumericUpDown[3];
    private readonly TextBlock _values = new();
    private readonly TextBlock _status = new();
    private readonly ComboBox _presets = new();
    private RgbMemoryMonitorSettings _settings;

    public RgbMemoryMonitorWindow(OrganismKey focus, RgbMemoryMonitorSettings settings, Action<RgbMemoryMonitorSettings> applySettings, bool closeOnApply = false)
    {
        _focus = focus;
        _settings = settings;
        _applySettings = applySettings ?? throw new ArgumentNullException(nameof(applySettings));
        CloseOnApply = closeOnApply;
        Title = $"Darwinbots — RGB Memory Monitor {focus.Slot}:{focus.Generation}";
        Width = 700;
        Height = 460;
        MinWidth = 620;
        MinHeight = 400;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;
        Background = Brush.Parse("#F4F0E6");

        _status.Foreground = Brush.Parse("#537B43");
        _status.FontFamily = new FontFamily("Bahnschrift");
        _status.FontSize = 11;
        _values.FontFamily = new FontFamily("Consolas");
        _values.FontSize = 16;
        _values.Text = "NO PUBLISHED ROBOT SNAPSHOT";

        _presets.ItemsSource = RgbMemoryMonitorSettings.Presets.Select(value => value.Name).ToArray();
        _presets.SelectedIndex = 0;
        _presets.SelectionChanged += (_, _) =>
        {
            if (_presets.SelectedItem is string name)
            {
                _settings = RgbMemoryMonitorSettings.Preset(name);
                SetControls(_settings);
            }
        };

        var grid = new Grid
        {
            ColumnDefinitions = new ColumnDefinitions("90,*,*,*"),
            RowDefinitions = new RowDefinitions("Auto,Auto,Auto,Auto"),
            ColumnSpacing = 8,
            RowSpacing = 7,
            Margin = new Thickness(18),
        };
        AddHeader(grid, "CHANNEL", 0, 0);
        AddHeader(grid, "MEMORY", 1, 0);
        AddHeader(grid, "FLOOR", 2, 0);
        AddHeader(grid, "CEILING", 3, 0);
        var labels = new[] { "RED", "GREEN", "BLUE" };
        var channels = new[] { settings.Red, settings.Green, settings.Blue };
        for (var index = 0; index < 3; index++)
        {
            var row = index + 1;
            grid.Children.Add(new TextBlock { Text = labels[index], FontWeight = FontWeight.Bold, VerticalAlignment = VerticalAlignment.Center, Foreground = Brush.Parse(index == 0 ? "#A13D35" : index == 1 ? "#3E7C3B" : "#356D9C") }.With(Grid.RowProperty, row));
            _addresses[index] = Number(1, 999, channels[index].Address);
            _floors[index] = Number(-32_000, 32_000, channels[index].Floor);
            _ceilings[index] = Number(-32_000, 32_000, channels[index].Ceiling);
            grid.Children.Add(_addresses[index].With(Grid.RowProperty, row).With(Grid.ColumnProperty, 1));
            grid.Children.Add(_floors[index].With(Grid.RowProperty, row).With(Grid.ColumnProperty, 2));
            grid.Children.Add(_ceilings[index].With(Grid.RowProperty, row).With(Grid.ColumnProperty, 3));
        }

        var body = new StackPanel { Spacing = 9, Margin = new Thickness(18) };
        body.Children.Add(new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, Children = { new TextBlock { Text = "PRESET", VerticalAlignment = VerticalAlignment.Center, FontWeight = FontWeight.Bold }, _presets } });
        body.Children.Add(grid);
        body.Children.Add(_values);
        body.Children.Add(new TextBlock { Text = "Each channel is normalized from its floor and ceiling to 0–255. The viewport uses this color for live Rust snapshot organisms.", TextWrapping = TextWrapping.Wrap, Foreground = Brush.Parse("#69716D"), FontSize = 11 });

        var apply = Button("APPLY", Apply_Click);
        var cancel = Button("CLOSE", (_, _) => Close());
        var actions = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right, Spacing = 8, Children = { apply, cancel } };
        var root = new Grid { RowDefinitions = new RowDefinitions("62,*,54") };
        root.Children.Add(new Border { Background = Brush.Parse("#FBF9F2"), BorderBrush = Brush.Parse("#C8C3B6"), BorderThickness = new Thickness(0, 0, 0, 1), Padding = new Thickness(20, 12), Child = new StackPanel { Spacing = 2, Children = { new TextBlock { Text = "RGB MEMORY MONITOR", FontFamily = new FontFamily("Bahnschrift"), FontSize = 21, FontWeight = FontWeight.Bold, LetterSpacing = 1.3 }, _status } } });
        Grid.SetRow(body, 1);
        root.Children.Add(new ScrollViewer { Content = body });
        Grid.SetRow(actions, 2);
        root.Children.Add(new Border { Background = Brush.Parse("#E5E0D4"), BorderBrush = Brush.Parse("#C8C3B6"), BorderThickness = new Thickness(0, 1, 0, 0), Padding = new Thickness(12, 8), Child = actions });
        Content = root;
        SetControls(settings);
    }

    public bool CloseOnApply { get; }
    public bool Accepted { get; private set; }
    public RgbMemoryMonitorSettings Settings => _settings;

    public void SetSnapshot(EngineSnapshot snapshot)
    {
        var organism = snapshot.Organisms.FirstOrDefault(value => value.Slot == _focus.Slot && value.Generation == _focus.Generation);
        if (organism is null)
        {
            _status.Text = $"FOCUS LOST · {_focus.Slot}:{_focus.Generation} IS NOT BOUND TO A REPLACEMENT";
            _values.Text = "NO LIVE VALUES";
            return;
        }
        var color = RgbMemoryMonitor.ColorFor(organism, _settings);
        _status.Text = $"LIVE ROBOT {_focus.Slot}:{_focus.Generation} · TICK {snapshot.Tick:N0}";
        _values.Text = $"RGB  {color.Red,3}  {color.Green,3}  {color.Blue,3}    #{color.Red:X2}{color.Green:X2}{color.Blue:X2}";
    }

    private void Apply_Click(object? sender, Avalonia.Interactivity.RoutedEventArgs e)
    {
        try
        {
            _settings = new RgbMemoryMonitorSettings(
                Channel(0), Channel(1), Channel(2));
            _settings.Validate();
            _applySettings(_settings);
            Accepted = true;
            _status.Text = "RGB SETTINGS APPLIED TO VIEWPORT";
            if (CloseOnApply) Close();
        }
        catch (Exception error) { _status.Text = $"RGB SETTINGS INVALID · {error.Message}"; }
    }

    private RgbMemoryChannel Channel(int index) => new(
        (ushort)(_addresses[index].Value ?? 1),
        (int)(_floors[index].Value ?? 0),
        (int)(_ceilings[index].Value ?? 32_000));

    private void SetControls(RgbMemoryMonitorSettings settings)
    {
        var channels = new[] { settings.Red, settings.Green, settings.Blue };
        for (var index = 0; index < channels.Length; index++)
        {
            _addresses[index].Value = channels[index].Address;
            _floors[index].Value = channels[index].Floor;
            _ceilings[index].Value = channels[index].Ceiling;
        }
    }

    private static NumericUpDown Number(decimal minimum, decimal maximum, decimal value) => new() { Minimum = minimum, Maximum = maximum, Value = value, Width = 140 };
    private static void AddHeader(Grid grid, string text, int column, int row) { var item = new TextBlock { Text = text, FontSize = 11, FontWeight = FontWeight.Bold, Foreground = Brush.Parse("#69716D") }.With(Grid.ColumnProperty, column).With(Grid.RowProperty, row); grid.Children.Add(item); }
    private static Button Button(string text, EventHandler<RoutedEventArgs> handler) { var button = new Button { Content = text, Background = Brush.Parse("#416E35"), Foreground = Brushes.White, FontWeight = FontWeight.Bold, Padding = new Thickness(12, 7) }; button.Click += handler; return button; }
}

internal static class ControlExtensions
{
    public static T With<T>(this T control, AvaloniaProperty property, object value) where T : Control { control.SetValue(property, value); return control; }
}
