# DB2 Menu and Settings Checkpoint Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the preserved July 23 historic-menu and live-settings work into an honest, tested DB2 2.48.32 checkpoint by fixing mutation-free saving, organism-wide deletion, hidden states, and historic shortcuts.

**Architecture:** Rust remains authoritative for saved mutation state and atomic organism removal. Avalonia routes typed commands through `SimulationSession` and the versioned C ABI, then renders catalog visibility and hotkeys. The public research package is versioned in-repository so tests and reviews use stable requirements.

**Tech Stack:** Rust 1.96, serde/serde_json, versioned C ABI, .NET 10, C#, Avalonia, xUnit, PowerShell verification.

## Global Constraints

- `docs/superpowers/specs/2026-07-28-db24832-modern-parity-program-design.md` governs architecture and delivery.
- The canonical behavior baseline is public Darwinbots 2.48.32; do not use the repository's local VB6 tree as authority.
- Online play, FTP, and network helpers are excluded. Existing Internet commands remain inert and make no network calls.
- Rust owns every gameplay-visible mutation and organism operation. C# owns presentation and input translation only.
- Preserve GPU acceleration and the existing CPU/GPU selection behavior. These tasks must not remove or bypass either backend.
- Save-without-mutations transforms a serialized copy. It must not change the live engine.
- Multi-organism deletion validates the full stable-ID set before changing any organism and publishes one resulting snapshot.
- Historic shortcuts must match `UI_AND_INTERACTION_SPEC.md` section 4. Modern shortcuts may remain only when they do not conflict.
- E-Grid is hidden in 2.48.32. Survival Info is hidden until a relevant offline mode explicitly exposes it.
- Stage only task-owned files. Preserve the primary checkout and its untracked HTTP logs.
- Each task ends with focused tests, a self-review, and one commit. No parity claim may exceed measured evidence.

---

### Task 1: Version the canonical public specification package

**Files:**
- Create: `docs/reference/db24832/DARWINBOTS_2_PRODUCT_SPEC.md`
- Create: `docs/reference/db24832/UI_AND_INTERACTION_SPEC.md`
- Create: `docs/reference/db24832/DNA_RUNTIME_SPEC.md`
- Create: `docs/reference/db24832/SYSVARS_REFERENCE.md`
- Create: `docs/reference/db24832/DECLARATIVE_UI_INVENTORY.md`
- Create: `docs/reference/db24832/HISTORY_AND_SOURCES.md`
- Create: `docs/reference/db24832/SOURCE_MANIFEST.md`
- Create: `docs/reference/db24832/verify.ps1`

**Interfaces:**
- Consumes: the immutable research package at `C:\Users\PC\Documents\Codex\2026-07-28\use-deep-research-to-become-a`.
- Produces: a repository-local, hash-verified specification root used by every later task brief and review.

- [ ] **Step 1: Write the failing package verifier**

Create `verify.ps1` with this exact contract:

```powershell
$ErrorActionPreference = 'Stop'
$expected = [ordered]@{
    'DARWINBOTS_2_PRODUCT_SPEC.md' = '43146CB5D511CC046372DA797A4C07738FA3D4B90DD7BB91A9FF262CC2627832'
    'UI_AND_INTERACTION_SPEC.md' = 'CDEE2F72CF09F7259164E61B40D3535BECA1EE4217FE15C89A3DED90DE936B36'
    'DNA_RUNTIME_SPEC.md' = '1A0C1C11E0C5DCFC41BFDFF19DD9CD35FCBE1F5292094868A4D67A6390F512D9'
    'SYSVARS_REFERENCE.md' = '7DCC7399D97EA5B306E5611FE6433C84D923F482C9F91C34808BDB4002387701'
    'DECLARATIVE_UI_INVENTORY.md' = 'C3CE0D2684ED7BE8FB6F925C54F52E225D208CA44C2C08DBAD022EE537C60595'
    'HISTORY_AND_SOURCES.md' = 'DD7E062CE448944210D9EA564DF8D63558FFB178CCA6CCC644A8F1991EAF2CD6'
}
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
foreach ($entry in $expected.GetEnumerator()) {
    $path = Join-Path $root $entry.Key
    if (-not (Test-Path -LiteralPath $path)) { throw "Missing canonical artifact: $($entry.Key)" }
    $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
    if ($actual -ne $entry.Value) { throw "Hash mismatch for $($entry.Key): $actual" }
}
Write-Output "DB2 2.48.32 reference package verified: $($expected.Count)/$($expected.Count)"
```

