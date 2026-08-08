using System.Globalization;
using System.Text;

namespace Darwinbots.Desktop.Core;

public sealed record LegacyGraphRobot(
    string Name,
    uint Color,
    IReadOnlyList<double> Values);

public sealed record LegacyGraphSegment(
    string Title,
    IReadOnlyList<LegacyGraphRobot> Robots);

public static class LegacyGraphJoin
{
    public const int SamplesPerSegment = 999;
    public const int SegmentTickWidth = 1_000;

    public static IReadOnlyList<LegacyGraphSegment> Parse(IEnumerable<string> segmentFiles)
    {
        ArgumentNullException.ThrowIfNull(segmentFiles);
        var segments = new List<LegacyGraphSegment>();
        foreach (var content in segmentFiles)
        {
            ArgumentNullException.ThrowIfNull(content);
            var lines = content.Replace("\r\n", "\n", StringComparison.Ordinal)
                .Replace('\r', '\n')
                .Split('\n');
            if (lines.Length < SamplesPerSegment + 2)
                throw new InvalidDataException("Legacy graph save is missing data rows.");

            var title = lines[0].Trim();
            if (title.Length == 0) throw new InvalidDataException("Legacy graph save has no title.");
            var robots = ParseRobots(lines[1]);
            if (robots.Count == 0) throw new InvalidDataException("Legacy graph save has no series.");

            var values = robots.Select(_ => new List<double>(SamplesPerSegment)).ToArray();
            for (var row = 0; row < SamplesPerSegment; row++)
            {
                var fields = lines[row + 2].Split('\t', StringSplitOptions.None);
                if (fields.Length < robots.Count)
                    throw new InvalidDataException($"Legacy graph save row {row} is missing series values.");
                for (var column = 0; column < robots.Count; column++)
                    values[column].Add(ParseValue(fields[column], row, column));
            }

            segments.Add(new LegacyGraphSegment(
                title,
                robots.Select((robot, index) => robot with { Values = values[index] }).ToArray()));
        }
        return segments;
    }

    public static IReadOnlyList<GraphSeriesViewModel> ToSeries(
        IReadOnlyList<LegacyGraphSegment> segments)
    {
        ArgumentNullException.ThrowIfNull(segments);
        return segments.SelectMany((segment, segmentIndex) => segment.Robots.Select((robot, robotIndex) =>
            new GraphSeriesViewModel(
                $"legacy-{segmentIndex + 1}-{robotIndex + 1}",
                GraphMetric.CustomizableGraph1,
                $"{segment.Title} · {robot.Name}",
                robot.Values.Select((value, sampleIndex) => new GraphSample(
                    (ulong)(segmentIndex * SegmentTickWidth + sampleIndex), value)).ToArray()))).ToArray();
    }

    private static IReadOnlyList<LegacyGraphRobot> ParseRobots(string line)
    {
        return line.Split(',', StringSplitOptions.RemoveEmptyEntries)
            .Select((value, index) =>
            {
                var separator = value.IndexOf(':');
                if (separator <= 0 || separator == value.Length - 1)
                    throw new InvalidDataException($"Legacy graph series {index} is malformed.");
                var colorText = value[..separator].Trim();
                if (!int.TryParse(colorText, NumberStyles.Integer, CultureInfo.InvariantCulture, out var color))
                    throw new InvalidDataException($"Legacy graph series {index} has an invalid color.");
                var name = value[(separator + 1)..].Trim();
                return new LegacyGraphRobot(name, unchecked((uint)color), []);
            }).ToArray();
    }

    private static double ParseValue(string text, int row, int column)
    {
        if (double.TryParse(text.Trim(), NumberStyles.Float, CultureInfo.InvariantCulture, out var value)
            && double.IsFinite(value)) return value;
        throw new InvalidDataException($"Legacy graph value at row {row}, series {column} is invalid.");
    }
}

public static class DatabaseExport
{
    private const string Header =
        "Rob id,Parent id,Founder name,Generation,Birth cycle,Age,Mutations,New mutations,Dna length,Offspring number,kills,Fitness,Energy,Chloroplasts";

    public static string Serialize(EngineSnapshot snapshot)
    {
        ArgumentNullException.ThrowIfNull(snapshot);
        var builder = new StringBuilder(Header).Append("\r\n");
        foreach (var organism in snapshot.Organisms.Where(value => value.DnaLength > 1))
        {
            var parent = organism.Parents?.FirstOrDefault()?.Slot ?? 0;
            var founder = organism.Species < snapshot.Species.Count
                ? snapshot.Species[(int)organism.Species].Name
                : $"Robot-{organism.Slot}";
            var energy = organism.Energy + organism.Body * 10L;
            var fitness = Math.Max(0L, energy);
            var fields = new object[]
            {
                organism.UniqueId == 0 ? organism.Slot : organism.UniqueId,
                parent,
                founder,
                organism.Generation,
                0UL,
                organism.Age,
                organism.TotalMutations,
                organism.NewMutations,
                organism.DnaLength,
                organism.Offspring,
                0,
                fitness,
                energy,
                organism.Chloroplasts,
            };
            builder.Append(string.Join(',', fields.Select(value => Escape(Convert.ToString(value, CultureInfo.InvariantCulture) ?? string.Empty))))
                .Append("\r\n");
        }
        return builder.ToString();
    }

    private static string Escape(string value) =>
        value.IndexOfAny([',', '"', '\r', '\n']) < 0
            ? value
            : $"\"{value.Replace("\"", "\"\"", StringComparison.Ordinal)}\"";
}
