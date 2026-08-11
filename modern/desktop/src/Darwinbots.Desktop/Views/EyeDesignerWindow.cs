using Avalonia;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Media;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed class EyeDesignerWindow : Window, IPersistentSnapshotToolWindow
{
    private readonly SimulationSession _session;
    private readonly OrganismKey _focus;
    private readonly TextBox[] _directions = new TextBox[EyeDesignerModel.EyeCount];
    private readonly TextBox[] _widths = new TextBox[EyeDesignerModel.EyeCount];
    private readonly TextBlock _status = new();
    private readonly EyePreviewControl _preview = new();
    private EyeDesignerModel _model;
    private bool _syncing;
    private bool _dirty;

    public EyeDesignerWindow(SimulationSession session, OrganismKey focus)
    {
        _session = session ?? throw new ArgumentNullException(nameof(session));
        _focus = focus;
        var organism = Find(session.LatestSnapshot);
        _model = EyeDesignerModel.FromOrganism(organism ?? throw new InvalidOperationException("The selected robot is no longer available."));
        Title = $"Darwinbots — Eye Designer {focus.Slot}:{focus.Generation}";
        Width = 760;
        Height = 680;
        MinWidth = 680;
        MinHeight = 560;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;
        Background = Brush.Parse("#F4F0E6");

        _status.Foreground = Brush.Parse("#537B43");
        _status.FontFamily = new FontFamily("Bahnschrift");
        _status.FontSize = 11;
        _status.Text = "EDIT THE NINE AUTHORITATIVE EYE ROWS";

        var rows = new Grid
        {
            RowDefinitions = new RowDefinitions("Auto,Auto,Auto,Auto,Auto,Auto,Auto,Auto,Auto,Auto"),
            ColumnDefinitions = new ColumnDefinitions("70,*,*,120"),
            RowSpacing = 6,
            ColumnSpacing = 8,
            Margin = new Thickness(18),
        };
        rows.Children.Add(Label("EYE", true));
        Grid.SetColumn(rows.Children[^1], 0);
        rows.Children.Add(Label("DIRECTION", true));
        Grid.SetColumn(rows.Children[^1], 1);
        rows.Children.Add(Label("WIDTH", true));
        Grid.SetColumn(rows.Children[^1], 2);
        rows.Children.Add(Label("RANGE", true));
        Grid.SetColumn(rows.Children[^1], 3);
        for (var index = 0; index < EyeDesignerModel.EyeCount; index++)
        {
            var row = index + 1;
            var number = index;
            var eye = new TextBlock { Text = $"eye{row}", VerticalAlignment = VerticalAlignment.Center, FontWeight = FontWeight.Bold };
            Grid.SetRow(eye, row);
            rows.Children.Add(eye);
            _directions[index] = new TextBox { MinWidth = 120 };
            _directions[index].TextChanged += (_, _) => DraftChanged(number);
            Grid.SetRow(_directions[index], row);
            Grid.SetColumn(_directions[index], 1);
            rows.Children.Add(_directions[index]);
            _widths[index] = new TextBox { MinWidth = 120 };
            _widths[index].TextChanged += (_, _) => DraftChanged(number);
            Grid.SetRow(_widths[index], row);
            Grid.SetColumn(_widths[index], 2);
            rows.Children.Add(_widths[index]);
            var range = new TextBlock { VerticalAlignment = VerticalAlignment.Center, Foreground = Brush.Parse("#69716D") };
            range.Name = $"Range{index}";
            Grid.SetRow(range, row);
            Grid.SetColumn(range, 3);
            rows.Children.Add(range);
        }

        var editor = new Border
        {
            Background = Brush.Parse("#FFFCF5"),
            BorderBrush = Brush.Parse("#C8C3B6"),
            BorderThickness = new Thickness(1),
            Child = new ScrollViewer { Content = rows },
        };
        var body = new Grid { ColumnDefinitions = new ColumnDefinitions("*,280"), ColumnSpacing = 12, Margin = new Thickness(16) };
        body.Children.Add(editor);
        var previewPanel = new StackPanel { Spacing = 8 };
        previewPanel.Children.Add(new TextBlock { Text = "PREVIEW", FontWeight = FontWeight.Bold, FontFamily = new FontFamily("Bahnschrift"), LetterSpacing = 1.2 });
        _preview.Height = 260;
        previewPanel.Children.Add(_preview);
        previewPanel.Children.Add(new TextBlock { Text = "Direction and width use DB2 memory units. Width zero keeps the standard eye width.", TextWrapping = TextWrapping.Wrap, Foreground = Brush.Parse("#69716D"), FontSize = 11 });
        Grid.SetColumn(previewPanel, 1);
        body.Children.Add(previewPanel);

        var write = Button("WRITE DNA", async (_, _) => await WriteDnaAsync());
        var reset = Button("RESET DRAFT", (_, _) => ResetDraft());
        var close = Button("CLOSE", (_, _) => Close());
        var actions = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right, Spacing = 8, Children = { write, reset, close } };
        var root = new Grid { RowDefinitions = new RowDefinitions("58,*,52") };
        root.Children.Add(new Border { Background = Brush.Parse("#FBF9F2"), BorderBrush = Brush.Parse("#C8C3B6"), BorderThickness = new Thickness(0, 0, 0, 1), Padding = new Thickness(20, 11), Child = new StackPanel { Spacing = 2, Children = { new TextBlock { Text = "EYE DESIGNER", FontFamily = new FontFamily("Bahnschrift"), FontSize = 21, FontWeight = FontWeight.Bold, LetterSpacing = 1.3 }, _status } } });
        Grid.SetRow(body, 1);
        root.Children.Add(body);
        Grid.SetRow(actions, 2);
        var footer = new Border { Background = Brush.Parse("#E5E0D4"), BorderBrush = Brush.Parse("#C8C3B6"), BorderThickness = new Thickness(0, 1, 0, 0), Padding = new Thickness(12, 8), Child = actions };
        Grid.SetRow(footer, 2);
        root.Children.Add(footer);
        Content = root;
        SetModel(_model);
    }

    public string LastWrittenDna { get; private set; } = string.Empty;

    public void SetSnapshot(EngineSnapshot snapshot)
    {
        var organism = Find(snapshot);
        if (organism is null)
        {
            _status.Text = $"FOCUS LOST · { _focus.Slot}:{_focus.Generation } IS NOT BOUND TO A REPLACEMENT";
            return;
        }
        if (!_dirty) SetModel(EyeDesignerModel.FromOrganism(organism));
    }

    private OrganismSnapshot? Find(EngineSnapshot snapshot) => snapshot.Organisms.FirstOrDefault(value =>
        value.Slot == _focus.Slot && value.Generation == _focus.Generation);

    private void SetModel(EyeDesignerModel model)
    {
        _model = model;
        _syncing = true;
        try
        {
            for (var index = 0; index < model.Rows.Count; index++)
            {
                _directions[index].Text = model.Rows[index].Direction.ToString(System.Globalization.CultureInfo.InvariantCulture);
                _widths[index].Text = model.Rows[index].Width.ToString(System.Globalization.CultureInfo.InvariantCulture);
            }
        }
        finally { _syncing = false; }
        _dirty = false;
        _preview.SetVision(model.Preview, model.Aim);
        _status.Text = $"LIVE ROBOT {_focus.Slot}:{_focus.Generation} · NINE EYES READY";
    }

    private void DraftChanged(int index)
    {
        if (_syncing) return;
        if (_model.TrySet(index, _directions[index].Text ?? string.Empty, _widths[index].Text ?? string.Empty, out var error))
        {
            _dirty = true;
            _preview.SetVision(_model.Preview, _model.Aim);
            _status.Text = "DRAFT VALID · PREVIEW UPDATED";
        }
        else _status.Text = error;
    }

    private void ResetDraft()
    {
        if (Find(_session.LatestSnapshot) is not { } organism)
        {
            _status.Text = $"FOCUS LOST · {_focus.Slot}:{_focus.Generation} IS NOT BOUND TO A REPLACEMENT";
            return;
        }
        SetModel(EyeDesignerModel.FromOrganism(organism));
    }

    public async Task WriteDnaAsync()
    {
        if (Find(_session.LatestSnapshot) is null)
        {
            _status.Text = $"FOCUS LOST · {_focus.Slot}:{_focus.Generation} IS NOT BOUND TO A REPLACEMENT";
            return;
        }
        for (var index = 0; index < EyeDesignerModel.EyeCount; index++)
        {
            if (!_model.TrySet(index, _directions[index].Text ?? string.Empty, _widths[index].Text ?? string.Empty, out var error))
            {
                _status.Text = error;
                return;
            }
        }
        LastWrittenDna = _model.WriteDna();
        try
        {
            await _session.ReplaceDnaAsync(_focus.Slot, _focus.Generation, LastWrittenDna);
            _dirty = false;
            _status.Text = "WRITE DNA · SELECTED RUST ROBOT UPDATED";
        }
        catch (Exception error) { _status.Text = $"WRITE DNA FAILED · {error.Message}"; }
    }

    private static TextBlock Label(string text, bool bold = false) => new() { Text = text, FontWeight = bold ? FontWeight.Bold : FontWeight.Normal, Foreground = Brush.Parse("#69716D"), FontSize = 11 };

    private static Button Button(string text, EventHandler<RoutedEventArgs> handler)
    {
        var button = new Button { Content = text, Background = Brush.Parse("#416E35"), Foreground = Brushes.White, FontWeight = FontWeight.Bold, Padding = new Thickness(12, 7) };
        button.Click += handler;
        return button;
    }
}

