# DB2 2.48.32 Offline Avalonia Frontend Implementation Plan

> **Execution:** Run this program through supervised Codex project tasks created from the main chat, one bounded goal and independent review at a time. Do not use subagents.

**Date:** 2026-07-30  
**Implementation base:** `ce8fed8caccc20bd1d98b1bfaff34824bf1adca7` (`ce8fed8`)  
**Goal:** Deliver a faithful but improved DB2 2.48.32 offline interface in Avalonia without copying VB6 pixels.  
**Architecture:** Avalonia owns presentation and ephemeral interaction state. Rust owns simulation truth, validation, telemetry, debug data, settings application, and backend execution.  
**Tech stack:** Avalonia/.NET desktop frontend, Rust engine and versioned JSON ABI, rendered desktop automation, deterministic screenshot evidence.

## Global constraints

- Preserve Rust gameplay authority. C# must not infer, mirror, or mutate authoritative simulation state.
- Preserve GPU acceleration and continuous CPU/GPU/backend/fallback visibility.
- Preserve Avalonia, responsive scaling, accessibility, diagnostics, and modern desktop usability.
- Keep all behavior local and offline. Exclude online play and network code. The three historical Internet commands may remain visible but must be disabled, inert, and clearly local-only.
- Faithfulness means complete offline menu behavior, arena-centered information architecture, correct settings timing, dedicated tools, rendering semantics, shortcuts, modality, and telemetry.
- The arena remains the primary workspace. Do not preserve the current dashboard/card information architecture.
- UI work that requires canonical state, sysvar lifecycle, VM telemetry, or cycle trace waits for its dependency in [the Phase 1 foundation plan](2026-07-30-db24832-phase1-foundation-implementation.md).
- Shell, layout, accessibility, and read-only projection may proceed only when they do not invent authority.

## Current baseline and limits

The independently clean Phase 0 checkpoint at `201aad7` proves only:

- Typed command routing.
- B1-B4 repairs.
- Exact retained shortcuts.
- Hidden E-Grid and Survival controls.
- Deterministic menu evidence.
- The current 109-control rendered matrix.

It does **not** close broad frontend parity. The [DB2 gap register](../../parity/db24832-gap-register.md) remains authoritative for open gaps, including the open 1500x900 dashboard evidence defect.

Existing catalog, routing, shortcut, and command-state work is the baseline for the menu phase. Do not rebuild it. Remaining menu work supplies distinct behavior, dedicated surfaces, complete state projection, and faithful interaction.

## Target architecture

| State | Authority | Frontend contract |
|---|---|---|
| World, organisms, objects, settings | Rust | Immutable, revisioned snapshots |
| Telemetry and graph samples | Rust | Cycle-stamped query snapshots |
| CPU/GPU/backend execution | Rust | Active backend, phase status, availability, fallback reason |
| Robot DNA, memory, sysvars, lineage | Rust | Stable-key inspection and cycle-trace queries |
| Settings draft | Avalonia | Working copy of one Rust settings revision |
| Camera, zoom lock, active tool | Avalonia | Ephemeral state that never changes simulation semantics |
| Focused organism | Avalonia key, Rust validation | Stale keys return a typed unavailable result |
| Simulation commands | Rust | Serialized, validated, and applied at a documented cycle boundary |
| Modal editor draft | Avalonia until commit | Closing or cancelling causes no mutation |
| Modeless tool | Avalonia window, Rust refresh | May follow focus while the simulation advances |

Use `UiCommandResult<T>(Succeeded, Code, Message, Value, AppliedTick)` for command results. Use stable organism and object keys, settings revisions, and cycle IDs across the ABI.

Settings apply modes:

- `StartNew`: validate and commit the complete draft, reset the world, then start the new simulation.
- `Change`: validate and apply only fields Rust marks mutable during a running simulation.
- `Cancel` or window close: discard the draft.
- Revision conflict: reject and offer reload. Never overwrite silently.

Window modality:

- Modal: simulation/global setting commits, detailed parameter editors, object editors, destructive confirmations, About, and first-run guidance.
- Modeless: Robot Details, Console, Gene Activations, Phylogeny, Color, RGB monitor, PlayerBot, Eye Designer, DNA editor, graphs, contest results, and help.

## Phase 0 — Keep the offline coverage contract authoritative

**Files:**

