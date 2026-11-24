namespace Darwinbots.Desktop.Core;

public sealed class SafeModeArchiveSchedule
{
    private const int HoursBetweenArchives = 6;
    private readonly TimeProvider _clock;
    private DateTime _lastBoundary;

    public SafeModeArchiveSchedule(TimeProvider clock)
    {
        _clock = clock ?? throw new ArgumentNullException(nameof(clock));
        _lastBoundary = CurrentBoundary(_clock.GetLocalNow().DateTime);
    }

    public (DateTime? DueBoundary, TimeSpan UntilNextArchive) Poll()
    {
        var now = _clock.GetLocalNow().DateTime;
        var boundary = CurrentBoundary(now);
        DateTime? due = null;
        if (boundary > _lastBoundary)
        {
            _lastBoundary = boundary;
            due = boundary;
        }
        return (due, _lastBoundary.AddHours(HoursBetweenArchives) - now);
    }

    private static DateTime CurrentBoundary(DateTime now) =>
        now.Date.AddHours(now.Hour / HoursBetweenArchives * HoursBetweenArchives);
}
