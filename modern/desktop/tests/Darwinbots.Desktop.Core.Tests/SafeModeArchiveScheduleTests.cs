using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class SafeModeArchiveScheduleTests
{
    [Fact]
    public void PollUsesLocalSixHourBoundariesAndDoesNotRepeatAfterAClockCorrection()
    {
        var clock = new ArchiveClock { UtcNow = new(2026, 11, 20, 2, 59, 57, TimeSpan.Zero) };
        var schedule = new SafeModeArchiveSchedule(clock);

        var before = schedule.Poll();
        Assert.Null(before.DueBoundary);
        Assert.Equal(TimeSpan.FromSeconds(3), before.UntilNextArchive);

        clock.UtcNow = clock.UtcNow.AddSeconds(3);
        var first = schedule.Poll();
        Assert.Equal(new DateTime(2026, 11, 20, 6, 0, 0), first.DueBoundary);
        Assert.Equal(TimeSpan.FromHours(6), first.UntilNextArchive);
        Assert.Null(schedule.Poll().DueBoundary);

        clock.UtcNow = clock.UtcNow.AddMinutes(-5);
        Assert.Null(schedule.Poll().DueBoundary);
        clock.UtcNow = clock.UtcNow.AddMinutes(6);
        Assert.Null(schedule.Poll().DueBoundary);

        clock.UtcNow = clock.UtcNow.AddHours(12);
        Assert.Equal(new DateTime(2026, 11, 20, 18, 0, 0), schedule.Poll().DueBoundary);
        Assert.Null(schedule.Poll().DueBoundary);
    }

    private sealed class ArchiveClock : TimeProvider
    {
        public DateTimeOffset UtcNow { get; set; }
        public override DateTimeOffset GetUtcNow() => UtcNow;
        public override TimeZoneInfo LocalTimeZone { get; } = TimeZoneInfo.CreateCustomTimeZone(
            "Archive test time", TimeSpan.FromHours(3), "Archive test time", "Archive test time");
    }
}
