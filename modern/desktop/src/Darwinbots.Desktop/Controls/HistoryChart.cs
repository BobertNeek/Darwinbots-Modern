using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Controls;

public enum HistoryMetric { Population, Energy }

public sealed class HistoryChart : Control
{
    private IReadOnlyList<HistorySampleSnapshot> _samples = [];
    public HistoryMetric Metric { get; set; }

    public void SetSnapshot(EngineSnapshot snapshot)
    {
        _samples = snapshot.History.Count <= 240 ? snapshot.History : snapshot.History.Skip(snapshot.History.Count - 240).ToArray();
        InvalidateVisual();
    }

    public override void Render(DrawingContext context)
    {
        base.Render(context);
        var area = new Rect(0, 0, Math.Max(1, Bounds.Width), Math.Max(1, Bounds.Height));
        var gridPen = new Pen(new SolidColorBrush(Color.Parse("#DDDAD1")), 0.6);
        for (var row = 1; row < 4; row++)
        {
            var y = area.Height * row / 4;
            context.DrawLine(gridPen, new Point(0, y), new Point(area.Width, y));
        }
        if (_samples.Count < 2)
        {
            var message = new FormattedText("Collecting every 100 ticks", System.Globalization.CultureInfo.InvariantCulture,
                FlowDirection.LeftToRight, new Typeface("Bahnschrift"), 10, new SolidColorBrush(Color.Parse("#69716D")));
            context.DrawText(message, new Point(4, Math.Max(2, (area.Height - message.Height) / 2)));
            return;
        }
        var values = _samples.Select(sample => Metric == HistoryMetric.Population
            ? (double)sample.Population
            : sample.TotalEnergy).ToArray();
        var minimum = values.Min();
        var maximum = values.Max();
        if (maximum <= minimum) maximum = minimum + 1;
        var pen = new Pen(new SolidColorBrush(Color.Parse(Metric == HistoryMetric.Population ? "#3F8E34" : "#D88724")), 1.8);
        Point PointAt(int index) => new(
            index * area.Width / Math.Max(1, values.Length - 1),
            area.Height - ((values[index] - minimum) / (maximum - minimum) * Math.Max(1, area.Height - 4)) - 2);
        var previous = PointAt(0);
        for (var index = 1; index < values.Length; index++)
        {
            var current = PointAt(index);
            context.DrawLine(pen, previous, current);
            previous = current;
        }
    }
}
