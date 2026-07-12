namespace Darwinbots.Desktop.Core;

public sealed record EngineCapabilities(
    int Version,
    string ActiveBackend,
    bool GpuAvailable,
    string? FallbackReason);
