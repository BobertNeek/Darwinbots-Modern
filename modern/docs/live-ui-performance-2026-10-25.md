# Selected bot overlays, runtime controls and CPU transaction performance

This change addresses Cassidy's October 25 report of per-bot detail labels,
duplicated runtime controls, and very slow simulation. It is based on completion
commit `0225101343367cbe297d4d23afda598cfa9f679a`.

## Behavior

- Resource gauges and movement vectors use the selected `slot:generation` key.
  No selection, a hidden bot, a disabled overlay, or a reused slot with a different
  generation produces no detail gauges. Projection creates three gauges for the
  selected bot instead of three for every visible organism.
- One horizontally scrollable runtime toolbar retains the distinct controls from
  both former strips. The menu bar remains available.
- Organisms and selected gauges are conservatively culled against viewport-local
  bounds before allocating drawing resources. Partially visible bodies and gauges
  remain visible at viewport edges after zooming and panning.
- CPU command batches fork an independent in-memory engine candidate instead of
  encoding and decoding the entire world before every batch. Validation,
  response construction, and commit ordering remain atomic. GPU command batches
  retain their previous transaction path.

The in-memory candidate retains the continuous inspector session's activation
history. DB3S deliberately does not persist activation bitmap, activation history,
or debug text. An actual load starts a new inspection history. The packaged smoke
test now checks this distinction explicitly: a continuous 10+3-cycle session has
13 frames, a loaded session has the matching last 3 frames, every other compared
snapshot field remains equal, and complete persistent save bytes are equal. The
test uses a fixed clock so timer-derived native inputs are reproducible. No
history is discarded at ordinary command boundaries to make a comparison pass.

## Measurements

The [compact receipt](receipts/live-ui-performance-2026-10-25.json) records samples,
input/output hashes and package hashes. The local private fixture and raw logs
were preserved; user world/configuration files are not included in this change.

Both native measurements loaded the same 21,375,585-byte DB3S fixture with a fixed
host clock. It starts with 2,132 organisms: 51 Alga Minimalis and 2,081
Fruit_Fliesv0.21. It was recreated from startup settings and DNA, **not recovered
from the user's evolved screenshot world**. Three successive ticks end at
populations 2,305, 2,330 and 2,338 in both runs. Final saves are byte-identical,
SHA-256 `652058D7314B29F92EAC6F7C27765EBB6EDBAA0A912BE7E2DF08CEE04B9115AC`.

| Measurement | Previous package | Corrected package |
| --- | ---: | ---: |
| Median native command tick, 3 samples | 5,410.2052 ms | 235.1099 ms |
| Headless viewport render/capture median, 7 samples | 488.1233 ms | 21.4097 ms |
| UI-thread allocation per render/capture median | 55,404,272 bytes | 571,112 bytes |
| Projected resource gauges, selected bot | 6,396 | 3 |

The native tick result is about 23 times faster on this fixture. It excludes the
subsequent snapshot JSON generation, UTF-8 transfer and managed parsing. Candidate
snapshot generation took about 49–57 ms and parsing about 252–484 ms. Those costs
remain. Phase diagnostics cover only part of each cycle and were not treated as a
complete attribution.

The viewport comparison used the identical initial snapshot, a 1280×720 headless
Skia viewport at 9.32× zoom, selected bot 817:0, three warmups and seven measured
renders. It measures CPU render/capture work, **not native live FPS or GPU speed**.

One normal tick is one simulation cycle. Displayed CPS is cycles advanced over
the most recent publication interval, including command, snapshot, parse and
scheduling time; it is not a long-term average. Turbo and Graphics Off share the
same flag. They hide the viewport and request adaptive batches, but still publish
and parse snapshots. Maximum speed targets 32-ms batches; Turbo targets 100 ms.
Both can reduce to a one-cycle batch when one update already exceeds the target.
The separate 15-CPS limiter changes the timer interval to 67 ms.

## Verification and limits

- 20 CPU FFI regressions passed: candidate independence, current inspection retention,
  fixed-clock serialized-reference state equality, late edit/tick failure
  rollback, actual-load inspection boundary, and existing native cache/raw-source
  transaction tests.
- 121 selected native integration and rendered UI checks passed with the new CPU
  DLL, including settings, DNA storage, robot files, startup, community leagues,
  toolbar controls and selected overlays. Earlier focused checks also exercised
  stable-generation selection and eight viewport-edge/zoom cases.
- Published Windows package smoke passed movement, persistent save-byte equality,
  continuous/resumed inspection-history checks and bundled bot availability.
- All 33 declared runtime assets and 48 bundled bot files are present; .NET 10.0.13
  is installed on the verification host.

The user later reported about 2.3 CPS with 180 organisms and 430 shots. That is a
different world from the benchmark. A read-only desktop/process check at 20:31 UTC
found no Darwinbots process, so that observation could not be profiled or assigned
to a loaded DLL at that time. No conclusion about that world's bottleneck or its
Turbo state is inferred from the screenshot alone.

No original Darwinbots performance baseline, exact evolved-world comparison,
GPU benchmark, or native graphics walkthrough of this final package was performed.
Cross-platform hosted CI is separate from the local evidence and must be checked
at the final commit. Existing worlds, profiles and other applications were not
closed, replaced, or modified by these checks. The corrected engine is delivered
in a separate package; replacing a running session requires preserving it first.
