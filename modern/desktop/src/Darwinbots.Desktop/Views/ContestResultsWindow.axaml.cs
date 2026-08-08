using Avalonia.Controls;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class ContestResultsWindow : Window
{
    public ContestResultsWindow() : this(ContestResultSnapshot.Empty)
    {
    }

    public ContestResultsWindow(ContestResultSnapshot result, LocalContestAdjudicationLog? log = null)
    {
        InitializeComponent();
        AdjudicationLog = log ?? new LocalContestAdjudicationLog();
        ViewModel = new ContestResultsViewModel(result, AdjudicationLog.Add);
        ResultsView.SetResult(ViewModel);
    }

    public ContestResultsViewModel ViewModel { get; }
    public LocalContestAdjudicationLog AdjudicationLog { get; }
}
