# DB2 2.48.32 Phase 1 Foundation Acceptance

## Scope

| Field | Receipt |
|---|---|
| Checkpoint | Phase 1 foundation acceptance; independent documentation review pending |
| Accepted integration HEAD | `7e9f0ea088d92d89a21150301e4b9b31ed24d649` |
| Branch | `codex/db24832-parity` |
| Authoritative references | [Product specification](../reference/db24832/DARWINBOTS_2_PRODUCT_SPEC.md), [Phase 1 plan](../superpowers/plans/2026-07-30-db24832-phase1-foundation-implementation.md), and [gap register](db24832-gap-register.md) |
| Boundary | This checkpoint accepts the state, DNA/VM, sysvar lifecycle, scheduler/trace, persistence/ABI, and bounded GPU foundation. It is not complete DB2 product, mechanics, frontend, legacy-interchange, or long-run GPU parity. |

## Integration lineage

| Boundary | Integrated commits |
|---|---|
| Canonical robot state | `244a8f5`, `a230558` |
| DNA tokenizer and VM | `271ae2f` through `3275c81` |
| Sysvar lifecycle and selected tie routing | `6f82268`, `a49d360`, `eaee70e` |
| Canonical scheduler | `52ff7c3`, `e673009` |
| Save, ABI, and GPU contracts | `903750a`, `72caab7`, `c199406` |
| Acceptance fixture and integration repairs | `866d98f`, `6552f11`, `03f2afa`, `e091aff`, `7c95389`, `c5d71f5`, `864daf1`, `b28daca` |
| Reviewed strict-Clippy repair | Reviewed `1c445c5f3c1f91abe4c5aba49a877bdb7e421fa3`; integrated as `7e9f0ea088d92d89a21150301e4b9b31ed24d649`. Both have stable patch ID `3a975271496c76066d37d0febf133a617496c026`. |

## Executed gates

These rows are executable receipts. Source/static mappings are recorded separately.

| Command or target | Result at accepted HEAD |
|---|---|
| `cargo test --workspace` | PASS, exit 0: 230/230 tests; engine 227 plus CLI 3; 0 failed, 0 ignored; 1,533.6 seconds |
| Task 7 focused targets within the workspace run | PASS, 48/48: `dna_runtime_248` 9, `vm_costs_telemetry` 4, `robot_state_248` 4, `sysvar_lifecycle_248` 19, `canonical_cycle_248` 12 |
| `cargo clippy -p darwinbots-engine --all-targets -- -D warnings` | PASS, strict all-targets receipt at exact HEAD |
| `cargo check -p darwinbots-engine` | PASS at exact HEAD |
| `cargo fmt --all -- --check` | PASS at exact HEAD |
| `git diff --check` | PASS at exact HEAD |
| [`docs/reference/db24832/verify.ps1`](../reference/db24832/verify.ps1) | PASS, exit 0: 6/6 canonical documents verified |

The successful workspace run was the single root-cause-corrected retry after an environmental `LNK1318` PDB limit failure executed zero tests with about 76 MiB free. The retry began with 9,395,183,616 free bytes, used one Cargo build job, and did not change source.

## Reference hashes

| Canonical document | SHA-256 |
|---|---|
| [DARWINBOTS_2_PRODUCT_SPEC.md](../reference/db24832/DARWINBOTS_2_PRODUCT_SPEC.md) | `43146CB5D511CC046372DA797A4C07738FA3D4B90DD7BB91A9FF262CC2627832` |
| [UI_AND_INTERACTION_SPEC.md](../reference/db24832/UI_AND_INTERACTION_SPEC.md) | `CDEE2F72CF09F7259164E61B40D3535BECA1EE4217FE15C89A3DED90DE936B36` |
| [DNA_RUNTIME_SPEC.md](../reference/db24832/DNA_RUNTIME_SPEC.md) | `1A0C1C11E0C5DCFC41BFDFF19DD9CD35FCBE1F5292094868A4D67A6390F512D9` |
| [SYSVARS_REFERENCE.md](../reference/db24832/SYSVARS_REFERENCE.md) | `7DCC7399D97EA5B306E5611FE6433C84D923F482C9F91C34808BDB4002387701` |
| [DECLARATIVE_UI_INVENTORY.md](../reference/db24832/DECLARATIVE_UI_INVENTORY.md) | `C3CE0D2684ED7BE8FB6F925C54F52E225D208CA44C2C08DBAD022EE537C60595` |
| [HISTORY_AND_SOURCES.md](../reference/db24832/HISTORY_AND_SOURCES.md) | `DD7E062CE448944210D9EA564DF8D63558FFB178CCA6CCC644A8F1991EAF2CD6` |

