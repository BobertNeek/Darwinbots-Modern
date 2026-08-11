using Avalonia.Controls;
using Avalonia.Interactivity;

namespace Darwinbots.Desktop.Views;

public sealed partial class DeleteConfirmationWindow : Window
{
    public DeleteConfirmationWindow(string message)
    {
        InitializeComponent();
        MessageText.Text = message;
    }

    public bool Confirmed { get; private set; }

    private void Confirm_Click(object? sender, RoutedEventArgs e)
    {
        Confirmed = true;
        Close();
    }

    private void Cancel_Click(object? sender, RoutedEventArgs e) => Close();
}
