using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;

namespace Darwinbots.Desktop.Views;

public sealed class LegacyValueWindow : Window
{
    private readonly TextBox _value;
    public bool Accepted { get; private set; }
    public string Value => _value.Text?.Trim() ?? string.Empty;

    public LegacyValueWindow(string title, string label, string initialValue, string hint)
    {
        Title = $"Darwinbots — {title}";
        Width = 520;
        Height = 250;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;
        CanResize = false;
        Background = Brush.Parse("#F4F0E6");
        var root = new Grid { RowDefinitions = new RowDefinitions("58,*,58") };
        root.Children.Add(new Border
        {
            Background = Brush.Parse("#FBF9F2"), BorderBrush = Brush.Parse("#C8C3B6"), BorderThickness = new Thickness(0, 0, 0, 1), Padding = new Thickness(18, 12),
            Child = new TextBlock { Text = title.ToUpperInvariant(), FontFamily = new FontFamily("Bahnschrift"), FontSize = 19, FontWeight = FontWeight.Bold, LetterSpacing = 1.2 },
        });
        _value = new TextBox { Text = initialValue, FontFamily = new FontFamily("Bahnschrift"), FontSize = 15 };
        var body = new StackPanel { Margin = new Thickness(18), Spacing = 7, Children = { new TextBlock { Text = label, FontWeight = FontWeight.SemiBold }, _value, new TextBlock { Text = hint, Foreground = Brush.Parse("#66706B"), FontSize = 11, TextWrapping = TextWrapping.Wrap } } };
        Grid.SetRow(body, 1); root.Children.Add(body);
        var cancel = new Button { Content = "CANCEL" }; cancel.Click += (_, _) => Close();
        var apply = new Button { Content = "APPLY", Background = Brush.Parse("#416E35"), Foreground = Brushes.White, FontWeight = FontWeight.Bold }; apply.Click += (_, _) => { Accepted = true; Close(); };
        var actions = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right, Spacing = 8, Children = { cancel, apply } };
        var footer = new Border { Background = Brush.Parse("#E5E0D4"), BorderBrush = Brush.Parse("#C8C3B6"), BorderThickness = new Thickness(0, 1, 0, 0), Padding = new Thickness(12, 9), Child = actions };
        Grid.SetRow(footer, 2); root.Children.Add(footer);
        Content = root;
        Opened += (_, _) => { _value.SelectAll(); _value.Focus(); };
    }
}
