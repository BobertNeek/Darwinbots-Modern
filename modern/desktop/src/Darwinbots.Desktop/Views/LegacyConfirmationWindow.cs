using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;

namespace Darwinbots.Desktop.Views;

public sealed class LegacyConfirmationWindow : Window
{
    public LegacyConfirmationWindow(string title, string question)
    {
        Title = $"Darwinbots — {title}";
        Width = 400;
        Height = 160;
        CanResize = false;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;
        Background = Brush.Parse("#F4F0E6");
        var prompt = new TextBlock { Text = question, TextWrapping = TextWrapping.Wrap };
        var yes = new Button { Content = "YES", IsDefault = true };
        var no = new Button { Content = "NO", IsCancel = true };
        yes.Click += (_, _) => Close(true);
        no.Click += (_, _) => Close(false);
        var actions = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            HorizontalAlignment = HorizontalAlignment.Right,
            Spacing = 8,
            Children = { yes, no },
        };
        Content = new StackPanel
        {
            Margin = new Thickness(18),
            Spacing = 18,
            Children = { prompt, actions },
        };
    }
}
