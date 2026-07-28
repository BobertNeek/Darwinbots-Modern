using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Controls;
using Darwinbots.Desktop.Core.LegacyMenus;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class LegacyMenuBarTests
{
    [Fact]
    public async Task BuildsEveryHistoricMenuAndCommandFromTheCatalog()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var menu = new LegacyMenuBar();

            Assert.Equal(LegacyMenuCatalog.TopLevel.Count, menu.TopLevelItems.Count);
            Assert.Equal(
                LegacyMenuCatalog.TopLevel.Select(item => item.Label),
                menu.TopLevelItems.Select(item => item.Header));

            foreach (var command in LegacyMenuCatalog.FlattenLeaves().Select(item => item.Command!.Value))
            {
                var commandItem = menu.FindCommandItem(command);
                Assert.NotNull(commandItem);
                Assert.Equal($"Command{command}", commandItem.Name);
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task EmitsTypedCommandAndRefreshesCheckedEnabledAndReasonState()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var menu = new LegacyMenuBar();
            LegacyCommandId? invoked = null;
            menu.CommandInvoked += (_, args) => invoked = args.Command;

            var vision = menu.FindCommandItem(LegacyCommandId.ViewShowVisionGrid)!;
            vision.RaiseEvent(new RoutedEventArgs(MenuItem.ClickEvent));
            Assert.Equal(LegacyCommandId.ViewShowVisionGrid, invoked);

            menu.RefreshStates(command => command switch
            {
                LegacyCommandId.ViewShowVisionGrid => new LegacyCommandState(true, true),
                LegacyCommandId.InternetEnabled => new LegacyCommandState(false, false, "Online play is outside the compatibility scope."),
                _ => LegacyCommandState.Available,
            });

            Assert.True(vision.IsChecked);
            var internet = menu.FindCommandItem(LegacyCommandId.InternetEnabled)!;
            Assert.False(internet.IsEnabled);
            Assert.Equal("Online play is outside the compatibility scope.", ToolTip.GetTip(internet));
        }, CancellationToken.None);
    }
}
