# DB2.48.32 Default Parity and Starter Movement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task.

**Goal:** Make every legacy-owned clean-start default match Darwinbots 2.48.32, preserve approved Rust/GPU modernization defaults, and prove that a fresh untouched starter animal moves without manual dragging.

**Architecture:** Rust remains the simulation authority. The C# domain mirrors Rust defaults, while Avalonia controls only present and edit those values. Legacy-owned defaults come from the VB6 clean-start path; modern-only GPU, capacity, seed, and offline behavior remain modern.

**Tech Stack:** Rust 2021, Cargo, C#/.NET 8, Avalonia, xUnit, PowerShell, Git.

## Global Constraints

- Work only in `D:\Darwinbots 2\.worktrees\db24832-parity` on `codex/db24832-parity`.
- Preserve `BackendPreference::Auto`, GPU acceleration, CPU fallback policy, deterministic seed controls, offline behavior, Zerobot assistance, and the 25,000 modern setup capacity.
- Do not restore removed network play or obsolete VB6 implementation details.
- Do not alter `EngineConfig::testing()` values unless a named test requires an explicit field.
- Serialize Cargo and native-library builds. Before retrying a timed-out build, inspect the existing Cargo process.
- Stage only files named in the active task. Commit each task after its focused gate passes.
- Use the smallest focused tests below. Do not run the complete Rust or desktop suites.

---

### Task 1: Freeze the Rust clean-start default contract

**Files:**
- Create: `modern/engine/tests/db24832_clean_defaults.rs`
- Modify: `modern/engine/src/config.rs`
- Modify: `modern/engine/src/modes.rs`

**Step 1: Add the failing Rust default test**

Create a test that constructs `EngineConfig::default()` and asserts these exact legacy-owned values:

```rust
#[test]
fn clean_start_defaults_match_db24832() {
    let config = darwinbots_engine::EngineConfig::default();

    assert_eq!(config.world_size, [16_000.0, 12_000.0]);
    assert_eq!(config.physics.max_velocity, 60.0);
    assert_eq!(config.physics.movement_efficiency, 0.66);
    assert_eq!(config.physics.surface_gravity, 0.0);
    assert_eq!(config.physics.static_friction, 0.0);
    assert_eq!(config.physics.kinetic_friction, 0.0);
    assert_eq!(config.physics.density, 0.0);
    assert_eq!(config.physics.viscosity, 0.0);
    assert_eq!(config.physics.elasticity, 0.0);
    assert_eq!(config.brownian_motion, 0.5);

    assert!(!config.corpse_decay.enabled);
    assert_eq!(config.corpse_decay.energy_threshold, 0);
    assert_eq!(config.corpse_decay.packet_energy, 0);
    assert_eq!(config.corpse_decay.period_ticks, 0);
    assert_eq!(config.recording.sampling_interval, 200);

    assert_eq!(config.vegetation.starting_chloroplasts, 16_000);
    assert_eq!(config.vegetation.max_energy, 100);
    assert_eq!(config.vegetation.minimum_vegetables, 50);
    assert_eq!(config.vegetation.repopulation_amount, 10);
    assert_eq!(config.vegetation.repopulation_cooldown, 10);
    assert_eq!(config.vegetation.feeding_to_body, 0.75);
    assert_eq!(config.vegetation.gradient, 1.02);
    assert!(config.vegetation.daytime);
    assert!(!config.vegetation.day_night_cycle);
}

#[test]
fn local_contest_defaults_match_db24832() {
    let config = darwinbots_engine::modes::LocalModeConfig::default();
    assert_eq!(config.minimum_rounds, 5);
    assert_eq!(config.maximum_rounds, 1);
    assert_eq!(config.maximum_cycles, 15_000);
    assert_eq!(config.maximum_population, 700);
    assert!(config.validate().is_ok());
}
```

Import the public types through the crate paths already used by neighboring integration tests.

**Step 2: Run the RED gate**

Run:

```powershell
cargo test -p darwinbots-engine --test db24832_clean_defaults -- --nocapture
```

Expected: FAIL on the current 180 maximum velocity, nonzero surface/friction values, zero Brownian motion, enabled corpse decay, 10-tick sampling, vegetation values, and modern contest values.

**Step 3: Change only the Rust defaults**

