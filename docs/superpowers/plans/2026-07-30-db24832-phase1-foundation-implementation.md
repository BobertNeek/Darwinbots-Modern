# DB2 2.48.32 Phase 1 Foundation Implementation Plan

> **For agentic workers:** Implement each task with test-first changes and an independent review. Later mechanics must use these interfaces without entering this phase's scope.

**Approved:** 2026-07-30

**Implementation base:** Commit `201aad7`. The Phase 0 checkpoint is independently clean.

**Goal:** Establish the canonical cycle, complete robot-state foundation, conformant DNA runtime, and lifecycle coverage for all DB2 2.48.32 sysvars.

**Architecture:** Keep Rust authoritative. Consolidate robot state behind a stable slot arena, run observable cycle work in canonical slot order, and retain GPU physics through an explicit gather/scatter adapter. Centralize DNA and sysvar behavior instead of scattering memory reads and clears through `world.rs`.

**Tech stack:** Rust, serde, Rayon, wgpu, existing JSON C ABI.

**Prioritized backlog:** [`docs/parity/db24832-gap-register.md`](../../../parity/db24832-gap-register.md)

## Current evidence

- `world.rs` is 2,496 lines. DNA runs with `par_iter_mut`, actions execute during DNA, and the phase order differs from section 4.
- Robot state is split among `Organism`, several SoA arrays, `BiologyState`, and pending vectors.
- The parser rejects unknown tokens, omits slash-header and appended-`end` behavior, and lacks metadata.
- The VM silently ignores accepted operators and has no 32,000-block limit, costs, or gene telemetry.
- Symbol lookup has all 255 names across 247 addresses, but `sysvar_is_active` deliberately leaves most behavior inert.

## Global constraints

- Use only the versioned DB2 2.48.32 reference package as behavioral authority.
- Do not use repository-local VB6 as authority.
- Rust remains the sole authority for simulation state and mechanics.
- Preserve stable `OrganismId` generation checks and deterministic seeds.
- Preserve GPU acceleration through the current backend and gather/scatter boundary.
- Preserve the explicit CPU fallback policy and report the active capability.
- Preserve current DB3S saves through an explicit migration.
- Do not implement desktop work or online play.
- Do not implement Phase 2 shot, tie, reproduction, physics, sense, mutation, or ecology semantics.
- Phase 1 may define their state and phase interfaces and route existing implementations through them.
- Do not claim Phase 2 parity from interface-only tests.

---

### Task 1: Consolidate canonical robot state

**Files**

- Create `modern/engine/src/robot_state.rs`
- Modify `modern/engine/src/world.rs`
- Modify `modern/engine/src/biology.rs`
- Modify `modern/engine/src/lib.rs`
- Test `modern/engine/tests/robot_state_248.rs`
- Test `modern/engine/tests/contracts.rs`

**Interfaces**

```rust
pub struct RobotState {
    pub identity: RobotIdentity,
    pub flags: RobotFlags,
    pub kinematics: RobotKinematics,
    pub materials: RobotMaterials,
    pub counters: RobotCounters,
    pub program: RobotProgramState,
    pub senses: RobotSenseState,
    pub effects: RobotEffectState,
    pub heredity: RobotHeredityState,
    pub ties: [Option<TieEndpointState>; 10],
    pub phenotype: VisualPhenotype,
}

impl Engine {
    pub fn robot_state(&self, id: OrganismId) -> Result<&RobotState, EngineError>;
}
```

`RobotProgramState` owns DNA, 1,000-cell sparse memory, 1,000 private variables, RNG, mutation metadata, cost remainder, debug text, current gene, condition count, and activation bitmap. The remaining groups cover section 3.3 identity, flags/restrictions, prior/current motion, materials, counters, eye/focus/touch/taste state, poison/venom forcing, stored virus/sperm, hereditary addresses, and epigenetic staging. They define storage only for later mechanisms.

`RobotArena` remains crate-private and provides `ids_in_slot_order`, `get`, `get_mut`, `spawn`, `remove`, `physics_batch`, and `apply_physics_batch`.

**Failing tests**

- `robot_state_contains_every_phase1_field`
- `slot_reuse_changes_generation_without_reordering_live_slots`
- `physics_batch_round_trip_changes_only_kinematics`
- `one_robot_has_one_writable_owner_for_energy_age_position_and_memory`

**Work**

- Replace `Organism`, `KinematicsSoa`, `LifecycleSoa`, parallel biology vectors, and forced-reproduction vectors with `RobotArena`.
- Keep sparse memory and private storage to avoid allocating 2,000 integers for every empty/default robot.
- Gather and scatter `PhysicsBatch` by stable ID. Do not change GPU kernels.
- Keep existing mechanism behavior mechanically equivalent during this state move.

