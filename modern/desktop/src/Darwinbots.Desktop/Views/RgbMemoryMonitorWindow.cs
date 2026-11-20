using Avalonia;
using Avalonia.Automation;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Media;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;

namespace Darwinbots.Desktop.Views;

public sealed class RgbMemoryMonitorWindow : Window, IPersistentSnapshotToolWindow
{
    private readonly OrganismKey? _focus;
    private readonly Action<RgbMemoryMonitorSettings> _applySettings;
    private readonly IDesktopStorageService _storage;
    private readonly NumericUpDown[] _addresses = new NumericUpDown[3];
    private readonly NumericUpDown[] _floors = new NumericUpDown[3];
    private readonly NumericUpDown[] _ceilings = new NumericUpDown[3];
    private readonly TextBlock _values = new();
    private readonly TextBlock _status = new();
    private readonly ComboBox _presets = new();
    private RgbMemoryMonitorSettings _settings;

    public RgbMemoryMonitorWindow(
        OrganismKey? focus, RgbMemoryMonitorSettings settings, Action<RgbMemoryMonitorSettings> applySettings,
        bool closeOnApply = false, IDesktopStorageService? storage = null)
    {
        _focus = focus;
        _settings = settings;
        _applySettings = applySettings ?? throw new ArgumentNullException(nameof(applySettings));
        _storage = storage ?? AvaloniaDesktopStorageService.Instance;
        CloseOnApply = closeOnApply;
        Title = closeOnApply || focus is not { } robot
            ? "Darwinbots — RGB Memory Monitor Settings"
            : $"Darwinbots — RGB Memory Monitor {robot.Slot}:{robot.Generation}";
        Width = 700;
        Height = 460;
        MinWidth = 620;
        MinHeight = 400;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;
        Background = Brush.Parse("#F4F0E6");

        _status.Foreground = Brush.Parse("#537B43");
        _status.FontFamily = DesktopFonts.Heading;
        _status.FontSize = 11;
        _values.FontFamily = DesktopFonts.Inspector;
        _values.FontSize = 16;
        _values.Text = "NO PUBLISHED ROBOT SNAPSHOT";
        _values.IsVisible = focus is not null;
        if (focus is null) _status.Text = "CHANNEL MAPPING FOR ALL ROBOTS";

        _presets.ItemsSource = RgbMemoryMonitorSettings.Presets.Select(value => value.Name).ToArray();
        _presets.PlaceholderText = "Choose preset";
        _presets.SelectedItem = RgbMemoryMonitorSettings.Presets.FirstOrDefault(value => value.Settings == settings)?.Name;
        _presets.SelectionChanged += (_, _) =>
        {
            if (_presets.SelectedItem is string name)
                SetControls(RgbMemoryMonitorSettings.Preset(name));
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
            _addresses[index] = Number(1, 999, channels[index].Address, $"{labels[index]} address");
            _floors[index] = Number(-32_000, 32_000, channels[index].Floor, $"{labels[index]} floor");
            _ceilings[index] = Number(-32_000, 32_000, channels[index].Ceiling, $"{labels[index]} ceiling");
            grid.Children.Add(_addresses[index].With(Grid.RowProperty, row).With(Grid.ColumnProperty, 1));
            grid.Children.Add(_floors[index].With(Grid.RowProperty, row).With(Grid.ColumnProperty, 2));
            grid.Children.Add(_ceilings[index].With(Grid.RowProperty, row).With(Grid.ColumnProperty, 3));
        }

        var body = new StackPanel { Spacing = 9, Margin = new Thickness(18) };
        var presetActions = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 8,
            Children =
            {
                new TextBlock { Text = "PRESET", VerticalAlignment = VerticalAlignment.Center, FontWeight = FontWeight.Bold },
                _presets,
            },
        };
        if (closeOnApply)
        {
            presetActions.Children.Add(Button("LOAD PRESET", LoadPreset_Click));
            presetActions.Children.Add(Button("SAVE PRESET", SavePreset_Click));
        }
        body.Children.Add(presetActions);
        body.Children.Add(grid);
        body.Children.Add(_values);
        body.Children.Add(new TextBlock { Text = "Each channel is normalized from its floor and ceiling to 0–255. The viewport uses this color for live Rust snapshot organisms.", TextWrapping = TextWrapping.Wrap, Foreground = Brush.Parse("#69716D"), FontSize = 11 });

