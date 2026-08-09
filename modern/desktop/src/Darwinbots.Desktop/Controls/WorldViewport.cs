using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Controls;

public sealed class WorldViewport : Control
{
    private Bitmap? _backgroundImage;
    private EngineSnapshot _snapshot = EngineSnapshot.Empty;
    private readonly Pen _gridPen = new(new SolidColorBrush(Color.Parse("#D9D8D0")), 0.6);
    private double _zoom = 1.0;
    private Vector _pan;
    private Point? _panAnchor;
    private Vector _panAtPress;
    private uint? _selectedSlot;
    private readonly HashSet<uint> _selectedSlots = [];
    private WorldFeatureSelection? _selectedFeature;
    private uint? _followSlot;
    private uint? _dragSlot;
    private float[]? _dragPosition;
    private Point? _dragOrigin;
    private bool _dragMoved;
    private bool _renderWaste = true;
    private bool _showSelectedVision = true;
    private bool _wrapX = true;
    private bool _wrapY = true;
    private bool _showMovementVectors;
    private bool _showShotImpacts = true;
    private bool _showResourceGauges;
    private bool _highlightTeleporters;
    private bool _energyGridEnabled;
    private EnergyGridSize _energyGridSize = EnergyGridSize.Large;
    private bool _flickerMode;
    private bool _renderSkins = true;
    private bool _showDeathPoffs = true;
    private bool _flickerFrame;

    public event Action<uint>? OrganismSelected;
    public event Action<float[]>? WorldClicked;
    public event Action<WorldFeatureSelection>? WorldFeatureSelected;
    public event Action<uint, float[]>? OrganismDragCompleted;

    public bool RenderWaste { get => _renderWaste; set { _renderWaste = value; InvalidateVisual(); } }
    public bool ShowSelectedVision { get => _showSelectedVision; set { _showSelectedVision = value; InvalidateVisual(); } }
    public bool ToroidalWorld
    {
        get => _wrapX && _wrapY;
        set { _wrapX = value; _wrapY = value; InvalidateVisual(); }
    }
    public bool WrapX { get => _wrapX; set { _wrapX = value; InvalidateVisual(); } }
    public bool WrapY { get => _wrapY; set { _wrapY = value; InvalidateVisual(); } }
    public bool ShowMovementVectors { get => _showMovementVectors; set { _showMovementVectors = value; InvalidateVisual(); } }
    public bool ShowShotImpacts { get => _showShotImpacts; set { _showShotImpacts = value; InvalidateVisual(); } }
    public bool ShowResourceGauges { get => _showResourceGauges; set { _showResourceGauges = value; InvalidateVisual(); } }
    public bool HighlightTeleporters { get => _highlightTeleporters; set { _highlightTeleporters = value; InvalidateVisual(); } }
    public bool EnergyGridEnabled { get => _energyGridEnabled; set { _energyGridEnabled = value; InvalidateVisual(); } }
    public EnergyGridSize EnergyGridSize { get => _energyGridSize; set { _energyGridSize = value; InvalidateVisual(); } }
    public bool FlickerMode { get => _flickerMode; set { _flickerMode = value; InvalidateVisual(); } }
    public bool RenderSkins { get => _renderSkins; set { _renderSkins = value; InvalidateVisual(); } }
    public bool ShowDeathPoffs { get => _showDeathPoffs; set { _showDeathPoffs = value; InvalidateVisual(); } }

    public void SelectSlot(uint? slot)
    {
        _selectedSlot = slot;
        _selectedSlots.Clear();
        if (slot is { } selected) _selectedSlots.Add(selected);
        InvalidateVisual();
    }

    public void SelectSlots(IEnumerable<uint> slots, uint primary)
    {
        _selectedSlot = primary;
        _selectedSlots.Clear();
        foreach (var slot in slots) _selectedSlots.Add(slot);
        _selectedSlots.Add(primary);
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
        if (_backgroundImage is not null) context.DrawImage(_backgroundImage, Bounds);
        DrawGrid(context);
        if (FlickerMode && (_flickerFrame = !_flickerFrame)) return;
        DrawEnergyGrid(context);
        DrawToroidalEdges(context);
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
            DrawOrganisms(context, scaleX, scaleY);
            DrawSelectedVision(context, scaleX, scaleY);
            DrawMovementVectors(context, scaleX, scaleY);
            DrawResourceGauges(context, scaleX, scaleY);
            return;
        }