**Gate:** `cargo test -p darwinbots-engine --test robot_state_248 --test contracts`

**Depends on:** none.

**Commit boundary:** `refactor: consolidate canonical robot state`

---

### Task 2: Implement the canonical DNA tokenizer and block model

**Files**

- Modify `modern/engine/src/dna.rs`
- Create `modern/engine/src/dna_tokens.rs`
- Modify `modern/engine/src/mutation.rs` only to consume the new block type
- Modify `modern/engine/src/lib.rs`
- Test `modern/engine/tests/dna_runtime_248.rs`
- Update the obsolete unknown-token assertion in `modern/engine/tests/contracts.rs`

**Interfaces**

```rust
pub const MAX_DNA_BLOCKS: usize = 32_000;

#[repr(u8)]
pub enum TokenClass {
    Number = 0, Dereference = 1, Basic = 2, Advanced = 3,
    Bitwise = 4, Condition = 5, Logic = 6, Store = 7,
    Reserved = 8, Flow = 9, End = 10,
}

pub struct DnaBlock { pub class: TokenClass, pub value: i32 }

pub struct DnaMetadata {
    pub generation: u32,
    pub mutations: u64,
    pub tag: String,
    pub legacy_hash: Option<String>,
    pub hash_valid: bool,
}
```

Retain `LegacyDna::parse`, `to_source`, `uses_new_move`, and compatibility warnings. Replace `instructions()` with `blocks()`.

**Failing tests**

- `slash_column_one_headers_are_ignored`
- `unknown_tokens_compile_to_number_zero`
- `missing_end_is_appended_within_32000_blocks`
- `metadata_hash_validates_and_round_trips`
- `defs_mixed_case_and_all_canonical_tokens_round_trip`
- `detokenization_never_emits_an_unmapped_command`

**Work**

- Parse tabs, apostrophe comments, slash-column-one lines, definitions, metadata, symbolic and numeric dereferences.
- Compile every documented token to a typed `tipo/value` block.
- Convert unknown source tokens to zero with a location warning.
- Append one master `end`; reject only malformed structural input that DB2 cannot classify.
- Preserve mutation behavior mechanically; mutation parity remains Phase 2.

**Gate:** `cargo test -p darwinbots-engine --test dna_runtime_248 --test contracts --test mutation_lifecycle`

**Depends on:** Task 1.

**Commit boundary:** `feat: add DB2 24832 DNA tokenizer`

---

### Task 3: Complete VM semantics, limits, costs, and telemetry

**Files**

- Create `modern/engine/src/dna_costs.rs`
- Modify `modern/engine/src/config.rs`
- Modify `modern/engine/src/vm.rs`
- Modify `modern/engine/src/robot_state.rs`
- Modify `modern/engine/src/lib.rs`
- Replace `modern/engine/tests/vm.rs` with table-driven coverage
- Test `modern/engine/tests/vm_costs_telemetry.rs`
- Retain `modern/engine/tests/vm_implicit_conditions.rs`

**Interfaces**

```rust
pub struct DnaCosts {
    pub number: f64, pub dereference: f64, pub basic: f64,
    pub advanced: f64, pub bitwise: f64, pub condition: f64,
    pub logic: f64, pub store: f64, pub flow: f64,
    pub dna_upkeep: f64, pub global_multiplier: f64,
}

pub struct VmContext<'a> {
    pub position: [f64; 2],
    pub costs: &'a DnaCosts,
    pub record_activations: bool,
}

pub enum VmTermination { End, ProgramEnd, BlockLimit }

pub struct VmReport {
    pub blocks_executed: u32,
    pub stores_applied: u32,
    pub conditions_evaluated: u32,
    pub current_gene: u16,
    pub activations: Vec<bool>,
    pub debug_text: String,
    pub energy_cost: f64,
    pub termination: VmTermination,
}

pub fn execute(
    &mut self,
    program: &LegacyDna,
    memory: &mut VmMemory,
    context: VmContext<'_>,
) -> Result<VmReport, EngineError>;
```

**Failing tests**

- `every_canonical_operator_has_stack_and_memory_results`
- `angle_dist_anglecmp_and_debug_commands_execute`
- `integer_and_boolean_stacks_evict_bottom_at_100`
- `empty_stacks_and_division_by_zero_use_db2_defaults`
- `stores_normalize_exact_positive_and_negative_32000_multiples`
- `rndstore_preserves_the_stored_value_sign`
- `execution_stops_at_32000_blocks`
- `gene_numbers_conditions_and_activations_match_flow`
- `token_and_relative_store_costs_are_charged_exactly`
- `zero_destination_store_has_zero_store_cost`

**Work**