        var apply = Button("APPLY", Apply_Click);
        var cancel = Button(closeOnApply ? "CANCEL" : "CLOSE", (_, _) => Close());
        var actions = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right, Spacing = 8, Children = { apply, cancel } };
        var root = new Grid { RowDefinitions = new RowDefinitions("62,*,54") };
        root.Children.Add(new Border { Background = Brush.Parse("#FBF9F2"), BorderBrush = Brush.Parse("#C8C3B6"), BorderThickness = new Thickness(0, 0, 0, 1), Padding = new Thickness(20, 12), Child = new StackPanel { Spacing = 2, Children = { new TextBlock { Text = "RGB MEMORY MONITOR", FontFamily = DesktopFonts.Heading, FontSize = 21, FontWeight = FontWeight.Bold, LetterSpacing = 1.3 }, _status } } });
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

    public void SetSettings(RgbMemoryMonitorSettings settings)
    {
        _settings = settings;
        StageSettings(settings);
    }

    private void StageSettings(RgbMemoryMonitorSettings settings)
    {
        _presets.SelectedItem = RgbMemoryMonitorSettings.Presets.FirstOrDefault(value => value.Settings == settings)?.Name;
        SetControls(settings);
    }

    public void SetSnapshot(EngineSnapshot snapshot)
    {
        if (_focus is not { } focus) return;
        var organism = snapshot.Organisms.FirstOrDefault(value => value.Slot == focus.Slot && value.Generation == focus.Generation);
        if (organism is null)
        {
            _status.Text = $"FOCUS LOST · {focus.Slot}:{focus.Generation} IS NOT BOUND TO A REPLACEMENT";
            _values.Text = "NO LIVE VALUES";
            return;
        }
        var color = RgbMemoryMonitor.ColorFor(organism, _settings);
        _status.Text = $"LIVE ROBOT {focus.Slot}:{focus.Generation} · TICK {snapshot.Tick:N0}";
        _values.Text = $"RGB  {color.Red,3}  {color.Green,3}  {color.Blue,3}    #{color.Red:X2}{color.Green:X2}{color.Blue:X2}";
    }

    private async void LoadPreset_Click(object? sender, RoutedEventArgs e)
    {
        try
        {
            if (await _storage.OpenRgbMonitorPresetAsync(this) is not { } bytes) return;
            StageSettings(Db2RgbMonitorPresetCodec.Decode(bytes));
            _status.Text = "RGB PRESET LOADED";
        }
        catch (Exception error) { _status.Text = $"RGB PRESET LOAD FAILED · {error.Message}"; }
    }

    private async void SavePreset_Click(object? sender, RoutedEventArgs e)
    {
        try
        {
            var bytes = Db2RgbMonitorPresetCodec.Encode(ReadControls());
            if (await _storage.SaveRgbMonitorPresetAsync(this, bytes))
                _status.Text = "RGB PRESET SAVED";
        }
        catch (Exception error) { _status.Text = $"RGB PRESET SAVE FAILED · {error.Message}"; }
    }

    private void Apply_Click(object? sender, RoutedEventArgs e)
    {
        try
        {
            var settings = ReadControls();
            settings.Validate();
            _applySettings(settings);
            _settings = settings;
            Accepted = true;
            if (_focus is null) _status.Text = "RGB SETTINGS APPLIED";
            if (CloseOnApply) Close();
        }
        catch (Exception error) { _status.Text = $"RGB SETTINGS INVALID · {error.Message}"; }
    }

    private RgbMemoryMonitorSettings ReadControls() => new(Channel(0), Channel(1), Channel(2));

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

    private static NumericUpDown Number(decimal minimum, decimal maximum, decimal value, string name) =>
        new NumericUpDown { Minimum = minimum, Maximum = maximum, Value = value, Width = 140 }
            .With(AutomationProperties.NameProperty, name);
    private static void AddHeader(Grid grid, string text, int column, int row) { var item = new TextBlock { Text = text, FontSize = 11, FontWeight = FontWeight.Bold, Foreground = Brush.Parse("#69716D") }.With(Grid.ColumnProperty, column).With(Grid.RowProperty, row); grid.Children.Add(item); }
    private static Button Button(string text, EventHandler<RoutedEventArgs> handler) { var button = new Button { Content = text, Background = Brush.Parse("#416E35"), Foreground = Brushes.White, FontWeight = FontWeight.Bold, Padding = new Thickness(12, 7) }; button.Click += handler; return button; }
}

internal static class ControlExtensions
{
    public static T With<T>(this T control, AvaloniaProperty property, object value) where T : Control { control.SetValue(property, value); return control; }
}