- [ ] **Step 2: Run the verifier and confirm it fails before the package is copied**

Run: `pwsh -NoProfile -File docs/reference/db24832/verify.ps1`

Expected: FAIL with `Missing canonical artifact: DARWINBOTS_2_PRODUCT_SPEC.md`.

- [ ] **Step 3: Copy the six canonical files without rewriting them**

Use exact `Copy-Item -LiteralPath ... -Destination ...` operations. Copy the product spec from the research root. Copy the other five files from its `reference` directory. Do not normalize line endings or edit their contents.

- [ ] **Step 4: Add the source manifest**

`SOURCE_MANIFEST.md` must name the six files, the hashes above, the public source commit `467a8a0c1564db2071cf65efe39dcfefc5ab7a0c`, the research date `2026-07-28`, and this policy: public 2.48.32 source establishes shipped behavior; the wiki explains user intent; repository-local legacy source is not canonical for this program.

- [ ] **Step 5: Verify the package**

Run: `pwsh -NoProfile -File docs/reference/db24832/verify.ps1`

Expected: `DB2 2.48.32 reference package verified: 6/6`.

- [ ] **Step 6: Commit**

```powershell
git add docs/reference/db24832
git commit -m "docs: version DB2 2.48.32 parity specification"
```

---

### Task 2: Implement authoritative mutation-free saves

**Files:**
- Modify: `modern/engine/src/persistence.rs`
- Modify: `modern/engine/src/ffi.rs`
- Modify: `modern/engine/tests/contracts.rs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/IEngineClient.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/NativeEngineClient.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/SimulationSession.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Views/MainWindow.axaml.cs`
- Modify: `modern/desktop/tests/Darwinbots.Desktop.Core.Tests/SimulationSessionTests.cs`
- Modify: `modern/desktop/tests/Darwinbots.Desktop.Tests/RuntimeControlSurfaceTests.cs`
- Modify: `modern/desktop/tests/Darwinbots.Desktop.Tests/RenderedControlSurfaceAuditTests.cs`

**Interfaces:**
- Consumes: existing `SaveFile::encode`, `db_engine_save`, `IEngineClient.Save()`, and `SimulationSession.SaveAsync()`.
- Produces: `SaveFile::encode_without_mutations(&Engine)`, native export `db_engine_save_without_mutations`, `IEngineClient.SaveWithoutMutations()`, and `SimulationSession.SaveWithoutMutationsAsync()`.

- [ ] **Step 1: Add a failing Rust copy-isolation test**

Add this behavior to `contracts.rs` using the existing default species:

```rust
#[test]
fn mutation_free_save_disables_saved_mutations_without_changing_live_engine() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine.update_mutation_settings(true, 2.5, &[(darwinbots_engine::SpeciesId(0), 3.5)]).unwrap();

    let bytes = SaveFile::encode_without_mutations(&engine).unwrap();
    let restored = SaveFile::decode(&bytes).unwrap();

    assert!(!restored.snapshot().mutations_enabled);
    assert_eq!(restored.snapshot().global_mutation_multiplier, 0.0);
    assert!(restored.snapshot().species.iter().all(|species| species.mutation_rate == 0.0));
    assert!(engine.snapshot().mutations_enabled);
    assert_eq!(engine.snapshot().global_mutation_multiplier, 2.5);
    assert_eq!(engine.snapshot().species[0].mutation_rate, 3.5);
}
```

- [ ] **Step 2: Run the Rust test and confirm the API is absent**

Run: `cargo test -p darwinbots-engine --test contracts mutation_free_save_disables_saved_mutations_without_changing_live_engine -- --exact`

