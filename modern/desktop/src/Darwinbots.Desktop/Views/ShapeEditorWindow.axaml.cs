using System.Globalization;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Views;

public sealed partial class ShapeEditorWindow : Window
{
    private readonly ObstacleSnapshot _original;

    public ShapeEditorWindow() : this(new ObstacleSnapshot(0, [0f, 0f], [1f, 1f]))
    {
    }

    public ShapeEditorWindow(ObstacleSnapshot obstacle)
    {
        _original = obstacle ?? throw new ArgumentNullException(nameof(obstacle));
        InitializeComponent();
        ColorText.Text = $"#{obstacle.Color:X8}";
        OpacityValue.Value = (decimal)obstacle.Opacity;
        DriftX.Value = (decimal)obstacle.Drift.ElementAtOrDefault(0);
        DriftY.Value = (decimal)obstacle.Drift.ElementAtOrDefault(1);
        VisionPolicy.SelectedIndex = obstacle.VisionPolicy.Equals("see_through", StringComparison.OrdinalIgnoreCase) ? 1 : 0;
        ShotPolicy.SelectedIndex = obstacle.ShotPolicy.Equals("pass_through", StringComparison.OrdinalIgnoreCase) ? 1 : 0;
    }

    public ObstacleSnapshot? Result { get; private set; }

    private void Apply_Click(object? sender, RoutedEventArgs e)
    {
        var rawColor = (ColorText.Text ?? string.Empty).Trim().TrimStart('#');
        if (!uint.TryParse(rawColor, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out var color))
            return;
        Result = _original with
        {
            Color = color,
            Opacity = (float)(OpacityValue.Value ?? 1m),
            Drift = [(float)(DriftX.Value ?? 0m), (float)(DriftY.Value ?? 0m)],
            VisionPolicy = VisionPolicy.SelectedIndex == 1 ? "see_through" : "opaque",
            ShotPolicy = ShotPolicy.SelectedIndex == 1 ? "pass_through" : "absorb",
        };
        Close();
    }

    private void Cancel_Click(object? sender, RoutedEventArgs e) => Close();
}