- Modify: `docs/parity/control-surface-matrix.json`
- Create or maintain: `docs/parity/db24832-offline-ui-matrix.json`
- Create or maintain: `docs/parity/db24832-offline-ui-matrix.md`
- Test: `modern/desktop/tests/Darwinbots.Desktop.Tests/OfflineUiMatrixTests.cs`
- Audit: `docs/verification/db24832-frontend/run-db24832-frontend-audit.ps1`

- [ ] Extend the existing catalog/routing baseline instead of replacing it.
- [ ] Map every retained offline inventory row to its component, state owner, shortcut, behavior test, and rendered scenario.
- [ ] Account for all 71 offline menu leaves plus three excluded online leaves that remain disabled, inert, and clearly local-only.
- [ ] Preserve the faithful disabled Internet baseline already proven at `201aad7`; never connect those historical commands to network behavior.
- [ ] Keep E-Grid hidden unless explicitly enabled and Survival Information visible only in its applicable mode.
- [ ] Fail the audit for unmapped, duplicated, inert, placeholder, or generic-proxy commands.

**Gate:** Deterministic routing evidence still passes, while every remaining row names its distinct behavior and final surface.

## Phase 1 — Extend the typed Rust/frontend projection contract

**Files:** `modern/engine/src/ui_contract.rs`, `world.rs`, `ffi.rs`, `lib.rs`; `FrontendContracts.cs`, `SimulationSettingsDocument.cs`, `EngineSnapshot.cs`, `NativeSnapshotParser.cs`, `IEngineClient.cs`, `NativeEngineClient.cs`, `SimulationSession.cs`.

- [ ] Add `TelemetrySnapshot`, `BackendExecutionSnapshot`, `SimulationSettingsSnapshot`, `RobotInspectionSnapshot`, `LineageSnapshot`, `GraphSnapshot`, and typed obstacle, teleporter, and shot visuals.
- [ ] Add `ApplySettingsAsync(document, mode, expectedRevision)`.
- [ ] Add `InspectRobotAsync(organismKey, memoryPhase)`, backed by canonical sysvar lifecycle and VM telemetry.
- [ ] Add `ReadLineageAsync(organismKey, depth)` and `ReadGraphAsync(graphId)`.
- [ ] Add `SetPlayerBotInputAsync(organismKey, input, targetTick)` with Rust timing validation.
- [ ] Reject stale revisions, stale keys, invalid ranges, and disallowed live mutations.
- [ ] Wait for the corresponding canonical-state, sysvar, VM-telemetry, and cycle-trace tasks in the Phase 1 foundation plan before wiring dependent UI.

**Gate:** ABI round-trip tests cover each type. C# contains no independently calculated gameplay value.

## Phase 2 — Replace the dashboard with an arena-first shell

**Files:** `App.axaml`, `App.axaml.cs`, `MainWindow.axaml`, `MainWindow.axaml.cs`, `Themes/Db2Theme.axaml`, `Controls/Db2ShellToolbar.axaml(.cs)`, `Controls/StatusInstrumentBar.axaml(.cs)`, `Controls/ArenaOverlayLayer.axaml`.

- [ ] Approve high-reasoning Image 2 blueprints in `docs/verification/db24832-frontend/blueprints/` before substantive visual implementation.
- [ ] Use a four-row shell: native menu, compact toolbar, arena using all flexible space, and telemetry status bar.
- [ ] Remove permanent dashboard cards and side rails. Put transient information inside arena overlays or dedicated windows.
- [ ] Open the shell first. If configuration is missing, open Simulation Settings modally over it.
- [ ] Keep the arena largest at 953x600, 1280x800, 1920x1080 at 150%, and 2560x1440 at 200%.
- [ ] Retire `SetupWindow` only after its workflow is covered by Simulation Settings.

## Phase 3 — Complete offline menus and shortcuts from the existing baseline

**Files:** existing `LegacyMenuCatalog`, menu models/views, `MainWindow`, and `Input/LegacyShortcutService.cs`.

