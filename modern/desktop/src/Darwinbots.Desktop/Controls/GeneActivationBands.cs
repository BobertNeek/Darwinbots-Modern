using System.Globalization;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Controls;

internal sealed class GeneActivationBands : Control
{
    private static readonly Typeface LabelTypeface = new(new FontFamily("Consolas, DejaVu Sans Mono, Menlo, $Default"));
    private GeneActivationMeterSnapshot? _meter;
    private bool _hasCurrentFocus;

    public void SetMeter(GeneActivationMeterSnapshot? meter, bool hasCurrentFocus)
    {
        _meter = meter;
        _hasCurrentFocus = hasCurrentFocus;
        InvalidateVisual();
    }

    public override void Render(DrawingContext context)
    {
        base.Render(context);
        const double labelHeight = 18;
        var bandsHeight = Math.Max(0, Bounds.Height - labelHeight);
        var weightedHeight = bandsHeight * 570 / 810;
        var currentHeight = bandsHeight - weightedHeight;
        var count = _meter?.Weighted.Count ?? 0;
        context.FillRectangle(Brushes.Black, new Rect(0, 0, Bounds.Width, bandsHeight));
        if (_hasCurrentFocus && _meter is { } meter && count > 0 && meter.Current.Count == count)
        {
            var columnWidth = Bounds.Width / count;
            var border = new Pen(Brushes.Black, Math.Min(1, columnWidth / 4));
            for (var index = 0; index < count; index++)
            {
                var left = index * columnWidth;
                var activeHeight = weightedHeight * Math.Clamp(meter.Weighted[index], 0, 100) / 100;
                context.FillRectangle(Brushes.Blue, new Rect(left, 0, columnWidth, weightedHeight));
                if (activeHeight > 0)
                    context.FillRectangle(Brushes.Cyan, new Rect(left, weightedHeight - activeHeight, columnWidth, activeHeight));
                context.DrawRectangle(null, border, new Rect(left, 0, columnWidth, weightedHeight));
                context.DrawRectangle(meter.Current[index] ? Brushes.Lime : Brushes.Red, border,
                    new Rect(left, weightedHeight, columnWidth, currentHeight));
            }
        }

        var tickPen = new Pen(Brushes.Black, 1);
        foreach (var label in GeneActivationMeterLayout.Labels(count))
        {
            if (label.Tick is { } tick)
            {
                var x = tick * Bounds.Width;
                context.DrawLine(tickPen, new Point(x, bandsHeight), new Point(x, bandsHeight + labelHeight / 2));
            }
            var text = new FormattedText(label.Number.ToString(CultureInfo.InvariantCulture), CultureInfo.InvariantCulture,
                FlowDirection.LeftToRight, LabelTypeface, 10, Brushes.Black);
            context.DrawText(text, new Point(label.Center * Bounds.Width - text.Width / 2, bandsHeight + 2));
        }
    }
}
