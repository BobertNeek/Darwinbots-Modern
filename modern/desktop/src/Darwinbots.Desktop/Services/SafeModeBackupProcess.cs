using System.Diagnostics;

namespace Darwinbots.Desktop.Services;

internal static class SafeModeBackupProcess
{
    internal const string StartupSwitch = "--safemode-backup-viewer";

    internal static void Start(string autosaveDirectory)
    {
        var processPath = Environment.ProcessPath
            ?? throw new InvalidOperationException("The Darwinbots executable could not be located.");
        using var process = Process.Start(CreateStartInfo(processPath, typeof(App).Assembly.Location, autosaveDirectory))
            ?? throw new InvalidOperationException("The safemode backup viewer could not be started.");
    }

    private static ProcessStartInfo CreateStartInfo(string processPath, string assemblyPath, string autosaveDirectory)
    {
        var start = new ProcessStartInfo(processPath) { UseShellExecute = false, CreateNoWindow = true };
        if (Path.GetFileNameWithoutExtension(processPath).Equals("dotnet", StringComparison.OrdinalIgnoreCase))
            start.ArgumentList.Add(assemblyPath);
        start.ArgumentList.Add(StartupSwitch);
        start.Environment[AvaloniaDesktopStorageService.AutosaveDirectoryVariable] = Path.GetFullPath(autosaveDirectory);
        return start;
    }
}
