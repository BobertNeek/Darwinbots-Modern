using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Threading;
using Darwinbots.Desktop.Core;
using Darwinbots.Desktop.Services;

namespace Darwinbots.Desktop.Views;

public sealed class SafeModeBackupWindow : Window
{
    private readonly IDesktopStorageService _storage;
    private readonly SafeModeArchiveSchedule _schedule;
    private readonly DispatcherTimer _timer = new() { Interval = TimeSpan.FromSeconds(1) };
    private readonly CancellationTokenSource _lifetime = new();
    private readonly TextBlock _countdown = new() { FontWeight = FontWeight.SemiBold };
    private readonly TextBlock _status = new() { TextWrapping = TextWrapping.Wrap };
    private bool _copying;

    public SafeModeBackupWindow(IDesktopStorageService storage, TimeProvider? clock = null)
    {
        _storage = storage ?? throw new ArgumentNullException(nameof(storage));
        _schedule = new SafeModeArchiveSchedule(clock ?? TimeProvider.System);
        Title = "Darwinbots — Safemode Backup";
        Width = 440;
        SizeToContent = SizeToContent.Height;
        CanResize = false;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        var close = new Button { Content = "CLOSE", HorizontalAlignment = HorizontalAlignment.Right };
        close.Click += (_, _) => Close();
        Content = new StackPanel
        {
            Margin = new Thickness(18),
            Spacing = 10,
            Children =
            {
                _countdown,
                new TextBlock
                {
                    Text = "Copies the latest autosave into the Backups folder every six hours while this window is open.",
                    TextWrapping = TextWrapping.Wrap,
                },
                _status,
                close,
            },
        };
        _timer.Tick += async (_, _) => await PollAsync();
        Opened += async (_, _) =>
        {
            _timer.Start();
            await PollAsync();
        };
        Closed += (_, _) =>
        {
            _timer.Stop();
            _lifetime.Cancel();
            _lifetime.Dispose();
        };
    }

    private async Task PollAsync()
    {
        if (_copying || _lifetime.IsCancellationRequested) return;
        var (due, untilNext) = _schedule.Poll();
        _countdown.Text = $"Next backup in {Math.Ceiling(untilNext.TotalSeconds):N0} seconds";
        if (due is not { } boundary) return;

        var cancellation = _lifetime.Token;
        _copying = true;
        try
        {
            var archived = await _storage.ArchiveLatestAutosaveAsync(boundary, cancellation);
            if (!cancellation.IsCancellationRequested)
                _status.Text = archived is null ? "No autosave was available to back up." : $"Saved {Path.GetFileName(archived)}";
        }
        catch (OperationCanceledException) when (cancellation.IsCancellationRequested) { }
        catch (Exception error)
        {
            if (!cancellation.IsCancellationRequested) _status.Text = $"BACKUP FAILED · {error.Message}";
        }
        finally { _copying = false; }
    }
}
