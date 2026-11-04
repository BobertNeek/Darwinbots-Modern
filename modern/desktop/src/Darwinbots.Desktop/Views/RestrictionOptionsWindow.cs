using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Platform.Storage;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed class RestrictionOptionsWindow : Window
{
    private readonly CheckBox[] _nonRepopulating;
    private readonly CheckBox[] _repopulating;
    private readonly TextBlock _status = new() { Foreground = Brush.Parse("#9A4A31"), FontSize = 11 };

    private static readonly string[] Labels =
    [
        "kill multibot",
        "kill disqualified",
        "disable chloroplasts",
        "fixed in place",
        "disable vision",
        "disable DNA",
        "disable reproduction",
        "virus immune",
        "disable mutations",
        "disable movement sysvars",
    ];

    public bool Accepted { get; private set; }

    public RestrictionOptionsWindow(RestrictionOptions options)
    {
        Title = "Darwinbots — Restriction Options";
        Width = 760;
        Height = 560;
        MinWidth = 640;
        MinHeight = 460;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;
        Background = Brush.Parse("#F4F0E6");
        _nonRepopulating = CreateChecks(options.NonRepopulating);
        _repopulating = CreateChecks(options.Repopulating);

        var columns = new Grid { ColumnDefinitions = new ColumnDefinitions("*,*") };
        var left = Column("NON-REPOPULATING", _nonRepopulating);
        var right = Column("REPOPULATING", _repopulating);
        columns.Children.Add(left);
        Grid.SetColumn(right, 1);
        columns.Children.Add(right);
        var body = new StackPanel
        {
            Margin = new Thickness(18),
            Spacing = 10,
            Children =
            {
                new TextBlock { Text = "EDITABLE PER-COLUMN RESTRICTIONS · APPLY WRITES LIVE ROBOT FLAGS", FontWeight = FontWeight.Bold, Foreground = Brush.Parse("#537B43") },
                columns,
                _status,
            },
        };
        var load = new Button { Content = "LOAD" };
        load.Click += async (_, _) => await LoadAsync();
        var save = new Button { Content = "SAVE" };
        save.Click += async (_, _) => await SaveAsync();
        var cancel = new Button { Content = "CANCEL" };
        cancel.Click += (_, _) => Close();
        var apply = new Button { Content = "APPLY", Background = Brush.Parse("#416E35"), Foreground = Brushes.White, FontWeight = FontWeight.Bold };
        apply.Click += (_, _) => { Accepted = true; Close(); };
        var footer = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right, Spacing = 8, Children = { load, save, cancel, apply } };
        var root = new Grid { RowDefinitions = new RowDefinitions("*,58") };
        root.Children.Add(body);
        var footerBorder = new Border { Background = Brush.Parse("#E5E0D4"), BorderBrush = Brush.Parse("#C8C3B6"), BorderThickness = new Thickness(0, 1, 0, 0), Padding = new Thickness(12, 9), Child = footer };
        Grid.SetRow(footerBorder, 1);
        root.Children.Add(footerBorder);
        Content = root;
    }

    public RestrictionOptions Options => new(Column(_nonRepopulating), Column(_repopulating));

    private static Border Column(string title, CheckBox[] checks)
    {
        var stack = new StackPanel { Spacing = 6 };
        stack.Children.Add(new TextBlock { Text = title, FontWeight = FontWeight.Bold });
        foreach (var check in checks) stack.Children.Add(check);
        return new Border
        {
            Background = Brush.Parse("#FFFCF5"),
            BorderBrush = Brush.Parse("#D4CFC1"),
            BorderThickness = new Thickness(1),
            Padding = new Thickness(12),
            Margin = new Thickness(0, 0, 8, 0),
            Child = stack,
        };
    }

    private static CheckBox[] CreateChecks(RestrictionColumn column)
    {
        var values = Values(column);
        return Labels.Select((label, index) => new CheckBox { Content = label, IsChecked = values[index] }).ToArray();
    }

    private static bool[] Values(RestrictionColumn column) =>
    [column.KillMultibot, column.KillDisqualified, column.DisableChloroplasts, column.FixedInPlace, column.DisableVision, column.DisableDna, column.DisableReproduction, column.VirusImmune, column.DisableMutations, column.DisableMovement];

    private static RestrictionColumn Column(CheckBox[] checks) => new(
        checks[0].IsChecked == true, checks[1].IsChecked == true, checks[2].IsChecked == true, checks[3].IsChecked == true, checks[4].IsChecked == true,
        checks[5].IsChecked == true, checks[6].IsChecked == true, checks[7].IsChecked == true, checks[8].IsChecked == true, checks[9].IsChecked == true);

    private async Task LoadAsync()
    {
        try
        {
            var files = await StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions { Title = "Load restriction preset", AllowMultiple = false, FileTypeFilter = [new FilePickerFileType("Restriction preset") { Patterns = ["*.resp"] }] });
            var path = files.FirstOrDefault()?.Path.LocalPath;
            if (path is null) return;
            var options = RestrictionPresetFile.Load(path);
            ApplyChecks(_nonRepopulating, options.NonRepopulating);
            ApplyChecks(_repopulating, options.Repopulating);
            _status.Text = "PRESET LOADED";
        }
        catch (Exception error) { _status.Text = error.Message; }
    }

    private async Task SaveAsync()
    {
        try
        {
            var file = await StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions { Title = "Save restriction preset", SuggestedFileName = "restrictions.resp", DefaultExtension = "resp", FileTypeChoices = [new FilePickerFileType("Restriction preset") { Patterns = ["*.resp"] }] });
            var path = file?.Path.LocalPath;
            if (path is null) return;
            RestrictionPresetFile.Save(path, Options);
            _status.Text = "PRESET SAVED";
        }
        catch (Exception error) { _status.Text = error.Message; }
    }

    private static void ApplyChecks(CheckBox[] checks, RestrictionColumn options)
    {
        var values = Values(options);
        for (var index = 0; index < checks.Length; index++) checks[index].IsChecked = values[index];
    }
}
