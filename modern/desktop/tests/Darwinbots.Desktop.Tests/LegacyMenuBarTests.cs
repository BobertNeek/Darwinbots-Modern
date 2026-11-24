using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.VisualTree;
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
            Assert.False(Assert.Single(menu.TopLevelItems, item => Equals(item.Header, "E-Grid")).IsVisible);

            foreach (var catalogItem in LegacyMenuCatalog.FlattenLeaves())
            {
                var command = catalogItem.Command!.Value;
                var commandItem = menu.FindCommandItem(command);
                Assert.NotNull(commandItem);
                Assert.Equal($"Command{command}", commandItem.Name);
                if (catalogItem.Shortcut is not null)
                {
                    var gesture = KeyGesture.Parse(catalogItem.Shortcut);
                    Assert.Equal(gesture, commandItem.InputGesture);
                    if (command is LegacyCommandId.HelpAbout or LegacyCommandId.FilePauseSimulation)
                        Assert.Null(commandItem.HotKey);
                    else
                        Assert.Equal(gesture, commandItem.HotKey);
                }
            }
        }, CancellationToken.None);
    }

    [Fact]
    public async Task FileMenuRendersShortcutLabelsAndDispatchesEachSaveGestureOnce()
    {
        await DesktopTestApplication.Session.Dispatch(() =>
        {
            var menu = new LegacyMenuBar();
            var run = new Button { Content = "Run" };
            var window = new Window { Content = new StackPanel { Children = { menu, run } } };
            var commands = new List<LegacyCommandId>();
            menu.CommandInvoked += (_, args) => commands.Add(args.Command);
            window.Show();
            try
            {
                var file = Assert.Single(menu.TopLevelItems, item => Equals(item.Header, "File"));
                file.IsSubMenuOpen = true;
                window.UpdateLayout();
                foreach (var (command, shortcut) in new (LegacyCommandId, string)[]
                {
                    (LegacyCommandId.FileNewSimulation, "F1"),
                    (LegacyCommandId.FileLoadSimulation, "F2"),
                    (LegacyCommandId.FileSaveSimulation, "F3"),
                    (LegacyCommandId.FileSaveWithoutMutations, "Ctrl+F3"),
                    (LegacyCommandId.FilePauseSimulation, "F12"),
                    (LegacyCommandId.FileQuit, "Ctrl+Q"),
                })
                {
                    var item = menu.FindCommandItem(command)!;
                    var label = Assert.Single(item.GetVisualDescendants().OfType<TextBlock>(),
                        text => text.Name == "PART_InputGestureText");
                    Assert.Equal(KeyGesture.Parse(shortcut).ToString("p", null), label.Text);
                }

                file.IsSubMenuOpen = false;
                Assert.True(run.Focus());
                Assert.Single(window.KeyBindings, binding => Equals(binding.Gesture, KeyGesture.Parse("F3")));
                Assert.Single(window.KeyBindings, binding => Equals(binding.Gesture, KeyGesture.Parse("Ctrl+F3")));
                window.KeyPress(Key.F3, RawInputModifiers.None, PhysicalKey.F3, null);
                window.KeyRelease(Key.F3, RawInputModifiers.None, PhysicalKey.F3, null);
                window.KeyPress(Key.F3, RawInputModifiers.Control, PhysicalKey.F3, null);
                window.KeyRelease(Key.F3, RawInputModifiers.Control, PhysicalKey.F3, null);
                Assert.Equal([LegacyCommandId.FileSaveSimulation, LegacyCommandId.FileSaveWithoutMutations], commands);
            }
            finally { window.Close(); }
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
                LegacyCommandId.RobotKill => new LegacyCommandState(false, false, "Select a robot first."),
                LegacyCommandId.HelpSurvivalInfo => new LegacyCommandState(true, false, Visible: false),
                _ => LegacyCommandState.Available,
            });

            Assert.True(vision.IsChecked);
            var kill = menu.FindCommandItem(LegacyCommandId.RobotKill)!;
            Assert.False(kill.IsEnabled);
            Assert.Equal("Select a robot first.", ToolTip.GetTip(kill));
            Assert.False(menu.FindCommandItem(LegacyCommandId.HelpSurvivalInfo)!.IsVisible);
        }, CancellationToken.None);
    }
}