In `EngineConfig::default()` and nested `Default` implementations, assign:

```rust
max_velocity: 60.0,
movement_efficiency: 0.66,
surface_gravity: 0.0,
static_friction: 0.0,
kinetic_friction: 0.0,
density: 0.0,
viscosity: 0.0,
elasticity: 0.0,
brownian_motion: 0.5,
```

Set clean-start corpse decay to disabled zero-init values, recording sampling to `200`, and vegetation to `100`, `50`, `10`, `0.75`, and `1.02` as asserted. Leave shot speed `40`, no-decay flags `false`, world size, gravity vector, drag, and backend `Auto` unchanged.

In `LocalModeConfig::default()`, assign `minimum_rounds: 5`, `maximum_rounds: 1`, `maximum_cycles: 15_000`, and `maximum_population: 700`. Replace the invalid `maximum_rounds < minimum_rounds` rule with independent nonzero validation because DB2 uses minimum rounds and the win threshold as separate concepts.

**Step 4: Run the GREEN gate**

Run the same focused Cargo command. Expected: both tests PASS.

**Step 5: Commit the Rust contract**

```powershell
git add modern/engine/src/config.rs modern/engine/src/modes.rs modern/engine/tests/db24832_clean_defaults.rs
git commit -m "fix: restore DB2 clean-start engine defaults"
```

---

### Task 2: Mirror the default contract in the desktop domain

**Files:**
- Create: `modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Db24832DefaultParityTests.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/EnvironmentUpdate.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/WorldSetupOptions.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/DesktopSettings.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Views/AdvancedSettingsWindow.axaml.cs`
- Test: `modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj`

**Step 1: Add the failing C# domain test**

Add xUnit tests that assert `EnvironmentUpdate.Default`, `new WorldSetupOptions()`, and `new DesktopSettings()` share the Task 1 values. Include these serialization assertions:

```csharp
Assert.Equal(60f, update.Physics.MaxVelocity);
Assert.Equal(0.5f, update.BrownianMotion);
Assert.False(update.CorpseDecay.Enabled);
Assert.Equal(200, update.Recording.SamplingInterval);
Assert.Equal(100, update.Vegetation.MaxEnergy);
Assert.Equal(50, update.Vegetation.MinimumVegetables);
Assert.Equal(10, update.Vegetation.RepopulationCooldown);
Assert.Equal(0.75f, update.Vegetation.FeedingToBody);
Assert.Equal(1.02f, update.Vegetation.Gradient);

var roundTrip = EnvironmentUpdateJson.Deserialize(EnvironmentUpdateJson.Serialize(update));
Assert.Equal(update, roundTrip);
```

Use the existing serializer type and equality pattern from `Db2EnvironmentSettingsTests.cs`. Assert `DesktopSettings.OrganismCapacity == 25_000`, `Backend == Auto`, and all modern-only settings remain unchanged.

**Step 2: Run the RED gate**

```powershell
dotnet test modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj --no-restore --filter FullyQualifiedName~Db24832DefaultParityTests
```

Expected: FAIL on the stale C# defaults and the 100,000 settings capacity.

**Step 3: Update the desktop-domain defaults**

Make `Db2PhysicsOptions.Default`, corpse decay, recording, vegetation, `EnvironmentUpdate.Default`, `WorldSetupOptions`, and `DesktopSettings` match Task 1. Change the missing JSON sampling fallback from `10` to `200`. Change the advanced-settings numeric fallback from `10` to `200`. Set the modern settings capacity to the approved setup capacity `25_000`; do not change backend selection or GPU flags.

**Step 4: Run the GREEN gate**

Run the Task 2 test command. Expected: all filtered tests PASS.

**Step 5: Commit the desktop-domain defaults**

```powershell
git add modern/desktop/src/Darwinbots.Desktop.Core/EnvironmentUpdate.cs modern/desktop/src/Darwinbots.Desktop.Core/WorldSetupOptions.cs modern/desktop/src/Darwinbots.Desktop.Core/DesktopSettings.cs modern/desktop/src/Darwinbots.Desktop/Views/AdvancedSettingsWindow.axaml.cs modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Db24832DefaultParityTests.cs
git commit -m "fix: mirror DB2 defaults in desktop settings"
```

---

### Task 3: Make the visible setup and contest controls open on DB2 defaults

