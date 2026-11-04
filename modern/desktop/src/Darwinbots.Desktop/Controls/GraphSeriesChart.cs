using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Controls;

public sealed class GraphSeriesChart : Control
{
    private IReadOnlyList<GraphSeriesViewModel> _series = [];

    public void SetSeries(GraphSeriesViewModel? series)
    {
        SetSeries(series is null ? [] : new[] { series });
    }

    public void SetSeries(IReadOnlyList<GraphSeriesViewModel> series)
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

        var series = _series.Where(value => value.Samples.Count >= 2).ToArray();
        if (series.Length == 0)
        {
            DrawLabel(context, "OPEN A GRAPH TO COLLECT SAMPLES", area);
            return;
        }
        var values = series.SelectMany(value => value.Samples).Where(value => double.IsFinite(value.Value)).ToArray();
        if (values.Length == 0) return;
        var minimum = Math.Min(0, values.Min(value => value.Value));
        var maximum = values.Max(value => value.Value);
        if (maximum <= minimum) maximum = minimum + 1;
        var firstTick = values.Min(value => value.Tick);
        var lastTick = values.Max(value => value.Tick);
        foreach (var graph in series)
        {
            var pen = new Pen(new SolidColorBrush(Color.FromUInt32(graph.Color)), 2.2);
            Point PointAt(GraphSample sample) => new(
                ((double)sample.Tick - firstTick) * area.Width / Math.Max(1, (double)lastTick - firstTick),
                area.Height - ((sample.Value - minimum) / (maximum - minimum) * Math.Max(1, area.Height - 24)) - 4);
            Point? previous = null;
            foreach (var sample in graph.Samples)
            {
                if (!double.IsFinite(sample.Value)) { previous = null; continue; }
                var current = PointAt(sample);
                if (previous is { } point) context.DrawLine(pen, point, current);
                previous = current;
            }
        }
        DrawLabel(context, string.Join(" · ", series.Select(graph =>
            $"{graph.SpeciesName ?? graph.Title}: {graph.CurrentValueText}")), area, top: true);
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
