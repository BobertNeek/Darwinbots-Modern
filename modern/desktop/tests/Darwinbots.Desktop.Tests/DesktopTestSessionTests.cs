using Avalonia.Threading;
using Xunit;

namespace Darwinbots.Desktop.Tests;

public sealed class DesktopTestSessionTests
{
    [Fact]
    public async Task TaskOnlyCallbacksStayInTheUiSessionUntilTheirWorkCompletes()
    {
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var finished = false;
        Func<Task> callback = async () =>
        {
            entered.SetResult();
            await release.Task;
            Dispatcher.UIThread.VerifyAccess();
            finished = true;
        };
        var operation = DesktopTestApplication.Session.Dispatch(callback, CancellationToken.None);
        try
        {
            await entered.Task.WaitAsync(TimeSpan.FromSeconds(10));
            await Assert.ThrowsAsync<TimeoutException>(() => operation.WaitAsync(TimeSpan.FromMilliseconds(50)));
        }
        finally
        {
            release.TrySetResult();
        }
        await operation.WaitAsync(TimeSpan.FromSeconds(10));
        Assert.True(finished);
    }

    [Fact]
    public async Task ExceptionsAfterAnAwaitRemainTestFailures()
    {
        var expected = new InvalidOperationException("UI callback failure after await");
        Func<Task> callback = async () => { await Task.Yield(); throw expected; };
        var actual = await Assert.ThrowsAsync<InvalidOperationException>(
            () => DesktopTestApplication.Session.Dispatch(callback, CancellationToken.None));
        Assert.Same(expected, actual);
    }
}