- Remove string-command fallthrough.
- Implement every basic, advanced, bitwise, condition, logic, flow, and store operation from the runtime spec.
- Keep immediate writes and permissive empty-condition behavior.
- Accumulate fractional DNA costs in `RobotProgramState`; deduct whole energy deterministically.
- Charge DNA upkeep separately from executed-token cost.
- Record debug and activation data only when requested.

**Gate:** `cargo test -p darwinbots-engine --test vm --test vm_implicit_conditions --test vm_costs_telemetry`

**Depends on:** Task 2.

**Commit boundary:** `feat: complete DB2 DNA VM semantics`

---

### Task 4: Define and enforce the complete sysvar lifecycle

**Files**

- Create `modern/engine/src/sysvar_runtime.rs`
- Create `modern/engine/src/db2_sysvar_lifecycle_248.txt`
- Modify `modern/engine/src/sysvars.rs`
- Modify `modern/engine/src/biology.rs`
- Modify `modern/engine/src/world.rs`
- Modify `modern/engine/src/lib.rs`
- Test `modern/engine/tests/sysvar_lifecycle_248.rs`
- Update `modern/engine/tests/sensing_compatibility.rs`
- Update the inert-warning FFI assertion in `modern/engine/tests/gpu_ffi.rs`

**Interfaces**

```rust
pub enum SysvarRole { Persistent, Command, Sensor, Readback, Duplex }

pub struct SysvarLifecycle {
    pub address: u16,
    pub role: SysvarRole,
    pub consume_at: Option<CyclePhase>,
    pub refresh_at: Option<CyclePhase>,
    pub clear_at: Option<CyclePhase>,
}

pub enum PhaseCommands {
    Movement(MovementCommands), Aim(AimCommands), Shot(ShotCommands),
    Tie(TieCommands), Material(MaterialCommands),
    Reproduction(ReproductionCommands), Virus(VirusCommands),
}

pub enum PhaseReadbacks {
    Core(CoreReadbacks), Vision(VisionReadbacks), Contact(ContactReadbacks),
    Tie(TieReadbacks), Materials(MaterialReadbacks),
}
```

`SysvarRuntime` exposes `definition`, `clear_transients_after_dna`, `consume`, and `publish`.

**Failing tests**

- `canonical_table_has_255_names_and_247_addresses`
- `every_canonical_address_has_one_lifecycle_definition`
- `aliases_share_identical_lifecycle`
- `known_sysvars_never_report_inert`
- `commands_clear_at_their_declared_consumption_phase`
- `touch_taste_edge_and_tie_references_clear_after_dna`
- `late_sensors_replace_prior_values_without_clearing_private_memory`
- `future_mechanism_commands_are_typed_and_phase_routed`

**Work**

- Delete `sysvar_is_active`.
- Replace direct address constants and scattered clearing loops with the lifecycle table.
- Capture commands into typed phase DTOs before clearing memory.
- Accept injected readback DTOs for mechanisms completed later.
- Do not implement the later mechanism behind those DTOs.

**Gate:** `cargo test -p darwinbots-engine --test sysvar_lifecycle_248 --test sensing_compatibility --test gpu_ffi`

**Depends on:** Tasks 1 and 2.

**Commit boundary:** `feat: enforce complete sysvar lifecycle`

---

### Task 5: Install the canonical scheduler and trace

**Files**

- Create `modern/engine/src/simulation_cycle.rs`
- Modify `modern/engine/src/world.rs`
- Modify `modern/engine/src/timing.rs`
- Modify `modern/engine/src/stats.rs`
- Modify `modern/engine/src/lib.rs`
- Create `modern/engine/tests/fixtures/db24832/canonical_cycle_phases.json`
- Test `modern/engine/tests/canonical_cycle_248.rs`

**Interfaces**

```rust
#[repr(u16)]
pub enum CyclePhase { /* one variant per section 4.1 and 4.2 stage */ }

impl CyclePhase { pub const ALL: &'static [CyclePhase]; }

pub enum CycleEvent {
    PhaseStarted { tick: u64, phase: CyclePhase },
    RobotVisited { tick: u64, phase: CyclePhase, id: OrganismId },
    MemoryWrite { tick: u64, phase: CyclePhase, id: OrganismId,
                  address: u16, before: i32, after: i32 },
    Effect { tick: u64, phase: CyclePhase,
             source: Option<OrganismId>, target: OrganismId, kind: String },
    QueueChange { tick: u64, phase: CyclePhase,
                  id: OrganismId, kind: QueueChangeKind },
}

pub struct CycleTrace { pub tick: u64, pub events: Vec<CycleEvent> }

impl Engine {
    pub fn tick_with_trace(&mut self) -> Result<CycleTrace, EngineError>;
}
```

`tick()` uses the same scheduler with allocation-free tracing disabled.

