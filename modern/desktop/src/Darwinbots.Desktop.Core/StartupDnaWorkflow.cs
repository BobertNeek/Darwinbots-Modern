namespace Darwinbots.Desktop.Core;

public sealed record StartupDnaImport(string Path, float[] Position);

public sealed class StartupDnaWorkflow
{
    private readonly float _worldWidth;
    private readonly float _worldHeight;
    private readonly string? _legacyMainDirectory;
    private readonly string _applicationBaseDirectory;

    public StartupDnaWorkflow(
        float worldWidth,
        float worldHeight,
        string? legacyMainDirectory = null,
        string? applicationBaseDirectory = null)
    {
        _worldWidth = worldWidth;
        _worldHeight = worldHeight;
        _legacyMainDirectory = string.IsNullOrWhiteSpace(legacyMainDirectory)
            ? null
            : Path.GetFullPath(legacyMainDirectory);
        _applicationBaseDirectory = Path.GetFullPath(applicationBaseDirectory ?? AppContext.BaseDirectory);
    }

    public IReadOnlyList<StartupDnaImport> Resolve(IReadOnlyList<string> arguments)
    {
        ArgumentNullException.ThrowIfNull(arguments);

        var requested = new List<string>();
        for (var index = 0; index < arguments.Count; index++)
        {
            if (string.Equals(arguments[index], "--bot", StringComparison.OrdinalIgnoreCase)
                && index + 1 < arguments.Count
                && !string.IsNullOrWhiteSpace(arguments[index + 1]))
            {
                requested.Add(arguments[++index]);
            }
        }

        return requested
            .Select(path => new StartupDnaImport(ResolvePath(path), [_worldWidth / 2f, _worldHeight / 2f]))
            .ToArray();
    }

    private string ResolvePath(string requestedPath)
    {
        var requested = Path.GetFullPath(requestedPath);
        if (File.Exists(requested)) return requested;

        var fileName = Path.GetFileName(requestedPath);
        if (string.IsNullOrWhiteSpace(fileName)) return requested;

        if (_legacyMainDirectory is not null)
        {
            var legacyPath = Path.Combine(_legacyMainDirectory, "Robots", fileName);
            if (File.Exists(legacyPath)) return legacyPath;
        }

        var bundledPath = Path.Combine(_applicationBaseDirectory, "Bots", fileName);
        return File.Exists(bundledPath) ? bundledPath : requested;
    }
}
