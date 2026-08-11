using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Platform.Storage;
using Darwinbots.Desktop.Core;
using System.Collections.ObjectModel;

namespace Darwinbots.Desktop.Views;

public sealed class PlayerBotSettingsWindow : Window
{
    private readonly ObservableCollection<PlayerBotBinding> _bindings;
    private readonly ListBox _list;
    private readonly TextBox _key = new() { PlaceholderText = "key code", Width = 95 };
    private readonly TextBox _memory = new() { PlaceholderText = "memory", Width = 95 };
    private readonly TextBox _value = new() { PlaceholderText = "value", Width = 95 };
    private readonly CheckBox _invert = new() { Content = "invert" };
    private readonly TextBlock _status = new() { Foreground = Brush.Parse("#9A4A31"), FontSize = 11 };
    private bool _capturing;

    public bool Accepted { get; private set; }
    public IReadOnlyList<PlayerBotBinding> Bindings => _bindings.ToArray();

    public PlayerBotSettingsWindow(IReadOnlyList<PlayerBotBinding> bindings)
    {
        Title = "Darwinbots — PlayerBot Key Map";
        Width = 620;
        Height = 520;
        MinWidth = 520;
        MinHeight = 420;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;
        Background = Brush.Parse("#F4F0E6");
        _bindings = new ObservableCollection<PlayerBotBinding>(bindings);
        _list = new ListBox { ItemsSource = _bindings, Background = Brush.Parse("#FFFCF5"), MinHeight = 180 };

        var capture = new Button { Content = "CAPTURE KEY" };
        capture.Click += (_, _) =>
        {
            _capturing = true;
            _status.Text = "PRESS A KEY NOW";
            Focus();
        };
        var add = new Button { Content = "ADD", Background = Brush.Parse("#416E35"), Foreground = Brushes.White };
        add.Click += (_, _) => AddBinding();
        var delete = new Button { Content = "DELETE" };
        delete.Click += (_, _) =>
        {
            if (_list.SelectedItem is PlayerBotBinding binding) _bindings.Remove(binding);
        };
        var load = new Button { Content = "LOAD" };
        load.Click += async (_, _) => await LoadAsync();
        var save = new Button { Content = "SAVE" };
        save.Click += async (_, _) => await SaveAsync();
        var apply = new Button { Content = "APPLY", Background = Brush.Parse("#416E35"), Foreground = Brushes.White, FontWeight = FontWeight.Bold };
        apply.Click += (_, _) => Apply();
        var cancel = new Button { Content = "CANCEL" };
        cancel.Click += (_, _) => Close();

        var input = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 6,
            Children = { _key, _memory, _value, _invert, capture, add, delete },
        };
        var body = new StackPanel
        {
            Margin = new Thickness(18),
            Spacing = 10,
            Children =
            {
                new TextBlock { Text = "KEY MAP · PRESSED XOR INVERT WRITES MEMORY AFTER SENSE CLEAR", FontWeight = FontWeight.Bold, Foreground = Brush.Parse("#537B43") },
                _list,
                input,
                new TextBlock { Text = "Key code · memory address 1–999 · value −32000…32000", Foreground = Brush.Parse("#66706B"), FontSize = 11 },
                _status,
            },
        };
        var footer = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right, Spacing = 8, Children = { load, save, cancel, apply } };
        var root = new Grid { RowDefinitions = new RowDefinitions("*,58") };
        root.Children.Add(body);
        var footerBorder = new Border { Background = Brush.Parse("#E5E0D4"), BorderBrush = Brush.Parse("#C8C3B6"), BorderThickness = new Thickness(0, 1, 0, 0), Padding = new Thickness(12, 9), Child = footer };
        Grid.SetRow(footerBorder, 1);
        root.Children.Add(footerBorder);
        Content = root;
        KeyDown += Capture_KeyDown;
    }

    private void Capture_KeyDown(object? sender, KeyEventArgs e)
    {
        if (!_capturing) return;
        _key.Text = ((int)e.Key).ToString(System.Globalization.CultureInfo.InvariantCulture);
        _capturing = false;
        _status.Text = $"CAPTURED {e.Key}";
        e.Handled = true;
    }

    private void AddBinding()
    {
        if (!int.TryParse(_key.Text, out var key) || !ushort.TryParse(_memory.Text, out var memory) || !int.TryParse(_value.Text, out var value))
        {
            _status.Text = "ENTER KEY, MEMORY, AND VALUE";
            return;
        }
        try
        {
            var binding = new PlayerBotBinding(key, memory, value, _invert.IsChecked == true);
            binding.Validate();
            _bindings.Add(binding);
            _status.Text = "BINDING ADDED";
        }
        catch (Exception error) { _status.Text = error.Message; }
    }

    private void Apply()
    {
        try
        {
            foreach (var binding in _bindings) binding.Validate();
            Accepted = true;
            Close();
        }
        catch (Exception error) { _status.Text = error.Message; }
    }

    private async Task LoadAsync()
    {
        try
        {
            var files = await StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions { Title = "Load PlayerBot key map", AllowMultiple = false, FileTypeFilter = [new FilePickerFileType("PlayerBot key map") { Patterns = ["*.pbkp"] }] });
            var path = files.FirstOrDefault()?.Path.LocalPath;
            if (path is null) return;
            _bindings.Clear();
            foreach (var binding in PlayerBotKeyMapFile.Load(path)) _bindings.Add(binding);
            _status.Text = "KEY MAP LOADED";
        }
        catch (Exception error) { _status.Text = error.Message; }
    }

    private async Task SaveAsync()
    {
        try
        {
            var file = await StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions { Title = "Save PlayerBot key map", SuggestedFileName = "playerbot.pbkp", DefaultExtension = "pbkp", FileTypeChoices = [new FilePickerFileType("PlayerBot key map") { Patterns = ["*.pbkp"] }] });
            var path = file?.Path.LocalPath;
            if (path is null) return;
            PlayerBotKeyMapFile.Save(path, _bindings);
            _status.Text = "KEY MAP SAVED";
        }
        catch (Exception error) { _status.Text = error.Message; }
    }
}
