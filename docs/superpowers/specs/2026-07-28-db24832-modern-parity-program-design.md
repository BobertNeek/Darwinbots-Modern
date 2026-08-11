# Darwinbots 2.48.32 Modern Parity Program Design

**Approved direction:** 2026-07-28

## Goal

Bring Darwinbots Modern into behavioral and front-end conformance with the
public Darwinbots 2.48.32 specification while preserving the modernization
work already present in the product: a Rust simulation engine, GPU
acceleration, a typed native boundary, and an Avalonia desktop application.
Online play is excluded.

The result must feel and behave like Darwinbots 2, not merely load some legacy
DNA or display historic menu labels. Modern improvements are welcome when they
do not change simulation semantics, hide required information, or replace a
distinct historic workflow with a generic placeholder.

## Approved product boundary

- Rust remains the sole authority for simulation state and mechanics.
- The existing Avalonia application remains the desktop front end.
- GPU acceleration remains a supported execution path.
- CPU execution is the readable conformance reference for deterministic tests.
- Every accelerated gameplay phase must match the CPU reference within the
  exact integer or documented floating-point tolerance for that phase.
- C# may own presentation, window state, file pickers, and input translation.
  It must not independently implement gameplay rules.
- Local and offline workflows remain in scope. Internet Mode, FTP exchange,
  online leagues, and network helpers remain out of scope and must never make
  network calls.
- Existing user work and history are preserved. No destructive reset, broad
  cleanup, or force push is permitted.

## Canonical evidence

The implementation is governed by the public-source research package created
for DB2 2.48.32. The package will be copied into a versioned project reference
directory before parity work relies on it in tests.

| Artifact | SHA-256 |
| --- | --- |
| `DARWINBOTS_2_PRODUCT_SPEC.md` | `43146CB5D511CC046372DA797A4C07738FA3D4B90DD7BB91A9FF262CC2627832` |
| `UI_AND_INTERACTION_SPEC.md` | `CDEE2F72CF09F7259164E61B40D3535BECA1EE4217FE15C89A3DED90DE936B36` |
| `DNA_RUNTIME_SPEC.md` | `1A0C1C11E0C5DCFC41BFDFF19DD9CD35FCBE1F5292094868A4D67A6390F512D9` |
| `SYSVARS_REFERENCE.md` | `7DCC7399D97EA5B306E5611FE6433C84D923F482C9F91C34808BDB4002387701` |
| `DECLARATIVE_UI_INVENTORY.md` | `C3CE0D2684ED7BE8FB6F925C54F52E225D208CA44C2C08DBAD022EE537C60595` |
| `HISTORY_AND_SOURCES.md` | `DD7E062CE448944210D9EA564DF8D63558FFB178CCA6CCC644A8F1991EAF2CD6` |

When sources disagree, the evidence policy in the product specification
governs. The public 2.48.32 source establishes shipped behavior. The wiki
explains user intent. Existing modern tests prove only the behavior they
assert; they do not override the canonical specification.

## Architecture

### Rust simulation core

The Rust engine owns the canonical cycle, robot and species state, DNA runtime,
sysvars, physics, senses, ecology, shots, ties, reproduction, mutation,
environment objects, history, persistence models, and offline competition
state. Large behavior currently concentrated in `world.rs` will move into
focused modules as each subsystem is made conformant. `world.rs` remains the
cycle orchestrator and integration boundary.

Simulation commands are applied at deterministic cycle boundaries. A command
either validates and commits completely or returns a structured error without
partially changing the world. Published snapshots are immutable views of one
completed cycle.

### CPU and GPU execution

CPU code supplies the conformance oracle because it is inspectable and can
produce fine-grained cycle traces. GPU kernels remain first-class acceleration,
not an abandoned experiment. Each accelerated phase receives differential
fixtures that feed identical seeded state to CPU and GPU paths and compare the
gameplay-visible result. Unsupported GPU phases execute in Rust on the CPU;
the application reports the active capability instead of silently claiming
full acceleration.

Optimization follows semantic parity. A faster result that changes targeting,
collision ordering, integer normalization, reproduction, mutation, or ecology
is a defect.

### Native boundary

Avalonia sends typed commands through the existing versioned JSON C ABI. The
Rust engine validates the full command, queues it for the simulation thread,
and returns structured success or error data. Rust publishes versioned
snapshots containing every field the UI needs for rendering, inspectors,
graphs, settings state, and command enablement.

The boundary must not expose raw pointers beyond the existing owned-buffer
contract. Panic containment, schema versions, finite-number checks, and stale
organism identity checks remain mandatory.

### Avalonia desktop

Avalonia owns windows, menus, controls, keyboard and pointer input, local file
selection, accessibility, scaling, and presentation state. The interface
retains DB2's recognizable structure:

- arena-first main window;
- complete offline menu taxonomy and command state;
- transport and display toolbar;
- dense simulation telemetry;
- species-first simulation settings;
- dedicated robot, DNA, console, genealogy, mutation, graph, contest, shape,
  teleporter, and eye-design tools;
- familiar arena colors, selection cues, eye fan, ties, shots, materials,
  obstacles, teleporters, and display modes.

Faithfulness does not require a pixel-for-pixel VB6 clone. Avalonia may improve
layout scaling, typography, contrast, keyboard navigation, validation,
responsive use of space, and information grouping. It may not collapse
semantically different tools into one generic document window, remap historic
shortcuts to unrelated actions, or report a command as implemented when it is
inert.

Online commands may be omitted or shown disabled with a clear local-only
explanation. They must not occupy required shortcuts or start network work.

### Persistence and offline workflows

