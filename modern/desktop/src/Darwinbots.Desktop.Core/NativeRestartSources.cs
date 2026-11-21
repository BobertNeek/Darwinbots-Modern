using System.Text.Json.Serialization;

namespace Darwinbots.Desktop.Core;

public sealed record NativeRestartSource(ulong LineageId, string Filename, string Directory);

public sealed record NativeRestartSources(bool Requested, IReadOnlyList<NativeRestartSource> Sources)
{
    public static NativeRestartSources Empty { get; } = new(false, []);
}

public sealed record NativeRestartBinding(
    [property: JsonPropertyName("lineage_id")] ulong LineageId,
    [property: JsonPropertyName("dna")] string Dna,
    [property: JsonPropertyName("selected_directory"), JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    string? SelectedDirectory = null);

public sealed class MissingNativeRepopulationSourceException(string message, NativeRestartSource source)
    : InvalidOperationException(message)
{
    public NativeRestartSource RequiredSource { get; } = source;
}
