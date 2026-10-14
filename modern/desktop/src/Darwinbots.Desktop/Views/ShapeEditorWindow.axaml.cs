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

    public ShapeEditorWindow(ObstacleSnapshot obstacle, bool shapesVisible = false)
    {
        _original = obstacle ?? throw new ArgumentNullException(nameof(obstacle));
        InitializeComponent();
        ShapesVisibleToggle.IsChecked = shapesVisible;
        MinimumX.Value = (decimal)obstacle.Minimum.ElementAtOrDefault(0);
        MinimumY.Value = (decimal)obstacle.Minimum.ElementAtOrDefault(1);
        MaximumX.Value = (decimal)obstacle.Maximum.ElementAtOrDefault(0);
        MaximumY.Value = (decimal)obstacle.Maximum.ElementAtOrDefault(1);
        ColorText.Text = $"#{obstacle.Color:X8}";
        OpacityValue.Value = (decimal)obstacle.Opacity;
        DriftX.Value = (decimal)obstacle.Drift.ElementAtOrDefault(0);
        DriftY.Value = (decimal)obstacle.Drift.ElementAtOrDefault(1);
        VisionPolicy.SelectedIndex = obstacle.VisionPolicy.Equals("see_through", StringComparison.OrdinalIgnoreCase) ? 1 : 0;
        ShotPolicy.SelectedIndex = obstacle.ShotPolicy.Equals("pass_through", StringComparison.OrdinalIgnoreCase) ? 1 : 0;
    }

    public ObstacleSnapshot? Result { get; private set; }
    public bool ShapesVisible => ShapesVisibleToggle.IsChecked == true;

    private void Apply_Click(object? sender, RoutedEventArgs e)
    {
        var rawColor = (ColorText.Text ?? string.Empty).Trim().TrimStart('#');
        if (!uint.TryParse(rawColor, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out var color))
            return;
        Result = ObjectEditorPolicy.BuildObstaclePayload(
            _original,
            [(float)(MinimumX.Value ?? 0m), (float)(MinimumY.Value ?? 0m)],
            [(float)(MaximumX.Value ?? 0m), (float)(MaximumY.Value ?? 0m)],
            color,
            (float)(OpacityValue.Value ?? 1m),
            [(float)(DriftX.Value ?? 0m), (float)(DriftY.Value ?? 0m)],
            VisionPolicy.SelectedIndex == 1 ? "see_through" : "opaque",
            ShotPolicy.SelectedIndex == 1 ? "pass_through" : "absorb");
        Close();
    }

    private void Cancel_Click(object? sender, RoutedEventArgs e) => Close();
}
