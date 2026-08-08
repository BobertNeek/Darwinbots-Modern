using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Controls;

public sealed class GraphSeriesChart : Control
{
    private GraphSeriesViewModel? _series;

    public void SetSeries(GraphSeriesViewModel? series)
    {
        _series = series;
        InvalidateVisual();
    }

    public override void Render(DrawingContext context)
    {
        base.Render(context);
        var area = new Rect(0, 0, Math.Max(1, Bounds.Width), Math.Max(1, Bounds.Height));
        var gridPen = new Pen(new SolidColorBrush(Color.Parse("#DAD7CE")), 0.7);
        for (var row = 1; row < 4; row++)
        {
            var y = area.Height * row / 4;
            context.DrawLine(gridPen, new Point(0, y), new Point(area.Width, y));
        }

        if (_series is null || _series.Samples.Count < 2)
        {
            DrawLabel(context, "OPEN A GRAPH TO COLLECT SAMPLES", area);
            return;
        }

        var values = _series.Samples.Select(value => value.Value).ToArray();
        var minimum = values.Min();
        var maximum = values.Max();
        if (maximum <= minimum) maximum = minimum + 1;
        var pen = new Pen(new SolidColorBrush(Color.Parse("#2D826F")), 2.2);
        Point PointAt(int index) => new(
            index * area.Width / Math.Max(1, values.Length - 1),
            area.Height - ((values[index] - minimum) / (maximum - minimum) * Math.Max(1, area.Height - 8)) - 4);
        var previous = PointAt(0);
        for (var index = 1; index < values.Length; index++)
        {
            var current = PointAt(index);
            context.DrawLine(pen, previous, current);
            previous = current;
        }

        DrawLabel(context, $"{_series.Title} · {_series.CurrentValueText}", area, top: true);
    }

    private static void DrawLabel(DrawingContext context, string text, Rect area, bool top = false)
    {
        var label = new FormattedText(
            text,
            System.Globalization.CultureInfo.InvariantCulture,
            FlowDirection.LeftToRight,
            new Typeface("Bahnschrift"),
            10,
            new SolidColorBrush(Color.Parse("#69716D")));
        context.DrawText(label, new Point(6, top ? 4 : Math.Max(4, (area.Height - label.Height) / 2)));
    }
}
