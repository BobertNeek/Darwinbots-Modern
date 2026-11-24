using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;

namespace Darwinbots.Desktop.Views;

public sealed class LegacyConfirmationWindow : Window
{
    public LegacyConfirmationWindow(string title, string question, bool informationOnly = false, bool okCancel = false)
    {
        Title = $"Darwinbots — {title}";
        Width = informationOnly ? 480 : 400;
        if (informationOnly) SizeToContent = SizeToContent.Height;
        else Height = 160;
        CanResize = false;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;
        Background = Brush.Parse("#F4F0E6");
        var prompt = new TextBlock { Text = question, TextWrapping = TextWrapping.Wrap };
        var accept = new Button { Content = informationOnly || okCancel ? "OK" : "YES", IsDefault = true, IsCancel = informationOnly };
        accept.Click += (_, _) => Close(true);
        var actions = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            HorizontalAlignment = HorizontalAlignment.Right,
            Spacing = 8,
            Children = { accept },
        };
        if (!informationOnly)
        {
            var no = new Button { Content = okCancel ? "CANCEL" : "NO", IsCancel = true };
            no.Click += (_, _) => Close(false);
            actions.Children.Add(no);
        }
        Content = new StackPanel
        {
            Margin = new Thickness(18),
            Spacing = 18,
            Children = { prompt, actions },
        };
    }
}
