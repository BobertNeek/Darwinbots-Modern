using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Core;

namespace Darwinbots.Desktop.Controls;

public sealed class WorldViewport : Control
{
    private Bitmap? _backgroundImage;
    private EngineSnapshot _snapshot = EngineSnapshot.Empty;
    private readonly Pen _gridPen = new(new SolidColorBrush(Color.Parse("#D9D8D0")), 0.6);
    private double _zoom = 1.0;
    private bool _zoomLocked;
    private Vector _pan;
    private Point? _panAnchor;
    private Vector _panAtPress;
    private OrganismKey? _selectedKey;
    private readonly HashSet<OrganismKey> _selectedKeys = [];
    private WorldFeatureSelection? _selectedFeature;
    private OrganismKey? _followKey;
    private OrganismKey? _dragKey;
    private float[]? _dragPosition;
    private Point? _dragOrigin;
    private bool _dragMoved;
    private bool _renderWaste = true;
    private bool _showSelectedVision = true;
    private bool _wrapX = true;
    private bool _wrapY = true;
    private bool _showMovementVectors = true;
    private bool _showShotImpacts = true;
    private bool _showResourceGauges = true;
    private bool _highlightTeleporters = true;
    private bool _energyGridEnabled;
    private EnergyGridSize _energyGridSize = EnergyGridSize.Large;
    private bool _flickerMode;
    private bool _renderSkins = true;
    private bool _showDeathPoffs = true;
    private bool _flickerFrame;
    private RgbMemoryMonitorSettings? _rgbMemoryMonitor;

    public event Action<uint>? OrganismSelected;
    public event Action<OrganismKey>? OrganismKeySelected;
    public event Action<OrganismKey, Point>? OrganismContextRequested;
    public event Action<OrganismKey>? OrganismDoubleClicked;
    public event Action<float[]>? WorldClicked;
    public event Action<WorldFeatureSelection>? WorldFeatureSelected;
    public event Action<uint, float[]>? OrganismDragCompleted;
    public event Action<OrganismKey, float[]>? OrganismKeyDragCompleted;

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
    public bool HasBackgroundImage => _backgroundImage is not null;
    public WorldOverlayProjection OverlayProjection => WorldOverlayProjection.Create(
        _snapshot.Organisms,
        _showMovementVectors,
        _showShotImpacts,
        _showResourceGauges,
        HasBackgroundImage);
    public bool HighlightTeleporters { get => _highlightTeleporters; set { _highlightTeleporters = value; InvalidateVisual(); } }
    public bool EnergyGridEnabled { get => _energyGridEnabled; set { _energyGridEnabled = value; InvalidateVisual(); } }
    public EnergyGridSize EnergyGridSize { get => _energyGridSize; set { _energyGridSize = value; InvalidateVisual(); } }
    public bool FlickerMode { get => _flickerMode; set { _flickerMode = value; InvalidateVisual(); } }
    public bool RenderSkins { get => _renderSkins; set { _renderSkins = value; InvalidateVisual(); } }
    public bool ShowDeathPoffs { get => _showDeathPoffs; set { _showDeathPoffs = value; InvalidateVisual(); } }
    public double RenderScale => Math.Max(0.1, TopLevel.GetTopLevel(this)?.RenderScaling ?? 1.0);
    public double InputHitRadius => 14.0 / RenderScale;
    public double Zoom => _zoom;
    public Vector Pan => _pan;
    public bool IsFollowing => _followKey is not null;
    public bool ZoomLocked
    {
        get => _zoomLocked;
        set
        {
            _zoomLocked = value;
            if (value) _pan = ClampPan(_pan);
            InvalidateVisual();
        }
    }

    public void ZoomIn() => SetZoomAt(_zoom * 1.2, new Point(Bounds.Width / 2, Bounds.Height / 2));

    public void ZoomOut() => SetZoomAt(_zoom / 1.2, new Point(Bounds.Width / 2, Bounds.Height / 2));

    public void SetRgbMemoryMonitor(RgbMemoryMonitorSettings? settings)
    {
        settings?.Validate();
        _rgbMemoryMonitor = settings;
        InvalidateVisual();
    }

    public void SelectSlot(uint? slot)
    {
        SelectKey(slot is { } value
            ? _snapshot.Organisms.FirstOrDefault(organism => organism.Slot == value) is { } organism
                ? new OrganismKey(organism.Slot, organism.Generation)
                : null
            : null);
    }