- [ ] Keep catalog IDs, typed routing, exact retained shortcuts, and deterministic evidence already proven at `201aad7`.
- [ ] Make menus, toolbar buttons, context actions, and shortcuts invoke the same command instances.
- [ ] Project visibility, enabled state, and checked state from one command-state source.
- [ ] Replace every remaining generic document/value target with its dedicated behavior or surface.
- [ ] Preserve File shortcuts `F1`, `F2`, `F3`, `Ctrl+F3`, `F12`, `Ctrl+Q`.
- [ ] Preserve settings shortcuts `Ctrl+I`, `Ctrl+X`, `Ctrl+P`, `F5`, `Ctrl+G`, `Ctrl+F`, `F11`, `F8`.
- [ ] Preserve robot shortcuts `Ctrl+R`, `Ctrl+T`, `Ctrl+M`, `Ctrl+C`, `Ctrl+Y`, `Ctrl+Z`, `Ctrl+K`, `Ctrl+S`.
- [ ] Preserve selection shortcuts `Ctrl+F1`, `Ctrl+F2`, `Ctrl+E`, `Ctrl+B` and help shortcuts `Ctrl+D`, `Ctrl+A`.

## Phase 4 — Restore toolbar, transport, telemetry, and backend visibility

The 24 toolbar slots are: New, Load, Save, separator; Play, Single Cycle, Pause, separator; 15 CPS, Fast, Flicker, No Skins, Disable Point Effects, Video Off, separator; Insert Organism, Species Selector, Find Best, Graph Selector, separator; Snapshot Living, separator; Stealth, Ignore Errors.

- [ ] Add Species Selector, Zoom In, Zoom Out, Zoom Lock, Sun, and Contest overlays. Add no internet overlay.
- [ ] Add a trailing CPU/GPU/Fallback badge after slot 24. Collapse it to an icon before taking arena space.
- [ ] Show active backend, GPU availability, per-phase execution, and fallback reason in its tooltip/detail surface.
- [ ] Implement `Paused | Running | SingleStepPending`; Single Cycle advances exactly one complete Rust cycle.
- [ ] Ensure display speed, fast mode, flicker, and video suppression never change simulation semantics.
- [ ] Render 13 status instruments: CPS, total/performance, robot count, chloroplasts, births, cycles, elapsed time, mutation multiplier, restarts, shots this cycle, total energy, energy delta, and cost multiplier.

## Phase 5 — Complete arena rendering and interaction semantics

**Files:** `Controls/WorldViewport.cs`, `Rendering/ArenaRenderer.cs`, `ArenaPalette.cs`, `ArenaRenderOptions.cs`, `Input/ArenaCameraController.cs`, `ArenaToolController.cs`.

- [ ] Draw in order: background; field/optional E-Grid; obstacles/teleporters; ties; robot body/skin/aim/nine eyes; shots/impacts; gauges/vectors; selection/handles; offline overlays.
- [ ] Draw energy shots white, virus cyan, poison yellow, and other shots in their owner color.
- [ ] Distinguish spring and hardened ties. Project obstacle color, fill, border, visibility, and motion.
- [ ] Show teleporter direction, enabled state, and local endpoint type. Preserve background aspect and camera behavior.
- [ ] Apply RGB-memory recoloring only from authoritative Rust memory data.
- [ ] Implement `Select | Pan | InsertSpecies | PlaceShapeFirst | PlaceShapeSecond | PlaceTeleporterSource | PlaceTeleporterDestination | DeleteFeature`.
- [ ] Make Escape cancel the active tool; double-click open Robot Details; context actions use a stable organism key.
- [ ] Require an explicit move action before robot dragging. Clamp zoom and make zoom lock preserve framing.

## Phase 6 — Replace setup and advanced settings

**Files:** `ViewModels/Settings/`; `Views/Settings/SimulationSettingsWindow.axaml`; `GlobalSettingsWindow.axaml`.

- [ ] Provide Simulation tabs: Species, General, Physics & Costs, Mutations, Restart & League.
- [ ] Provide footer actions: Load, Save, Global, Start New, Change, Cancel.
- [ ] Provide Global tabs: Main, Mutations, Leagues, Evolution.
- [ ] Label each field as live, next-cycle, or new-simulation-only from Rust capability data.
- [ ] Keep local validation responsive, but make Rust the final validator.
- [ ] Explain disabled fields and show commit errors beside the affected input.
- [ ] Reflow below 900 DIPs into one scrollable column without hiding footer actions.
- [ ] Retire `SetupWindow` and `AdvancedSettingsWindow` only after migration tests pass.

## Phase 7 — Implement detailed parameter editors

