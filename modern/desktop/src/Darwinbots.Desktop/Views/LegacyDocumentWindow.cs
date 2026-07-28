using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;

namespace Darwinbots.Desktop.Views;

public sealed class LegacyDocumentWindow : Window
{
    public LegacyDocumentWindow(string title, string section, string content)
    {
        Title = $"Darwinbots — {title}";
        Width = 660;
        Height = 560;
        MinWidth = 480;
        MinHeight = 360;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;
        Background = Brush.Parse("#F4F0E6");

        var root = new Grid { RowDefinitions = new RowDefinitions("72,*,58") };
        var header = new Border
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
                    new TextBlock { Text = title.ToUpperInvariant(), FontFamily = new FontFamily("Bahnschrift"), FontSize = 22, FontWeight = FontWeight.Bold, LetterSpacing = 1.4 },
                    new TextBlock { Text = section.ToUpperInvariant(), Foreground = Brush.Parse("#537B43"), FontFamily = new FontFamily("Bahnschrift"), FontSize = 11, FontWeight = FontWeight.Bold, LetterSpacing = 1.2 },
                },
            },
        };
        root.Children.Add(header);

        var document = new TextBox
        {
            Text = content,
            IsReadOnly = true,
            AcceptsReturn = true,
            TextWrapping = TextWrapping.Wrap,
            Background = Brush.Parse("#FFFCF5"),
            Foreground = Brush.Parse("#1D2824"),
            BorderBrush = Brush.Parse("#D4CFC1"),
            BorderThickness = new Thickness(1),
            Padding = new Thickness(18),
            Margin = new Thickness(18),
            FontFamily = new FontFamily("Georgia"),
            FontSize = 14,
        };
        Grid.SetRow(document, 1);
        root.Children.Add(document);

        var close = new Button
        {
            Content = "CLOSE",
            HorizontalAlignment = HorizontalAlignment.Right,
            Background = Brush.Parse("#416E35"),
            Foreground = Brushes.White,
            FontWeight = FontWeight.Bold,
            Padding = new Thickness(20, 8),
        };
        close.Click += (_, _) => Close();
        var footer = new Border
        {
            Background = Brush.Parse("#E5E0D4"),
            BorderBrush = Brush.Parse("#C8C3B6"),
            BorderThickness = new Thickness(0, 1, 0, 0),
            Padding = new Thickness(14, 9),
            Child = close,
        };
        Grid.SetRow(footer, 2);
        root.Children.Add(footer);
        Content = root;
    }
}
