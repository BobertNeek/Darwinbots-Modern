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
}
