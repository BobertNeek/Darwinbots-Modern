namespace Darwinbots.Desktop.Core;

public sealed record PhylogenyNode(
    OrganismKey Key,
    int Depth,
    bool IsFocused,
    bool IsHighlighted,
    ulong LineageId,
    OrganismKey? Parent1,
    OrganismKey? Parent2);

public sealed record PhylogenyGraph(
    OrganismKey Focus,
    int MaxDepth,
    IReadOnlyList<PhylogenyNode> Nodes,
    IReadOnlySet<OrganismKey> Highlighted);

public sealed record PhylogenyLink(OrganismKey Parent, OrganismKey Child);

public static class PhylogenyModel
{
    public static int? CountNativeDescendants(EngineSnapshot snapshot, OrganismKey focus)
    {
        var ancestor = snapshot.Organisms.FirstOrDefault(robot =>
            robot.Exists && robot.Slot == focus.Slot && robot.Generation == focus.Generation);
        if (ancestor is null) return null;

        // DB2 discendenti scans existing slots by primary absolute parent, including corpses.
        var children = snapshot.Organisms
            .Where(robot => robot.Exists && robot.ParentAbsoluteId.HasValue)
            .OrderBy(robot => robot.Slot)
            .ToLookup(robot => robot.ParentAbsoluteId!.Value, robot => robot.UniqueId);
        var count = 0;
        CountChildren(ancestor.UniqueId);
        return count;

        void CountChildren(ulong absoluteId)
        {
            // The VB6 ByRef count gates entry only. An open scan still counts its remaining children.
            if (count >= 1_000) return;
            foreach (var child in children[absoluteId])
            {
                count++;
                CountChildren(child);
            }
        }
    }

    public static IReadOnlyList<PhylogenyLink> DescendantLinks(EngineSnapshot snapshot, OrganismKey focus, int maxDepth) =>
        new PrimaryLineage(snapshot).DescendantLinks(focus, maxDepth);

    public static IReadOnlyList<PhylogenyLink> WholeLineage(EngineSnapshot snapshot, OrganismKey focus) =>
        new PrimaryLineage(snapshot).WholeLineage(focus);

    public static PhylogenyGraph Build(EngineSnapshot snapshot, OrganismKey focus, int maxDepth)
    {
        ArgumentNullException.ThrowIfNull(snapshot);
        maxDepth = Math.Clamp(maxDepth, 1, 1_000);
        var organisms = snapshot.Organisms.ToDictionary(
            value => new OrganismKey(value.Slot, value.Generation));
        if (!organisms.ContainsKey(focus))
            return new(focus, maxDepth, [], new HashSet<OrganismKey>());

        var depths = new Dictionary<OrganismKey, int> { [focus] = 0 };
        var pending = new Queue<OrganismKey>();
        pending.Enqueue(focus);
        while (pending.Count > 0)
        {
            var current = pending.Dequeue();
            var currentDepth = depths[current];
            if (Math.Abs(currentDepth) >= maxDepth) continue;

            var currentOrganism = organisms[current];
            foreach (var parent in currentOrganism.Parents?.OfType<OrganismKey>() ?? [])
                Add(parent, currentDepth - 1);

            foreach (var child in organisms.Values.Where(value =>
                         value.Parents?.Any(parent => parent == current) == true))
                Add(new OrganismKey(child.Slot, child.Generation), currentDepth + 1);
        }

        var nodes = depths
            .OrderBy(value => value.Value)
            .ThenBy(value => value.Key.Slot)
            .Select(value =>
            {
                var organism = organisms[value.Key];
                return new PhylogenyNode(
                    value.Key,
                    value.Value,
                    value.Key == focus,
                    true,
                    organism.Phenotype.LineageId,
                    organism.Parents?.ElementAtOrDefault(0),
                    organism.Parents?.ElementAtOrDefault(1));
            })
            .ToArray();
        return new(focus, maxDepth, nodes, depths.Keys.ToHashSet());

        void Add(OrganismKey key, int depth)
        {
            if (!organisms.ContainsKey(key) || Math.Abs(depth) > maxDepth || depths.ContainsKey(key)) return;
            depths[key] = depth;
            pending.Enqueue(key);
        }
    }

    private sealed class PrimaryLineage
    {
        private readonly bool _native;
        private readonly Dictionary<OrganismKey, OrganismSnapshot> _robots;
        private readonly ILookup<ulong, OrganismSnapshot> _robotsByAbsoluteId;
        private readonly ILookup<ulong, OrganismKey> _childrenByAbsoluteId;
        private readonly ILookup<OrganismKey?, OrganismKey> _childrenByPrimaryKey;

        public PrimaryLineage(EngineSnapshot snapshot)
        {
            _native = snapshot.NativeCycle.HasValue;
            _robots = snapshot.Organisms.ToDictionary(KeyOf);
            _robotsByAbsoluteId = snapshot.Organisms.Where(robot => robot.UniqueId != 0).ToLookup(robot => robot.UniqueId);
            _childrenByAbsoluteId = snapshot.Organisms.Where(robot => robot.ParentAbsoluteId is > 0)
                .ToLookup(robot => robot.ParentAbsoluteId!.Value, KeyOf);
            _childrenByPrimaryKey = snapshot.Organisms.Where(robot => !_native || robot.ParentAbsoluteId is null)
                .ToLookup(robot => robot.Parents?.FirstOrDefault(), KeyOf);
        }

        public IReadOnlyList<PhylogenyLink> WholeLineage(OrganismKey focus)
        {
            if (!_robots.ContainsKey(focus)) return [];
            var ancestor = focus;
            var visited = new HashSet<OrganismKey> { focus };
            while (PrimaryParent(_robots[ancestor]) is { } parent && _robots.ContainsKey(parent) && visited.Add(parent))
                ancestor = parent;
            return DescendantLinks(ancestor, 1_000);
        }

        public IReadOnlyList<PhylogenyLink> DescendantLinks(OrganismKey focus, int maxDepth)
        {
            if (!_robots.ContainsKey(focus)) return [];
            maxDepth = Math.Clamp(maxDepth, 1, 1_000);
            var links = new List<PhylogenyLink>();
            var visited = new HashSet<OrganismKey> { focus };
            var pending = new Queue<(OrganismKey Key, int Depth)>();
            pending.Enqueue((focus, 0));
            while (pending.TryDequeue(out var current))
            {
                if (current.Depth >= maxDepth) continue;
                var children = _childrenByPrimaryKey[current.Key];
                if (_native)
                    children = children.Concat(_childrenByAbsoluteId[_robots[current.Key].UniqueId]);
                foreach (var child in children)
                {
                    if (!visited.Add(child)) continue;
                    links.Add(new(current.Key, child));
                    pending.Enqueue((child, current.Depth + 1));
                }
            }
            return links;
        }

        private OrganismKey? PrimaryParent(OrganismSnapshot robot)
        {
            if (!_native || robot.ParentAbsoluteId is not { } absoluteId)
                return robot.Parents?.FirstOrDefault();
            // VB6 parent() scans increasing slots and retains the last absolute-ID match.
            return _robotsByAbsoluteId[absoluteId].MaxBy(candidate => candidate.Slot) is { } parent ? KeyOf(parent) : null;
        }

        private static OrganismKey KeyOf(OrganismSnapshot robot) => new(robot.Slot, robot.Generation);
    }
}
