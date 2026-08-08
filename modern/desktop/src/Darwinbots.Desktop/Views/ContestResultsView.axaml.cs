using Avalonia.Controls;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class ContestResultsView : UserControl
{
    private ContestResultsViewModel _viewModel;

    public ContestResultsView() : this(new ContestResultsViewModel(ContestResultSnapshot.Empty))
    {
    }

    public ContestResultsView(ContestResultsViewModel viewModel)
    {
        _viewModel = viewModel ?? throw new ArgumentNullException(nameof(viewModel));
        InitializeComponent();
        DataContext = _viewModel;
        EmptyText.IsVisible = _viewModel.Competitors.Count == 0;
    }

    public ContestResultsViewModel ViewModel => _viewModel;

    public void SetResult(ContestResultsViewModel viewModel)
    {
        _viewModel = viewModel ?? throw new ArgumentNullException(nameof(viewModel));
        DataContext = _viewModel;
        EmptyText.IsVisible = _viewModel.Competitors.Count == 0;
    }

    private void ManualWin_Click(object? sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: string id }) _viewModel.AwardManualWin(id);
    }

    private void InstantWin_Click(object? sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: string id }) _viewModel.AwardInstantWin(id);
    }

    private void Close_Click(object? sender, RoutedEventArgs e)
    {
        if (TopLevel.GetTopLevel(this) is Window window) window.Close();
    }
}
