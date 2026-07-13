# DB2 Physics, Shots, and Vegetables Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port Darwinbots 2 impulse physics, persistent projectiles, and chloroplast-driven vegetable ecology into the modern Rust engine and expose their settings through the existing Avalonia application.

**Architecture:** `world.rs` remains the ordered phase coordinator. Formula-heavy behavior moves into focused modules under `physics/`, `shot/`, and `vegetation.rs`, with the CPU implementation defining behavior and existing GPU kernels accelerating only compatible uniform phases. Versioned FFI commands and immutable snapshots carry settings and projectile state to Avalonia.

**Tech Stack:** Rust 2024, Rayon, serde/serde_json, wgpu/WGSL, .NET 10, C#, Avalonia, xUnit, PowerShell, Computer Use keyboard automation.

## Global Constraints

- The VB6 files under `Darwinbots2/` are authoritative for formulas and phase ordering.
- `Darwin2.48.32.exe` is the black-box reference where source behavior is ambiguous.
- DarwinbotsC is reference-only and cannot override VB6 behavior.
- Preserve symbolic and numeric legacy DNA compatibility, including chloroplast addresses `920` through `923`.
- Behavioral parity is required; deterministic evolution and cycle-identical random sequences are not.
- CPU behavior is authoritative and must work without a GPU.
- Existing GPU acceleration must fall back safely when a DB2 phase is not GPU-compatible.
- Keep organism and projectile storage structure-of-arrays compatible.
- Starter mode uses normal metabolism; Zerobot sustenance overrides apply only to Zerobot modes.
- Keep the existing DB2-style viewport and setup layout conventions.
- Before changing a visual surface, create a high-reasoning image blueprint from the current screenshot and compare the implementation against clipboard screenshots.
- Test first: every production behavior change starts with a focused failing test and an observed expected failure.
- Do not claim the production bug fixed until physics, shots, and vegetables pass together in a fresh packaged simulation.

---

## Planned File Structure

### Rust engine files to create

- `modern/engine/src/physics/movement.rs`: DB2 movement commands, mass behavior, voluntary impulse, and speed safeguards.
- `modern/engine/src/physics/environment.rs`: gravity, Brownian motion, friction, density, viscosity, and resistance impulses.
- `modern/engine/src/physics/collision.rs`: swept-circle and organism collision helpers shared by organisms and shots.
- `modern/engine/src/shot/projectile.rs`: projectile structure-of-arrays pool, creation, integration, aging, and snapshot conversion.
- `modern/engine/src/shot/effects.rs`: typed projectile impact effects for feeding, donation, venom, waste, poison, body, memory, reproduction, virus, and sperm shots.
- `modern/engine/src/vegetation.rs`: DB2 light, chloroplast feeding, body/energy allocation, and repopulation runtime.
- `modern/engine/tests/support/mod.rs`: shared integration-test helpers.
- `modern/engine/tests/support/db2_fixtures.rs`: source-derived DB2 constants and expected formula outputs.
- `modern/engine/tests/db2_physics.rs`: movement and environment formula regressions.
- `modern/engine/tests/db2_projectiles.rs`: projectile creation, movement, collision, decay, and expiry regressions.
- `modern/engine/tests/db2_vegetation.rs`: sysvar, feeding, chloroplast, and repopulation regressions.
- `modern/engine/tests/db2_starter_ecology.rs`: integrated starter-world acceptance scenario.
- `modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Db2EnvironmentSettingsTests.cs`: .NET settings serialization and validation regressions.

### Existing Rust files to modify

- `modern/engine/src/lib.rs`: register and export the new modules and public configuration types.
- `modern/engine/src/config.rs`: add versioned DB2 physics, projectile, and vegetation settings with defaults.
- `modern/engine/src/sysvars.rs`: correct chloroplast/light addresses and reverse mappings.
- `modern/engine/src/dna.rs`: expose `NewMove` detection without coupling physics to parser internals.
- `modern/engine/src/biology.rs`: use legacy chloroplast addresses and retain fractional plant-energy carry.
- `modern/engine/src/physics.rs`: host focused physics submodules and extend CPU/GPU batch contracts.
- `modern/engine/src/shot.rs`: host shot submodules and replace trail-only state with projectile snapshots.
- `modern/engine/src/world.rs`: own the new SoA stores and call their phases in DB2 order.
- `modern/engine/src/species.rs`: add starting chloroplast/body settings used by initial and repopulated vegetables.
- `modern/engine/src/persistence.rs`: increase save version and validate new state.
- `modern/engine/src/ffi.rs`: add versioned configuration command fields without changing ABI ownership rules.
- `modern/engine/src/spatial.rs`: expose segment candidate queries for projectile broad phase.
- `modern/engine/src/physics.wgsl`: accept CPU-computed velocity while preserving GPU position integration.
- Existing engine tests whose instant-shot or absolute-velocity expectations are intentionally superseded.

### Existing desktop files to modify

- `modern/desktop/src/Darwinbots.Desktop.Core/EnvironmentUpdate.cs`: represent all live DB2 settings.
- `modern/desktop/src/Darwinbots.Desktop.Core/WorldSetupOptions.cs`: use normal DB2 defaults.
- `modern/desktop/src/Darwinbots.Desktop.Core/EngineSnapshot.cs`: represent live projectile snapshots and new counters.
- `modern/desktop/src/Darwinbots.Desktop.Core/NativeEngineClient.cs`: serialize extended creation/update commands.
- `modern/desktop/src/Darwinbots.Desktop.Core/NativeSnapshotParser.cs`: parse projectile and statistics fields.
- `modern/desktop/src/Darwinbots.Desktop/Views/SetupWindow.axaml.cs`: scope sustenance to Zerobot modes.
- `modern/desktop/src/Darwinbots.Desktop/Views/AdvancedSettingsWindow.axaml`: expose DB2 settings in grouped sections.
- `modern/desktop/src/Darwinbots.Desktop/Views/AdvancedSettingsWindow.axaml.cs`: load, validate, and return the new values.
- `modern/desktop/src/Darwinbots.Desktop/Views/MainWindow.axaml.cs`: keep live settings state and submit complete updates.
- `modern/desktop/src/Darwinbots.Desktop/Controls/WorldViewport.cs`: draw short projectile motion segments and impact flashes.
- Existing desktop tests for setup defaults, snapshot parsing, and settings commands.

---

### Task 1: Lock DB2 Formula Fixtures and Correct Chloroplast Sysvars

**Files:**
- Create: `modern/engine/tests/support/mod.rs`
- Create: `modern/engine/tests/support/db2_fixtures.rs`
- Create: `modern/engine/tests/db2_vegetation.rs`
- Modify: `modern/engine/src/sysvars.rs`
- Modify: `modern/engine/src/biology.rs`
- Modify: `modern/engine/src/world.rs`
- Modify: `modern/engine/tests/species_ecology.rs`

**Interfaces:**
- Consumes: VB6 addresses from `Darwinbots2/DNATokenizing.bas`.
- Produces: constants `MEM_CHLR`, `MEM_MAKE_CHLR`, `MEM_REMOVE_CHLR`, `MEM_LIGHT`, and `MEM_AVAILABILITY` used by later vegetation tasks.

- [ ] **Step 1: Add source-derived fixture constants**

