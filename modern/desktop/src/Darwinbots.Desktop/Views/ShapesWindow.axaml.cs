using Avalonia.Controls;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;

namespace Darwinbots.Desktop.Views;

public sealed partial class ShapesWindow : Window
{
    private readonly Action<ShapeCreationDimensions> _dimensionsChanged;
    private readonly Action<MazeDimensions> _mazeDimensionsChanged;
    private readonly Func<Task<LegacyCommandResult>> _addShape;
    private readonly Func<bool?, bool?, short?, Task<LegacyCommandResult>> _changeDrift;
    private bool _updatingDimensions;
    private bool _updatingDrift;

    public ShapesWindow(
        ShapeCreationDimensions dimensions,
        Action<ShapeCreationDimensions> dimensionsChanged,
        Func<Task<LegacyCommandResult>> addShape,
        Db2ShapeDriftOptions? drift,
        Func<bool?, bool?, short?, Task<LegacyCommandResult>> changeDrift,
        MazeDimensions mazeDimensions,
        Action<MazeDimensions> mazeDimensionsChanged)
    {
        _dimensionsChanged = dimensionsChanged;
        _mazeDimensionsChanged = mazeDimensionsChanged;
        _addShape = addShape;
        _changeDrift = changeDrift;
        InitializeComponent();
        SetDimensions(dimensions);
        SetDrift(drift);
        MazeCorridorSlider.Value = mazeDimensions.CorridorWidth;
        MazeWallThicknessSlider.Value = mazeDimensions.WallThickness;
        ShowMazeDimensions(mazeDimensions);
        MazeCorridorSlider.ValueChanged += (_, _) => UpdateMazeDimensions();
        MazeWallThicknessSlider.ValueChanged += (_, _) => UpdateMazeDimensions();
        ShapeWidthSlider.ValueChanged += (_, _) => UpdateDimensions();
        ShapeHeightSlider.ValueChanged += (_, _) => UpdateDimensions();
        HorizontalDriftCheck.IsCheckedChanged += async (_, _) =>
            await ChangeDriftAsync(horizontal: HorizontalDriftCheck.IsChecked == true);
        VerticalDriftCheck.IsCheckedChanged += async (_, _) =>
            await ChangeDriftAsync(vertical: VerticalDriftCheck.IsChecked == true);
        DriftRateSlider.ValueChanged += async (_, _) =>
            await ChangeDriftAsync(rate: (short)Math.Clamp(DriftRateSlider.Value, 1, 1000));
    }

    public void SetDimensions(ShapeCreationDimensions dimensions)
    {
        _updatingDimensions = true;
        ShapeWidthSlider.Value = dimensions.WidthPermille;
        ShapeHeightSlider.Value = dimensions.HeightPermille;
        ShowDimensions(dimensions);
        ShapeStatus.Text = string.Empty;
        _updatingDimensions = false;
    }

    private void UpdateDimensions()
    {
        if (_updatingDimensions) return;
        var dimensions = new ShapeCreationDimensions((int)ShapeWidthSlider.Value, (int)ShapeHeightSlider.Value);
        ShowDimensions(dimensions);
        _dimensionsChanged(dimensions);
    }

    private void ShowDimensions(ShapeCreationDimensions dimensions)
    {
        ShapeWidthValue.Text = $"{dimensions.WidthPermille / 10m:0.0}%";
        ShapeHeightValue.Text = $"{dimensions.HeightPermille / 10m:0.0}%";
    }

    private void UpdateMazeDimensions()
    {
        var dimensions = new MazeDimensions((int)MazeCorridorSlider.Value, (int)MazeWallThicknessSlider.Value);
        ShowMazeDimensions(dimensions);
        _mazeDimensionsChanged(dimensions);
    }

    private void ShowMazeDimensions(MazeDimensions dimensions)
    {
        MazeCorridorValue.Text = dimensions.CorridorWidth.ToString();
        MazeWallThicknessValue.Text = dimensions.WallThickness.ToString();
    }

    public void SetDrift(Db2ShapeDriftOptions? drift)
    {
        drift ??= new();
        _updatingDrift = true;
        HorizontalDriftCheck.IsChecked = drift.Horizontal;
        VerticalDriftCheck.IsChecked = drift.Vertical;
        ShowDriftRate(drift.Rate);
        _updatingDrift = false;
    }

    private void ShowDriftRate(short rate)
    {
        // Keep a saved value outside the slider's editing range visible until a real edit.
        DriftRateSlider.Minimum = Math.Min(1, (int)rate);
        DriftRateSlider.Maximum = Math.Max(1000, (int)rate);
        DriftRateSlider.Value = rate;
        DriftRateValue.Text = rate.ToString();
    }

    private async Task ChangeDriftAsync(bool? horizontal = null, bool? vertical = null, short? rate = null)
    {
        if (_updatingDrift) return;
        if (rate is { } value)
        {
            _updatingDrift = true;
            ShowDriftRate(value);
            _updatingDrift = false;
        }
        try
        {
            ShapeStatus.Text = (await _changeDrift(horizontal, vertical, rate)).Status;
        }
        catch (Exception error)
        {
            ShapeStatus.Text = $"SHAPE MOVEMENT UPDATE FAILED · {error.Message}";
        }
    }

    private async void AddShape_Click(object? sender, RoutedEventArgs e)
    {
        AddShapeButton.IsEnabled = false;
        try
        {
            ShapeStatus.Text = (await _addShape()).Status;
        }
        catch (Exception error)
        {
            ShapeStatus.Text = $"SHAPE CREATION FAILED · {error.Message}";
        }
        finally
        {
            AddShapeButton.IsEnabled = true;
        }
    }

    private void Close_Click(object? sender, RoutedEventArgs e) => Close();
}