Expected: compile failure because `encode_without_mutations` does not exist.

- [ ] **Step 3: Implement copy-only mutation suppression in `SaveFile`**

Keep `encode(&Engine)` unchanged for normal saves. Add `encode_without_mutations(&Engine)` that serializes the engine to a `serde_json::Value`, requires a top-level object, sets `mutations_enabled` to `false`, sets `global_mutation_multiplier` to `0.0`, and sets every serialized species `mutation_rate` to `0.0`. Frame the transformed payload through the same `DB3S` version-2 header helper used by ordinary saves. Return `EngineError::Invariant` if the serialized engine is not an object or its `species` field is not an array. Never mutate the supplied `Engine`.

The public signatures are:

```rust
pub fn encode(engine: &Engine) -> Result<Vec<u8>, EngineError>;
pub fn encode_without_mutations(engine: &Engine) -> Result<Vec<u8>, EngineError>;
```

- [ ] **Step 4: Add the native and managed save path**

Add a panic-contained native export parallel to `db_engine_save`:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn db_engine_save_without_mutations(
    engine: *const Engine,
    out_buffer: *mut DbBuffer,
) -> DbStatus;
```

It must apply the same null and non-empty-output-buffer checks as `db_engine_save` and call `SaveFile::encode_without_mutations`.

Add these managed signatures and implementations:

```csharp
byte[] SaveWithoutMutations();
public async Task<byte[]> SaveWithoutMutationsAsync(CancellationToken cancellationToken = default);
```

`NativeEngineClient.SaveWithoutMutations()` calls the new entrypoint and frees its owned buffer in `finally`. Update every test fake implementing `IEngineClient`; return a distinct byte sequence and record that the mutation-free path was used.

- [ ] **Step 5: Route the historic menu command to the new path**

Change `SaveSimulationCommandAsync` to accept `bool withoutMutations`. Normal save passes `false`; `FileSaveWithoutMutations` passes `true`. Select bytes with:

```csharp
var bytes = withoutMutations
    ? await _session.SaveWithoutMutationsAsync()
    : await _session.SaveAsync();
```

Do not change the live snapshot or settings dialog after saving.

- [ ] **Step 6: Add managed and rendered regressions**

In `SimulationSessionTests`, assert that `SaveWithoutMutationsAsync` runs on the single worker thread, returns the fake's distinct bytes, and does not call ordinary save. In `RuntimeControlSurfaceTests`, click `FileSaveWithoutMutations`, wait for the recording engine's mutation-free save marker, and assert the injected storage service received those distinct bytes.

- [ ] **Step 7: Run focused tests**

```powershell
cargo test -p darwinbots-engine --test contracts mutation_free_save_disables_saved_mutations_without_changing_live_engine -- --exact
dotnet test modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj --no-restore --filter FullyQualifiedName~SaveWithoutMutations
dotnet test modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj --no-restore --filter FullyQualifiedName~SaveWithoutMutations
```

Expected: all focused tests pass.

- [ ] **Step 8: Commit**

```powershell
git add modern/engine/src/persistence.rs modern/engine/src/ffi.rs modern/engine/tests/contracts.rs modern/desktop
git commit -m "fix: save mutation-free simulation copies"
```

---

### Task 3: Delete whole tied organisms atomically

**Files:**
- Modify: `modern/engine/src/world.rs`
- Modify: `modern/engine/src/ffi.rs`
- Modify: `modern/engine/tests/manual_tools.rs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/IEngineClient.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/NativeEngineClient.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/SimulationSession.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Views/MainWindow.axaml.cs`
- Modify: `modern/desktop/tests/Darwinbots.Desktop.Core.Tests/SimulationSessionTests.cs`
- Modify: `modern/desktop/tests/Darwinbots.Desktop.Tests/RuntimeControlSurfaceTests.cs`
- Modify: `modern/desktop/tests/Darwinbots.Desktop.Tests/RenderedControlSurfaceAuditTests.cs`

**Interfaces:**
- Consumes: stable `OrganismId`, current tie snapshots, `Engine::remove`, and `SelectedOrganismGroup()`.
- Produces: atomic `Engine::remove_many(&[OrganismId])`, ABI command `remove_many`, `IEngineClient.RemoveMany`, and `SimulationSession.RemoveManyAsync`.

- [ ] **Step 1: Add failing atomic-removal tests**

Add two focused cases to `manual_tools.rs`:

```rust
#[test]
fn remove_many_rejects_the_whole_batch_before_mutating_any_slot() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let stale = engine.spawn(LegacyDna::parse("start\nstop").unwrap()).unwrap();
    engine.remove(stale).unwrap();
    let valid = engine.spawn(LegacyDna::parse("start\nstop").unwrap()).unwrap();
    assert!(engine.remove_many(&[valid, stale]).is_err());
    assert_eq!(engine.snapshot().organisms.len(), 1);
}