    public void SelectKey(OrganismKey? key)
    {
        _selectedKey = key is { } value && _snapshot.Organisms.Any(organism =>
            organism.Slot == value.Slot && organism.Generation == value.Generation) ? key : null;
        _selectedKeys.Clear();
        if (_selectedKey is { } selected) _selectedKeys.Add(selected);
        InvalidateVisual();
    }

    public void SelectSlots(IEnumerable<uint> slots, uint primary)
    {
        var keys = slots.Select(slot => _snapshot.Organisms.FirstOrDefault(value => value.Slot == slot))
            .OfType<OrganismSnapshot>()
            .Select(value => new OrganismKey(value.Slot, value.Generation));
        var primaryKey = _snapshot.Organisms.FirstOrDefault(value => value.Slot == primary) is { } organism
            ? new OrganismKey(organism.Slot, organism.Generation)
            : new OrganismKey(primary, 0);
        SelectKeys(keys, primaryKey);
    }

    public void SelectKeys(IEnumerable<OrganismKey> keys, OrganismKey primary)
    {
        _selectedKey = primary;
        _selectedKeys.Clear();
        foreach (var key in keys) _selectedKeys.Add(key);
        _selectedKeys.Add(primary);
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
        if (_selectedKey is { } selected && !Contains(snapshot, selected))
        {
            _selectedKey = null;
            _selectedKeys.Clear();
        }
        _selectedKeys.RemoveWhere(key => !Contains(snapshot, key));
        if (_followKey is { } followed && snapshot.Organisms.FirstOrDefault(value => KeyOf(value) == followed) is { } organism)
        {
            var scaleX = Bounds.Width / Math.Max(1.0, snapshot.WorldSize[0]);
            var scaleY = Bounds.Height / Math.Max(1.0, snapshot.WorldSize[1]);
            _pan = new Vector(Bounds.Width / 2 - organism.Position[0] * scaleX * _zoom,
                Bounds.Height / 2 - organism.Position[1] * scaleY * _zoom);
        }
        else if (_followKey is not null)
        {
            _followKey = null;
        }
        _pan = ClampPan(_pan);
        InvalidateVisual();
    }

    public void FollowSlot(uint? slot)
    {
        _followKey = slot is { } value
            ? _snapshot.Organisms.FirstOrDefault(organism => organism.Slot == value) is { } organism
                ? KeyOf(organism)
                : null
            : null;
        InvalidateVisual();
    }

    public void FollowKey(OrganismKey? key)
    {
        _followKey = key;
        if (key is { } followed && _snapshot.Organisms.FirstOrDefault(value => KeyOf(value) == followed) is { } organism)
        {
            var scaleX = Bounds.Width / Math.Max(1.0, _snapshot.WorldSize[0]);
            var scaleY = Bounds.Height / Math.Max(1.0, _snapshot.WorldSize[1]);
            _pan = new Vector(Bounds.Width / 2 - organism.Position[0] * scaleX * _zoom,
                Bounds.Height / 2 - organism.Position[1] * scaleY * _zoom);
            _pan = ClampPan(_pan);
        }
        InvalidateVisual();
    }

    public override void Render(DrawingContext context)
    {
        base.Render(context);
        var overlay = OverlayProjection;
        context.FillRectangle(new SolidColorBrush(Color.Parse("#F9F8F2")), Bounds);
        if (overlay.HasBackgroundImage && _backgroundImage is not null) context.DrawImage(_backgroundImage, Bounds);
        DrawGrid(context);
        if (FlickerMode && (_flickerFrame = !_flickerFrame)) return;
        DrawEnergyGrid(context);
        DrawToroidalEdges(context);
        DrawWorldFeatures(context, overlay);
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
            DrawMovementVectors(context, scaleX, scaleY, overlay);
            DrawResourceGauges(context, scaleX, scaleY, overlay);
            return;
        }

