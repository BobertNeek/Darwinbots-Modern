# Darwinbots Modern Verification Ledger

## Current offline assessment — 2026-11-04

Target: functional Darwinbots 2.48.32 offline behavior on native Windows, with Windows/Linux/Intel macOS/Apple silicon CI. Internet Mode is removed from menus, toolbar, setup, advanced/global settings and teleporter choices. Local and file teleporters remain supported. Historical VB6/helper sources remain reference material.

Baseline: `9dc4c91`, plus documentation corrections from `181821d`. The current work also fixes a real route gap: **Edit → Species previously opened mutation settings**. The repaired editor applies an atomic startup/repopulation roster without replacing living robots, preserves stable IDs, supports cancellation and restart with the edited roster, and retains source DNA, comments and placement bounds. Removed species retain their living identities and export source while repopulation is disabled. Their retirement survives modern save/resume.

Other current repairs: Internet-marked legacy gateways cannot become active local gateways even when an old local flag remains set; hidden Internet metadata remains readable and round-trippable; event diagnostics scroll and wrap at normal window sizes; corrupted status text and stale six-tab expectations are corrected. Surviving menu command identifiers keep their numeric values. No ABI or snapshot version is changed; the JSON command protocol gains `update_species_settings`, and configuration entries expose their world-local `species_id`. Optional template metadata uses backward-compatible defaults when reading older modern saves.

### Fresh feature checklist

The checklist maps the pinned original behavior to production paths and executable checks. A passing domain means its covered scenarios passed; it does not imply universal historical-file or arbitrary-program equivalence. Qualification is currently in progress; final results and package identity will be recorded below before merge.

| Feature contract | Production boundary | Behavioral checks required for closure |
| --- | --- | --- |
| Cycle/state timing | Canonical scheduler and slot-ordered native robot pipeline | `canonical_cycle_248`, `core_state_contracts`, `native_continuation_qualification` |
| DNA, stacks, flow, costs and sysvars | DNA parser/VM and native sysvar lifecycle | `dna_runtime_248`, `sysvar_lifecycle_248`, `vm`, shipped DNA corpus probes |
| Physics, topology and nine-eye/touch/taste senses | Native physics, sensing, collisions and vision | `db2_physics`, `native_reference_senses_248`, `touch_taste_248`, `toroidal_world` |
| Shots, feeding, defenses, viruses and sperm | Native projectile and toxin pipelines | `db2_projectiles`, `shot_state`, native genuine-feeding controls, virus/sexual cases |
| Ties, multibots and resource/information sharing | Native tie creation, physics, sharing and readback | `ties_multibot`, ordered cycle fixtures, native organism interchange |
| Reproduction and genealogy | Native birth/heredity and lineage bookkeeping | `reproduction_heredity_248`, `robot_state_248`, rendered phylogeny/details tools |
| Mutation and speciation | Standard/advanced mutation, lifelong scheduling and native speciation | `db2_mutation_248`, `mutation_advanced_248`, mutation-enabled save/continuation |
| Ecology, corpses, waste and energy | Native biology, feeding, vegetation and corpse decay | `db2_vegetation`, `corpses_collisions`, genuine chloroplast/body/feeding fixtures, ecological soaks |
| World objects and local exchange | Obstacles/mazes, local relocation and transactional file gateways | `environment_world`, `legacy_transport`, native file-transfer tests and rendered teleporter workflows |
| Modern and original file boundaries | DB3S migrations, native `.sim`/`.dbo`, `.set`/`.gset`, DNA export | `final_state_persistence_248`, `LegacySettingsNativeTests`, native robot/organism round trips |
| Species working copy and policy | Dedicated species mode, stable identities and atomic roster command | Species-menu apply/cancel/reopen test, `SpeciesSettingsNativeTests`, native atomicity/save-resume tests |
| Leagues, contests and evolution | Offline mode authority, classic tournament orchestration and staged evolution | `local_modes`, `zerobot_progression`, community league and rendered mode workflows |
| Graphs, recording and recovery | Native graph metrics/history, snapshots, safe-mode and autosave controllers | `history_records`, rendered graph/export and recovery tests, native Windows walkthrough |
| Robot tools and display controls | Live details/console/activations/phylogeny, PlayerBot, eye/RGB/color tools | Dedicated rendered/causal tool tests and display-only state invariants |
| Offline interface and accessibility | Menu catalog/router, settings, viewport, keyboard and tray | Internet-absence tests, 139-ID inventory, layout checks and native Windows tray checks |
| CPU/GPU execution and continuity | Physical backend, explicit failure/fallback policy and complete state saves | Six strict physical GPU groups, including 10,000-cycle native-world CPU/GPU/save-resume comparison |
| Distribution | Published application folder and bundled robot assets | Four-platform tests/publish/smoke and local Windows package smoke, including species editing |