**Files:**
- Modify: `modern/desktop/src/Darwinbots.Desktop/Views/MainWindow.axaml`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Views/MainWindow.axaml.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop/ViewModels/LocalModeViewModel.cs`
- Modify: `modern/desktop/tests/Darwinbots.Desktop.Tests/SetupControlSurfaceTests.cs`
- Modify: `modern/desktop/tests/Darwinbots.Desktop.Tests/LocalModeWorkflowTests.cs`
- Test: `modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj`

**Step 1: Add failing rendered-control assertions**

Extend the existing UI tests to construct the relevant windows/view models and assert the untouched controls expose:

```csharp
Assert.Equal(5m, Named<NumericUpDown>(window, "ModeMinimumRounds").Value);
Assert.Equal(1m, Named<NumericUpDown>(window, "ModeMaximumRounds").Value);
Assert.Equal(15_000m, Named<NumericUpDown>(window, "ModeMaximumCycles").Value);
Assert.Equal(700m, Named<NumericUpDown>(window, "ModeMaximumPopulation").Value);
```

Also assert the clean presentation toggles remain enabled for the vision grid, shot impacts, resource gauges, movement vectors, and teleporter highlighting. Use existing `Named<T>` helpers and actual control names from `MainWindow.axaml`.

**Step 2: Run the RED gate**

```powershell
dotnet test modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj --no-restore --filter "FullyQualifiedName~SetupControlSurfaceTests|FullyQualifiedName~LocalModeWorkflowTests"
```

Expected: FAIL on the current contest values `1`, `3`, `10_000`, and `25_000`. Presentation assertions should already pass; if they fail, change only the initial checked state or matching view-model default.

**Step 3: Update XAML, fallbacks, and view-model initialization**

Set the four contest `NumericUpDown` values to `5`, `1`, `15000`, and `700`. Change matching `ReadUInt` fallbacks in `MainWindow.axaml.cs`. Initialize `LocalModeViewModel` fields to the same values. Do not change the runtime snapshot overwrite path.

**Step 4: Run the GREEN gate**

Run the Task 3 test command. Expected: both filtered test classes PASS.

**Step 5: Commit the visible defaults**

```powershell
git add modern/desktop/src/Darwinbots.Desktop/Views/MainWindow.axaml modern/desktop/src/Darwinbots.Desktop/Views/MainWindow.axaml.cs modern/desktop/src/Darwinbots.Desktop/ViewModels/LocalModeViewModel.cs modern/desktop/tests/Darwinbots.Desktop.Tests/SetupControlSurfaceTests.cs modern/desktop/tests/Darwinbots.Desktop.Tests/LocalModeWorkflowTests.cs
git commit -m "fix: present DB2 defaults in desktop controls"
```

---

### Task 4: Prove fresh starter movement on CPU and physical GPU

**Files:**
- Create: `modern/engine/tests/db24832_starter_movement.rs`
- Reference: `modern/engine/tests/w4_evolution_gpu.rs`
- Reference: `modern/engine/tests/db2_starter_ecology.rs`

**Step 1: Add a causal untouched-start regression test**

Build a helper that:

- starts from `EngineConfig::default()`;
- overrides Brownian motion to `0.0` so random drift cannot satisfy the assertion;
- sets metabolism and unrelated environmental drains to zero;
- selects either CPU or GPU through a parameter;
- loads the bundled `Animal_Minimalis.txt` and `Alga_Minimalis.txt` DNA used by the existing starter ecology tests;
- spawns one animal and one alga in vision range without moving either through an API;
- records the animal position, advances two ticks, and asserts a nonzero position delta and `intentional_movement_events > 0`.

Add two tests named `untouched_starter_animal_moves_on_cpu` and
`untouched_starter_animal_moves_on_physical_gpu`. Each test calls the shared
helper with its explicit backend preference.

For the GPU test, set `allow_cpu_fallback = false`, assert `BackendKind::Gpu`, require `gpu_adapter_receipt()`, and print adapter/backend details exactly as `w4_evolution_gpu.rs` does.

**Step 2: Run the CPU diagnostic gate**

```powershell
cargo test -p darwinbots-engine --test db24832_starter_movement untouched_starter_animal_moves_on_cpu -- --nocapture
```

Expected: PASS. A failure is a real engine freeze and authorizes the smallest simulation fix proven by the failing assertion.

**Step 3: Run the physical-GPU diagnostic gate**

```powershell
cargo test -p darwinbots-engine --test db24832_starter_movement untouched_starter_animal_moves_on_physical_gpu -- --nocapture
```

Expected: PASS with a physical adapter/backend receipt. CPU fallback is a failure.

**Step 4: Commit the movement guard**

```powershell
git add modern/engine/tests/db24832_starter_movement.rs
git commit -m "test: guard untouched starter movement"
```

---

### Task 5: Refresh the default audit and rebuild the game

**Files:**
- Modify: `docs/verification/db2-defaults-and-sysvars-audit.md`
- Build: `modern/desktop/src/Darwinbots.Desktop/Darwinbots.Desktop.csproj`

**Step 1: Update the audit from verified values**

Update the audit's modern-value column and disposition for every value changed in Tasks 1-3. Remove the old intentional Brownian/default deviations. Add a short preserved-modernization note for `Auto` GPU backend, 25,000 organism capacity, seed controls, and offline-only behavior. Record the CPU and physical-GPU movement gate names without claiming more than those tests prove.

**Step 2: Run the smallest documentation consistency check**

```powershell
rg -n "180|surface gravity.*2|Brownian.*intentional deviation|sampling.*10|minimum vegetables.*10|maximum energy.*40" docs/verification/db2-defaults-and-sysvars-audit.md
```

Expected: no stale-current-default matches. Historical authority quotations may retain original source numbers only when clearly labeled.

**Step 3: Stop the stale desktop process if it is still running**

Resolve the exact executable path first:

```powershell
Get-Process Darwinbots.Desktop -ErrorAction SilentlyContinue | Select-Object Id,Path
```

Stop only a process whose path is under this canonical worktree's desktop output directory.

**Step 4: Build one fresh desktop binary**

```powershell
dotnet build modern/desktop/src/Darwinbots.Desktop/Darwinbots.Desktop.csproj --no-restore
```

Expected: build succeeds and refreshes both `Darwinbots.Desktop.exe` and `darwinbots_engine.dll` in the active output directory.

**Step 5: Verify artifact freshness and repository scope**

```powershell
Get-Item modern/desktop/src/Darwinbots.Desktop/bin/Debug/net8.0/Darwinbots.Desktop.exe, modern/desktop/src/Darwinbots.Desktop/bin/Debug/net8.0/darwinbots_engine.dll | Select-Object Name,LastWriteTimeUtc,Length
git status --short
```

Expected: both artifacts have current timestamps. Only the audit file is uncommitted before the final commit.

**Step 6: Commit the evidence update**

```powershell
git add docs/verification/db2-defaults-and-sysvars-audit.md
git commit -m "docs: verify DB2 clean-start default parity"
```

---

### Task 6: Final bounded verification and launch

**Files:**
- Verify only; no planned source edits.

**Step 1: Run the four focused gates once against final HEAD**

```powershell
cargo test -p darwinbots-engine --test db24832_clean_defaults -- --nocapture
cargo test -p darwinbots-engine --test db24832_starter_movement -- --nocapture
dotnet test modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj --no-restore --filter FullyQualifiedName~Db24832DefaultParityTests
dotnet test modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj --no-restore --filter "FullyQualifiedName~SetupControlSurfaceTests|FullyQualifiedName~LocalModeWorkflowTests"
```

Expected: all focused gates PASS; the GPU movement test prints a physical adapter receipt.

**Step 2: Confirm branch and clean tracked state**

```powershell
git branch --show-current
git log -6 --oneline
git status --short
```

Expected: branch is `codex/db24832-parity`; all named commits exist; no tracked changes remain.

**Step 3: Launch the fresh binary**

```powershell
Start-Process -FilePath "D:\Darwinbots 2\.worktrees\db24832-parity\modern\desktop\src\Darwinbots.Desktop\bin\Debug\net8.0\Darwinbots.Desktop.exe" -WorkingDirectory "D:\Darwinbots 2\.worktrees\db24832-parity" -WindowStyle Normal
```

Report the process ID, final commit SHA, exact focused gate counts, GPU adapter/backend receipt, and any untracked files. Do not claim complete gameplay parity from these bounded checks.
