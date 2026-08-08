using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.ViewModels;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class LocalModeWorkflowTests
{
    [Fact]
    public void LeagueTabWorkflowBuildsConfigStartsObservesAndCompletesLocalMode()
    {
        var viewModel = new MainWindowViewModel();

        viewModel.LocalMode.SelectKind(LocalModeKind.Contest);
        viewModel.LocalMode.Preset = LeaguePreset.F1;
        viewModel.LocalMode.MinimumRounds = 1;
        viewModel.LocalMode.MaximumRounds = 2;
        viewModel.LocalMode.MaximumCycles = 120;
        viewModel.LocalMode.MaximumPopulation = 16;
        viewModel.LocalMode.RestartOnExtinction = true;
        viewModel.LocalMode.ProhibitViruses = true;

        var config = viewModel.LocalMode.BuildConfiguration();
        Assert.Equal(LocalModeKind.Contest, config.Kind);
        Assert.Equal(LeaguePreset.F1, config.Preset);
        Assert.True(config.Restrictions.ProhibitViruses);
        Assert.Equal("CONFIGURED", viewModel.LocalMode.StatusLabel);

        viewModel.LocalMode.Observe(new LocalModeSnapshot(
            LocalModeStatus.Running,
            LocalModeKind.Contest,
            LeaguePreset.F1,
            1,
            2,
            18,
            120,
            0,
            [],
            null,
            config.Restrictions));
        Assert.Equal("ROUND 1 / 2", viewModel.LocalMode.RoundLabel);
        Assert.Equal("RUNNING", viewModel.LocalMode.StatusLabel);

        viewModel.LocalMode.Observe(new LocalModeSnapshot(
            LocalModeStatus.Completed,
            LocalModeKind.Contest,
            LeaguePreset.F1,
            2,
            2,
            120,
            120,
            0,
            [new LocalModeResult(2, 1, 90, 12_400, "operator decision")],
            new LocalModeAdjudication(1, "operator decision", false),
            config.Restrictions));
        Assert.Equal("COMPLETE", viewModel.LocalMode.StatusLabel);
        Assert.Equal("WINNER · SPECIES 1", viewModel.LocalMode.AdjudicationLabel);
    }
}