#[test]
fn remove_many_publishes_one_world_without_the_validated_batch() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let first = engine.spawn(LegacyDna::parse("start\nstop").unwrap()).unwrap();
    let second = engine.spawn(LegacyDna::parse("start\nstop").unwrap()).unwrap();
    engine.remove_many(&[first, second]).unwrap();
    assert!(engine.snapshot().organisms.is_empty());
}
```

- [ ] **Step 2: Run the Rust tests and confirm the API is absent**

Run: `cargo test -p darwinbots-engine --test manual_tools remove_many -- --nocapture`

Expected: compile failure because `remove_many` does not exist.

- [ ] **Step 3: Implement one validated Rust mutation**

Add:

```rust
pub fn remove_many(&mut self, ids: &[OrganismId]) -> Result<(), EngineError>;
```

Reject an empty slice and duplicate IDs with `EngineError::Invariant`. Validate every ID through `valid_slot` before changing state. Then clear each slot, increment its generation, deactivate kinematics/lifecycle, reset biology and forced reproduction, add its slot once to `free_slots`, remove ties that reference any deleted ID, and publish exactly once. Change `remove(id)` to delegate to `remove_many(&[id])`.

- [ ] **Step 4: Expose `remove_many` through the native boundary**

Add a deserializable `{ slot, generation }` command value and:

```rust
RemoveMany { organisms: Vec<OrganismIdCommand> }
```

Convert every item to `OrganismId` and call `engine.remove_many(&ids)`. Keep the existing single `remove` command for compatibility.

Add managed APIs:

```csharp
void RemoveMany(IReadOnlyList<OrganismKey> organisms);
public Task RemoveManyAsync(IReadOnlyList<OrganismKey> organisms, CancellationToken cancellationToken = default);
```

Reject null or empty lists before serialization. Serialize one `remove_many` command containing stable slot/generation objects. Update all test fakes.

- [ ] **Step 5: Route organism deletion and filter organism export ties**

Add `KillSelectedOrganismAsync()`. Capture `SelectedOrganismGroup()` once, convert it to `OrganismKey[]`, call `_session.RemoveManyAsync(keys)`, clear selection, and report the exact deleted count. Keep `RobotKill` on the single-robot path; route `RobotKillEntireOrganism` to the new method.

In `SaveEntireOrganismAsync`, filter serialized ties so both endpoints belong to the captured member-key set. Do not write ties to outside robots.

- [ ] **Step 6: Add managed and rendered regressions**

In `SimulationSessionTests`, assert one `RemoveManyAsync` call preserves the list and runs on the dedicated engine thread. In `RuntimeControlSurfaceTests`, use a snapshot containing two tied members and one unrelated robot, invoke `RobotKillEntireOrganism`, then assert one recorded `RemoveMany` call contains exactly the tied pair. Add an export test proving outside ties are omitted.

- [ ] **Step 7: Run focused tests**

```powershell
cargo test -p darwinbots-engine --test manual_tools remove_many -- --nocapture
dotnet test modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj --no-restore --filter FullyQualifiedName~RemoveMany
dotnet test modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj --no-restore --filter "FullyQualifiedName~KillEntireOrganism|FullyQualifiedName~SaveEntireOrganism"
```

Expected: all focused tests pass.

- [ ] **Step 8: Commit**

```powershell
git add modern/engine/src/world.rs modern/engine/src/ffi.rs modern/engine/tests/manual_tools.rs modern/desktop
git commit -m "fix: apply whole-organism commands atomically"
```

---

### Task 4: Enforce historic visibility and keyboard contracts

**Files:**
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/LegacyMenus/LegacyCommandState.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/LegacyMenus/LegacyMenuItem.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/LegacyMenus/LegacyMenuCatalog.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Controls/LegacyMenuBar.axaml.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Views/MainWindow.axaml.cs`
- Modify: `modern/desktop/tests/Darwinbots.Desktop.Core.Tests/LegacyMenuCatalogTests.cs`
- Modify: `modern/desktop/tests/Darwinbots.Desktop.Core.Tests/LegacyCommandRouterTests.cs`
- Modify: `modern/desktop/tests/Darwinbots.Desktop.Tests/LegacyMenuBarTests.cs`
- Modify: `modern/desktop/tests/Darwinbots.Desktop.Tests/RuntimeControlSurfaceTests.cs`

