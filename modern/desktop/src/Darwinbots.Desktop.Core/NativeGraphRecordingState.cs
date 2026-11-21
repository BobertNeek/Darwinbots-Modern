namespace Darwinbots.Desktop.Core;

public sealed record NativeGraphRecordingState(string RunStamp, IReadOnlyList<NativeGraphRecordingRow> Graphs);

public sealed record NativeGraphRecordingRow(int GraphIndex, bool Enabled, int Counter);

public sealed record NativeGraphRecordingSample(
    ulong Tick,
    int NativeCycle,
    IReadOnlyList<SpeciesGraphSampleSnapshot> SpeciesGraphs,
    IReadOnlyList<double> DynamicCostSeries);
