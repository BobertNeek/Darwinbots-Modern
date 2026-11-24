namespace Darwinbots.Desktop.Core;

public sealed record NativeGraphLayoutState(
    IReadOnlyList<NativeGraphLayoutRow> Graphs,
    IReadOnlyList<string> CustomQueries);

// Indices follow the original graph order. Positions are signed VB6 twips.
public sealed record NativeGraphLayoutRow(
    int GraphIndex,
    bool? Visible = null,
    int? Left = null,
    int? Top = null);

public sealed record NativeGraphCustomQuery(int Slot, string Expression);