        DrawFallbackOrganisms(context, scaleX, scaleY);
        DrawSelectedVision(context, scaleX, scaleY);
        DrawMovementVectors(context, scaleX, scaleY);
        DrawResourceGauges(context, scaleX, scaleY);
    }

    private void DrawOrganisms(DrawingContext context, double scaleX, double scaleY)
    {
        var radiusScale = Math.Min(scaleX, scaleY);
        var stride = _snapshot.RenderInstances.Count > 100_000 && _zoom < 1 ? 8
            : _snapshot.RenderInstances.Count > 50_000 && _zoom < 2 ? 4
            : _snapshot.RenderInstances.Count > 20_000 && _zoom < 1 ? 2 : 1;
        var selectedDrawn = false;
        for (var index = 0; index < _snapshot.RenderInstances.Count; index += stride)
        {
            var instance = _snapshot.RenderInstances[index];
            var radius = Math.Clamp(instance.Radius * radiusScale * _zoom, 1.5, 28.0);
            var details = radius >= 3.25
                && !(_snapshot.RenderInstances.Count > 20_000 && _zoom < 1)
                && stride == 1;
            DrawOrganism(context, instance, scaleX, scaleY, details);
            selectedDrawn |= _selectedSlots.Contains(instance.Slot);
        }

        if (!selectedDrawn && _selectedSlot is { } selected
            && _snapshot.RenderInstances.FirstOrDefault(candidate => candidate.Slot == selected) is { } selectedInstance)
        {
            DrawOrganism(context, selectedInstance, scaleX, scaleY, details: true);
        }
    }

    private void DrawOrganism(
        DrawingContext context,
        RenderInstanceSnapshot instance,
        double scaleX,
        double scaleY,
        bool details)
    {
        var position = _dragSlot == instance.Slot && _dragPosition is not null ? _dragPosition : instance.Position;
        var center = ScreenPoint(position, scaleX, scaleY);
        var radiusScale = Math.Min(scaleX, scaleY);
        var radius = Math.Clamp(instance.Radius * radiusScale * _zoom, 1.5, 28.0);
        var color = Color.FromUInt32(instance.Color);
        var selected = _selectedSlots.Contains(instance.Slot);
        var edgeColor = ContrastColor(color, strong: false);
        context.DrawEllipse(
            new SolidColorBrush(color),
            new Pen(new SolidColorBrush(edgeColor), selected ? 1.1 : 0.65),
            center,
            radius,
            radius);

        if (selected)
        {
            var selectionRadius = OrganismVisualGeometry.SelectionRingRadius((float)radius);
            context.DrawEllipse(null, new Pen(Brushes.Black, 2.2), center, selectionRadius, selectionRadius);
        }
        if (!details)
        {
            return;
        }

        if (RenderSkins) DrawSkin(context, instance, center, radius, color);
        DrawHeading(context, instance, center, radius, color);
    }

    private static void DrawSkin(
        DrawingContext context,
        RenderInstanceSnapshot instance,
        Point center,
        double radius,
        Color fill)
    {
        var points = OrganismVisualGeometry.SkinPoints(instance.Skin, (float)radius, instance.Aim);
        if (points.Length < 2)
        {
            return;
        }
        var pen = new Pen(new SolidColorBrush(ContrastColor(fill, strong: true)), Math.Clamp(radius * 0.09, 0.65, 1.35));
        for (var index = 1; index < points.Length; index++)
        {
            context.DrawLine(
                pen,
                new Point(center.X + points[index - 1].X, center.Y - points[index - 1].Y),
                new Point(center.X + points[index].X, center.Y - points[index].Y));
        }
    }

    private static void DrawHeading(
        DrawingContext context,
        RenderInstanceSnapshot instance,
        Point center,
        double radius,
        Color fill)
    {
        var inner = OrganismVisualGeometry.HeadingPoint((float)(radius * 0.78), instance.Aim);
        var outer = OrganismVisualGeometry.HeadingPoint((float)(radius + Math.Clamp(radius * 0.28, 2.0, 4.0)), instance.Aim);
        context.DrawLine(
            new Pen(new SolidColorBrush(ContrastColor(fill, strong: true)), 1.15),
            new Point(center.X + inner.X, center.Y - inner.Y),
            new Point(center.X + outer.X, center.Y - outer.Y));
    }

    private void DrawSelectedVision(DrawingContext context, double scaleX, double scaleY)
    {
        if (!ShowSelectedVision) return;
        if (_selectedSlot is not { } selected
            || _snapshot.Organisms.FirstOrDefault(organism => organism.Slot == selected) is not { } organism)
        {
            return;
        }
        var center = ScreenPoint(organism.Position, scaleX, scaleY);
        var radiusScale = Math.Min(scaleX, scaleY) * _zoom;
        var organismRadius = _snapshot.RenderInstances
            .FirstOrDefault(instance => instance.Slot == selected)?.Radius ?? 8f;
        var sectors = OrganismVisualGeometry.EyeSectors(organism.Vision, organism.Aim, organismRadius);
        foreach (var sector in sectors)
        {
            var screenRange = Math.Clamp(
                sector.Range * radiusScale,
                18.0,
                Math.Max(18.0, Math.Min(Bounds.Width, Bounds.Height) * 0.46));
            DrawVisionSector(context, center, screenRange, sector);
        }
    }

    private static void DrawVisionSector(
        DrawingContext context,
        Point center,
        double range,
        EyeSectorGeometry sector)
    {
        const int curveSteps = 8;
        var geometry = new StreamGeometry();
        using (var geometryContext = geometry.Open())
        {
            geometryContext.BeginFigure(center, true);
            for (var step = 0; step <= curveSteps; step++)
            {
                var angle = sector.StartRadians + sector.SweepRadians * step / curveSteps;
                geometryContext.LineTo(new Point(
                    center.X + Math.Sin(angle) * range,
                    center.Y - Math.Cos(angle) * range));
            }
            geometryContext.EndFigure(true);
        }
        var fill = new SolidColorBrush(Color.Parse(sector.Focused ? "#28E26722" : "#1E239AC0"));
        var edge = new Pen(new SolidColorBrush(Color.Parse(sector.Focused ? "#A8E26722" : "#7A239AC0")), 0.65);
        context.DrawGeometry(fill, edge, geometry);
    }

    private void DrawFallbackOrganisms(DrawingContext context, double scaleX, double scaleY)
    {
        foreach (var organism in _snapshot.Organisms)
        {
            var center = ScreenPoint(organism.Position, scaleX, scaleY);
            var color = Color.FromUInt32(organism.Phenotype.Color);
            var selected = _selectedSlots.Contains(organism.Slot);
            context.DrawEllipse(
                new SolidColorBrush(color),
                new Pen(selected ? Brushes.Black : new SolidColorBrush(ContrastColor(color, false)), selected ? 2.2 : 0.8),
                center,
                3.2,
                3.2);
        }
    }

    private Point ScreenPoint(float[] position, double scaleX, double scaleY) => new(
        position[0] * scaleX * _zoom + _pan.X,
        position[1] * scaleY * _zoom + _pan.Y);

    private static Color ContrastColor(Color color, bool strong)
    {
        var luminance = (0.2126 * color.R + 0.7152 * color.G + 0.0722 * color.B) / 255.0;
        if (luminance > 0.58)
        {
            return Color.Parse(strong ? "#26302C" : "#4C5752");
        }
        return Color.Parse(strong ? "#F4F2E8" : "#D8D8CF");
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
        _dragSlot = selected;
        _dragPosition = WorldPoint(point.Position);
        _dragOrigin = point.Position;
        _dragMoved = false;
        e.Pointer.Capture(this);
        e.Handled = true;
    }

    protected override void OnPointerMoved(PointerEventArgs e)
    {
        base.OnPointerMoved(e);
        if (_dragSlot is not null)
        {
            var pointer = e.GetPosition(this);
            if (_dragOrigin is { } origin && DesktopControlRules.IsDragGesture(pointer.X - origin.X, pointer.Y - origin.Y))
                _dragMoved = true;
            if (!_dragMoved) return;
            _dragPosition = WorldPoint(pointer);
            InvalidateVisual();
            e.Handled = true;
            return;
        }
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
        if (_dragMoved && _dragSlot is { } slot && _dragPosition is { } position)
            OrganismDragCompleted?.Invoke(slot, position);
        _dragSlot = null;
        _dragPosition = null;
        _dragOrigin = null;
        _dragMoved = false;
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

    private float[] WorldPoint(Point point)
    {
        var scaleX = Bounds.Width / Math.Max(1.0, _snapshot.WorldSize[0]);
        var scaleY = Bounds.Height / Math.Max(1.0, _snapshot.WorldSize[1]);
        return [
            (float)Math.Clamp((point.X - _pan.X) / (scaleX * _zoom), 0, _snapshot.WorldSize[0]),
            (float)Math.Clamp((point.Y - _pan.Y) / (scaleY * _zoom), 0, _snapshot.WorldSize[1]),
        ];
    }

    private void DrawToroidalEdges(DrawingContext context)
    {
        var pen = new Pen(new SolidColorBrush(Color.Parse("#239AC0"), 0.35), 2);
        if (WrapX)
        {
            context.DrawLine(pen, new Point(1, 0), new Point(1, Bounds.Height));
            context.DrawLine(pen, new Point(Bounds.Width - 1, 0), new Point(Bounds.Width - 1, Bounds.Height));
        }
        if (WrapY)
        {
            context.DrawLine(pen, new Point(0, 1), new Point(Bounds.Width, 1));
            context.DrawLine(pen, new Point(0, Bounds.Height - 1), new Point(Bounds.Width, Bounds.Height - 1));
        }
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
        foreach (var teleporter in _snapshot.FileTeleporters.AsEnumerable().Reverse())
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
            if (!RenderWaste && shot.Kind == -4) continue;
            if (!ShowShotImpacts && shot.ImpactFlash) continue;
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
            context.FillRectangle(new SolidColorBrush(Color.FromUInt32(obstacle.Color), Math.Clamp(obstacle.Opacity, 0f, 1f)), rectangle);
            var selected = _selectedFeature is { Kind: WorldFeatureKind.Obstacle } feature && feature.Id == obstacle.Id;
            context.DrawRectangle(null, new Pen(new SolidColorBrush(Color.Parse(selected ? "#202725" : "#69716D")), selected ? 2.4 : 1.2), rectangle);
        }
        foreach (var teleporter in _snapshot.Teleporters)
        {
            var center = new Point(teleporter.Center[0] * scaleX + _pan.X, teleporter.Center[1] * scaleY + _pan.Y);
            var radius = Math.Max(4.0, teleporter.Radius * Math.Min(scaleX, scaleY));
            var selected = _selectedFeature is { Kind: WorldFeatureKind.Teleporter } feature && feature.Id == teleporter.Id;
            var highlighted = selected || HighlightTeleporters;
            context.DrawEllipse(
                new SolidColorBrush(Color.Parse(HighlightTeleporters ? "#55239AC0" : "#35239AC0")),
                new Pen(new SolidColorBrush(Color.Parse(highlighted ? "#E26722" : "#239AC0")), highlighted ? 3 : 2),
                center,
                radius,
                radius);
        }
        foreach (var teleporter in _snapshot.FileTeleporters)
        {
            var center = new Point(teleporter.Center[0] * scaleX + _pan.X, teleporter.Center[1] * scaleY + _pan.Y);
            var radius = Math.Max(4.0, teleporter.Radius * Math.Min(scaleX, scaleY));
            var selected = _selectedFeature is { Kind: WorldFeatureKind.Teleporter } feature && feature.Id == teleporter.Id;
            var highlighted = selected || HighlightTeleporters;
            context.DrawEllipse(
                new SolidColorBrush(Color.Parse("#553E7B36")),
                new Pen(new SolidColorBrush(Color.Parse(highlighted ? "#E26722" : "#3E7B36")), highlighted ? 3 : 2),
                center,
                radius,
                radius);
        }
        if (!ShowDeathPoffs) return;
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

    public void SetBackgroundImage(Bitmap? image)
    {
        _backgroundImage?.Dispose();
        _backgroundImage = image;
        InvalidateVisual();
    }

    private void DrawMovementVectors(DrawingContext context, double scaleX, double scaleY)
    {
        if (!ShowMovementVectors) return;
        var stride = _snapshot.Organisms.Count > 20_000 ? 8 : _snapshot.Organisms.Count > 5_000 ? 3 : 1;
        var pen = new Pen(new SolidColorBrush(Color.Parse("#A5239AC0")), 0.9);
        for (var index = 0; index < _snapshot.Organisms.Count; index += stride)
        {
            var organism = _snapshot.Organisms[index];
            var start = ScreenPoint(organism.Position, scaleX, scaleY);
            var end = new Point(
                start.X + organism.Velocity[0] * scaleX * _zoom * 12,
                start.Y + organism.Velocity[1] * scaleY * _zoom * 12);
            context.DrawLine(pen, start, end);
        }
    }

    private void DrawResourceGauges(DrawingContext context, double scaleX, double scaleY)
    {
        if (!ShowResourceGauges) return;
        var stride = _snapshot.Organisms.Count > 10_000 ? 6 : _snapshot.Organisms.Count > 3_000 ? 2 : 1;
        for (var index = 0; index < _snapshot.Organisms.Count; index += stride)
        {
            var organism = _snapshot.Organisms[index];
            var center = ScreenPoint(organism.Position, scaleX, scaleY);
            var width = 18.0;
            var ratio = Math.Clamp(organism.Energy / 20_000.0, 0.0, 1.0);
            var origin = new Point(center.X - width / 2, center.Y - 12);
            context.FillRectangle(new SolidColorBrush(Color.Parse("#A0D0CEC5")), new Rect(origin, new Size(width, 2.5)));
            context.FillRectangle(new SolidColorBrush(Color.Parse("#D045A23D")), new Rect(origin, new Size(width * ratio, 2.5)));
        }
    }

    private void DrawEnergyGrid(DrawingContext context)
    {
        if (!EnergyGridEnabled || _snapshot.Organisms.Count == 0) return;
        var cellSize = _energyGridSize switch
        {
            EnergyGridSize.Large => 128.0,
            EnergyGridSize.Medium => 80.0,
            _ => 48.0,
        };
        var scaleX = Bounds.Width / Math.Max(1.0, _snapshot.WorldSize[0]);
        var scaleY = Bounds.Height / Math.Max(1.0, _snapshot.WorldSize[1]);
        var cells = new Dictionary<(int X, int Y), (int Count, double Energy)>();
        foreach (var organism in _snapshot.Organisms)
        {
            var point = ScreenPoint(organism.Position, scaleX, scaleY);
            var key = ((int)Math.Floor(point.X / cellSize), (int)Math.Floor(point.Y / cellSize));
            cells.TryGetValue(key, out var value);
            cells[key] = (value.Count + 1, value.Energy + organism.Energy);
        }
        var maximum = Math.Max(1.0, cells.Values.Max(value => value.Energy));
        foreach (var (cell, value) in cells)
        {
            var intensity = (byte)Math.Clamp(24 + value.Energy / maximum * 92, 24, 116);
            context.FillRectangle(
                new SolidColorBrush(Color.FromArgb(intensity, 63, 145, 56)),
                new Rect(cell.X * cellSize, cell.Y * cellSize, cellSize, cellSize));
        }
    }

}

public enum EnergyGridSize { Large, Medium, Small }
public enum WorldFeatureKind { Obstacle, Teleporter }
public sealed record WorldFeatureSelection(WorldFeatureKind Kind, uint Id);
