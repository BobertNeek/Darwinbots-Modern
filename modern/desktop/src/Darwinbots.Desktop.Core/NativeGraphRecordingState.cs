namespace Darwinbots.Desktop.Core;

public sealed record NativeGraphRecordingState(string RunStamp, IReadOnlyList<NativeGraphRecordingRow> Graphs);

public sealed record NativeGraphRecordingRow(int GraphIndex, bool Enabled, int Counter);
