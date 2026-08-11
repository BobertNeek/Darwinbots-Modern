using Avalonia.Automation;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Media;
using Darwinbots.Desktop.Core.LegacyMenus;

namespace Darwinbots.Desktop.Controls;

public partial class LegacyMenuBar : UserControl
{
    private const string DefaultIconData = "M 3,3 L 13,3 13,13 3,13 Z";
    private const string RobotIconData = "M 8,1 A 7,7 0 1 0 8,15 A 7,7 0 1 0 8,1 M 5,7 A 1,1 0 1 0 5,9 M 11,7 A 1,1 0 1 0 11,9";
    private const string SettingsIconData = "M 7,1 L 9,1 10,4 13,3 14,5 12,7 15,8 14,11 11,10 10,13 7,13 6,10 3,11 2,8 5,7 3,5 4,3 7,4 Z";
    private const string ViewIconData = "M 1,8 Q 8,1 15,8 Q 8,15 1,8 M 8,5 A 3,3 0 1 0 8,11 A 3,3 0 1 0 8,5";
    private const string SaveIconData = "M 2,2 L 12,2 14,4 14,14 2,14 Z M 5,2 L 5,6 11,6 11,2 M 5,10 L 11,10 11,14 5,14 Z";
    private readonly Dictionary<LegacyCommandId, MenuItem> _commandItems = [];
    private readonly List<MenuItem> _topLevelItems = [];

    public LegacyMenuBar()
    {
        InitializeComponent();
        foreach (var menu in LegacyMenuCatalog.TopLevel)
        {
            var control = BuildMenu(menu, true);
            _topLevelItems.Add(control);
            RootMenu.Items.Add(control);
        }
    }

    public event EventHandler<LegacyCommandInvokedEventArgs>? CommandInvoked;

    public IReadOnlyList<MenuItem> TopLevelItems => _topLevelItems;

    public MenuItem? FindCommandItem(LegacyCommandId command) =>
        _commandItems.GetValueOrDefault(command);

    public void RefreshStates(Func<LegacyCommandId, LegacyCommandState> stateProvider)
    {
        ArgumentNullException.ThrowIfNull(stateProvider);
        foreach (var (command, item) in _commandItems)
        {
            var state = stateProvider(command);
            item.IsEnabled = state.Enabled;
            item.IsChecked = state.Checked;
            item.IsVisible = state.Visible;
            ToolTip.SetTip(item, state.DisabledReason);
        }
    }

    private MenuItem BuildMenu(LegacyMenuItem model, bool topLevel = false)
    {
        var item = new MenuItem
        {
            Header = model.Label,
            Name = topLevel ? $"Menu{Sanitize(model.Label)}" : null,
            IsVisible = model.VisibleByDefault,
        };
        AutomationProperties.SetAutomationId(item, item.Name ?? $"MenuGroup{Sanitize(model.Label)}");

        if (model.Children.Count > 0)
        {
            foreach (var child in model.Children)
            {
                item.Items.Add(child.IsSeparator ? new Separator() : BuildMenu(child));
            }
        }

        if (model.Command is { } command)
        {
            item.Name = $"Command{command}";
            AutomationProperties.SetAutomationId(item, item.Name);
            item.Icon = BuildIcon(model.IconKey ?? string.Empty);
            if (model.Shortcut is not null) item.HotKey = KeyGesture.Parse(model.Shortcut);
            item.ToggleType = model.CheckKind switch
            {
                LegacyMenuCheckKind.Check => MenuItemToggleType.CheckBox,
                LegacyMenuCheckKind.Radio => MenuItemToggleType.Radio,
                _ => MenuItemToggleType.None,
            };
            item.Click += (_, _) => CommandInvoked?.Invoke(this, new LegacyCommandInvokedEventArgs(command));
            _commandItems.Add(command, item);
        }

        return item;
    }

    private static PathIcon BuildIcon(string iconKey)
    {
        var data = iconKey switch
        {
            "save" or "save-clean" or "dna-save" or "organism-save" => SaveIconData,
            "eye" or "vision" or "info" or "gauges" => ViewIconData,
            "settings" or "physics" or "mutation" or "rgb-settings" => SettingsIconData,
            "insert-bot" or "species" or "player" or "player-toggle" or "speciate" => RobotIconData,
            _ => DefaultIconData,
        };
        return new PathIcon
        {
            Data = StreamGeometry.Parse(data),
            Width = 15,
            Height = 15,
            Foreground = new SolidColorBrush(Color.Parse("#52605A")),
        };
    }

    private static string Sanitize(string value) =>
        new(value.Where(char.IsLetterOrDigit).ToArray());
}