### Current qualification

Pending final local workspace/Core/Desktop/package qualification, physical GPU qualification, native Windows walkthrough and repaired cross-platform CI. The initial CI run at `7a3db6b` passed Rust and Core on Windows, Linux and Apple silicon but failed desktop checks on stale six-tab and corrupted-text expectations; those failures are retained as failures, not counted as current acceptance.

### Scope and evidence limits

The original source and pinned genuine executable captures define behavior. Current tests can recheck retained original oracles without claiming new original-GUI observations. Identical arbitrary random trajectories, every historical file/version, public hosting and a public release are outside this target. Native Linux/macOS tray and window-manager interaction is outside the selected acceptance boundary. Raw user saves, profile backups and private desktop diagnostics remain local.

---

## Historical October 2026 qualification

Last reconciled: 2026-10-25, against completion checkpoint `0225101343367cbe297d4d23afda598cfa9f679a` and its saved qualification. This update changes documentation only; it is not a new engine, GPU or desktop qualification.

The agreed functional offline work was completed on October 21. No known required code repair remains in that scope. The [completion report](../../docs/parity/db24832-completion-report-2026-10-21.html) and [structured receipt](../../output/native-ui65-20261021/completion.json) separate actual native observations, automated coverage and remaining limits. The July Phase 0 ledger below is retained as history, not the current backlog.

## Qualified revisions

| Revision | What its evidence establishes |
| --- | --- |
| `73e350297f54e88a7a690c435517ebb13615f953` | Qualified engine, CLI and Desktop.Core source: CPU65, strict physical GPU65 and sustained GPU capacity. Those source trees are unchanged at the completion checkpoint. |
| `0e57519e809e38736ce5090cae84e13c24951b44` | Managed UI repair for clipped controls and local-mode round summaries; critical native Windows walkthrough and four-platform hosted suite. |
| `5f0f7d800adc2439e0dd09bc2bd913f0bca4b494` | Test-only display/turbo race repair. Three-platform diagnostics passed afterward; product source and the native package did not change. |
| `0225101343367cbe297d4d23afda598cfa9f679a` | Final report/evidence checkpoint. It is not a fresh hardware qualification. |

## Completed evidence