```rust
// modern/engine/tests/support/db2_fixtures.rs
pub const MEM_CHLR: i32 = 920;
pub const MEM_MAKE_CHLR: i32 = 921;
pub const MEM_REMOVE_CHLR: i32 = 922;
pub const MEM_LIGHT: i32 = 923;
pub const START_CHLR: i32 = 16_000;
pub const MAX_BOT_VALUE: i32 = 32_000;
pub const DEFAULT_MAX_VELOCITY: f32 = 60.0;
pub const DEFAULT_MOVEMENT_EFFICIENCY: f32 = 0.66;
pub const SHOT_SPEED: f32 = 40.0;
```

```rust
// modern/engine/tests/support/mod.rs
pub mod db2_fixtures;
```

- [ ] **Step 2: Write failing symbolic and numeric sysvar tests**

```rust
mod support;

use darwinbots_engine::sysvar_address;
use support::db2_fixtures::{MEM_CHLR, MEM_LIGHT, MEM_MAKE_CHLR, MEM_REMOVE_CHLR};

#[test]
fn chloroplast_sysvars_use_db2_memory_addresses() {
    assert_eq!(sysvar_address("chlr"), Some(MEM_CHLR));
    assert_eq!(sysvar_address("mkchlr"), Some(MEM_MAKE_CHLR));
    assert_eq!(sysvar_address("rmchlr"), Some(MEM_REMOVE_CHLR));
    assert_eq!(sysvar_address("light"), Some(MEM_LIGHT));
}
```

- [ ] **Step 3: Run the focused test and observe the expected failure**

Run: `cargo test -p darwinbots-engine --test db2_vegetation chloroplast_sysvars_use_db2_memory_addresses`

Expected: FAIL because the current implementation returns `250`, `251`, `252`, and `253`.

- [ ] **Step 4: Replace incorrect aliases and engine constants**

```rust
// modern/engine/src/sysvars.rs and biology/world constants
const MEM_CHLOROPLASTS: i32 = 920;
const MEM_MAKE_CHLOROPLASTS: i32 = 921;
const MEM_REMOVE_CHLOROPLASTS: i32 = 922;
const MEM_LIGHT: i32 = 923;
const MEM_AVAILABILITY: i32 = 924;

for (name, address) in [
    ("chlr", MEM_CHLOROPLASTS),
    ("mkchlr", MEM_MAKE_CHLOROPLASTS),
    ("rmchlr", MEM_REMOVE_CHLOROPLASTS),
    ("light", MEM_LIGHT),
    ("availability", MEM_AVAILABILITY),
] {
    variables.insert(name.to_owned(), address);
}
```

Update `species_ecology.rs` to assert the actual DB2 addresses instead of parser table indexes.

- [ ] **Step 5: Run sysvar and parser regressions**

Run: `cargo test -p darwinbots-engine --test db2_vegetation --test species_ecology --test legacy_distribution`

Expected: PASS with symbolic and bundled legacy DNA importing successfully.

- [ ] **Step 6: Commit the sysvar boundary**

```powershell
git add modern/engine/src/sysvars.rs modern/engine/src/biology.rs modern/engine/src/world.rs modern/engine/tests/support modern/engine/tests/db2_vegetation.rs modern/engine/tests/species_ecology.rs
git commit -m "Correct DB2 chloroplast sysvars"
```

### Task 2: Add Versioned DB2 Simulation Settings and Save Boundary

**Files:**
- Modify: `modern/engine/src/config.rs`
- Modify: `modern/engine/src/species.rs`
- Modify: `modern/engine/src/persistence.rs`
- Modify: `modern/engine/src/ffi.rs`
- Modify: `modern/engine/tests/contracts.rs`
- Modify: `modern/engine/tests/gpu_ffi.rs`

**Interfaces:**
- Consumes: fixture defaults from Task 1.
- Produces: `PhysicsSettings`, `ShotSettings`, and `VegetationSettings`, embedded in `EngineConfig` and serialized through save version `2`.

- [ ] **Step 1: Write failing configuration-default tests**

```rust
#[test]
fn db2_defaults_are_exposed_by_engine_config() {
    let config = EngineConfig::default();
    assert_eq!(config.physics.max_velocity, 60.0);
    assert_eq!(config.physics.movement_efficiency, 0.66);
    assert_eq!(config.shots.speed, 40.0);
    assert_eq!(config.vegetation.start_chloroplasts, 16_000);
    assert_eq!(config.vegetation.repopulation_amount, 10);
    assert_eq!(config.vegetation.repopulation_cooldown, 10);
}
```

- [ ] **Step 2: Run the test and observe missing configuration types**

Run: `cargo test -p darwinbots-engine --test contracts db2_defaults_are_exposed_by_engine_config`

Expected: FAIL to compile because the nested settings do not exist.

- [ ] **Step 3: Add serializable setting types with exact defaults**

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PhysicsSettings {
    pub max_velocity: f32,
    pub movement_efficiency: f32,
    pub surface_gravity: f32,
    pub static_friction: f32,
    pub kinetic_friction: f32,
    pub density: f64,
    pub viscosity: f64,
    pub elasticity: f32,
}

