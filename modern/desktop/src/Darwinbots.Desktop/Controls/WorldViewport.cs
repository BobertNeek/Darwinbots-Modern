using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Media;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Controls;

public sealed class WorldViewport : Control
{
    private EngineSnapshot _snapshot = EngineSnapshot.Empty;
    private readonly Pen _gridPen = new(new SolidColorBrush(Color.Parse("#D9D8D0")), 0.6);
    private double _zoom = 1.0;
    private Vector _pan;
    private Point? _panAnchor;
    private Vector _panAtPress;
    private uint? _selectedSlot;
    private WorldFeatureSelection? _selectedFeature;
    private uint? _followSlot;

    public event Action<uint>? OrganismSelected;
    public event Action<float[]>? WorldClicked;
    public event Action<WorldFeatureSelection>? WorldFeatureSelected;

    public void SelectSlot(uint? slot)
    {
        _selectedSlot = slot;
        InvalidateVisual();
    }

    public void SelectFeature(WorldFeatureSelection? feature)
    {
        _selectedFeature = feature;
        InvalidateVisual();
    }

    public void SetSnapshot(EngineSnapshot snapshot)
    {
        _snapshot = snapshot;
        if (_followSlot is { } followed && snapshot.Organisms.FirstOrDefault(value => value.Slot == followed) is { } organism)
        {
            var scaleX = Bounds.Width / Math.Max(1.0, snapshot.WorldSize[0]);
            var scaleY = Bounds.Height / Math.Max(1.0, snapshot.WorldSize[1]);
            _pan = new Vector(Bounds.Width / 2 - organism.Position[0] * scaleX * _zoom,
                Bounds.Height / 2 - organism.Position[1] * scaleY * _zoom);
        }
        InvalidateVisual();
    }

    public void FollowSlot(uint? slot)
    {
        _followSlot = slot;
        InvalidateVisual();
    }

    public override void Render(DrawingContext context)
    {
        base.Render(context);
        context.FillRectangle(new SolidColorBrush(Color.Parse("#F9F8F2")), Bounds);
        DrawGrid(context);
        DrawWorldFeatures(context);
        if (_snapshot.Organisms.Count == 0)
        {
            var message = new FormattedText(
                "IMPORT A LEGACY BOT TO BEGIN",
                System.Globalization.CultureInfo.InvariantCulture,
                FlowDirection.LeftToRight,
                new Typeface("Bahnschrift"),
                16,
                new SolidColorBrush(Color.Parse("#69716D")));
            context.DrawText(message, new Point((Bounds.Width - message.Width) / 2, (Bounds.Height - message.Height) / 2));
            return;
        }

        var scaleX = Bounds.Width / Math.Max(1.0, _snapshot.WorldSize[0]);
        var scaleY = Bounds.Height / Math.Max(1.0, _snapshot.WorldSize[1]);
        if (_snapshot.RenderInstances.Count > 0)
        {
            var radiusScale = Math.Min(scaleX, scaleY);
            var stride = _snapshot.RenderInstances.Count > 100_000 && _zoom < 1 ? 8
                : _snapshot.RenderInstances.Count > 50_000 && _zoom < 2 ? 4
                : _snapshot.RenderInstances.Count > 20_000 && _zoom < 1 ? 2 : 1;
            for (var index = 0; index < _snapshot.RenderInstances.Count; index += stride)
            {
                var instance = _snapshot.RenderInstances[index];
                var x = instance.Position[0] * scaleX * _zoom + _pan.X;
                var y = instance.Position[1] * scaleY * _zoom + _pan.Y;
                var radius = Math.Clamp(instance.Radius * radiusScale * _zoom, 1.5, 10.0);
                var color = Color.FromUInt32(instance.Color);
                var outline = _selectedSlot == instance.Slot ? new Pen(Brushes.Black, 2.2) : new Pen(Brushes.White, 0.8);
                context.DrawEllipse(new SolidColorBrush(color), outline, new Point(x, y), radius, radius);
            }
            return;
        }

        foreach (var organism in _snapshot.Organisms)
        {
            var x = organism.Position[0] * scaleX * _zoom + _pan.X;
            var y = organism.Position[1] * scaleY * _zoom + _pan.Y;
            var hue = (organism.Slot * 47) % 360;
            var color = Hsv(hue, 0.68, 0.72);
            var outline = _selectedSlot == organism.Slot ? new Pen(Brushes.Black, 2.2) : new Pen(Brushes.White, 0.8);
            context.DrawEllipse(new SolidColorBrush(color), outline, new Point(x, y), 3.2, 3.2);
            var velocity = organism.Velocity;
            context.DrawLine(new Pen(new SolidColorBrush(color), 1), new Point(x, y), new Point(x + velocity[0] * 0.35, y + velocity[1] * 0.35));
        }
    }

