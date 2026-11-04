using Darwinbots.Desktop.Core.LegacyMenus;
using Xunit;

namespace Darwinbots.Desktop.Core.Tests;

public sealed class LegacyCommandRouterTests
{
    [Fact]
    public void RouterRejectsAnyMissingHistoricalCommand()
    {
        var handlers = CreateHandlers();
        handlers.Remove(LegacyCommandId.EditGlobalMutationRates);

        var error = Assert.Throws<ArgumentException>(() => new LegacyCommandRouter(handlers));

        Assert.Contains(nameof(LegacyCommandId.EditGlobalMutationRates), error.Message);
    }

    [Fact]
    public void RouterRejectsCommandsOutsideTheHistoricalCatalog()
    {
        var handlers = CreateHandlers();
        handlers[(LegacyCommandId)int.MaxValue] = new RecordingHandler();

        var error = Assert.Throws<ArgumentException>(() => new LegacyCommandRouter(handlers));

        Assert.Contains(int.MaxValue.ToString(), error.Message);
    }

    [Fact]
    public async Task RouterExecutesTheRegisteredHandlerAndReturnsItsResult()
    {
        var handlers = CreateHandlers();
        var handler = new RecordingHandler(
            new LegacyCommandState(true, true),
            LegacyCommandResult.Success("VISION GRID ENABLED"));
        handlers[LegacyCommandId.ViewShowVisionGrid] = handler;
        var router = new LegacyCommandRouter(handlers);

        var result = await router.ExecuteAsync(LegacyCommandId.ViewShowVisionGrid);

        Assert.Equal(1, handler.ExecutionCount);
        Assert.True(result.Succeeded);
        Assert.False(result.Canceled);
        Assert.Equal("VISION GRID ENABLED", result.Status);
        Assert.Equal(new LegacyCommandState(true, true), router.GetState(LegacyCommandId.ViewShowVisionGrid));
    }

    [Fact]
    public async Task DisabledCommandsDoNotExecuteAndExplainWhy()
    {
        var handlers = CreateHandlers();
        var handler = new RecordingHandler(new LegacyCommandState(false, false, "Select a robot first."));
        handlers[LegacyCommandId.RobotMutationRates] = handler;
        var router = new LegacyCommandRouter(handlers);

        var result = await router.ExecuteAsync(LegacyCommandId.RobotMutationRates);

        Assert.Equal(0, handler.ExecutionCount);
        Assert.False(result.Succeeded);
        Assert.False(result.Canceled);
        Assert.Equal("Select a robot first.", result.Status);
    }

    [Fact]
    public async Task HandlerExceptionsBecomeStructuredFailures()
    {
        var handlers = CreateHandlers();
        handlers[LegacyCommandId.FileSaveSimulation] = new ThrowingHandler();
        var router = new LegacyCommandRouter(handlers);

        var result = await router.ExecuteAsync(LegacyCommandId.FileSaveSimulation);

        Assert.False(result.Succeeded);
        Assert.False(result.Canceled);
        Assert.IsType<InvalidOperationException>(result.Error);
        Assert.Contains("disk unavailable", result.Status);
    }

    private static Dictionary<LegacyCommandId, ILegacyCommandHandler> CreateHandlers() =>
        LegacyMenuCatalog.FlattenLeaves().ToDictionary(
            item => item.Command!.Value,
            _ => (ILegacyCommandHandler)new RecordingHandler());

    private sealed class RecordingHandler(
        LegacyCommandState? state = null,
        LegacyCommandResult? result = null) : ILegacyCommandHandler
    {
        public int ExecutionCount { get; private set; }

        public LegacyCommandState GetState() => state ?? LegacyCommandState.Available;

        public Task<LegacyCommandResult> ExecuteAsync(CancellationToken cancellationToken = default)
        {
            cancellationToken.ThrowIfCancellationRequested();
            ExecutionCount++;
            return Task.FromResult(result ?? LegacyCommandResult.Success("EXECUTED"));
        }
    }

    private sealed class ThrowingHandler : ILegacyCommandHandler
    {
        public LegacyCommandState GetState() => LegacyCommandState.Available;

        public Task<LegacyCommandResult> ExecuteAsync(CancellationToken cancellationToken = default) =>
            throw new InvalidOperationException("disk unavailable");
    }
}
