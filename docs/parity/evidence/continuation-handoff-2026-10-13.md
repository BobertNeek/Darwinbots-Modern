# Darwinbots parity continuation

LATEST USER WORKFLOW (overrides the earlier incremental gate sequence): Finish ALL remaining authorized code changes and write their tests first. Do not run builds, tests, smoke checks, lint, benchmarks or application/original-runtime verification until the implementation batch is complete. Then run the combined verification and repair pass. Do not push source changes during the implementation batch because that starts CI. Static source reads, edits and local checkpoint commits remain available. Local runner 74385 completed before the change; cancellation was requested for the remaining hosted run 38511804390. Latest pushed HEAD is f8da410; reference-velocity source cf7be4d and hardware-gate source 5f00f16 are committed. Corpse migration and Release/package-smoke CI changes are currently uncommitted. Preserve unrelated root WIP and all earlier evidence.

Continue fixing all previously authorized offline parity gaps. Internet Mode is excluded. Do not spawn agents. Preserve the unrelated dirty root checkout at D:/Darwinbots 2.

Owned worktree: C:/Users/PC/.codex/worktrees/db24832-completion/Darwinbots 2, branch codex/db24832-completion.

Current pushed HEAD: c8499a5af4f783e59cc16dc063c828678964f158.
Source commits: 9446ce1 native species graphs and mutation counters; 4795f23 cached-radius lifecycle; ab5673d package smoke and historical/league bot copying. Report commits: 83f049f and c8499a5.

Confirmed current checks: complete Debug engine 571 passed/0 failed/2 long tests excluded; strict Clippy passed; complete Release desktop 105 passed/0 failed. Core Release 143/0 at graph repair. Published Windows package at modern/artifacts/cached-radius-win-x64 passed --smoke-test: CPU native loading, movement, continuous versus save/resumed equality through tick 13. All 48 bot files match source SHA-256 hashes.

Owned sequential runner session 19015: current Release Rust workspace (two named long ecology tests skipped), then 20,000-cycle db2_starter_ecology, then 10,000-cycle starter_ecology. Logs: parity-cached-radius-workspace-release.log, parity-cached-radius-db2-long-soak.log, parity-cached-radius-starter-long-soak.log. Keep Rust source frozen until qualification completes or the next test binary has compiled; record exact source state for every run. Do not start duplicate builds.

Earlier long CPU ecology scenarios passed: 20,000 cycles at 0206429 in 1157.30 seconds; 10,000 cycles on graph draft before species filename fix in 631.22 seconds. These do not qualify current cached-radius code. Receipt: native-long-ecology-receipt.json.

Fresh report: docs/parity/db24832-fresh-audit-2026-10-11.html. Root mirror is D:/Darwinbots 2/docs/parity/db24832-fresh-audit-2026-10-11.html. New receipts: native-graphs-receipt.json, cached-radius-receipt.json, cached-radius-package-smoke.json, cached-radius-package-assets.json. JSON and browser DOM verified. Rendered graph image was visually inspected. HTML browser screenshots previously failed; do not claim HTML pixel QA.

Next concrete gaps researched:
- Senses.bas 309-324 projects target raw velocity, subtracts observer rounded memory velup/veldx, rounds Single X/Y, clamps, negates PRIOR refvelsx, and calculates scalar from rounded up/dx. Port currently projects absolute target velocity, does not subtract observer memory, and does not publish refvelscalar (695). Current world.rs sensing around 6320.
- Ties.bas 770-783 copies target memory velocity fields for your-frame values, clamps target physical X/Y to +/-16000, then computes my-frame Double projections minus observer memory. Port recomputes both velocities and subtracts rounded projections. Current world.rs tie references around 7120.
- Older complete archived corpses still need automatic migration into arena state, preserving ties, parent and shot ownership. native_world_with_selected_corpses currently clones using SaveFile::decode; changing restore migration requires avoiding recursive duplication in this helper. Complete native_state/native_id/native_ties are retained; partial projections cannot recover missing DNA/memory.
- Adaptive dynamic-cost controller still modern approximation. Original Master.bas 243-302 uses population history, countdown and incremental multiplier correction; graph/cost slot mappings now correct (52 zero threshold, 57 upper percent, 58 lower percent).
- Global evolution options mostly stored without consumers. Original Evo.bas and frmGset.frm provide rules. Auto graphs only original restart modes 4/5, with population, average mutations, and total energy per species. Do not wire to the wrong modern mode just to check a box.
- League MB/SG/SS qualification, original phase order/global RNG, original trajectory comparisons and long GPU/performance qualification remain open.