| Area | Recorded result | Evidence |
| --- | --- | --- |
| CPU engine and integration | 94 ordinary steps passed; 1,049 Rust cases across 68 targets, 314 Core cases and 215 Desktop cases before the later UI repair. 25 genuine original controls / 55,849 projected fields matched. | [CPU65 qualification](../../docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass65/qualification-summary.json) |
| Physical GPU | All six strict groups passed on RTX 3050 / Vulkan / DiscreteGpu, including the unchanged 10,000-cycle original-world CPU/GPU comparison and repeated save/resume. Groups overlap; do not add them as unique test counts. | [GPU65 receipt](../../output/gpu65-collected.json) |
| Sustained GPU capacity | Fixed populations of 1,000, 4,000, 8,000 and 32,000 passed exact-count, movement and at least 60-second measurement requirements. These are headless controlled probes, not rendered frame rates or ecological throughput. | [Capacity receipt](../../output/capacity65-controlled-gpu-collected.json) |
| Final UI hosted matrix | Windows x64, Linux x64, Intel macOS and ARM macOS each passed 1,064 Rust cases across 68 targets, 314 Core cases, 221 Desktop cases and package publish/smoke steps. The 12 hardware-only Rust cases were ignored on hosted machines. | [Run 39572329105](https://github.com/BobertNeek/Darwinbots-Modern/actions/runs/39572329105), [collected results](../../output/native-ui65-20261021/hosted-initial-collected.json) |
| Repaired diagnostics | Linux, Intel macOS and ARM macOS each passed 221 Desktop cases. The earlier Intel diagnostic race remains recorded as a failure at its original revision. The repair pauses through the real Pause control and retains the strict assertions. | [Run 39579819052](https://github.com/BobertNeek/Darwinbots-Modern/actions/runs/39579819052), [fixture receipt](../../output/native-ui65-20261021/hosted-fixture-collected.json) |
| Native Windows workflows | Settings file save/change/load; global/live settings reopen; small CPU world transport; simulation save/advance/load; SimpleSurvival progression/restart; graphs and CSV export. The isolated official original Restarter also opened the normal original shell and New Simulation dialog. | [Native walkthrough receipt](../../output/native-ui65-20261021/completion.json) |
| Final local package | Focused UI checks passed, CPU movement/save-resume smoke passed and 48 bundled DNA files matched their source bytes. Seven original profile hashes were restored. | [Package and focused-check receipt](../../output/native-ui65-20261021/completion.json) |

The October 25 recovery rechecked the existing package's engine/desktop hashes and all 48 bot assets, confirmed unchanged engine/CLI/Core source, and read the successful hosted run metadata at the exact UI/test commits. It did not rerun the historical suites or launch an application. A green hosted run alone is not the basis for the completion claim.

## Compatibility and observation limits

- DB2 2.48.32 native `.sim` import/export and `.dbo` organism interchange are implemented. This supersedes the old blanket statement that only modern saves and text DNA exist. See [native simulation export](../engine/src/world/native_sim_export.rs), [native organism interchange](../engine/src/world/native_organism_io.rs) and [desktop native integration tests](../desktop/tests/Darwinbots.Desktop.Tests/LegacySettingsNativeTests.cs).
- The integration tests load a pinned genuine original simulation and an original-executable continuation of a port export, step the loaded worlds, and compare controlled modern checkpoint continuation. Native organism evidence includes an original simulation saved after `.dbo` import. This is finite fixture coverage, not a guarantee for every historical save, DB2 version or unknown record layout.
- Modern `DB3S` saves retain their own versioned continuation contract. Native DB2 export and modern save/resume are distinct paths. Identical arbitrary future random trajectories and incidental VB6 bugs are outside the agreed functional scope.
- All 74 original main-menu actions are represented; three Internet actions remain disabled by agreement. Offline exchange is included. Registration alone is not behavioral proof: the report distinguishes automated interactions from critical workflows observed natively.
- Native tray hide was observed; tray restore has automated coverage but was not observed natively. Linux/macOS native window-manager walkthroughs and a manual click of every menu action were not performed. No corresponding product failure was established; these remain explicit verification limits.

## Existing Windows package

The original completion worktree contains `artifacts/win-x64-native-ui66-final/Darwinbots.Desktop.exe`. This framework-dependent Windows x64 package uses .NET 10. Keep it with its adjacent DLLs and `Bots` directory; it is generated local output and is not stored in Git.

| File | SHA-256 |
| --- | --- |
| `darwinbots_engine.dll` | `EB13AA54BC6456A30F7F3CD5F8AA56CDC4ECE43CECD27B67066F19E4FFEF6C1E` |
| `Darwinbots.Desktop.dll` | `3555F7BE066C88247AE7316E133A5C21BB5162C374D48CA6BCE0BB4BB366D729` |

The native DLL is the unchanged qualified engine; the managed DLL contains the final UI repair. A different `target/` build must not be silently substituted for this package. Source builds and hosted packages have their own provenance.

## Delivery and additional acceptance

The completed work remains on `codex/db24832-completion`, separate from `master`. As checked on October 25, the only published release is [legacy reference material](https://github.com/BobertNeek/Darwinbots-Modern/releases/tag/legacy-db2-reference-v1), not a modern application distribution. A merge or modern release is a separate delivery decision; the completion evidence does not imply one has happened.

No required code change remains in the recovered offline checklist. Additional native tray-restore, non-Windows desktop interaction or broader historical-file corpus checks would extend the evidence. They have not been run by this documentation update. New physical GPU/graphics qualification on the training machine requires a coordinated interval; preserve the active A-Life run.

<details>
<summary>Historical Phase 0 ledger — July 30, 2026</summary>

The following checkpoint is preserved with its original commands, results and then-open findings. Its references to current/next work apply to that July revision only.

Last updated: 2026-07-30

This ledger records a measured DB2 2.48.32 Phase 0 checkpoint through `95818f6af58bcb2365ec21f6a49e707ae15ed343`. A passing test proves only its stated boundary. Catalog registration and green unit counts are not behavioral parity.

## Current checkpoint gate

| Area | Command or evidence | Result |
|---|---|---|
| Reference package | `pwsh -NoProfile -File docs/reference/db24832/verify.ps1` | 6/6 verified |
| Selected Rust engine | `cargo test -p darwinbots-engine --test contracts --test manual_tools --test live_legacy_menu_settings` | 23 passed; 0 failed |
| Desktop core | `dotnet test modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj --no-restore --nologo` | 62 passed; 0 failed; 0 skipped |
| Rendered desktop | `dotnet test modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj --no-restore --nologo` | 26 passed; 0 failed; 0 skipped |
| Control inventory | `pwsh -NoProfile -File docs/verification/control-surface-audit/run-rendered-audit.ps1` | 109/109 passing rows validated; 26 rendered tests passed |
| Whitespace/error scan | `git diff --check` | Pass; exit 0 with line-ending warnings only |

Run these commands serially from `D:\Darwinbots 2\.worktrees\db24832-parity`. Do not start competing Cargo, GPU, or desktop builds.

## Phase 0 deterministic evidence

- Typed catalog commands route through explicit production handlers.
- Live mutation controls queue authoritative engine mutations for enabled state, global multiplier, and per-species rates.
- Mutation-free save transforms only the serialized copy; the running state is unchanged.
- Permanent-tie whole-organism removal uses stable `slot:generation` capture and an atomic native `remove_many` command.
- Successful remove-many execution produces exactly one publication for the batch and runs off the caller thread.
- Native command parsing accepts `remove_many` JSON.
- End-to-end organism picker JSON contains only captured group members and ties internal to that group.
- Default E-Grid and Survival Info visibility matches the checkpoint contract.
- Retained shortcut strings are applied as exact input gestures and tested through keyboard input.
- The three online commands are disabled and interaction tests show no network work.
- Current view toggles and object tools have bounded rendered interaction coverage.

## Compatibility and simulation limits

- The recorded 2.48.32 executable hash remains a reference artifact, but the original GUI lacks successful black-box execution evidence.
- Modern `DB3S` persistence and plain-text DNA interchange do not provide DB2 simulation, settings, organism, snapshot, graph, or recovery compatibility.
- Current engine tests cover useful modern kernels and selected integrated behavior. They do not close the P0/P1 cycle ordering, DNA/VM, shots, ties, reproduction, senses, physics, ecology, mutation, or persistence findings.
- CPU/GPU differential coverage remains incomplete across interaction-bearing phases and long ecological scenarios.
- Online networking is excluded, not complete.

## Rendered evidence

The audit writes ignored evidence under `docs/verification/control-surface-audit/modern/`. See `docs/parity/historic-menu-parity-report.md` for the inspected runtime screenshot paths, exact dimensions, and visible findings.

## Next acceptance work

Use `docs/parity/db24832-gap-register.md` as the prioritized non-online backlog. It preserves generic tool substitutes, collapsed settings, shell and viewport differences, engine/runtime gaps, persistence/offline gaps, GPU differentials, and missing original-GUI evidence as open or partial.

</details>