**Interfaces:**
- Consumes: catalog `Shortcut`, `LegacyCommandState`, Avalonia `MenuItem.HotKey`, and current window key handling.
- Produces: static top-level visibility, dynamic leaf visibility, exact retained historic gestures, and conflict-free modern additions.

- [ ] **Step 1: Add failing catalog and menu tests**

Assert these exact shortcut strings in `LegacyMenuCatalogTests`:

```text
FileNewSimulation=F1; FileLoadSimulation=F2; FileSaveSimulation=F3;
FileSaveWithoutMutations=Ctrl+F3; FilePauseSimulation=F12; FileQuit=Ctrl+Q;
EditInsertOrganism=Ctrl+I; EditSpecies=Ctrl+X; EditGeneralSettings=Ctrl+P;
EditPhysicsAndCosts=F5; EditGlobalMutationRates=Ctrl+G;
EditRestartAndLeagues=Ctrl+F; EditEnablePlayerBot=F11;
EditRestrictionOverwrites=F8; RobotShowInfo=Ctrl+R;
RobotShowPhilogeny=Ctrl+T; RobotMutationRates=Ctrl+M;
RobotChangeColor=Ctrl+C; RobotOpenConsole=Ctrl+Y;
RobotViewGeneActivations=Ctrl+Z; RobotKill=Ctrl+K;
RobotSaveDna=Ctrl+S; RobotSelectEntireOrganism=Ctrl+F1;
RobotSaveEntireOrganism=Ctrl+F2; RobotKillEntireOrganism=Ctrl+E;
RobotFindBest=Ctrl+B; HelpDna=Ctrl+D; HelpAbout=Ctrl+A.
```

Assert the E-Grid top-level model is hidden by default. In `LegacyMenuBarTests`, assert hidden top-level items are not visible, every non-null shortcut becomes the expected `KeyGesture`, and a command state with `Visible=false` hides its leaf. In `RuntimeControlSurfaceTests`, assert Survival Info is hidden in the ordinary setup and that F5/F8/Ctrl+S/Ctrl+E dispatch the historic commands rather than modern transport/save/edit actions.

- [ ] **Step 2: Run the tests and confirm visibility/hotkeys fail**

```powershell
dotnet test modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj --no-restore --filter "FullyQualifiedName~LegacyMenuCatalog|FullyQualifiedName~LegacyCommandRouter"
dotnet test modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj --no-restore --filter "FullyQualifiedName~LegacyMenuBar|FullyQualifiedName~HistoricShortcut"
```

Expected: failures for missing shortcuts and visibility properties.

- [ ] **Step 3: Add visibility to the catalog and command state**

Extend `LegacyCommandState` with `bool Visible = true`. Extend top-level `LegacyMenuItem.Top` with optional `bool visible = true`, stored as `VisibleByDefault`. Mark E-Grid `visible: false`. Preserve E-Grid commands in the typed catalog so future tools can expose them without inventing IDs.