Linux attempt: Ubuntu is registered, but wsl cannot start (virtualization/nested virtualization unavailable). No host settings changed. macOS unverified. GitHub workflows yielded no runs earlier; repository actions permissions were enabled, so do not say Actions is disabled.

Report localhost server 8771 still owned. Browser IAB id 2, report tab 11, persistent binding checkpointReportTab. Use cua.rewriteDocumentation after compaction. Native computer use is separate Node/Sky route; Roblox was the only visible native window in last inventory. Do not close or interrupt a newly opened Roblox session.

Pause automation resume-darwinbots-after-roblox is PAUSED. Original goal tool returned null. No PR created. No master merge yet.

CI correction: always pass --repo BobertNeek/Darwinbots-Modern to gh commands. Auto-default picked the original darwinbots/Darwinbots2 repository. Explicit API reads show our cross-platform push CI active: ab5673d queued, earlier 9446ce1 and 0206429 in progress. No new PR is needed just to trigger CI. Reference-velocity draft exists untracked at modern/engine/src/world/native_reference_velocity.rs and is not integrated or tested.

CI run 38506430843 at ab5673d is now in progress on Windows, Ubuntu, macOS Intel and macOS ARM. Cancellation requests were submitted for superseded same-branch runs 38493111555 (316ff88), 38494393365 (0206429), and 38501961574 (9446ce1). Verify statuses later. Local Release runner 19015 has compiled and is executing the 23-bot CLI probe.


Implementation-only batch update (2026-10-14): source is intentionally NOT pushed or executed. Added complete archived-corpse migration, native adaptive costs, visible/moving shape references and C# visibility persistence/editor control, multibot requirement timers and inherited child timers, native contest sampling/adaptive limits/per-species caps/first-round five-bot reset, native family fitness by absolute parent numbers, and a VB6 Rnd/Randomize/Gauss implementation. The shared stream is being integrated into VM, mutation, clocks, shots, toxins, waste, corpses and Brownian motion. New modules under modern/engine/src/world and modern/engine/src/native_random.rs are untracked until explicitly staged. All work after the user's batching instruction is UNCOMPILED/UNTESTED. Do not run formatting, builds, lint, tests, benchmarks, smoke checks or the original executable until ALL implementation work is finished. Do not push source during this phase because CI would start.

Still to code/review before the combined gate: native per-robot phase ordering (especially late mutation/material/waste/shot/senses/death order), remaining global RNG consumers such as placement/repopulation/sun controls; F2/F3 action disqualification and SG/SS/short-bot qualification rules with source/convention evidence; persistent league ladder/tournament workflow; original staged evolution/restart states and global evolution controls; native private-variable ordering and other remaining report ledger gaps. The global RNG draft currently needs static integration review across helper call sites and Engine initializers. Existing complete old-source gates remain in receipts; do not claim them as current source qualification.

Latest delivered tested Windows package remains D:/Darwinbots 2/modern/artifacts/db24832-parity-ab5673d-win-x64 (48 bots; DLL SHA256 2b64885af8c9bd7df0f725b55047dc1d67cfaebe9c31a5294e60528aea7bf0d8). Both ab5673d CPU ecology scenarios passed: 20,000 cycles 880.80s and 10,000 cycles 348.18s. Current pushed report f8da410 records reference source cf7be4d (158/0, five physical cases excluded) and hardware source 5f00f16 (eight explicit GPU checks passed locally). These results precede the implementation-only batch.