    protected override void OnPointerPressed(PointerPressedEventArgs e)
    {
        base.OnPointerPressed(e);
        var point = e.GetCurrentPoint(this);
        if (point.Properties.IsMiddleButtonPressed || point.Properties.IsRightButtonPressed)
        {
            _panAnchor = point.Position;
            _panAtPress = _pan;
            e.Pointer.Capture(this);
            e.Handled = true;
            return;
        }
        var selected = FindNearest(point.Position);
        if (selected is null)
        {
            var feature = FindFeature(point.Position);
            if (feature is not null)
            {
                SelectFeature(feature);
                WorldFeatureSelected?.Invoke(feature);
                e.Handled = true;
                return;
            }
            var scaleX = Bounds.Width / Math.Max(1.0, _snapshot.WorldSize[0]);
            var scaleY = Bounds.Height / Math.Max(1.0, _snapshot.WorldSize[1]);
            WorldClicked?.Invoke([
                (float)Math.Clamp((point.Position.X - _pan.X) / (scaleX * _zoom), 0, _snapshot.WorldSize[0]),
                (float)Math.Clamp((point.Position.Y - _pan.Y) / (scaleY * _zoom), 0, _snapshot.WorldSize[1]),
            ]);
            return;
        }
        SelectSlot(selected);
        OrganismSelected?.Invoke(selected.Value);
        e.Handled = true;
    }

    protected override void OnPointerMoved(PointerEventArgs e)
    {
        base.OnPointerMoved(e);
        if (_panAnchor is not { } anchor) return;
        _followSlot = null;
        var point = e.GetPosition(this);
        _pan = _panAtPress + (point - anchor);
        InvalidateVisual();
    }

    protected override void OnPointerReleased(PointerReleasedEventArgs e)
    {
        base.OnPointerReleased(e);
        _panAnchor = null;
        e.Pointer.Capture(null);
    }

    protected override void OnPointerWheelChanged(PointerWheelEventArgs e)
    {
        base.OnPointerWheelChanged(e);
        var cursor = e.GetPosition(this);
        var oldZoom = _zoom;
        _zoom = Math.Clamp(_zoom * (e.Delta.Y > 0 ? 1.2 : 1 / 1.2), 0.35, 12.0);
        var ratio = _zoom / oldZoom;
        _pan = new Vector(cursor.X - (cursor.X - _pan.X) * ratio, cursor.Y - (cursor.Y - _pan.Y) * ratio);
        InvalidateVisual();
        e.Handled = true;
    }

    private uint? FindNearest(Point pointer)
    {
        var scaleX = Bounds.Width / Math.Max(1.0, _snapshot.WorldSize[0]);
        var scaleY = Bounds.Height / Math.Max(1.0, _snapshot.WorldSize[1]);
        uint? best = null;
        var bestDistance = 14.0 * 14.0;
        foreach (var organism in _snapshot.Organisms)
        {
            var x = organism.Position[0] * scaleX * _zoom + _pan.X;
            var y = organism.Position[1] * scaleY * _zoom + _pan.Y;
            var distance = (pointer.X - x) * (pointer.X - x) + (pointer.Y - y) * (pointer.Y - y);
            if (distance >= bestDistance) continue;
            bestDistance = distance;
            best = organism.Slot;
        }
        return best;
    }

    private WorldFeatureSelection? FindFeature(Point pointer)
    {
        var scaleX = Bounds.Width / Math.Max(1.0, _snapshot.WorldSize[0]) * _zoom;
        var scaleY = Bounds.Height / Math.Max(1.0, _snapshot.WorldSize[1]) * _zoom;
        foreach (var obstacle in _snapshot.Obstacles.AsEnumerable().Reverse())
        {
            var bounds = new Rect(
                obstacle.Minimum[0] * scaleX + _pan.X,
                obstacle.Minimum[1] * scaleY + _pan.Y,
                (obstacle.Maximum[0] - obstacle.Minimum[0]) * scaleX,
                (obstacle.Maximum[1] - obstacle.Minimum[1]) * scaleY);
            if (bounds.Contains(pointer)) return new WorldFeatureSelection(WorldFeatureKind.Obstacle, obstacle.Id);
        }
        foreach (var teleporter in _snapshot.Teleporters.AsEnumerable().Reverse())
        {
            var x = teleporter.Center[0] * scaleX + _pan.X;
            var y = teleporter.Center[1] * scaleY + _pan.Y;
            var radius = Math.Max(6, teleporter.Radius * Math.Min(scaleX, scaleY));
            if ((pointer.X - x) * (pointer.X - x) + (pointer.Y - y) * (pointer.Y - y) <= radius * radius)
                return new WorldFeatureSelection(WorldFeatureKind.Teleporter, teleporter.Id);
        }
        return null;
    }

    private void DrawGrid(DrawingContext context)
    {
        const double spacing = 48;
        for (double x = spacing; x < Bounds.Width; x += spacing)
            context.DrawLine(_gridPen, new Point(x, 0), new Point(x, Bounds.Height));
        for (double y = spacing; y < Bounds.Height; y += spacing)
            context.DrawLine(_gridPen, new Point(0, y), new Point(Bounds.Width, y));
    }

