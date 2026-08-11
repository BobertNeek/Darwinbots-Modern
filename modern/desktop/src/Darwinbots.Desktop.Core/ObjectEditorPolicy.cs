using Darwinbots.Desktop.Core.LegacyMenus;

namespace Darwinbots.Desktop.Core;

public enum ObjectEditorAction
{
    Create,
    Edit,
    Delete,
    DeleteAll,
}

public enum ObjectEditorKind
{
    Shape,
    Teleporter,
}

public sealed record ObjectEditorSelection(ObjectEditorKind Kind, uint Id);

public sealed record ObjectEditorRoute(
    ObjectEditorAction Action,
    ObjectEditorKind Kind,
    uint? TargetId,
    bool Allowed,
    string? DisabledReason = null);

public static class ObjectEditorPolicy
{
    public static ObjectEditorRoute Route(
        LegacyCommandId command,
        ObjectEditorSelection? selected) => command switch
        {
            LegacyCommandId.ObjectsNewShape => Create(ObjectEditorKind.Shape),
            LegacyCommandId.ObjectsNewTeleporter => Create(ObjectEditorKind.Teleporter),
            LegacyCommandId.ObjectsDeleteShape => RouteDelete(ObjectEditorKind.Shape, selected),
            LegacyCommandId.ObjectsDeleteTeleporter => RouteDelete(ObjectEditorKind.Teleporter, selected),
            LegacyCommandId.ObjectsDeleteAllShapes => DeleteAll(ObjectEditorKind.Shape),
            LegacyCommandId.ObjectsDeleteAllTeleporters => DeleteAll(ObjectEditorKind.Teleporter),
            _ => throw new ArgumentOutOfRangeException(nameof(command), command, "Command is not an object editor command."),
        };

    public static ObjectEditorRoute RouteEdit(
        ObjectEditorKind kind,
        ObjectEditorSelection? selected) =>
        Matches(kind, selected)
            ? new(ObjectEditorAction.Edit, kind, selected!.Id, true)
            : Disabled(ObjectEditorAction.Edit, kind);

    public static ObjectEditorRoute RouteDelete(
        ObjectEditorKind kind,
        ObjectEditorSelection? selected) =>
        Matches(kind, selected)
            ? new(ObjectEditorAction.Delete, kind, selected!.Id, true)
            : Disabled(ObjectEditorAction.Delete, kind);

    public static bool CanEdit(ObjectEditorKind kind, ObjectEditorSelection? selected) =>
        RouteEdit(kind, selected).Allowed;

    public static bool CanDelete(ObjectEditorKind kind, ObjectEditorSelection? selected) =>
        RouteDelete(kind, selected).Allowed;

    public static ObstacleSnapshot BuildObstaclePayload(
        ObstacleSnapshot original,
        float[] minimum,
        float[] maximum,
        uint color,
        float opacity,
        float[] drift,
        string visionPolicy,
        string shotPolicy) => ObstacleEditorPolicy.BuildPayload(
            original,
            minimum,
            maximum,
            color,
            opacity,
            drift,
            visionPolicy,
            shotPolicy);

    public static TeleporterSnapshot BuildTeleporterPayload(
        TeleporterSnapshot original,
        string mode,
        float[] center,
        float radius,
        float[] destination,
        uint pollInterval,
        int capacity,
        uint backflowLimit,
        bool teleportVegetables,
        bool teleportCorpses,
        bool teleportHeterotrophs,
        bool fixedPosition,
        bool respectShapes,
        string path,
        string inboundPath,
        string outboundPath)
    {
        ArgumentNullException.ThrowIfNull(original);
        ValidateVector(center, nameof(center));
        ValidateVector(destination, nameof(destination));
        if (!float.IsFinite(radius) || radius <= 0f)
            throw new ArgumentOutOfRangeException(nameof(radius));
        if (pollInterval == 0)
            throw new ArgumentOutOfRangeException(nameof(pollInterval));
        if (capacity <= 0)
            throw new ArgumentOutOfRangeException(nameof(capacity));
        ArgumentException.ThrowIfNullOrWhiteSpace(mode);
        ArgumentNullException.ThrowIfNull(path);
        ArgumentNullException.ThrowIfNull(inboundPath);
        ArgumentNullException.ThrowIfNull(outboundPath);

        var normalizedMode = mode.Trim().ToLowerInvariant();
        if (normalizedMode is not ("intrasim" or "inbound" or "outbound"))
            throw new ArgumentException("Internet teleporters are disabled in offline mode.", nameof(mode));

        return original with
        {
            Mode = normalizedMode,
            Center = [.. center],
            Radius = radius,
            Destination = [.. destination],
            PollInterval = pollInterval,
            Capacity = capacity,
            BackflowLimit = backflowLimit,
            TeleportVegetables = teleportVegetables,
            TeleportCorpses = teleportCorpses,
            TeleportHeterotrophs = teleportHeterotrophs,
            FixedPosition = fixedPosition,
            RespectShapes = respectShapes,
            Path = path.Trim(),
            InboundPath = inboundPath.Trim(),
            OutboundPath = outboundPath.Trim(),
        };
    }

    private static ObjectEditorRoute Create(ObjectEditorKind kind) =>
        new(ObjectEditorAction.Create, kind, null, true);

    private static ObjectEditorRoute DeleteAll(ObjectEditorKind kind) =>
        new(ObjectEditorAction.DeleteAll, kind, null, true);

    private static bool Matches(ObjectEditorKind kind, ObjectEditorSelection? selected) =>
        selected is { Kind: var selectedKind } && selectedKind == kind;

    private static ObjectEditorRoute Disabled(ObjectEditorAction action, ObjectEditorKind kind) =>
        new(action, kind, null, false, $"Select a {kind.ToString().ToLowerInvariant()} first.");

    private static void ValidateVector(float[] value, string name)
    {
        ArgumentNullException.ThrowIfNull(value, name);
        if (value.Length != 2 || value.Any(number => !float.IsFinite(number)))
            throw new ArgumentException("Teleporter vectors must contain two finite values.", name);
    }
}