public sealed class EyePreviewControl : Control
{
    private VisionSnapshot _vision = VisionSnapshot.Default;
    private int _aim;

    public void SetVision(VisionSnapshot vision, int aim)
    {
        _vision = vision;
        _aim = aim;
        InvalidateVisual();
    }

    public override void Render(DrawingContext context)
    {
        base.Render(context);
        context.FillRectangle(Brushes.Transparent, Bounds);
        var center = new Point(Bounds.Width / 2, Bounds.Height / 2);
        var radius = Math.Max(20, Math.Min(Bounds.Width, Bounds.Height) * 0.42);
        context.DrawEllipse(null, new Pen(Brush.Parse("#34473B"), 2), center, 12, 12);
        var sectors = OrganismVisualGeometry.EyeSectors(_vision, _aim, 1f);
        foreach (var sector in sectors)
        {
            var half = sector.SweepRadians / 2;
            var geometry = new StreamGeometry();
            using (var path = geometry.Open())
            {
                path.BeginFigure(center, true);
                for (var step = 0; step <= 8; step++)
                {
                    var angle = sector.StartRadians + sector.SweepRadians * step / 8f;
                    path.LineTo(new Point(center.X + Math.Sin(angle) * radius, center.Y - Math.Cos(angle) * radius));
                }
                path.EndFigure(true);
            }
            var fill = sector.Focused ? "#55E26722" : "#30239AC0";
            context.DrawGeometry(Brush.Parse(fill), new Pen(Brush.Parse(sector.Focused ? "#D8A749" : "#6E9BC0"), 0.8), geometry);
            _ = half;
        }
    }
}