## Source and static mapping

These rows map accepted source and executed test coverage. A mapping does not claim unrun behavior.

| Requirement | Receipt and boundary |
|---|---|
| Sysvar catalog/lifecycle | Independent source count and [`sysvar_lifecycle_248.rs`](../../modern/engine/tests/sysvar_lifecycle_248.rs): 255 names, 247 unique catalog addresses, 247 lifecycle rows, 247 unique lifecycle addresses, and zero differences in either set direction |
| Forbidden sysvar path | `sysvar_is_active`: 0 production hits. Known sysvars emit no inert warning; the only resolver warning is for unknown sysvars. The executed `known_sysvars_never_report_inert` test passed. |
| Typed command routing | [`PhaseCommands`](../../modern/engine/src/sysvar_runtime.rs) contains seven typed variants and no `String` fallthrough. Phase 2 behavior behind typed interfaces is not inferred. |
| DNA ordering | [`execute_dna_phase`](../../modern/engine/src/world.rs) uses one serial slot-index loop and contains no Rayon marker. Other parallel engine sites are outside DNA execution. |
| Canonical trace | [`canonical_cycle_248.rs`](../../modern/engine/tests/canonical_cycle_248.rs) passed 12/12. It covers the 56-phase manifest, three-cycle command/sensor visibility, post-DNA tied-write ordering, shot effects, same-cycle senses, and deferred birth/death commit. |
| Persistence | [`contracts.rs`](../../modern/engine/tests/contracts.rs) passed 25/25, including DB3S v2 full-field migration, parent-default field omission, and complete Phase 1 DB3S v3 round-trip. [`SaveFile`](../../modern/engine/src/persistence.rs) writes version 3. |
| ABI and snapshot | [`ABI_VERSION`](../../modern/engine/src/ffi.rs) is 1; the executed FFI contract tests passed. The snapshot schema source value is 2; the executed schema contract passed. |
| Task 5 ordering | The executed canonical target matched all 56 fixture phases. [`CyclePhase::ALL`](../../modern/engine/src/simulation_cycle.rs) has no diff from scheduler correction `e673009e1cf644558439e5984556842e4c7839f1`. |

## Physical GPU receipt

| Requirement | Executed receipt |
|---|---|
| Device | Physical `NVIDIA GeForce RTX 3050`; NVIDIA driver `581.80` |
| Gameplay state | `cpu_and_gpu_match_gameplay_state_within_existing_physics_tolerance` passed with CPU fallback disabled in the GPU engine configuration |
| Phase trace | `cpu_and_gpu_emit_identical_nontiming_cycle_traces` passed after removing wall-clock timing fields, with CPU fallback disabled |
| Extended state | `cpu_and_gpu_match_toxin_state_memory_and_position` passed with CPU fallback disabled |
| Post-submit rollback | Private physical-GPU tests passed for byte-identical rollback after submission failure, later traced tick recovery, and atomic `tick_many`; fallback-disabled failure remained on the GPU backend |
| Boundary | These tests establish the bounded Phase 1 differential and rollback contract, not every Phase 2 interaction or long ecological GPU parity. |

## Architecture boundary

| Constraint | Mapping |
|---|---|
| Rust/FFI authority | Rust owns mutable simulation state and mechanics. Desktop production uses [`NativeEngineClient`](../../modern/desktop/src/Darwinbots.Desktop.Core/NativeEngineClient.cs) command batches and projects native snapshots. |
| Integration ownership | [`world.rs`](../../modern/engine/src/world.rs) orchestrates cycle and backend integration around the canonical [`RobotState`](../../modern/engine/src/robot_state.rs) arena and [`SysvarRuntime`](../../modern/engine/src/sysvar_runtime.rs); it does not introduce a second authoritative robot or lifecycle store. |
| GPU preservation | The existing GPU backend and explicit gather/scatter boundary remain active; physical parity and failure receipts above executed on the accepted integration. |
| Offline scope | The three historical online handlers in [`MainWindow.axaml.cs`](../../modern/desktop/src/Darwinbots.Desktop/Views/MainWindow.axaml.cs) remain visible but disabled with an offline-scope explanation. No online-play acceptance claim is made. |
| Remaining work | The [gap register](db24832-gap-register.md) retains 29 open and 12 partial findings across mechanics, frontend, persistence/interchange, evolution, rendered evidence, and long-run GPU coverage. |
