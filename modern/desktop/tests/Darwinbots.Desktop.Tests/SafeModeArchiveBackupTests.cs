using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.VisualTree;
using Darwinbots.Desktop.Services;
using Darwinbots.Desktop.Views;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class SafeModeArchiveBackupTests
{
    [Fact]
    public async Task DisposableProfileArchivesTheLatestCheckpointOnceAndKeepsItOutOfRecovery()
    {
        var profile = Directory.CreateTempSubdirectory("db3-safemode-archive-").FullName;
        try
        {
            var storage = new AvaloniaDesktopStorageService(profile);
            var boundary = new DateTime(2026, 11, 20, 6, 0, 0);
            Assert.Null(await storage.ArchiveLatestAutosaveAsync(boundary, CancellationToken.None));

            var autosaves = Directory.CreateDirectory(Path.Combine(profile, "Autosaves")).FullName;
            var older = Path.Combine(autosaves, "old.db3s");
            var latest = Path.Combine(autosaves, "latest.db3s");
            var checkpoint = new byte[] { 68, 66, 51, 83, 3, 0, 5, 4 };
            await File.WriteAllBytesAsync(older, [1]);
            File.SetLastWriteTimeUtc(older, DateTime.UnixEpoch.AddDays(1));
            await File.WriteAllBytesAsync(latest, checkpoint);
            File.SetLastWriteTimeUtc(latest, DateTime.UnixEpoch.AddDays(2));

            var archives = await Task.WhenAll(
                storage.ArchiveLatestAutosaveAsync(boundary, CancellationToken.None),
                storage.ArchiveLatestAutosaveAsync(boundary, CancellationToken.None));
            var expected = Path.Combine(autosaves, "Backups", "autosave-20261120-060000.db3s");
            Assert.All(archives, path => Assert.Equal(expected, path));
            Assert.Equal(checkpoint, await File.ReadAllBytesAsync(expected));
            Assert.Equal(latest, storage.FindLatestAutosave());

            var newer = Path.Combine(autosaves, "newer.db3s");
            await File.WriteAllBytesAsync(newer, [9]);
            File.SetLastWriteTimeUtc(newer, DateTime.UnixEpoch.AddDays(3));
            Assert.Equal(expected, await storage.ArchiveLatestAutosaveAsync(boundary, CancellationToken.None));
            Assert.Equal(checkpoint, await File.ReadAllBytesAsync(expected));
            Assert.Equal(newer, storage.FindLatestAutosave());
            await Assert.ThrowsAnyAsync<OperationCanceledException>(() =>
                storage.ArchiveLatestAutosaveAsync(boundary.AddHours(6), new CancellationToken(true)));
            Assert.Equal(expected, Assert.Single(Directory.GetFiles(Path.GetDirectoryName(expected)!)));
        }
        finally { Directory.Delete(profile, recursive: true); }
    }

    [Fact]
    public async Task ClosingTheViewerCancelsItsInFlightBoundaryCopy()
    {
        await DesktopTestApplication.Session.Dispatch(async () =>
        {
            var clock = new ArchiveClock { UtcNow = new(2026, 11, 20, 5, 59, 58, TimeSpan.Zero) };
            var copying = new TaskCompletionSource<DateTime>(TaskCreationOptions.RunContinuationsAsynchronously);
            var cancelled = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
            var storage = new AuditStorageService
            {
                ArchiveAutosave = async (boundary, cancellation) =>
                {
                    copying.TrySetResult(boundary);
                    try { await Task.Delay(Timeout.InfiniteTimeSpan, cancellation); }
                    catch (OperationCanceledException)
                    {
                        cancelled.TrySetResult();
                        throw;
                    }
                    return null;
                },
            };
            var window = new SafeModeBackupWindow(storage, clock);
            try
            {
                window.Show();
                Assert.Contains(window.GetVisualDescendants().OfType<TextBlock>(), label => label.Text == "Next backup in 2 seconds");
                Assert.False(copying.Task.IsCompleted);
                clock.UtcNow = clock.UtcNow.AddSeconds(2);
                Assert.Equal(new DateTime(2026, 11, 20, 6, 0, 0), await copying.Task.WaitAsync(TimeSpan.FromSeconds(8)));

                var close = Assert.Single(window.GetVisualDescendants().OfType<Button>(), button => Equals(button.Content, "CLOSE"));
                close.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                await cancelled.Task.WaitAsync(TimeSpan.FromSeconds(8));
                Assert.False(window.IsVisible);
            }
            finally { if (window.IsVisible) window.Close(); }
        }, CancellationToken.None);
    }

    private sealed class ArchiveClock : TimeProvider
    {
        public DateTimeOffset UtcNow { get; set; }
        public override DateTimeOffset GetUtcNow() => UtcNow;
        public override TimeZoneInfo LocalTimeZone => TimeZoneInfo.Utc;
    }
}
