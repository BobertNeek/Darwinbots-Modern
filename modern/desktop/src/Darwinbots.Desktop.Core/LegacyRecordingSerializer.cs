using Darwinbots.Desktop.Core.LegacyMenus;

namespace Darwinbots.Desktop.Core;

public enum LegacyRecordingFormat
{
    Db2GenomeRecords,
}

public sealed record LegacyRecordingSelection(
    LegacyRecordingFormat Format,
    string Extension);

public static class LegacyRecordingSerializer
{
    public static LegacyRecordingSelection Select(LegacyCommandId command) => command switch
    {
        LegacyCommandId.RecordingSnapshotLiving => new(LegacyRecordingFormat.Db2GenomeRecords, "snp"),
        LegacyCommandId.RecordingDeadEnable => new(LegacyRecordingFormat.Db2GenomeRecords, "snp"),
        LegacyCommandId.RecordingDeadExcludeRepopulating => new(LegacyRecordingFormat.Db2GenomeRecords, "snp"),
        _ => throw new ArgumentOutOfRangeException(nameof(command), command, "Command does not select a genome recording format."),
    };

    public static string SerializeLiving(
        EngineSnapshot snapshot,
        IReadOnlyDictionary<OrganismKey, string> dnaByOrganism,
        LegacyRecordingFormat format) => format switch
        {
            LegacyRecordingFormat.Db2GenomeRecords => Db2GenomeRecordFile.EncodeLiving(snapshot, dnaByOrganism),
            _ => throw new ArgumentOutOfRangeException(nameof(format), format, "Unsupported legacy recording format."),
        };

    public static string SerializeDead(
        IEnumerable<CorpseSnapshot> corpses,
        IReadOnlyList<SpeciesSnapshot> species,
        bool includeHeader,
        LegacyRecordingFormat format) => format switch
        {
            LegacyRecordingFormat.Db2GenomeRecords => Db2GenomeRecordFile.EncodeDead(corpses, species, includeHeader),
            _ => throw new ArgumentOutOfRangeException(nameof(format), format, "Unsupported legacy recording format."),
        };
}
