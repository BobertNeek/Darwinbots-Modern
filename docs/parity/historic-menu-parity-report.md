# DB2 2.48.32 Historical Menu Phase 0 Checkpoint

Last verified: 2026-07-30

## Scope and result

This is a measured Phase 0 checkpoint for the local/offline Darwinbots Modern desktop. It is not a claim of complete DB2 behavioral parity. Registering 74 historical leaf commands in a typed catalog, routing those entries, or obtaining green unit-test counts does not prove that each command reproduces DB2 behavior.

The checkpoint covers the branch history through `95818f6af58bcb2365ec21f6a49e707ae15ed343`. Rust remains the simulation authority, GPU acceleration remains available, and Avalonia remains the desktop shell. Online play is excluded from this program.

## Behaviors evidenced at this checkpoint

- Typed `LegacyMenuCommand` values route catalog entries to explicit production handlers and enabled-state policies.
- Live mutation settings queue authoritative engine commands for mutation enabled state, the global multiplier, and per-species rates.
- `Save Sim Without Mutations` makes a copy-only save transform. The saved copy has mutation disabled without changing the running world.
- `Kill Entire Organism` captures the selected permanent-tie component by stable `slot:generation` identity and removes the whole captured group atomically.
- A successful native `remove_many` command publishes exactly one state update for the batch.
- Native command execution is verified off the caller thread, and the native command interface accepts `remove_many` JSON.
- The end-to-end organism picker JSON contains only the captured tied group and ties whose two endpoints are inside that group.
- E-Grid is absent from the default 2.48.32 menu surface. Survival Info remains hidden unless a relevant mode exposes it.
- Every retained catalog shortcut string is parsed as its exact Avalonia input gesture. The shortcut tests cover the retained F-key, Ctrl-key, and Robot-command accelerators rather than synthetic menu clicks.
- `Internet Enabled`, `Internet Teleporter`, and `Internet Options` are disabled and inert. Interaction tests verify that they perform no network work.
- Current view toggles change the modern viewport state. Current object tools cover shot lifetime, ties, obstacle placement/removal, teleporter placement, and maze actions within their documented modern limits.

## Open limits

- Several distinct DB2 tools still use generic document or single-value windows.
- Species, general, physics/costs, and mutation settings remain collapsed into broader modern dialogs.
- The dashboard shell, toolbar, status instrument, graphs, and modal/nonmodal behavior differ from DB2.
- Viewport semantics remain incomplete, including legacy gauges, eyes, object properties, cursor modes, and zoom behavior.
- Engine mechanics, runtime ordering, DNA/VM behavior, persistence/offline workflows, and CPU/GPU differentials remain open in the gap register.
- No successful black-box run of the exact original 2.48.32 GUI is available. Source catalogs and modern rendered tests cannot replace that evidence.

## Verification receipts

All commands ran serially from `D:\Darwinbots 2\.worktrees\db24832-parity`.

| Check | Result |
|---|---|
| Reference package | 6/6 verified |
| Selected Rust engine tests | 23 passed; 0 failed |
| Desktop core tests | 62 passed; 0 failed; 0 skipped |
| Rendered desktop interaction tests | 26 passed; 0 failed; 0 skipped |
| Control-surface matrix | 109/109 passing rows validated |
| Historical catalog registration | 74 typed leaf commands; registration is not behavioral parity |
| Excluded online commands | 3 disabled commands; no-network behavior is interaction-test evidence |

## Rendered evidence

- `docs/verification/control-surface-audit/modern/runtime-initial.png` — 1500 x 900 pixels
- `docs/verification/control-surface-audit/modern/runtime-exercised.png` — 1500 x 900 pixels

Visual inspection: both captures show readable File, Edit, View, Robot, Objects, Internet, Recording, and Help labels; E-Grid is absent. Toolbar and status labels remain readable without overlap. The left and right rails and bottom dashboard do not cover the central arena. The current layout is a three-column dashboard with a scrollable settings rail, central arena, scrollable organism inspector, and bottom Population, Energy, and Event Log panels. Below-fold rail content uses scrollbars; no clipping or overlap defect is visible in these captures.

The screenshots are generated, ignored evidence and are not committed. Disabled Internet behavior is attributed to the rendered interaction test unless a screenshot visibly proves it.