        DrawFallbackOrganisms(context, scaleX, scaleY);
        DrawSelectedVision(context, scaleX, scaleY);
        DrawMovementVectors(context, scaleX, scaleY, overlay);
        DrawResourceGauges(context, scaleX, scaleY, overlay);
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
            selectedDrawn |= _selectedKeys.Contains(KeyOf(instance));
        }

        if (!selectedDrawn && _selectedKey is { } selected
            && _snapshot.RenderInstances.FirstOrDefault(candidate => KeyOf(candidate) == selected) is { } selectedInstance)
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
        var position = _dragKey == KeyOf(instance) && _dragPosition is not null ? _dragPosition : instance.Position;
        var center = ScreenPoint(position, scaleX, scaleY);
        var radiusScale = Math.Min(scaleX, scaleY);
        var radius = Math.Clamp(instance.Radius * radiusScale * _zoom, 1.5, 28.0);
        var organism = _snapshot.Organisms.FirstOrDefault(value => KeyOf(value) == KeyOf(instance));
        var color = organism is not null && _rgbMemoryMonitor is { } monitor
            ? Color.FromUInt32(RgbMemoryMonitor.ColorFor(organism, monitor).ToArgb())
            : Color.FromUInt32(instance.Color);
        var selected = _selectedKeys.Contains(KeyOf(instance));
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
        if (_selectedKey is not { } selected
            || _snapshot.Organisms.FirstOrDefault(organism => KeyOf(organism) == selected) is not { } organism)
        {
            return;
        }
        var center = ScreenPoint(organism.Position, scaleX, scaleY);
        var radiusScale = Math.Min(scaleX, scaleY) * _zoom;
        var organismRadius = _snapshot.RenderInstances
            .FirstOrDefault(instance => KeyOf(instance) == selected)?.Radius ?? 8f;
        var sectors = OrganismVisualGeometry.EyeSectors(organism.Vision, organism.Aim, organismRadius);
        for (var index = 0; index < sectors.Length; index++)
        {
            var sector = sectors[index];
            var eye = organism.Vision.Eyes is { Length: > 0 } eyes && index < eyes.Length
                ? eyes[index]
                : null;
            var range = eye is { Value: > 0 }
                ? 1.0 / Math.Sqrt(eye.Value) * (sector.Range + organismRadius) + organismRadius
                : sector.Range;
            var screenRange = Math.Clamp(
                range * radiusScale,
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
            var color = _rgbMemoryMonitor is { } monitor
                ? Color.FromUInt32(RgbMemoryMonitor.ColorFor(organism, monitor).ToArgb())
                : Color.FromUInt32(organism.Phenotype.Color);
            var selected = _selectedKeys.Contains(KeyOf(organism));
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
        if (point.Properties.IsMiddleButtonPressed)
        {
            _panAnchor = point.Position;
            _panAtPress = _pan;
            e.Pointer.Capture(this);
            e.Handled = true;
            return;
        }
        var selected = FindNearest(point.Position);
        if (point.Properties.IsRightButtonPressed)
        {
            if (selected is { } contextKey)
            {
                SelectKey(contextKey);
                OrganismSelected?.Invoke(contextKey.Slot);
                OrganismKeySelected?.Invoke(contextKey);
                OrganismContextRequested?.Invoke(contextKey, point.Position);
                e.Handled = true;
            }
            return;
        }
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
        SelectKey(selected);
        OrganismSelected?.Invoke(selected.Value.Slot);
        OrganismKeySelected?.Invoke(selected.Value);
        if (e.ClickCount >= 2)
        {
            OrganismDoubleClicked?.Invoke(selected.Value);
            e.Handled = true;
            return;
        }
        _dragKey = selected;
        _dragPosition = WorldPoint(point.Position);
        _dragOrigin = point.Position;
        _dragMoved = false;
        e.Pointer.Capture(this);
        e.Handled = true;
    }

    protected override void OnPointerMoved(PointerEventArgs e)
    {
        base.OnPointerMoved(e);
        if (_dragKey is not null)
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
        _followKey = null;
        var point = e.GetPosition(this);
        _pan = ClampPan(_panAtPress + (point - anchor));
        InvalidateVisual();
    }

    protected override void OnPointerReleased(PointerReleasedEventArgs e)
    {
        base.OnPointerReleased(e);
        _panAnchor = null;
        if (_dragMoved && _dragKey is { } key && _dragPosition is { } position)
        {
            OrganismKeyDragCompleted?.Invoke(key, position);
        }
        _dragKey = null;
        _dragPosition = null;
        _dragOrigin = null;
        _dragMoved = false;
        e.Pointer.Capture(null);
    }

    protected override void OnPointerWheelChanged(PointerWheelEventArgs e)
    {
        base.OnPointerWheelChanged(e);
        var cursor = e.GetPosition(this);
        SetZoomAt(_zoom * (e.Delta.Y > 0 ? 1.2 : 1 / 1.2), cursor);
        e.Handled = true;
    }

    private void SetZoomAt(double zoom, Point anchor)
    {
        var oldZoom = _zoom;
        _zoom = Math.Clamp(zoom, 0.35, 12.0);
        var ratio = _zoom / oldZoom;
        _pan = ClampPan(new Vector(
            anchor.X - (anchor.X - _pan.X) * ratio,
            anchor.Y - (anchor.Y - _pan.Y) * ratio));
        InvalidateVisual();
    }

    private Vector ClampPan(Vector pan)
    {
        if (!_zoomLocked) return pan;
        var worldWidth = Bounds.Width * _zoom;
        var worldHeight = Bounds.Height * _zoom;
        var minX = Math.Min(0, Bounds.Width - worldWidth);
        var maxX = Math.Max(0, Bounds.Width - worldWidth);
        var minY = Math.Min(0, Bounds.Height - worldHeight);
        var maxY = Math.Max(0, Bounds.Height - worldHeight);
        return new Vector(
            Math.Clamp(pan.X, minX, maxX),
            Math.Clamp(pan.Y, minY, maxY));
    }

    private OrganismKey? FindNearest(Point pointer)
    {
        var scaleX = Bounds.Width / Math.Max(1.0, _snapshot.WorldSize[0]);
        var scaleY = Bounds.Height / Math.Max(1.0, _snapshot.WorldSize[1]);
        OrganismKey? best = null;
        var bestDistance = InputHitRadius * InputHitRadius;
        foreach (var organism in _snapshot.Organisms)
        {
            var x = organism.Position[0] * scaleX * _zoom + _pan.X;
            var y = organism.Position[1] * scaleY * _zoom + _pan.Y;
            var distance = (pointer.X - x) * (pointer.X - x) + (pointer.Y - y) * (pointer.Y - y);
            if (distance >= bestDistance) continue;
            bestDistance = distance;
            best = KeyOf(organism);
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

    private static OrganismKey KeyOf(OrganismSnapshot organism) =>
        new(organism.Slot, organism.Generation);

    private static OrganismKey KeyOf(RenderInstanceSnapshot instance) =>
        new(instance.Slot, instance.Generation);

    private static bool Contains(EngineSnapshot snapshot, OrganismKey key) =>
        snapshot.Organisms.Any(organism => KeyOf(organism) == key);

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

    private void DrawWorldFeatures(DrawingContext context, WorldOverlayProjection overlay)
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
            if (!overlay.ShowShotImpacts && shot.ImpactFlash) continue;
            var color = shot.Kind switch
            {
                -1 => "#FFFFFF",
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

    private void DrawMovementVectors(
        DrawingContext context,
        double scaleX,
        double scaleY,
        WorldOverlayProjection overlay)
    {
        if (!overlay.ShowMovementVectors) return;
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

    private void DrawResourceGauges(
        DrawingContext context,
        double scaleX,
        double scaleY,
        WorldOverlayProjection overlay)
    {
        if (!overlay.ShowResourceGauges) return;
        var stride = _snapshot.Organisms.Count > 10_000 ? 6 : _snapshot.Organisms.Count > 3_000 ? 2 : 1;
        for (var index = 0; index < _snapshot.Organisms.Count; index += stride)
        {
            var organism = _snapshot.Organisms[index];
            var center = ScreenPoint(organism.Position, scaleX, scaleY);
            var width = 26.0;
            var origin = new Point(center.X - width / 2, center.Y - 17);
            var gaugeStart = index * 3;
            var gaugeEnd = Math.Min(gaugeStart + 3, overlay.Gauges.Count);
            for (var gaugeIndex = gaugeStart; gaugeIndex < gaugeEnd; gaugeIndex++)
            {
                var gauge = overlay.Gauges[gaugeIndex];
                var row = gaugeIndex - gaugeStart;
                var rowOrigin = new Point(origin.X, origin.Y + row * 6);
                context.FillRectangle(
                    new SolidColorBrush(Color.Parse("#A0D0CEC5")),
                    new Rect(rowOrigin, new Size(width, 3.5)));
                context.FillRectangle(
                    new SolidColorBrush(Color.Parse(gauge.Color)),
                    new Rect(rowOrigin, new Size(width * gauge.Ratio, 3.5)));
                var text = new FormattedText(
                    gauge.Text,
                    System.Globalization.CultureInfo.InvariantCulture,
                    FlowDirection.LeftToRight,
                    new Typeface("Bahnschrift"),
                    7,
                    new SolidColorBrush(Color.Parse(gauge.Color)));
                context.DrawText(text, new Point(origin.X + width + 4, rowOrigin.Y - 2));
            }
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
