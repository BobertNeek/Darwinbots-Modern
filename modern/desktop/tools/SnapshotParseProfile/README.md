# Isolated snapshot parsing profile

Run this manual diagnostic against a preserved native `db_engine_snapshot_json`
UTF-8 payload. A managed `EngineSnapshot` JSON serialization or a simulation save
is a different format. The tool reads the input file without contacting a running
engine or desktop app.

```text
dotnet run --project modern/desktop/tools/SnapshotParseProfile -c Release --artifacts-path <temporary-build-directory> -- <native-snapshot-utf8.json> <result.json>
```

The tool checks complete projected snapshot equality, then performs five warmups
and fifteen alternating matched pairs of UTF-8 decode plus string parsing versus
direct UTF-8 parsing. It records individual elapsed times, current-thread managed
allocations, medians, input/model hashes and the loaded Core DLL hash.

It also isolates JSON-to-DTO deserialization and DTO-to-snapshot projection using
diagnostic reflection into the same parser. Those phase measurements include
type-based serializer dispatch or reflection invocation and reuse a materialized
DTO for projection. Their independent medians are not an additive decomposition
of a full parse. They do not include native serialization, native buffer copying,
engine ticks, UI work or live CPS.