In `LegacyMenuBar`, initialize every item's `IsVisible` from `VisibleByDefault`. During `RefreshStates`, assign leaf `IsVisible = state.Visible`. Change the Survival Info handler to return `Visible=false` until an actual Survival/Eco mode exists.

- [ ] **Step 4: Bind the exact shortcut table**

Populate every catalog `Shortcut` listed in Step 1. In `LegacyMenuBar.BuildMenuItem`, parse non-null strings with Avalonia `KeyGesture.Parse` and assign `MenuItem.HotKey`. A malformed catalog gesture is a construction error and must fail tests; do not swallow it.

- [ ] **Step 5: Remove conflicting manual key routes**

Delete manual mappings that conflict with the historic table: F5, F8, Ctrl+B, Ctrl+I, Ctrl+S, Ctrl+R, and Ctrl+E. Let the menu hotkeys dispatch those commands. Keep only non-conflicting modern additions such as F6/F7/F9, Ctrl+Comma, Ctrl+1/Ctrl+2/Ctrl+Delete, and Alt movement tools. F12 is owned by the historic Pause command.

- [ ] **Step 6: Run focused tests**

Run the two commands from Step 2.

Expected: all focused tests pass and the ordinary runtime exposes neither E-Grid nor Survival Info.

- [ ] **Step 7: Commit**

```powershell
git add modern/desktop
git commit -m "fix: restore DB2 menu visibility and shortcuts"
```

---

### Task 5: Publish an honest checkpoint and run its complete gate

**Files:**
- Modify: `docs/parity/historic-menu-parity-report.md`
- Modify: `docs/parity/gui-parity-report.md`
- Modify: `modern/docs/verification.md`
- Create: `docs/parity/db24832-gap-register.md`

**Interfaces:**
- Consumes: Tasks 1-4 commits, the four audit reports in the research workspace, and current test output.
- Produces: a measured checkpoint report and prioritized non-online gap register for the next subsystem plans.

- [ ] **Step 1: Replace broad parity claims with measured checkpoint language**

State that catalog registration is not behavioral parity. Record the exact implemented/fixed behaviors: typed routing, live mutation settings, mutation-free copy save, atomic whole-organism deletion, hidden E-Grid/Survival state, exact retained shortcuts, inert online commands, view toggles, and existing object tools. Keep generic document/value-window substitutions and collapsed settings dialogs listed as open gaps.

- [ ] **Step 2: Create the gap register**

Create sections for cycle/state, DNA/sysvars, physics/senses, shots/defenses, ties/multibots, reproduction/genealogy, ecology/corpses, mutation/speciation, persistence/offline modes, GPU differentials, desktop shell/settings/tools/viewport, and acceptance evidence. For each section, list the P0/P1 findings already recorded in the four audits. Mark only Tasks 1-4 findings complete. Do not mark a subsystem complete from catalog or unit-test counts.

- [ ] **Step 3: Run the checkpoint gate serially**

```powershell
pwsh -NoProfile -File docs/reference/db24832/verify.ps1
cargo test -p darwinbots-engine --test contracts --test manual_tools --test live_legacy_menu_settings
dotnet test modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj --no-restore --nologo
dotnet test modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj --no-restore --nologo
pwsh -NoProfile -File docs/verification/control-surface-audit/run-rendered-audit.ps1
```

Expected: reference package 6/6; all selected Rust tests pass; all desktop-core and rendered tests pass; rendered audit regenerates its screenshots and validates the control matrix.

- [ ] **Step 4: Inspect rendered evidence**

Open the generated runtime screenshot and confirm: the Internet menu is inert, E-Grid is absent, menu labels remain readable, the arena is not obscured, and no toolbar/status layout regressed. Record the screenshot path and dimensions in `historic-menu-parity-report.md`.

- [ ] **Step 5: Commit**

```powershell
git add docs/parity modern/docs/verification.md
git commit -m "docs: record DB2 menu checkpoint evidence"
```