**Files:** `CustomPhysicsWindow.axaml`, `SimulationCostsWindow.axaml`, `EnergyManagementWindow.axaml`, `MutationRatesWindow.axaml`, `RestrictionOptionsWindow.axaml`.

- [ ] Give each editor a modal working copy, explicit units, valid ranges, documented defaults, and synchronized sliders/numeric inputs.
- [ ] Block invalid commits and preserve the source revision until Rust accepts the edit.
- [ ] Surface physics, costs, energy/weather, mutation rates, and restriction fields without collapsing them into generic key/value forms.

## Phase 8 — Implement local object editors

**Files:** `Darwinbots.Desktop.Core/Objects/ObjectDrafts.cs`, `Views/Objects/ShapeEditorWindow.axaml`, `LocalTeleporterEditorWindow.axaml`.

- [ ] Edit shape position, size, color, fill, border, visibility, movement vector, behavior, and stable identity.
- [ ] Edit teleporter source, destination, direction, enabled state, and local inbound/outbound/intrasimulation type.
- [ ] Exclude internet teleporter types and all network fields.
- [ ] Provide placement preview, cursor feedback, Escape cancellation, Rust validation, and confirmation for destructive bulk removal.

## Phase 9 — Replace proxy robot windows with dedicated tools

**Files:** `Services/ToolWindowRegistry.cs`, `ViewModels/Tools/FocusedRobotToolViewModel.cs`, dedicated views for Robot Details, Console, Gene Activations, Phylogeny, Robot Color, RGB Memory Monitor, PlayerBot, Eye Designer, and the DNA editor.

- [ ] Keep tools modeless. Let unpinned tools follow focus and pinned tools retain a stable organism key.
- [ ] Replace stale content with an unavailable state when the organism disappears.
- [ ] Show Robot Details facts plus DNA and memory before/after execution.
- [ ] Make Console a domain debugger, never a host shell.
- [ ] Show current gene activation, Rust lineage, RGB memory mapping, and authoritative eye parameters.
- [ ] Validate PlayerBot input and target-cycle timing in Rust.

## Phase 10 — Restore graphs, contest results, and offline mode surfaces

**Files:** `Darwinbots.Desktop.Core/Graphs/GraphCatalog.cs`, `Services/GraphWindowManager.cs`, `Controls/Db2GraphControl.cs`, `Views/Graphs/GraphWindow.axaml`, `Views/Modes/ContestResultsWindow.axaml`.

- [ ] Implement Population; Average Mutations; Mutations/DNA Length; Average Age; Average Descendants; Average Energy; Average DNA Length; Average DNA Conditions; Total Energy; Dynamic Cost Statistics; Species Diversity; Average Chloroplasts; Selective Genetic Distance; Generational Distance; Slow Genetic Distance; Custom 1, 2, and 3.
- [ ] Support multiple modeless graph windows, update/pause, reset, data list, and export.
- [ ] Source samples and timestamps from Rust.
- [ ] Provide five contest result rows and manual winner entry.
- [ ] Show Survival Information only while its offline mode is active.

## Phase 11 — Finish support, persistence, accessibility, and scaling

**Files:** `Services/DialogService.cs`, `Views/Help/DnaHelpWindow.axaml`, `AboutWindow.axaml`, `FirstRunWindow.axaml`, `Controls/ArenaNotice.axaml`, `Assets/Help/`, app tray integration.

- [ ] Complete living/dead organism snapshots, save/load progress, safe-mode messaging, and destructive confirmations.
- [ ] Add buoyancy, video-off, and PlayerBot arena notices. Add no internet notice.
- [ ] Implement tray-based Stealth restore, first-run guidance, About, and DNA help.
- [ ] Restore focus after dialogs and announce validation and backend fallback changes.
- [ ] Add Automation IDs/names, ordered keyboard traversal, visible focus, non-color cues, high contrast, reduced motion, and locale-safe numeric parsing.
- [ ] Verify 953x600/100%, 1280x800/100%, 1920x1080/150%, and 2560x1440/200% with the arena always dominant.

## Phase 12 — Produce final deterministic evidence

**Files:** `docs/verification/db24832-frontend/manifest.json`, `goldens/`, `actual/`, and `run-db24832-frontend-audit.ps1`.

