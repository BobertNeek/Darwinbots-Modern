using Avalonia.Controls;
using Avalonia.Interactivity;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Core.LegacyMenus;

namespace Darwinbots.Desktop.Views;

public sealed partial class ShapesWindow : Window
{
    private readonly Action<ShapeCreationDimensions> _dimensionsChanged;
    private readonly Func<Task<LegacyCommandResult>> _addShape;
    private bool _updatingDimensions;

    public ShapesWindow(
        ShapeCreationDimensions dimensions,
        Action<ShapeCreationDimensions> dimensionsChanged,
        Func<Task<LegacyCommandResult>> addShape)
    {
        _dimensionsChanged = dimensionsChanged;
        _addShape = addShape;
        InitializeComponent();
        SetDimensions(dimensions);
        ShapeWidthSlider.ValueChanged += (_, _) => UpdateDimensions();
        ShapeHeightSlider.ValueChanged += (_, _) => UpdateDimensions();
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