Modern `DB3S` saves remain supported. Compatibility adapters add the DB2 DNA,
settings, organism, snapshot, graph, and relevant simulation interchange
behaviors described by the specification. Adapters convert into explicit Rust
models before touching the live engine. Mutation-free saves transform a copy,
never the running world.

Offline leagues, contests, Simple Survival, Zerobot, Eco Survival, graphing,
recording, safemode recovery, local file teleporters, and diagnostics use
shared Rust services. The CLI and desktop invoke those services rather than
duplicating rules.

## State ownership

| State | Owner | Consumers |
| --- | --- | --- |
| World, robots, species, DNA, ties, shots, objects | Rust engine | CLI, ABI snapshots, saves |
| Mutation, costs, physics, ecology and mode policy | Rust engine | Settings UI, persistence, tests |
| GPU capability and phase receipts | Rust backend | Desktop status, CLI, differential tests |
| Offline run, round, stage and recovery state | Rust workflow services | Desktop tools, CLI, persistence |
| Selection, camera, open windows, local theme | Avalonia | Desktop only |
| File-picker result and user confirmation | Avalonia | Typed Rust command or adapter |

No gameplay-visible value has two independent writable owners.

## Delivery sequence

### Phase 0: protect and repair current work

Preserve the existing menu/settings slice on its current feature branch. Fix
the reviewed blockers before describing it as a parity checkpoint:

- mutation-free save must alter only the saved copy;
- kill-entire-organism must traverse permanent ties;
- E-Grid and Survival Info visibility must follow mode state;
- historic shortcuts must be bound without conflicting modern mappings.

Run the focused Rust, desktop-core, and rendered desktop tests, correct the
parity report, and commit the coherent checkpoint. Generated HTTP logs are not
part of the checkpoint.

### Phase 1: cycle, robot state, DNA, and sysvars

Implement the canonical cycle trace and complete robot state first. Bring
tokenization, VM operators, limits, costs, gene telemetry, sysvar lifecycle,
and command clearing into conformance. Later mechanics depend on this timing
and state model.

### Phase 2: simulation mechanisms

Deliver reviewed slices for physics and bounds, senses, shots and defenses,
ties and multibots, reproduction and genealogy, ecology and corpses, mutation
and speciation, then obstacles and local teleporters. Each slice includes CPU
conformance tests and GPU differential coverage for accelerated work it changes.

### Phase 3: persistence and offline modes

Add compatibility adapters, deterministic save/replay, mutation-free export,
organism and snapshot records, graphs, safemode recovery, leagues and contest,
Survival variants, and durable Zerobot progression. Expose the same services
through the CLI and desktop.

### Phase 4: faithful improved frontend

Build the arena-centered shell, full offline menu and shortcut contract,
toolbar and telemetry, working-copy simulation settings, dedicated tool
windows, graph family, contest surfaces, and detailed viewport interactions.
Retain modern scaling, validation, accessibility, and GPU/backend visibility.

### Phase 5: product acceptance

Map every non-online product-spec requirement and every retained UI inventory
control to code, a test or rendered workflow, and a current receipt. Run full
Rust and desktop suites, deterministic CPU replay, GPU differential gates,
legacy bot-corpus probes, persistence round trips, packaged desktop smoke tests,
and screenshot review of every major surface. Missing or indirect evidence is
not a pass.

## Supervision and thread discipline

This thread remains the controller. It owns the canonical specification,
dependency order, task ledger, integration decisions, and completion audit.

- Read-only audits may run in parallel when their scopes do not overlap.
- Only one implementation worker runs at a time because `world.rs`, the ABI,
  and desktop snapshots are shared integration surfaces.
- Every worker receives one task brief, an explicit goal, exact file scope,
  focused tests, a report path, and a commit requirement.
- Every implementation commit receives an independent specification and code
  review before the next task starts.
- Review fixes return to the original worker for up to three rounds, then use
  a fresh worker. Five rounds is the hard cap.
- The durable ledger records task status, commits, test commands, review
  verdicts, deferred minor findings, and blockers. Completed tasks are never
  re-dispatched after context compaction.
- Workers read task briefs and referenced files, not the entire program history.
  Reports live in files and worker replies remain short.

## Error handling and recovery

- Invalid DNA, settings, files, and commands return location-aware errors.
- Legacy-tolerant parsing follows DB2's specified recovery rules; it does not
  turn every malformed input into a fatal modern parser error.
- Destructive UI actions require the specified confirmation and operate on
  stable organism identities.
- Saves use temporary files, validation, and atomic replacement where the
  platform supports it. The previous valid checkpoint remains recoverable.
- GPU failure reports the failed capability and uses the configured fallback
  policy. It never corrupts or partially advances the world.
- Safemode keeps diagnostic artifacts separate from ordinary user saves.

## Verification contract

Each task starts with a failing focused test or fixture derived from the public
specification. The implementer records the exact command and output. A reviewer
checks both spec compliance and code quality from the task diff.

Subsystem completion requires:

1. canonical behavior fixtures for every changed mechanism;
2. deterministic seeded replay across save/load boundaries;
3. CPU/GPU differential tests for every changed accelerated phase;
4. ABI schema and error-path tests for every changed command or snapshot;
5. headless Avalonia interaction tests for every changed control;
6. rendered screenshots for layout, state, scaling, and visual semantics;
7. no placeholder, inert-success, unknown-command, or mislabeled behavior;
8. documentation that states measured coverage without broad parity claims.

Product completion requires a requirement-by-requirement evidence matrix for
the complete non-online product specification. Green unit tests alone cannot
prove this program complete.