- [ ] Run Rust engine tests and both desktop test projects.
- [ ] Run packaged desktop smoke checks in explicit CPU, available GPU, and forced-fallback modes.
- [ ] Record viewport, DPI, theme, seed, cycle, backend, and active command for each capture.
- [ ] Reject placeholders, generic proxies, unintended inert handlers, hidden fallback, active online/network behavior, and screenshots of static mockups.

## Rendered scenarios and screenshot gates

### R01 — Arena shell

Capture paused and running shells at 953x600 and 1440x900, including the open 1500x900 dashboard defect replacement evidence.

### R02 — Menus and shortcuts

Open every offline menu; show exact shortcuts, synchronized check states, disabled-state explanations, hidden E-Grid, and the three disabled historical Internet commands.

### R03 — Robot rendering semantics

Use one selected robot scene showing body, skin, aim, nine eyes, gauges, ties, all shot colors, impacts, and vectors.

### R04 — Arena tools

Capture pan, zoom lock, species insertion, shape placement, local teleporter placement, preview cursors, and cancellation.

### R05 — Simulation Settings

Capture all five tabs plus clean, dirty, invalid, Start New, Change, revision-conflict, and Cancel states.

### R06 — Global and detailed settings

Capture all four Global tabs and the five detailed modal parameter editors with validation and timing labels.

### R07 — Dedicated robot tools

Capture every modeless robot tool refreshing while simulation advances, including pinned, focus-following, and unavailable states.

### R08 — Graphs, contest, and telemetry

Capture multiple graph windows, contest results, all 18 graph choices, and all 13 live status instruments.

### R09 — Support and persistence

Capture help, first run, tray restore, offline overlays, save progress, safe-mode notice, and confirmations.

### R10 — Scaling and accessibility

Capture 100%, 150%, and 200% scaling, keyboard-only navigation, visible focus, high contrast, and narrow settings reflow.

Screenshot acceptance:

- Use a deterministic seed and fixed cycle for comparable captures.
- Allow masks only for explicitly documented dynamic text.
- Require no clipping, overlap, inaccessible off-screen controls, or unintended truncation.
- Require visual difference at or below 1% outside approved masks.
- Require semantic sign-off against the approved blueprint and canonical reference behavior.
- Treat screenshots as proof only of displayed state; interaction tests must prove the route into each state.

## Acceptance matrix

| Area | Acceptance requirement | Evidence |
|---|---|---|
| Rust authority | No mutable gameplay truth or duplicate calculation in C# | ABI, revision, lifecycle, and authority tests |
| Foundation dependencies | Canonical state, sysvars, VM telemetry, and cycle trace precede dependent UI | Phase 1 foundation task receipts |
| Offline menus | 71 offline leaves have distinct behavior; three historical online leaves remain disabled, inert, and local-only | Coverage matrix and dispatch tests |
| Shortcuts | Every retained shortcut invokes its menu command | Keyboard automation |
| Shell | Arena-first layout, exact 24 slots, and 13 status instruments | R01, R02, R08 |
| Backend visibility | CPU/GPU/fallback and per-phase details remain inspectable | CPU, GPU, forced-fallback smoke evidence |
| Settings | Five simulation tabs, four global tabs, correct working-copy timing | Contract tests, R05, R06 |
| Detailed editors | Physics, costs, energy, mutations, restrictions use dedicated modal tools | R06 and commit/discard tests |
| Objects | Shapes and local teleporters support preview, validation, and stable identity | R04 and Rust command tests |
| Robot tools | All nine dedicated tools have live, pinned, and stale-key behavior | R07 and inspection tests |
| Rendering | Robots, eyes, ties, shots, impacts, objects, gauges, vectors, backgrounds match semantics | Seeded renderer tests and R03 |
| Interaction | Focus, pan, zoom, insertion, placement, context actions, and cancellation work | Input-state tests and R04 |
| Modality | Commit editors are modal; inspectors, graphs, contest, and help are modeless | Window-owner tests and R07 |
| Graphs/contest | All 18 graph families, multi-window controls, export, and contest results work | Graph tests and R08 |
| Persistence/support | Save/load, snapshots, tray, help, onboarding, progress, and confirmations work | R09 and restart smoke tests |
| Accessibility/scaling | Keyboard, automation metadata, focus, contrast, reflow, and DPI gates pass | Accessibility automation and R10 |
| Final parity | No placeholder, generic proxy, unintended inert command, active online/network behavior, silent fallback, or open 1500x900 dashboard defect remains | Full audit and signed screenshot manifest |
