namespace Darwinbots.Desktop.Core;

public sealed record Db2RobotFile(string Dna, MutationProfileUpdate? MutationProfile);

public static class Db2RobotFileStorage
{
    public static async Task<Db2RobotFile> ReadAsync(
        string path, CancellationToken cancellationToken = default)
    {
        var (dnaPath, _) = Paths(path);
        cancellationToken.ThrowIfCancellationRequested();
        var bytes = await File.ReadAllBytesAsync(dnaPath, cancellationToken);
        var dna = LegacyDnaText.Decode(bytes);
        var profile = await ReadMutationProfileAsync(dnaPath, cancellationToken);
        return new(dna, profile);
    }

    public static async Task<MutationProfileUpdate?> ReadMutationProfileAsync(
        string dnaPath, CancellationToken cancellationToken = default)
    {
        var (_, sidecarPath) = Paths(dnaPath);
        cancellationToken.ThrowIfCancellationRequested();
        sidecarPath = ResolveSidecar(sidecarPath);
        if (Directory.Exists(sidecarPath))
            throw new IOException("The robot mutation-rate sidecar is a directory.");
        byte[] bytes;
        try { bytes = await File.ReadAllBytesAsync(sidecarPath, cancellationToken); }
        catch (FileNotFoundException) { return null; }
        catch (DirectoryNotFoundException) { return null; }
        return Db2MutationRateCodec.Decode(bytes);
    }

    public static async Task WriteAsync(
        string path, string dna, MutationProfileUpdate? profile = null,
        CancellationToken cancellationToken = default)
    {
        var (dnaPath, sidecarPath) = Paths(path);
        cancellationToken.ThrowIfCancellationRequested();
        var dnaBytes = LegacyDnaText.Encode(dna);
        var profileBytes = profile is null ? null : Db2MutationRateCodec.Encode(profile);
        if (profileBytes is not null) sidecarPath = ResolveSidecar(sidecarPath);
        EnsureDistinct(dnaPath, sidecarPath);

        // Validate both named targets before creating even a staging file.
        var writes = new List<(string Path, byte[] Bytes, bool Existed)>
        {
            (dnaPath, dnaBytes, ValidateTarget(dnaPath)),
        };
        if (profileBytes is not null)
            writes.Add((sidecarPath, profileBytes, ValidateTarget(sidecarPath)));

        var staged = new Dictionary<string, string>(StringComparer.Ordinal);
        var backups = new Dictionary<string, string>(StringComparer.Ordinal);
        var ownedTemps = new List<string>();
        var ownedBackups = new List<string>();
        var committed = new List<string>();
        var rollbackErrors = new List<Exception>();
        try
        {
            foreach (var write in writes)
            {
                cancellationToken.ThrowIfCancellationRequested();
                var temporary = SiblingPath(write.Path, "tmp");
                await using (var stream = OpenOwned(temporary, ownedTemps))
                {
                    await stream.WriteAsync(write.Bytes, cancellationToken);
                    await stream.FlushAsync(cancellationToken);
                }
                staged.Add(write.Path, temporary);
            }
            foreach (var write in writes.Where(write => write.Existed))
            {
                cancellationToken.ThrowIfCancellationRequested();
                var backup = SiblingPath(write.Path, "backup");
                await using (var source = new FileStream(write.Path, FileMode.Open, FileAccess.Read,
                    FileShare.Read, 4096, FileOptions.Asynchronous))
                await using (var destination = OpenOwned(backup, ownedBackups))
                {
                    await source.CopyToAsync(destination, cancellationToken);
                    await destination.FlushAsync(cancellationToken);
                }
                backups.Add(write.Path, backup);
            }
            foreach (var write in writes)
            {
                cancellationToken.ThrowIfCancellationRequested();
                // A newly occupied target must not be overwritten without a backup.
                File.Move(staged[write.Path], write.Path, overwrite: write.Existed);
                ownedTemps.Remove(staged[write.Path]);
                committed.Add(write.Path);
            }
        }
        catch (Exception failure)
        {
            // Restore in reverse order without cancellation interrupting recovery.
            foreach (var target in committed.AsEnumerable().Reverse())
            {
                try
                {
                    if (backups.TryGetValue(target, out var backup))
                    {
                        File.Move(backup, target, overwrite: true);
                        ownedBackups.Remove(backup);
                    }
                    else File.Delete(target);
                }
                catch (Exception recoveryFailure)
                {
                    rollbackErrors.Add(recoveryFailure);
                }
            }
            if (rollbackErrors.Count > 0)
            {
                failure.Data["Db2RobotFileStorage.RecoveryBackups"] =
                    ownedBackups.Where(File.Exists).ToArray();
                failure.Data["Db2RobotFileStorage.RollbackFailures"] = rollbackErrors.ToArray();
            }
            throw;
        }
        finally
        {
            foreach (var temporary in ownedTemps) TryDelete(temporary);
            // Keep recoverable old bytes if any rollback operation failed.
            if (rollbackErrors.Count == 0)
                foreach (var backup in ownedBackups) TryDelete(backup);
        }
    }

    private static (string Dna, string Sidecar) Paths(string path)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(path);
        var dna = Path.GetFullPath(path);
        if (Path.GetFileName(dna).Length == 0)
            throw new ArgumentException("The robot DNA path must name a file.", nameof(path));
        var sidecar = Path.ChangeExtension(dna, ".mrate");
        EnsureDistinct(dna, sidecar);
        return (dna, sidecar);
    }

    private static void EnsureDistinct(string dna, string sidecar)
    {
        if (StringComparer.OrdinalIgnoreCase.Equals(dna, sidecar))
            throw new ArgumentException("Robot DNA and mutation-rate sidecar paths must be distinct.");
    }

    private static string ResolveSidecar(string path)
    {
        var directory = Path.GetDirectoryName(path)!;
        if (!Directory.Exists(directory)) return path;
        var name = Path.GetFileName(path);
        var matches = Directory.EnumerateFileSystemEntries(directory)
            .Where(entry => string.Equals(Path.GetFileName(entry), name, StringComparison.OrdinalIgnoreCase))
            .Take(2).ToArray();
        if (matches.Length > 1)
            throw new IOException("The robot has ambiguous mutation-rate sidecars differing only by case.");
        return matches.Length == 1 ? matches[0] : path;
    }

    private static bool ValidateTarget(string path)
    {
        FileAttributes attributes;
        try { attributes = File.GetAttributes(path); }
        catch (FileNotFoundException) { return false; }
        catch (DirectoryNotFoundException) { return false; }
        if ((attributes & (FileAttributes.Directory | FileAttributes.ReparsePoint)) != 0)
            throw new IOException("Robot file targets must be regular files, not directories or links.");
        return true;
    }

    private static string SiblingPath(string path, string suffix) =>
        Path.Combine(Path.GetDirectoryName(path)!,
            $".{Path.GetFileName(path)}.{Guid.NewGuid():N}.db2-{suffix}");

    private static FileStream OpenOwned(string path, List<string> owned)
    {
        var stream = new FileStream(path, FileMode.CreateNew, FileAccess.Write, FileShare.None,
            4096, FileOptions.Asynchronous);
        owned.Add(path);
        return stream;
    }

    private static void TryDelete(string path)
    {
        try { File.Delete(path); }
        catch { /* Keep the original failure, or leave an owned backup recoverable. */ }
    }
}
