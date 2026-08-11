using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed class RobotColorEditorWindow : Window
{
    private readonly RobotColorEditorState _state;
    private readonly Slider _redSlider;
    private readonly Slider _greenSlider;
    private readonly Slider _blueSlider;
    private readonly TextBlock _redValue;
    private readonly TextBlock _greenValue;
    private readonly TextBlock _blueValue;
    private readonly Border _colorPreview;
    private readonly TextBlock _status;
    private readonly Func<RobotColorEdit, Task>? _apply;

    public RobotColorEditorWindow(uint initialColor) : this(initialColor, null)
    {
    }

    public RobotColorEditorWindow(uint initialColor, Func<RobotColorEdit, Task>? apply)
    {
        _state = new RobotColorEditorState(initialColor);
        _apply = apply;

        Title = "Darwinbots — Custom Color";
        Width = 650;
        Height = 410;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;
        CanResize = false;
        Background = Brush.Parse("#F4F0E6");

        _redSlider = ChannelSlider("RedSlider", _state.Red);
        _greenSlider = ChannelSlider("GreenSlider", _state.Green);
        _blueSlider = ChannelSlider("BlueSlider", _state.Blue);
        _redValue = ChannelValue("RedValue", _state.Red);
        _greenValue = ChannelValue("GreenValue", _state.Green);
        _blueValue = ChannelValue("BlueValue", _state.Blue);
        _colorPreview = new Border
        {
            Name = "ColorPreview",
            Width = 150,
            Height = 150,
            Background = new SolidColorBrush(Avalonia.Media.Color.FromRgb(_state.Red, _state.Green, _state.Blue)),
            BorderBrush = Brush.Parse("#202725"),
            BorderThickness = new Thickness(2),
            CornerRadius = new CornerRadius(4),
        };
        _status = new TextBlock { Name = "Status", Foreground = Brush.Parse("#66706B"), TextWrapping = TextWrapping.Wrap };

        _redSlider.ValueChanged += (_, _) => UpdateState();
        _greenSlider.ValueChanged += (_, _) => UpdateState();
        _blueSlider.ValueChanged += (_, _) => UpdateState();

        var root = new Grid { RowDefinitions = new RowDefinitions("58,*,72") };
        root.Children.Add(new Border
        {
            Background = Brush.Parse("#FBF9F2"),
            BorderBrush = Brush.Parse("#C8C3B6"),
            BorderThickness = new Thickness(0, 0, 0, 1),
            Padding = new Thickness(18, 12),
            Child = new TextBlock
            {
                Text = "CUSTOM COLOR",
                FontFamily = new FontFamily("Bahnschrift"),
                FontSize = 19,
                FontWeight = FontWeight.Bold,
                LetterSpacing = 1.2,
            },
        });

        var channels = new StackPanel
        {
            Spacing = 12,
            Children =
            {
                ChannelRow("R", _redSlider, _redValue),
                ChannelRow("G", _greenSlider, _greenValue),
                ChannelRow("B", _blueSlider, _blueValue),
            },
        };
        var preview = new StackPanel
        {
            Spacing = 8,
            HorizontalAlignment = HorizontalAlignment.Center,
            Children =
            {
                _colorPreview,
                new TextBlock { Text = "LIVE PREVIEW", HorizontalAlignment = HorizontalAlignment.Center, FontWeight = FontWeight.SemiBold },
                _status,
            },
        };
        var body = new Grid
        {
            ColumnDefinitions = new ColumnDefinitions("*,190"),
            ColumnSpacing = 24,
            Margin = new Thickness(20, 18),
            Children = { channels, preview },
        };
        Grid.SetColumn(preview, 1);
        Grid.SetRow(body, 1);
        root.Children.Add(body);

        var write = new Button
        {
            Name = "WriteColorToDnaButton",
            Content = "WRITE COLOR AS DATA INTO DNA",
            Background = Brush.Parse("#E6E2D8"),
        };
        write.Click += async (_, _) => await CommitAsync(true);
        var use = new Button
        {
            Name = "UseColorButton",
            Content = "USE THIS COLOR",
            Background = Brush.Parse("#416E35"),
            Foreground = Brushes.White,
            FontWeight = FontWeight.Bold,
        };
        use.Click += async (_, _) => await CommitAsync(false);
        var actions = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            HorizontalAlignment = HorizontalAlignment.Right,
            Spacing = 8,
            Children = { write, use },
        };
        var footer = new Border
        {
            Background = Brush.Parse("#E5E0D4"),
            BorderBrush = Brush.Parse("#C8C3B6"),
            BorderThickness = new Thickness(0, 1, 0, 0),
            Padding = new Thickness(12, 9),
            Child = actions,
        };
        Grid.SetRow(footer, 2);
        root.Children.Add(footer);

        Content = root;
    }

    public bool Accepted { get; private set; }

    public bool WriteToDna { get; private set; }

    public uint Color => _state.Color;

    public string PreviewHex => _state.PreviewHex;

    private static Slider ChannelSlider(string name, byte value) => new()
    {
        Name = name,
        Minimum = 0,
        Maximum = 255,
        Value = value,
        TickFrequency = 1,
    };

    private static TextBlock ChannelValue(string name, byte value) => new()
    {
        Name = name,
        Text = value.ToString(),
        HorizontalAlignment = HorizontalAlignment.Right,
        FontFamily = new FontFamily("Cascadia Mono, Consolas"),
    };

    private static Control ChannelRow(string label, Slider slider, TextBlock value)
    {
        var grid = new Grid
        {
            ColumnDefinitions = new ColumnDefinitions("48,*,48"),
            ColumnSpacing = 10,
        };
        grid.Children.Add(new TextBlock
        {
            Text = label,
            FontSize = 16,
            FontWeight = FontWeight.Bold,
            VerticalAlignment = VerticalAlignment.Center,
        });
        Grid.SetColumn(slider, 1);
        grid.Children.Add(slider);
        Grid.SetColumn(value, 2);
        grid.Children.Add(value);
        return grid;
    }

    private void UpdateState()
    {
        _state.SetRed((byte)Math.Clamp((int)Math.Round(_redSlider.Value), 0, 255));
        _state.SetGreen((byte)Math.Clamp((int)Math.Round(_greenSlider.Value), 0, 255));
        _state.SetBlue((byte)Math.Clamp((int)Math.Round(_blueSlider.Value), 0, 255));
        _redValue.Text = _state.Red.ToString();
        _greenValue.Text = _state.Green.ToString();
        _blueValue.Text = _state.Blue.ToString();
        _colorPreview.Background = new SolidColorBrush(Avalonia.Media.Color.FromRgb(_state.Red, _state.Green, _state.Blue));
    }

    private async Task CommitAsync(bool writeToDna)
    {
        try
        {
            if (_apply is not null)
                await _apply(new RobotColorEdit(Color, writeToDna));
            WriteToDna = writeToDna;
            Accepted = true;
            Close();
        }
        catch (Exception error)
        {
            _status.Text = error.Message;
        }
    }
}
