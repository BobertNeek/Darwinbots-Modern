using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;

namespace Darwinbots.Desktop.Views;

internal enum LiveQuitChoice { Cancel, SaveSettings, KeepSettings }

internal sealed class LiveQuitSettingsWindow : Window
{
    public LiveQuitSettingsWindow(bool hasLastExitSettings)
    {
        Title = "Warning!";
        Width = 410;
        SizeToContent = SizeToContent.Height;
        CanResize = false;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        var question = new TextBlock
        {
            Text = hasLastExitSettings
                ? "Would you like to save changes to the settings? Press Cancel to return to the program."
                : "Are you sure you want to quit?",
            TextWrapping = Avalonia.Media.TextWrapping.Wrap,
        };
        var actions = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            HorizontalAlignment = HorizontalAlignment.Right,
            Spacing = 8,
        };
        var yes = new Button { Content = "Yes", IsDefault = true };
        yes.Click += (_, _) => Close(LiveQuitChoice.SaveSettings);
        var no = new Button { Content = "No", IsCancel = !hasLastExitSettings };
        no.Click += (_, _) => Close(hasLastExitSettings ? LiveQuitChoice.KeepSettings : LiveQuitChoice.Cancel);
        actions.Children.Add(yes);
        actions.Children.Add(no);
        if (hasLastExitSettings)
        {
            var cancel = new Button { Content = "Cancel", IsCancel = true };
            cancel.Click += (_, _) => Close(LiveQuitChoice.Cancel);
            actions.Children.Add(cancel);
        }
        Content = new StackPanel { Margin = new Thickness(20), Spacing = 20, Children = { question, actions } };
    }
}
