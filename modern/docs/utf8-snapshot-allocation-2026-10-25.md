# UTF-8 snapshot allocation qualification

`NativeEngineClient.Snapshot` now passes its existing managed byte copy directly
to a `ReadOnlySpan<byte>` parser overload. The string API remains available. Both
APIs use the same deserialization options and the same unchanged projection of
DTO fields into `EngineSnapshot`. The native buffer copy and `finally` release
remain in place.

The change removes an intermediate UTF-16 string from the snapshot path. All
inspection, history, memory, resource and settings fields remain materialized.
The new byte API rejects malformed JSON and invalid UTF-8; the existing string
API retains its validation behavior.

## Matched measurements

The [receipt](receipts/utf8-snapshot-allocation-2026-10-25.json) contains every sample,
input/output hashes and package hashes. The
[manual profiling tool](../desktop/tools/SnapshotParseProfile/README.md) accepts a
preserved native UTF-8 snapshot without connecting to the live application.

The input is one immutable 14,133,580-byte native JSON payload, SHA-256
`C8F627BB53673F94A650A8C08C0C6F44385E433CC0DBFF807DAF5F4CA735C512`.
It represents scheduling tick 3 and 2,338 organisms after advancing the previously recreated
2,132-organism CPU fixture. It is not the user's evolved screenshot world. Raw
snapshot bytes and fixture files remain local.

Each run used five warmups and fifteen matched pairs, alternating execution order.
The first two used a separate build of the same Core source. The final run used
the exact candidate package Core DLL.

| Run | Decode + string parse median | Direct UTF-8 parse median |
| --- | ---: | ---: |
| Initial | 309.7685 ms | 310.8773 ms |
| Repeat | 288.1811 ms | 265.7694 ms |
| Exact packaged Core | 288.4622 ms | 272.6598 ms |

Managed allocations consistently fell from approximately **86.39 MB to 58.12 MB**
per parse: **28.27 MB fewer bytes, about 32.7%** of the isolated old parser path.
This excludes the unchanged native-to-managed byte copy and native serialization.
Complete projected snapshots were byte-equal in every run, SHA-256
`DF05F45360D43F94515581AD937B2B4936653E0E67940BFFC8D520AE0D2FD2F5`.

Elapsed-time results are mixed. The retained qualification is the repeatable
allocation reduction; a reliable latency improvement or live CPS increase has
not been established.

Separate diagnostic phase measurements put JSON deserialization at roughly
241–276 ms and DTO projection at roughly 26–28 ms. Deserialization allocated about
44.31 MB and projection about 13.81 MB. The defaults allocated then overwritten
inside projection are a subset of that projection work; their specific cost was
not isolated. Projection was not rewritten. The phase diagnostics include
type-based serializer dispatch or reflection and their independent medians must
not be added as an exact full-parse decomposition.

## Verification

- 36 focused parser tests passed. New cases compare entire snapshots for raw
  Unicode, inspection history, memory, settings, missing/null/default fields and
  case-insensitive names; malformed inputs are rejected. Mutating the source
  byte array after parsing leaves the materialized snapshot unchanged.
- All 325 managed core tests passed against the exact packaged Core DLL.
- The full rendered suite passed against the exact packaged DLLs: 233 passed,
  zero failed, and one opt-in profiling test skipped as intended.
- The separate package smoke passed movement, persistent save-byte equality,
  actual-load inspection-history clearing and continuous-session retention.

The native engine DLL is unchanged from the earlier CPU transaction qualification.
This trial made no live-app actions and did not replace the user's running package.
Hosted cross-platform CI must be assessed at the new commit separately from these
local checks. Live CPS, the user's evolved worlds and GPU performance were not
measured.
