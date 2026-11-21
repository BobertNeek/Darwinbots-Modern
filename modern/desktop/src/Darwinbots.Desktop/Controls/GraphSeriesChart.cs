using System.ComponentModel;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;
using Avalonia.VisualTree;
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
        if (!_series.SequenceEqual(series))
        {
            var attached = this.IsAttachedToVisualTree();
            if (attached) UnsubscribeSeries();
            _series = series;
            if (attached) SubscribeSeries();
        }
        InvalidateVisual();
    }

    protected override void OnAttachedToVisualTree(VisualTreeAttachmentEventArgs e)
    {
        SubscribeSeries();
        base.OnAttachedToVisualTree(e);
    }

    protected override void OnDetachedFromVisualTree(VisualTreeAttachmentEventArgs e)
    {
        UnsubscribeSeries();
        base.OnDetachedFromVisualTree(e);
    }

    private void SubscribeSeries()
    {
        foreach (var graph in _series) graph.PropertyChanged += Series_PropertyChanged;
    }

    private void UnsubscribeSeries()
    {
        foreach (var graph in _series) graph.PropertyChanged -= Series_PropertyChanged;
    }

    private void Series_PropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName is nameof(GraphSeriesViewModel.Samples) or nameof(GraphSeriesViewModel.UsesSampleOrder)
            or nameof(GraphSeriesViewModel.Title)) InvalidateVisual();
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

        var series = _series.Where(value => value.Samples.Count > 0).ToArray();
        if (series.Length == 0)
        {
            if (_series.Count == 0)
                DrawLabel(context, "OPEN A GRAPH TO COLLECT SAMPLES", area);
            return;
        }
        var values = series.SelectMany(value => value.Samples).Where(value => double.IsFinite(value.Value)).ToArray();
        if (values.Length == 0) return;
        var minimum = Math.Min(0, values.Min(value => value.Value));
        var maximum = values.Max(value => value.Value);
        if (maximum <= minimum) maximum = minimum + 1;
        var timedTicks = series.Where(graph => !graph.UsesSampleOrder).SelectMany(graph => graph.Samples)
            .Where(sample => double.IsFinite(sample.Value)).Select(sample => sample.Tick).DefaultIfEmpty();
        var firstTick = timedTicks.Min();
        var tickSpan = Math.Max(1, (double)timedTicks.Max() - firstTick);
        var orderedSampleCount = series.Where(graph => graph.UsesSampleOrder).Select(graph => graph.Samples.Count)
            .DefaultIfEmpty(1).Max();
        var sampleSpan = Math.Max(1, orderedSampleCount - 1);
        foreach (var graph in series)
        {
            var brush = new SolidColorBrush(Color.FromUInt32(graph.Color));
            var pen = new Pen(brush, 2.2);
            Point? previous = null;
            for (var index = 0; index < graph.Samples.Count; index++)
            {
                var sample = graph.Samples[index];
                if (!double.IsFinite(sample.Value)) { previous = null; continue; }
                // A native chart feed occupies the next position even when Update Now repeats a cycle.
                var x = graph.UsesSampleOrder
                    ? (orderedSampleCount - graph.Samples.Count + index) * area.Width / sampleSpan
                    : ((double)sample.Tick - firstTick) * area.Width / tickSpan;
                var y = area.Height - ((sample.Value - minimum) / (maximum - minimum) * Math.Max(1, area.Height - 24)) - 4;
                var current = new Point(x, y);
                if (previous is { } point) context.DrawLine(pen, point, current);
                else if (graph.Samples.Count == 1)
                    context.DrawEllipse(brush, null, new Point(Math.Max(2.5, current.X), current.Y), 2.5, 2.5);
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
