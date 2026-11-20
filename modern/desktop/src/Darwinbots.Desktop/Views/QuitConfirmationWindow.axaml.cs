using Avalonia.Controls;
using Avalonia.Interactivity;

namespace Darwinbots.Desktop.Views;

public sealed partial class QuitConfirmationWindow : Window
{
    public QuitConfirmationWindow() => InitializeComponent();

    private void Yes_Click(object? sender, RoutedEventArgs e) => Close(true);

    private void No_Click(object? sender, RoutedEventArgs e) => Close(false);
}
