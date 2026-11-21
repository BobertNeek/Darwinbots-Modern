namespace Darwinbots.Desktop.Core;

public sealed record NativeRestartSource(ulong LineageId, string Filename, string Directory);

public sealed record NativeRestartSources(bool Requested, IReadOnlyList<NativeRestartSource> Sources)
{
    public static NativeRestartSources Empty { get; } = new(false, []);
}

public sealed record NativeRestartBinding(ulong LineageId, string Dna);
