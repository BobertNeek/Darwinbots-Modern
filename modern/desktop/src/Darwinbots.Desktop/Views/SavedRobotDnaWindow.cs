using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;

namespace Darwinbots.Desktop.Views;

internal sealed class SavedRobotDnaWindow : Window
{
    public SavedRobotDnaWindow(string filename)
    {
        Title = "Robot DNA saved";
        Width = 410;
        SizeToContent = SizeToContent.Height;
        CanResize = false;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        var question = new TextBlock
        {
            Text = $"Do you want to change robot's name to {filename} ?",
            TextWrapping = TextWrapping.Wrap,
        };
        var yes = new Button { Content = "Yes", IsDefault = true };
        var no = new Button { Content = "No", IsCancel = true };
        yes.Click += (_, _) => Close(true);
        no.Click += (_, _) => Close(false);
        var actions = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            HorizontalAlignment = HorizontalAlignment.Right,
            Spacing = 8,
            Children = { yes, no },
        };
        Content = new StackPanel { Margin = new Thickness(20), Spacing = 20, Children = { question, actions } };
    }
}