    private void DrawWorldFeatures(DrawingContext context)
    {
        var scaleX = Bounds.Width / Math.Max(1.0, _snapshot.WorldSize[0]) * _zoom;
        var scaleY = Bounds.Height / Math.Max(1.0, _snapshot.WorldSize[1]) * _zoom;
        if (_snapshot.Ties.Count > 0)
        {
            var positions = _snapshot.Organisms.ToDictionary(
                organism => new OrganismKey(organism.Slot, organism.Generation),
                organism => organism.Position);
            var tiePen = new Pen(new SolidColorBrush(Color.Parse("#718A68")), 0.9);
            foreach (var tie in _snapshot.Ties)
            {
                if (!positions.TryGetValue(tie.First, out var first) || !positions.TryGetValue(tie.Second, out var second)) continue;
                context.DrawLine(tiePen,
                    new Point(first[0] * scaleX + _pan.X, first[1] * scaleY + _pan.Y),
                    new Point(second[0] * scaleX + _pan.X, second[1] * scaleY + _pan.Y));
            }
        }
        foreach (var shot in _snapshot.Shots)
        {
            var color = shot.Kind switch
            {
                -1 => "#E26722",
                -2 => "#45A23D",
                -3 => "#239AC0",
                -4 => "#8A6748",
                -5 => "#D9AA24",
                _ => "#D8483E",
            };
            var pen = new Pen(new SolidColorBrush(Color.Parse(color)), shot.ImpactFlash ? 1.2 : 1.0);
            var end = new Point(shot.End[0] * scaleX + _pan.X, shot.End[1] * scaleY + _pan.Y);
            if (shot.ImpactFlash)
            {
                var radius = Math.Clamp(20.0 * Math.Min(scaleX, scaleY), 2.0, 8.0);
                context.DrawEllipse(null, pen, end, radius, radius);
            }
            else
            {
                context.DrawLine(
                    pen,
                    new Point(shot.Start[0] * scaleX + _pan.X, shot.Start[1] * scaleY + _pan.Y),
                    end);
            }
        }
        foreach (var obstacle in _snapshot.Obstacles)
        {
            var rectangle = new Rect(
                obstacle.Minimum[0] * scaleX + _pan.X,
                obstacle.Minimum[1] * scaleY + _pan.Y,
                (obstacle.Maximum[0] - obstacle.Minimum[0]) * scaleX,
                (obstacle.Maximum[1] - obstacle.Minimum[1]) * scaleY);
            context.FillRectangle(new SolidColorBrush(Color.Parse("#B9BDB6")), rectangle);
            var selected = _selectedFeature is { Kind: WorldFeatureKind.Obstacle } feature && feature.Id == obstacle.Id;
            context.DrawRectangle(null, new Pen(new SolidColorBrush(Color.Parse(selected ? "#202725" : "#69716D")), selected ? 2.4 : 1.2), rectangle);
        }
        foreach (var teleporter in _snapshot.Teleporters)
        {
            var center = new Point(teleporter.Center[0] * scaleX + _pan.X, teleporter.Center[1] * scaleY + _pan.Y);
            var radius = Math.Max(4.0, teleporter.Radius * Math.Min(scaleX, scaleY));
            var selected = _selectedFeature is { Kind: WorldFeatureKind.Teleporter } feature && feature.Id == teleporter.Id;
            context.DrawEllipse(new SolidColorBrush(Color.Parse("#35239AC0")), new Pen(new SolidColorBrush(Color.Parse(selected ? "#202725" : "#239AC0")), selected ? 3 : 2), center, radius, radius);
        }
        foreach (var corpse in _snapshot.Corpses)
        {
            var center = new Point(corpse.Position[0] * scaleX + _pan.X, corpse.Position[1] * scaleY + _pan.Y);
            var worldRadius = Math.Clamp(Math.Sqrt(Math.Max(1, corpse.Body)) * 0.45, 2.0, 24.0);
            var radius = Math.Clamp(worldRadius * Math.Min(scaleX, scaleY), 1.5, 9.0);
            context.DrawEllipse(
                new SolidColorBrush(Color.Parse("#858982")),
                new Pen(new SolidColorBrush(Color.Parse("#555A55")), 0.8),
                center,
                radius,
                radius);
        }
    }

    private static Color Hsv(double hue, double saturation, double value)
    {
        var chroma = value * saturation;
        var component = chroma * (1 - Math.Abs((hue / 60 % 2) - 1));
        var match = value - chroma;
        var (r, g, b) = hue switch
        {
            < 60 => (chroma, component, 0d),
            < 120 => (component, chroma, 0d),
            < 180 => (0d, chroma, component),
            < 240 => (0d, component, chroma),
            < 300 => (component, 0d, chroma),
            _ => (chroma, 0d, component),
        };
        return Color.FromRgb((byte)((r + match) * 255), (byte)((g + match) * 255), (byte)((b + match) * 255));
    }
}

public enum WorldFeatureKind { Obstacle, Teleporter }
public sealed record WorldFeatureSelection(WorldFeatureKind Kind, uint Id);