**Failing tests**

- `trace_matches_the_complete_24832_phase_manifest`
- `dna_and_observable_robot_visits_use_ascending_slot_order`
- `three_cycle_sensor_and_command_visibility_is_canonical`
- `post_dna_tie_write_precedes_shot_and_reproduction_consumption`
- `shot_effect_occurs_after_dna_and_before_robot_passes`
- `births_and_deaths_commit_only_after_the_final_robot_pass`
- `one_tick_publishes_one_completed_cycle`

**Work**

- Increment the cycle before DNA.
- Run DNA serially in ascending live-slot order.
- Separate DNA evaluation from movement, material, shot, tie, and reproduction consumption.
- Order transient clearing, shots, prior-position capture, three robot passes, queued birth/death commit, actual velocity, objects, repopulation, sunlight, and publication exactly as section 4 requires.
- Split current large phases only enough to attach them to their canonical locations.
- Preserve current Phase 2 mechanism math.
- Invoke the existing CPU/GPU physics backend only at its declared integration phase.

**Gate:** `cargo test -p darwinbots-engine --test canonical_cycle_248 --test world_systems --test contracts`

**Depends on:** Tasks 1, 3, and 4.

**Commit boundary:** `feat: install canonical DB2 cycle scheduler`

---

### Task 6: Preserve save, snapshot, ABI, and GPU contracts

**Files**

- Modify `modern/engine/src/persistence.rs`
- Modify `modern/engine/src/ffi.rs`
- Modify `modern/engine/src/world.rs`
- Modify `modern/engine/tests/contracts.rs`
- Modify `modern/engine/tests/gpu_ffi.rs`
- Add `modern/engine/tests/fixtures/db3s_v2_phase1.bin`

**Interfaces**

- Add `schema_version: 2` to `Snapshot`.
- Add gene/cycle telemetry fields to `OrganismSnapshot`.
- Keep C `ABI_VERSION == 1`; all JSON additions are backward-compatible.
- Encode new saves as DB3S v3 and migrate DB3S v2 through a dedicated compatibility model.
- Keep `tick_with_trace` Rust-only during Phase 1.

**Failing tests**

- `db3s_v2_migrates_every_existing_robot_field`
- `db3s_v3_round_trip_preserves_complete_phase1_state`
- `snapshot_v2_exposes_phase1_telemetry`
- `ffi_v1_consumers_accept_the_additive_snapshot`
- `cpu_and_gpu_emit_identical_nontiming_cycle_traces`
- `cpu_and_gpu_match_gameplay_state_within_existing_physics_tolerance`
- `gpu_failure_does_not_partially_advance_the_cycle`

**Work**

- Migrate old split state into `RobotState`.
- Exclude optional trace buffers from ordinary saves.
- Compare CPU/GPU traces after removing wall-clock timings.
- Retain the current explicit fallback capability and reason.

**Gate:** `cargo test -p darwinbots-engine --test contracts --test gpu_ffi`

An unavailable physical GPU blocks the GPU acceptance receipt. It does not justify a false pass.

**Depends on:** Task 5.

**Commit boundary:** `feat: integrate phase1 state across save ABI and GPU`

---

### Task 7: Phase 1 acceptance

**Files**

- No new production scope.
- Update only affected existing regression tests and evidence documentation.

**Acceptance gates**

```powershell
cargo fmt --all -- --check
cargo test -p darwinbots-engine --test dna_runtime_248
cargo test -p darwinbots-engine --test vm_costs_telemetry
cargo test -p darwinbots-engine --test robot_state_248
cargo test -p darwinbots-engine --test sysvar_lifecycle_248
cargo test -p darwinbots-engine --test canonical_cycle_248
cargo test -p darwinbots-engine
cargo test --workspace
cargo clippy -p darwinbots-engine --all-targets -- -D warnings
```

Additionally require:

- Reference document hashes still match the approved design.
- Exactly 255 names and 247 lifecycle-covered addresses.
- No `sysvar_is_active`, inert-known-sysvar warning, string-command fallthrough, or parallel DNA execution remains.
- The golden cycle trace covers three cycles, competing tied writes, shot effects, sense timing, and deferred birth/death.
- DB3S v2 migration and v3 round-trip pass.
- A real GPU run matches CPU state and phase traces when GPU hardware is available.
- No Phase 2 mechanism receives a parity claim from interface-only tests.
- `world.rs` owns orchestration and integration, not duplicated robot or lifecycle state.

**Depends on:** Tasks 1-6.

## Dependency order

```text
Task 1 -> Task 2 -> Task 3
   `------------------.
Tasks 1 + 2 --------> Task 4
Tasks 1 + 3 + 4 ----> Task 5 -> Task 6 -> Task 7
```