impl Default for PhysicsSettings {
    fn default() -> Self {
        Self {
            max_velocity: 60.0,
            movement_efficiency: 0.66,
            surface_gravity: 0.0,
            static_friction: 0.6,
            kinetic_friction: 0.4,
            density: 0.000_000_1,
            viscosity: 0.000_025,
            elasticity: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShotSettings {
    pub speed: f32,
    pub range_multiplier: f32,
    pub decay: f32,
    pub energy_shots_do_not_decay: bool,
    pub waste_shots_do_not_decay: bool,
}

impl Default for ShotSettings {
    fn default() -> Self {
        Self { speed: 40.0, range_multiplier: 1.0, decay: 40.0, energy_shots_do_not_decay: false, waste_shots_do_not_decay: false }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VegetationSettings {
    pub start_chloroplasts: i32,
    pub max_energy_per_tick: i32,
    pub minimum_chloroplast_equivalents: usize,
    pub repopulation_amount: usize,
    pub repopulation_cooldown: u64,
    pub feeding_to_body: f32,
    pub daytime: bool,
    pub day_night_enabled: bool,
    pub cycle_length: u64,
}

impl Default for VegetationSettings {
    fn default() -> Self {
        Self {
            start_chloroplasts: 16_000,
            max_energy_per_tick: 100,
            minimum_chloroplast_equivalents: 50,
            repopulation_amount: 10,
            repopulation_cooldown: 10,
            feeding_to_body: 0.75,
            daytime: true,
            day_night_enabled: false,
            cycle_length: 10_000,
        }
    }
}
```

- [ ] **Step 4: Increase the save version and test rejection of version 1**

```rust
// persistence.rs
const VERSION: u16 = 2;
```

```rust
#[test]
fn save_version_one_is_rejected_after_db2_state_upgrade() {
    let engine = Engine::new(EngineConfig::testing()).unwrap();
    let mut bytes = SaveFile::encode(&engine).unwrap();
    bytes[4..6].copy_from_slice(&1u16.to_le_bytes());
    assert!(SaveFile::decode(&bytes).unwrap_err().to_string().contains("unsupported version 1"));
}
```

- [ ] **Step 5: Extend versioned FFI update commands**

Add optional nested `physics`, `shots`, and `vegetation` fields to `EngineCommand::UpdateEnvironment`. Missing fields preserve current values, so old version-1 command producers remain valid during the desktop migration.

```rust
UpdateEnvironment {
    metabolism_cost: i32,
    vegetable_energy_per_tick: i32,
    sunlight_energy: i32,
    gravity: [f32; 2],
    drag: f32,
    brownian_motion: f32,
    #[serde(default)] physics: Option<PhysicsSettings>,
    #[serde(default)] shots: Option<ShotSettings>,
    #[serde(default)] vegetation: Option<VegetationSettings>,
}
```

- [ ] **Step 6: Run save and FFI tests**

Run: `cargo test -p darwinbots-engine --test contracts --test gpu_ffi`

Expected: PASS, including owned-buffer and corrupt-save behavior.

- [ ] **Step 7: Commit settings and persistence**

```powershell
git add modern/engine/src/config.rs modern/engine/src/species.rs modern/engine/src/persistence.rs modern/engine/src/ffi.rs modern/engine/tests/contracts.rs modern/engine/tests/gpu_ffi.rs
git commit -m "Add DB2 simulation settings"
```

### Task 3: Port DB2 Voluntary Impulse and Momentum

**Files:**
- Create: `modern/engine/src/physics/movement.rs`
- Create: `modern/engine/tests/db2_physics.rs`
- Modify: `modern/engine/src/physics.rs`
- Modify: `modern/engine/src/dna.rs`
- Modify: `modern/engine/src/world.rs`
- Modify: `modern/engine/tests/contracts.rs`

**Interfaces:**
- Consumes: `PhysicsSettings` from Task 2 and organism body/shell/chloroplast values.
- Produces: `MovementInput`, `MovementState`, `derived_mass`, and `apply_voluntary_impulse` used by environment and integration phases.

- [ ] **Step 1: Write failing impulse, coasting, and clamp tests**

```rust
mod support;

use darwinbots_engine::{Engine, EngineConfig, LegacyDna};

#[test]
fn movement_command_adds_impulse_and_bot_coasts_without_new_thrust() {
    let mut engine = Engine::new(EngineConfig { metabolism_cost: 0, drag: 0.0, ..EngineConfig::testing() }).unwrap();
    let dna = LegacyDna::parse("cond\n*.robage 1 <\nstart\n10 .up store\nstop").unwrap();
    let id = engine.spawn_at(dna, [100.0, 100.0]).unwrap();

    engine.tick().unwrap();
    let first = engine.organism(id).unwrap();
    engine.tick().unwrap();
    let second = engine.organism(id).unwrap();

    assert!(first.velocity[1] > 0.0);
    assert_eq!(second.velocity, first.velocity);
    assert!(second.position[1] > first.position[1]);
}

#[test]
fn voluntary_acceleration_is_clamped_before_efficiency_multiplier() {
    let mut engine = Engine::new(EngineConfig { metabolism_cost: 0, drag: 0.0, ..EngineConfig::testing() }).unwrap();
    let id = engine.spawn_at(LegacyDna::parse("start\n10000 .up store\nstop").unwrap(), [100.0, 100.0]).unwrap();
    engine.tick().unwrap();
    assert!((engine.organism(id).unwrap().velocity[1] - 39.6).abs() < 0.01);
}
```

- [ ] **Step 2: Run focused physics tests and observe the stationary-coast failure**

Run: `cargo test -p darwinbots-engine --test db2_physics`

Expected: FAIL because velocity is currently replaced by the current movement command.

- [ ] **Step 3: Implement focused movement types and formulas**

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct MovementInput {
    pub up: i32,
    pub down: i32,
    pub left: i32,
    pub right: i32,
    pub aim_radians: f32,
    pub new_move: bool,
}

pub(crate) fn derived_mass(body: i32, shell: i32, chloroplasts: i32) -> f32 {
    (body.max(0) as f32 / 1_000.0)
        + (shell.max(0) as f32 / 200.0)
        + (chloroplasts.clamp(0, 32_000) as f32 / 32_000.0) * 31_680.0
}

pub(crate) fn voluntary_impulse(input: MovementInput, mass: f32, settings: &PhysicsSettings) -> [f32; 2] {
    let multiplier = if input.new_move { 1.0 } else { mass.max(1.0) };
    let forward = (input.up as i64 - input.down as i64) as f32 * multiplier;
    let lateral = (input.left as i64 - input.right as i64) as f32 * multiplier;
    let mut world = [
        forward * input.aim_radians.sin() + lateral * input.aim_radians.cos(),
        forward * input.aim_radians.cos() - lateral * input.aim_radians.sin(),
    ];
    let magnitude = world[0].hypot(world[1]);
    if magnitude > settings.max_velocity { world = [world[0] / magnitude * settings.max_velocity, world[1] / magnitude * settings.max_velocity]; }
    [world[0] * settings.movement_efficiency, world[1] * settings.movement_efficiency]
}
```

Add `LegacyDna::uses_new_move() -> bool`, populated by the parser when a `NewMove` directive is present.

- [ ] **Step 4: Change intent from assignment to impulse accumulation**

```rust
velocity[0] += voluntary[0];
velocity[1] += voluntary[1];
```

Do not apply drag in this step. Clear movement output sysvars after capturing the impulse.

- [ ] **Step 5: Run focused and existing movement tests**

Run: `cargo test -p darwinbots-engine --test db2_physics --test contracts`

Expected: PASS after updating the one-tick movement expectation to DB2 movement efficiency.

- [ ] **Step 6: Commit impulse movement**

```powershell
git add modern/engine/src/physics.rs modern/engine/src/physics/movement.rs modern/engine/src/dna.rs modern/engine/src/world.rs modern/engine/tests/db2_physics.rs modern/engine/tests/contracts.rs
git commit -m "Port DB2 impulse movement"
```

### Task 4: Port DB2 Environmental Forces and Integration

**Files:**
- Create: `modern/engine/src/physics/environment.rs`
- Create: `modern/engine/src/physics/collision.rs`
- Modify: `modern/engine/src/physics.rs`
- Modify: `modern/engine/src/world.rs`
- Modify: `modern/engine/tests/db2_physics.rs`
- Modify: `modern/engine/tests/environment_world.rs`
- Modify: `modern/engine/tests/corpses_collisions.rs`

**Interfaces:**
- Consumes: current velocity, derived mass/radius, world settings, and voluntary impulse.
- Produces: `environment_impulse`, `apply_resistance`, `integrate_body`, and shared swept-circle helpers.

- [ ] **Step 1: Add failing momentum-resistance and collision tests**

```rust
#[test]
fn drag_reduces_retained_momentum_instead_of_replacing_it() {
    let mut engine = engine_with_one_tick_thrust(0.25);
    let id = only_organism(&engine);
    engine.tick().unwrap();
    let first = engine.organism(id).unwrap().velocity[1];
    engine.tick().unwrap();
    let second = engine.organism(id).unwrap().velocity[1];
    assert!(second > 0.0);
    assert!(second < first);
}

#[test]
fn elasticity_separates_colliding_bots_and_preserves_finite_velocity() {
    let mut engine = collision_fixture(0.5);
    engine.tick().unwrap();
    for bot in &engine.snapshot().organisms {
        assert!(bot.position.iter().all(|value| value.is_finite()));
        assert!(bot.velocity.iter().all(|value| value.is_finite()));
    }
    assert!(distance(&engine.snapshot().organisms[0], &engine.snapshot().organisms[1]) > 0.0);
}
```

- [ ] **Step 2: Run the tests and observe replacement-velocity behavior**

Run: `cargo test -p darwinbots-engine --test db2_physics --test environment_world --test corpses_collisions`

Expected: FAIL in the retained-momentum assertion.

- [ ] **Step 3: Implement environmental force functions**

```rust
pub(crate) fn apply_linear_drag(velocity: &mut [f32; 2], drag: f32) {
    let retention = 1.0 - drag.clamp(0.0, 0.99);
    velocity[0] *= retention;
    velocity[1] *= retention;
    if velocity[0].abs() < 0.000_000_1 { velocity[0] = 0.0; }
    if velocity[1].abs() < 0.000_000_1 { velocity[1] = 0.0; }
}

pub(crate) fn apply_surface_friction(velocity: &mut [f32; 2], mass: f32, settings: &PhysicsSettings) {
    if settings.surface_gravity <= 0.0 { return; }
    let speed = velocity[0].hypot(velocity[1]);
    if speed <= f32::EPSILON { return; }
    let impulse = (mass * settings.surface_gravity * settings.kinetic_friction).min(speed);
    velocity[0] -= velocity[0] / speed * impulse;
    velocity[1] -= velocity[1] / speed * impulse;
}
```

Port the VB6 sphere-drag coefficient and viscosity calculation into `environment.rs` with `f64` intermediates. Seed Brownian impulses from engine seed, tick, and stable slot.

- [ ] **Step 4: Integrate in DB2 order**

Apply voluntary impulse, gravity/Brownian impulse, surface friction, fluid drag, position integration, boundary response, organism collisions, and environment features. Record actual velocity as `position - previous_position` after collision correction.

- [ ] **Step 5: Run all physics-focused tests**

Run: `cargo test -p darwinbots-engine --test db2_physics --test environment_world --test corpses_collisions --test ties_multibot`

Expected: PASS with finite state and retained momentum.

- [ ] **Step 6: Commit environmental physics**

```powershell
git add modern/engine/src/physics.rs modern/engine/src/physics/environment.rs modern/engine/src/physics/collision.rs modern/engine/src/world.rs modern/engine/tests/db2_physics.rs modern/engine/tests/environment_world.rs modern/engine/tests/corpses_collisions.rs
git commit -m "Port DB2 environmental physics"
```

### Task 5: Introduce Persistent Projectile Storage and Creation

**Files:**
- Create: `modern/engine/src/shot/projectile.rs`
- Create: `modern/engine/src/shot/effects.rs`
- Create: `modern/engine/tests/db2_projectiles.rs`
- Modify: `modern/engine/src/shot.rs`
- Modify: `modern/engine/src/lib.rs`
- Modify: `modern/engine/src/world.rs`
- Modify: `modern/engine/tests/shot_state.rs`

**Interfaces:**
- Consumes: post-movement organism position, actual velocity, aim, body, shot command, and `ShotSettings`.
- Produces: `ProjectilePool::spawn`, `ProjectilePool::advance`, and `ProjectileSnapshot`.

- [ ] **Step 1: Write failing projectile-creation tests**

```rust
mod support;

use darwinbots_engine::{Engine, EngineConfig, LegacyDna};
use support::db2_fixtures::SHOT_SPEED;

#[test]
fn firing_creates_a_moving_projectile_instead_of_an_instant_hit_line() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let attacker = engine.spawn_at(LegacyDna::parse("start\n-1 .shoot store\nstop").unwrap(), [100.0, 100.0]).unwrap();
    engine.spawn_at(LegacyDna::parse("start\nstop").unwrap(), [300.0, 100.0]).unwrap();
    engine.tick().unwrap();
    let shot = &engine.snapshot().shots[0];
    assert_eq!(shot.owner, attacker);
    assert!((segment_length(shot.start, shot.end) - SHOT_SPEED).abs() < 0.01);
    assert_ne!(shot.end, [300.0, 100.0]);
}

#[test]
fn shot_velocity_inherits_the_firers_actual_velocity() {
    let mut engine = moving_shooter_fixture();
    engine.tick().unwrap();
    let shot = &engine.snapshot().shots[0];
    assert!(shot.velocity[1] > 40.0);
}
```

- [ ] **Step 2: Run and observe the instant-line failure**

Run: `cargo test -p darwinbots-engine --test db2_projectiles firing_creates_a_moving_projectile_instead_of_an_instant_hit_line`

Expected: FAIL because the snapshot currently spans directly from attacker to target.

- [ ] **Step 3: Implement the projectile SoA pool**

```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct ProjectilePool {
    owners: Vec<OrganismId>,
    positions: Vec<[f32; 2]>,
    previous_positions: Vec<[f32; 2]>,
    velocities: Vec<[f32; 2]>,
    ages: Vec<u32>,
    ranges: Vec<u32>,
    energies: Vec<f32>,
    kinds: Vec<i32>,
    values: Vec<i32>,
    alive: Vec<bool>,
    impact_flash: Vec<bool>,
    free_slots: Vec<u32>,
}
```

Provide `spawn(request: ProjectileSpawn) -> u32`, `deactivate(slot)`, `len()`, and `snapshots()` methods. Reuse dead slots before extending buffers.

- [ ] **Step 4: Port DB2 creation formulas**

```rust
let direction = [aim.cos(), -aim.sin()];
let position = add(owner_position, scale(direction, owner_radius));
let velocity = add(owner_actual_velocity, scale(direction, settings.speed));
let raw_energy = owner_virtual_body.abs().ln() * 60.0 * settings.range_multiplier;
let range = if owner_virtual_body > 10.0 { ((raw_energy + 41.0) / 40.0).floor().max(1.0) as u32 } else { settings.range_multiplier.max(1.0) as u32 };
let energy = range as f32 * 40.0;
```

Apply backshot, aimshoot, and the seeded DB2 angular spread before creating the direction vector.

- [ ] **Step 5: Replace instant target selection with projectile requests**

`interactions_phase` collects shot commands but does not choose a target or apply damage. It spawns a projectile and clears `.shoot`, `.shootval`, `.backshot`, and `.aimshoot` according to DB2 one-cycle semantics.

- [ ] **Step 6: Run projectile creation and save round-trip tests**

Run: `cargo test -p darwinbots-engine --test db2_projectiles --test shot_state --test contracts`

Expected: PASS with live projectile state surviving save/load.

- [ ] **Step 7: Commit projectile storage**

```powershell
git add modern/engine/src/shot.rs modern/engine/src/shot modern/engine/src/lib.rs modern/engine/src/world.rs modern/engine/tests/db2_projectiles.rs modern/engine/tests/shot_state.rs
git commit -m "Add persistent DB2 projectiles"
```

### Task 6: Port Projectile Motion, Decay, Collision, and Effects

**Files:**
- Modify: `modern/engine/src/shot/projectile.rs`
- Modify: `modern/engine/src/shot/effects.rs`
- Modify: `modern/engine/src/physics/collision.rs`
- Modify: `modern/engine/src/spatial.rs`
- Modify: `modern/engine/src/world.rs`
- Modify: `modern/engine/tests/db2_projectiles.rs`
- Modify: `modern/engine/tests/world_systems.rs`
- Modify: `modern/engine/tests/species_ecology.rs`
- Modify: `modern/engine/tests/shot_state.rs`

**Interfaces:**
- Consumes: projectile pool, organism/corpse positions and radii, spatial segment candidates, and biology/lifecycle buffers.
- Produces: `ProjectileImpact` records and updated world effects/statistics.

- [ ] **Step 1: Write failing travel, expiry, swept-hit, and effect tests**

```rust
#[test]
fn projectile_moves_once_per_tick_and_expires_after_range() {
    let mut engine = projectile_fixture_without_target();
    engine.tick().unwrap();
    let first = engine.snapshot().shots[0].clone();
    engine.tick().unwrap();
    let second = engine.snapshot().shots[0].clone();
    assert_eq!(second.start, first.end);
    for _ in 0..20 { engine.tick().unwrap(); }
    assert!(engine.snapshot().shots.is_empty());
}

#[test]
fn swept_collision_hits_a_bot_between_projectile_endpoints() {
    let mut engine = swept_collision_fixture();
    let target = target_id(&engine);
    let before = engine.organism(target).unwrap().energy;
    engine.tick().unwrap();
    assert!(engine.organism(target).unwrap().energy < before);
}

#[test]
fn newborn_is_immune_to_parent_stream_for_one_tick() {
    let mut engine = parent_shot_stream_fixture();
    engine.tick().unwrap();
    assert_eq!(newborn_energy(&engine), newborn_initial_energy());
}
```

- [ ] **Step 2: Run tests and observe missing motion/impact behavior**

Run: `cargo test -p darwinbots-engine --test db2_projectiles`

Expected: FAIL because projectiles are only stored, not advanced or collided.

- [ ] **Step 3: Add spatial segment queries and swept-circle intersection**

```rust
pub(crate) fn segment_circle_fraction(start: [f32; 2], end: [f32; 2], center: [f32; 2], radius: f32) -> Option<f32> {
    let delta = [end[0] - start[0], end[1] - start[1]];
    let offset = [start[0] - center[0], start[1] - center[1]];
    let a = delta[0] * delta[0] + delta[1] * delta[1];
    let b = 2.0 * (offset[0] * delta[0] + offset[1] * delta[1]);
    let c = offset[0] * offset[0] + offset[1] * offset[1] - radius * radius;
    let discriminant = b * b - 4.0 * a * c;
    if a <= f32::EPSILON || discriminant < 0.0 { return None; }
    let root = (-b - discriminant.sqrt()) / (2.0 * a);
    (0.0..=1.0).contains(&root).then_some(root)
}
```

`SpatialIndex::segment_candidates(start, end, padding)` returns unique candidate slots from all crossed cells. Select the smallest collision fraction.

- [ ] **Step 4: Port aging and nonlinear energy decay**

For each survivor: copy current to previous, add velocity, test collision, increment age unless DB2 no-decay applies, calculate `tempnum = age / range`, and apply the VB6 arctangent decay ratio. Mark an impact as a one-tick flash and remove it on the following update. Remove unimpacted shots when `age > range`.

- [ ] **Step 5: Move all shot effects behind typed impacts**

```rust
pub(crate) enum ProjectileEffect {
    ReleaseEnergy,
    DonateEnergy,
    Venom,
    Waste,
    Poison,
    ReleaseBody,
    AddGene,
    Sperm,
    WriteMemory { address: i32, value: i32 },
    ForceReproduction { percentage: i32 },
}
```

`apply_impact` receives one effect and mutates lifecycle/biology/DNA state once. Preserve shell absorption, corpse multipliers, energy return shots, poison countershots, parent/newborn immunity, and existing statistics.

- [ ] **Step 6: Run projectile and existing ecology tests**

Run: `cargo test -p darwinbots-engine --test db2_projectiles --test world_systems --test species_ecology --test shot_state --test corpses_collisions`

Expected: PASS with no instant-target assumptions remaining.

- [ ] **Step 7: Commit projectile behavior**

```powershell
git add modern/engine/src/shot modern/engine/src/physics/collision.rs modern/engine/src/spatial.rs modern/engine/src/world.rs modern/engine/tests/db2_projectiles.rs modern/engine/tests/world_systems.rs modern/engine/tests/species_ecology.rs modern/engine/tests/shot_state.rs
git commit -m "Port DB2 projectile behavior"
```

### Task 7: Initialize and Inherit DB2 Vegetable Biology

**Files:**
- Modify: `modern/engine/src/biology.rs`
- Modify: `modern/engine/src/species.rs`
- Modify: `modern/engine/src/world.rs`
- Modify: `modern/engine/tests/db2_vegetation.rs`
- Modify: `modern/engine/tests/species_ecology.rs`

**Interfaces:**
- Consumes: species vegetable flag and `VegetationSettings.start_chloroplasts`.
- Produces: `BiologyState::for_species`, `split_for_offspring`, and correctly initialized/reproduced plants.

- [ ] **Step 1: Write failing initialization and inheritance tests**

```rust
#[test]
fn initial_and_repopulated_vegetables_start_with_db2_chloroplasts() {
    let mut engine = vegetable_fixture(1, true);
    let initial = engine.snapshot().organisms[0].clone();
    assert_eq!(initial.chloroplasts, 16_000);
    engine.remove(initial.id).unwrap();
    engine.tick_many(10).unwrap();
    assert_eq!(engine.snapshot().organisms[0].chloroplasts, 16_000);
}

#[test]
fn vegetable_reproduction_splits_parent_biology_without_resetting_child() {
    let mut engine = reproducing_vegetable_fixture();
    engine.tick().unwrap();
    let plants = vegetable_snapshots(&engine);
    assert_eq!(plants.len(), 2);
    assert_eq!(plants.iter().map(|bot| bot.chloroplasts).sum::<i32>(), 16_000);
}
```

- [ ] **Step 2: Run tests and observe zero-chloroplast failure**

Run: `cargo test -p darwinbots-engine --test db2_vegetation`

Expected: FAIL because all new biology currently starts with zero chloroplasts.

- [ ] **Step 3: Add species-aware biology initialization**

```rust
pub(crate) fn for_species(vegetable: bool, start_chloroplasts: i32) -> Self {
    Self { chloroplasts: if vegetable { start_chloroplasts.clamp(0, 32_000) } else { 0 }, ..Self::default() }
}
```

Use this path for initial imports and reseeding. Do not apply it to ordinary offspring.

- [ ] **Step 4: Split parent biology during reproduction**

Use the same reproduction percentage as energy. Split body, chloroplasts, shell, slime, venom, poison, and waste using integer-safe proportional transfer. Keep at least one body point with the parent and child where DB2 lifecycle requires a living body.

- [ ] **Step 5: Run vegetation and lifecycle tests**

Run: `cargo test -p darwinbots-engine --test db2_vegetation --test species_ecology --test mutation_lifecycle`

Expected: PASS with conserved transferable biology.

- [ ] **Step 6: Commit vegetable initialization**

```powershell
git add modern/engine/src/biology.rs modern/engine/src/species.rs modern/engine/src/world.rs modern/engine/tests/db2_vegetation.rs modern/engine/tests/species_ecology.rs
git commit -m "Initialize DB2 vegetable biology"
```

### Task 8: Port DB2 Light Feeding and Repopulation

**Files:**
- Create: `modern/engine/src/vegetation.rs`
- Modify: `modern/engine/src/lib.rs`
- Modify: `modern/engine/src/world.rs`
- Modify: `modern/engine/src/biology.rs`
- Modify: `modern/engine/tests/db2_vegetation.rs`
- Modify: `modern/engine/tests/starter_ecology.rs`

**Interfaces:**
- Consumes: world geometry, obstacles, organism radii/positions, biology, lifecycle energy, and `VegetationSettings`.
- Produces: `VegetationRuntime::feed`, `VegetationRuntime::advance_repopulation`, light sysvars, and plant-energy statistics.

- [ ] **Step 1: Write failing feeding and cooldown tests**

```rust
#[test]
fn full_chloroplast_plant_receives_db2_light_and_body_split() {
    let mut engine = isolated_full_chlr_plant_fixture();
    let before = engine.snapshot().organisms[0].clone();
    engine.tick().unwrap();
    let after = engine.snapshot().organisms[0].clone();
    assert!(after.energy > before.energy);
    assert!(after.body > before.body);
    assert_eq!(engine.memory_at(after.id, 923).unwrap(), 1);
}

#[test]
fn darkness_prevents_chloroplast_feeding() {
    let mut engine = night_plant_fixture();
    let before = engine.snapshot().organisms[0].energy;
    engine.tick().unwrap();
    assert!(engine.snapshot().organisms[0].energy <= before);
}

#[test]
fn repopulation_waits_for_cooldown_and_spawns_configured_batch() {
    let mut engine = depleted_vegetable_fixture(3, 2);
    engine.tick_many(2).unwrap();
    assert_eq!(engine.vegetable_population(), 0);
    engine.tick().unwrap();
    assert_eq!(engine.vegetable_population(), 2);
}
```

- [ ] **Step 2: Run tests and observe flat-bonus behavior**

Run: `cargo test -p darwinbots-engine --test db2_vegetation`

Expected: FAIL because the engine applies unconditional integer vegetable energy and count-based immediate reseeding.

- [ ] **Step 3: Implement light calculation and fractional carry**

```rust
pub(crate) fn photosynthesis_delta(input: PlantLightInput) -> f32 {
    if !input.daytime || input.chloroplasts <= 0 { return 0.0; }
    let light_available = (input.total_robot_area / input.usable_world_area.max(1.0)).clamp(0.0, 1.0);
    let area_correction = (1.0 - light_available).powi(2) * 4.0;
    let mut token = input.max_energy_per_tick.max(0) as f32;
    if input.pond_mode { token = input.light_intensity / input.depth.max(1.0).powf(input.gradient); }
    token = token.max(0.0) / 3.5;
    let chloroplast_correction = input.chloroplasts as f32 / 16_000.0;
    let add_rate = area_correction * chloroplast_correction * 1.25;
    let subtract_rate = (input.chloroplasts as f32 / 32_000.0).powi(2);
    (add_rate - subtract_rate) * token - input.age as f32 * input.chloroplasts as f32 / 1_000_000_000.0
}
```

Store fractional energy and body carries on `BiologyState`; transfer only whole units into integer lifecycle/body buffers and retain the remainder.

- [ ] **Step 4: Replace unconditional vegetable bonus**

Remove the normal `vegetable_energy_per_tick` addition from lifecycle. Keep the old field as an explicit compatibility/debug flat bonus defaulting to zero. Publish `.light = 1` in lit regions and `.light = 0` in darkness; publish `.availability` from the DB2 availability calculation.

- [ ] **Step 5: Port chloroplast-equivalent repopulation**

`VegetationRuntime` stores a serializable cooldown counter. Calculate `total_chloroplasts / 16_000`; below the minimum, increment cooldown. On expiry, spawn `repopulation_amount` eligible reseeding vegetables at seeded random positions, bounded by organism capacity and the hard vegetable cap, then subtract the cooldown interval.

- [ ] **Step 6: Run vegetation and starter tests**

Run: `cargo test -p darwinbots-engine --test db2_vegetation --test species_ecology --test starter_ecology`

Expected: PASS with no flat-bonus dependency.

- [ ] **Step 7: Commit DB2 vegetation**

```powershell
git add modern/engine/src/vegetation.rs modern/engine/src/lib.rs modern/engine/src/world.rs modern/engine/src/biology.rs modern/engine/tests/db2_vegetation.rs modern/engine/tests/starter_ecology.rs
git commit -m "Port DB2 vegetable ecology"
```

### Task 9: Integrate Physics, Projectiles, and Vegetation in Tick Order

**Files:**
- Create: `modern/engine/tests/db2_starter_ecology.rs`
- Modify: `modern/engine/src/world.rs`
- Modify: `modern/engine/src/stats.rs`
- Modify: `modern/engine/tests/starter_ecology.rs`
- Modify: `modern/engine/tests/history_records.rs`

**Interfaces:**
- Consumes: completed physics, projectile, and vegetation modules.
- Produces: DB2 phase ordering, integrated statistics, and a representative starter-world acceptance test.

- [ ] **Step 1: Write the failing integrated starter test**

```rust
#[test]
fn starter_world_sustains_moving_predators_finite_shots_and_bounded_plants() {
    let mut engine = randomized_starter_world(300, 100, 500);
    let initial_animals = animal_positions(&engine);
    engine.tick_many(20_000).unwrap();
    let snapshot = engine.snapshot();
    let animals = species_organisms(snapshot, "Animal Minimalis");
    let moving = animals.iter().filter(|bot| bot.velocity[0].hypot(bot.velocity[1]) > 0.1).count();
    let displaced = animals.iter().filter(|bot| displacement_from_founder(bot, &initial_animals) > 250.0).count();

    assert!(moving > animals.len() / 20);
    assert!(displaced > animals.len() / 4);
    assert!(snapshot.shots.iter().all(|shot| segment_length(shot.start, shot.end) <= 200.0));
    assert!(snapshot.shots.len() < snapshot.stats.shots_fired as usize);
    assert!(engine.vegetable_population() <= 500);
    assert!(snapshot.stats.projectile_impacts > 0);
    assert!(snapshot.stats.energy_harvested > 0);
    assert!(snapshot.stats.births > 0 && snapshot.stats.deaths > 0);
}
```

- [ ] **Step 2: Run the integrated test and observe ordering/counter failures**

Run: `cargo test -p darwinbots-engine --test db2_starter_ecology -- --nocapture`

Expected: FAIL until the world phase order and new statistics are connected.

- [ ] **Step 3: Reorder `tick_internal` explicitly**

```rust
self.publish_prior_senses_phase()?;
self.execute_dna_phase()?;
self.voluntary_impulse_phase()?;
self.environment_force_phase();
self.physics_integration_phase()?;
self.spatial_index_phase();
self.projectile_spawn_phase();
self.projectile_update_phase()?;
self.tie_interactions_phase();
self.vegetation_phase()?;
self.lifecycle_phase()?;
self.mutation_phase();
```

Retain per-phase timing fields and add `projectiles` and `vegetation` timings rather than folding them into interactions/lifecycle.

- [ ] **Step 4: Add statistics for created shots and impacts**

Keep `shots_fired` as accepted projectile creation count. Add `projectile_impacts` and `plant_energy_generated`. Update history and snapshots with serde defaults.

- [ ] **Step 5: Run integrated and invariant tests**

Run: `cargo test -p darwinbots-engine --test db2_starter_ecology --test starter_ecology --test history_records --test contracts`

Expected: PASS with births, deaths, movement, projectile impacts, and feeding all nonzero.

- [ ] **Step 6: Commit integrated tick behavior**

```powershell
git add modern/engine/src/world.rs modern/engine/src/stats.rs modern/engine/tests/db2_starter_ecology.rs modern/engine/tests/starter_ecology.rs modern/engine/tests/history_records.rs
git commit -m "Integrate DB2 simulation phases"
```

### Task 10: Preserve GPU Acceleration and CPU Fallback

**Files:**
- Modify: `modern/engine/src/physics.rs`
- Modify: `modern/engine/src/physics.wgsl`
- Modify: `modern/engine/src/world.rs`
- Modify: `modern/engine/tests/gpu_ffi.rs`
- Modify: `modern/engine/tests/environment_world.rs`

**Interfaces:**
- Consumes: CPU-computed velocities and projectile broad-phase requirements.
- Produces: GPU position integration compatible with DB2 force output and explicit CPU fallback for unsupported fused phases.

- [ ] **Step 1: Write a CPU/GPU differential test with retained momentum**

```rust
#[test]
fn gpu_position_integration_matches_cpu_after_db2_forces_when_adapter_is_available() {
    let Some((mut cpu, mut gpu)) = paired_db2_physics_engines() else { return; };
    cpu.tick_many(200).unwrap();
    gpu.tick_many(200).unwrap();
    for (left, right) in cpu.snapshot().organisms.iter().zip(&gpu.snapshot().organisms) {
        assert!((left.position[0] - right.position[0]).abs() < 0.05);
        assert!((left.position[1] - right.position[1]).abs() < 0.05);
        assert!((left.velocity[0] - right.velocity[0]).abs() < 0.05);
        assert!((left.velocity[1] - right.velocity[1]).abs() < 0.05);
    }
}
```

- [ ] **Step 2: Run the differential test**

Run: `cargo test -p darwinbots-engine --test gpu_ffi gpu_position_integration_matches_cpu_after_db2_forces_when_adapter_is_available -- --nocapture`

Expected: FAIL if fused sensing/integration bypasses CPU-computed DB2 forces.

- [ ] **Step 3: Make force accumulation backend-independent**

Both backends consume the same post-force velocity buffer. GPU kernels integrate positions and prepare render instances only. Disable fused sensing/integration whenever it would recompute velocity from stale pre-force values.

```rust
let fusion_safe = self.ties.is_empty()
    && self.projectiles.is_empty()
    && self.config.physics.surface_gravity == 0.0
    && self.config.physics.density == 0.0;
```

Projectile integration and impact effects remain CPU in this release; their storage stays GPU-upload compatible.

- [ ] **Step 4: Verify fallback after forced GPU failure**

Run: `cargo test -p darwinbots-engine --test gpu_ffi`

Expected: PASS or adapter-dependent skip with existing structured fallback reason.

- [ ] **Step 5: Commit GPU contract changes**

```powershell
git add modern/engine/src/physics.rs modern/engine/src/physics.wgsl modern/engine/src/world.rs modern/engine/tests/gpu_ffi.rs modern/engine/tests/environment_world.rs
git commit -m "Align GPU integration with DB2 physics"
```

### Task 11: Update Desktop Settings and Starter-Mode Semantics

**Files:**
- Create: `modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Db2EnvironmentSettingsTests.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/EnvironmentUpdate.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/WorldSetupOptions.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/NativeEngineClient.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Views/SetupWindow.axaml.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Views/AdvancedSettingsWindow.axaml`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Views/AdvancedSettingsWindow.axaml.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Views/MainWindow.axaml.cs`
- Modify: existing setup and native-client tests.

**Interfaces:**
- Consumes: versioned Rust settings contract from Task 2.
- Produces: `Db2PhysicsOptions`, `Db2ShotOptions`, and `Db2VegetationOptions` records serialized by `NativeEngineClient`.

- [ ] **Step 1: Capture and generate the required visual blueprint**

Use Computer Use keyboard automation to focus Advanced Settings, send `Alt + Print Screen`, save the clipboard image, and inspect it. Use image generation/editing at high reasoning to create a blueprint preserving the existing warm DB2-style visual language while grouping Physics, Shots, and Vegetation in the current scrollable advanced panel. Save the blueprint under `docs/design/advanced-settings-db2-blueprint.png`.

- [ ] **Step 2: Write failing settings and starter-mode tests**

```csharp
[Fact]
public void StarterModeUsesNormalMetabolismEvenWhenZerobotSustenanceDefaultsToDisabled()
{
    var options = WorldSetupOptionsFactory.CreateForTest(StartingMode.StarterBotsAndVegetables, ZerobotSustenance.DisabledMetabolism);
    Assert.Equal(1, options.MetabolismCost);
}

[Fact]
public void Db2SettingsSerializeIntoVersionedEnvironmentCommand()
{
    var update = Db2EnvironmentFixtures.Normal;
    var json = NativeCommandSerializer.SerializeEnvironment(update);
    Assert.Contains("\"max_velocity\":60", json);
    Assert.Contains("\"movement_efficiency\":0.66", json);
    Assert.Contains("\"speed\":40", json);
    Assert.Contains("\"start_chloroplasts\":16000", json);
}
```

- [ ] **Step 3: Run desktop tests and observe failures**

Run: `dotnet test modern\desktop\tests\Darwinbots.Desktop.Core.Tests\Darwinbots.Desktop.Core.Tests.csproj -c Release --no-restore --filter "Db2EnvironmentSettingsTests|StarterMode"`

Expected: FAIL because the records and scoped metabolism behavior do not exist.

- [ ] **Step 4: Add immutable .NET setting records**

```csharp
public sealed record Db2PhysicsOptions(
    float MaxVelocity, float MovementEfficiency, float SurfaceGravity,
    float StaticFriction, float KineticFriction, double Density,
    double Viscosity, float Elasticity);

public sealed record Db2ShotOptions(
    float Speed, float RangeMultiplier, float Decay,
    bool EnergyShotsDoNotDecay, bool WasteShotsDoNotDecay);

public sealed record Db2VegetationOptions(
    int StartChloroplasts, int MaxEnergyPerTick,
    int MinimumChloroplastEquivalents, int RepopulationAmount,
    ulong RepopulationCooldown, float FeedingToBody,
    bool Daytime, bool DayNightEnabled, ulong CycleLength);
```

Embed them in `WorldSetupOptions` and `EnvironmentUpdate`, and serialize nested snake_case objects matching Rust.

- [ ] **Step 5: Scope Zerobot sustenance**

```csharp
var metabolism = CurrentMode() == StartingMode.StarterBotsAndVegetables
    ? _metabolismCost
    : sustenance == ZerobotSustenance.DisabledMetabolism ? 0 : _metabolismCost;
```

Disable the sustenance selector visually when Starter Bots + Vegetables is selected and add explanatory text without changing the setup layout.

- [ ] **Step 6: Extend Advanced Settings from the blueprint**

Add grouped numeric controls with DB2 defaults and validation ranges. Existing live settings remain. The dialog returns a complete `EnvironmentUpdate`; `MainWindow` stores and resubmits the complete record after each accepted edit.

- [ ] **Step 7: Run desktop tests**

Run: `dotnet test modern\desktop\tests\Darwinbots.Desktop.Core.Tests\Darwinbots.Desktop.Core.Tests.csproj -c Release --no-restore`

Expected: PASS with the new records and prior settings tests.

- [ ] **Step 8: Build and compare the visual surface**

Run: `dotnet publish modern\desktop\src\Darwinbots.Desktop\Darwinbots.Desktop.csproj -c Release -r win-x64 --self-contained true -o modern\dist\win-x64`

Launch the packaged app, capture Setup and Advanced Settings through `Alt + Print Screen`, and compare against the blueprint. Correct clipping, grouping, and keyboard navigation before committing.

- [ ] **Step 9: Commit desktop settings**

```powershell
git add docs/design/advanced-settings-db2-blueprint.png modern/desktop/src modern/desktop/tests
git commit -m "Expose DB2 simulation settings"
```

### Task 12: Update Projectile Snapshot Rendering and Telemetry

**Files:**
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/EngineSnapshot.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/NativeSnapshotParser.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Controls/WorldViewport.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop/ViewModels/MainWindowViewModel.cs`
- Modify: desktop parser/view-model tests.

**Interfaces:**
- Consumes: Rust projectile snapshots and new statistics fields.
- Produces: short projectile segments, one-tick impact flashes, and separate fired/impact telemetry.

- [ ] **Step 1: Write failing snapshot-parser tests**

```csharp
[Fact]
public void ParsesProjectileVelocityAgeRangeAndImpactState()
{
    var snapshot = NativeSnapshotParser.Parse(Db2SnapshotFixtures.OneProjectile, "CPU");
    var shot = Assert.Single(snapshot.Shots);
    Assert.Equal(new[] { 40f, 0f }, shot.Velocity);
    Assert.Equal(2u, shot.Age);
    Assert.Equal(7u, shot.Range);
    Assert.False(shot.ImpactFlash);
}
```

- [ ] **Step 2: Run parser tests and observe missing fields**

Run: `dotnet test modern\desktop\tests\Darwinbots.Desktop.Core.Tests\Darwinbots.Desktop.Core.Tests.csproj -c Release --no-restore --filter NativeSnapshotParser`

Expected: FAIL to compile because projectile fields do not exist.

- [ ] **Step 3: Extend snapshot records and parser**

```csharp
public sealed record ShotSnapshot(
    OrganismKey Owner, float[] Start, float[] End, float[] Velocity,
    int Kind, int Value, uint Age, uint Range, bool ImpactFlash);
```

Add `ProjectileImpacts` and `PlantEnergyGenerated` to statistics with zero defaults for missing JSON.

- [ ] **Step 4: Render short motion segments and flashes**

Draw `Start -> End` with one-pixel DB2 shot colors. For `ImpactFlash`, draw a small 20-world-unit ring at `End` for the one published tick. Never extend a shot line to an organism target.

- [ ] **Step 5: Run desktop tests**

Run: `dotnet test modern\desktop\tests\Darwinbots.Desktop.Core.Tests\Darwinbots.Desktop.Core.Tests.csproj -c Release --no-restore`

Expected: PASS.

- [ ] **Step 6: Commit projectile rendering**

```powershell
git add modern/desktop/src/Darwinbots.Desktop.Core/EngineSnapshot.cs modern/desktop/src/Darwinbots.Desktop.Core/NativeSnapshotParser.cs modern/desktop/src/Darwinbots.Desktop/Controls/WorldViewport.cs modern/desktop/src/Darwinbots.Desktop/ViewModels/MainWindowViewModel.cs modern/desktop/tests
git commit -m "Render persistent DB2 projectiles"
```

### Task 13: Full Verification, Packaged Playtest, and Publication

**Files:**
- Modify only files required by failures discovered during this task.
- Output: `modern/dist/win-x64/`
- Output screenshots: `docs/verification/db2-simulation/`

**Interfaces:**
- Consumes: all prior tasks.
- Produces: verified Windows alpha package and GitHub publication.

- [ ] **Step 1: Run the full Rust suite**

Run: `cargo test --workspace`

Expected: all engine, CLI, GPU/fallback, persistence, DNA, and ecology tests pass.

- [ ] **Step 2: Run the full desktop suite**

Run: `dotnet test modern\desktop\tests\Darwinbots.Desktop.Core.Tests\Darwinbots.Desktop.Core.Tests.csproj -c Release --no-restore`

Expected: all desktop tests pass with zero failures.

- [ ] **Step 3: Run headless stress and benchmark receipts**

Run: `cargo run -p darwinbots-cli --release -- stress --bot "Installer\bots\Animal_Minimalis.txt" --ticks 1000000 --population 1000 --seed 7`

Expected: completes one million ticks with invariant checks, bounded memory, and a final state hash.

Run: `cargo run -p darwinbots-cli --release -- bench --bot "Installer\bots\Animal_Minimalis.txt" --population 100000 --ticks 300 --backend auto`

Expected: reports selected backend, elapsed time, ticks per second, and population without an engine error.

- [ ] **Step 4: Publish the self-contained Windows folder**

Run: `dotnet publish modern\desktop\src\Darwinbots.Desktop\Darwinbots.Desktop.csproj -c Release -r win-x64 --self-contained true -o modern\dist\win-x64`

Expected: `modern/dist/win-x64/Darwinbots.Desktop.exe` and the native Rust library are replaced successfully.

- [ ] **Step 5: Conduct a controlled fresh-world visual playtest**

Create Starter Bots + Vegetables with defaults. Run at 1x for 200 ticks, then Maximum for at least 20,000 ticks. At setup, tick 200, tick 2,000, and tick 20,000, use Computer Use keyboard automation to send `Alt + Print Screen`, save clipboard PNGs under `docs/verification/db2-simulation/`, and inspect them.

Acceptance evidence:

- Animal Minimalis changes position and retains visible momentum after firing starts.
- At least 5 percent of living animals have nonzero velocity at the long-run sample.
- Shots are short moving particles or impact flashes and do not remain attacker-to-target beams.
- Vegetable count stays at or below the hard cap.
- Plants initialize with chloroplasts and recover after deliberate depletion.
- Births, deaths, projectile impacts, feeding, and plant energy are all nonzero.
- The application remains responsive and does not freeze during the observation period.

- [ ] **Step 6: Verify patch hygiene and commit any acceptance corrections**

Run: `git diff --check`

Expected: no whitespace errors. Stage only files changed by this plan and commit each independently reviewable correction rather than combining unrelated failures.

- [ ] **Step 7: Push and merge safely**

If execution occurred directly on `master`, run `git push origin master`; no merge is required. If execution used a `codex/` worktree branch, push it, merge with `git merge --no-ff <branch>` from a clean `master` worktree, rerun the full suites, and push `master`. Never reset or overwrite unrelated work.

---

## Plan Self-Review Results

- Spec coverage: every included physics, projectile, vegetation, persistence, GPU-boundary, settings, and visual-acceptance requirement maps to at least one task.
- Scope: the three systems remain integrated because projectile feeding and momentum directly determine vegetable ecology; splitting them would not produce independently playable acceptance states.
- Placeholder scan: the plan contains no deferred implementation markers. Each behavior task names concrete files, interfaces, tests, commands, expected failures, implementation formulas, and commit boundaries.
- Type consistency: Rust settings names match the snake_case FFI payload and the .NET records; projectile snapshot fields match between Rust and C#; statistics names remain stable from engine through parser and UI.
- Architecture: formula code is removed from the world coordinator into focused modules while current CPU/GPU backend ownership remains intact.
