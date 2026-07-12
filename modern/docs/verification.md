# Darwinbots Modern Verification Ledger

This file records measured acceptance evidence. A gate is complete only when the listed evidence exists.

## Verified

- Legacy reference: Darwin2.48.32, SHA-256 `7F580D9CE8F1095A01FD162E9BCAC03FE437599E080786FD670B676671C31990`.
- Legacy DNA import: every robot shipped under `Installer/bots` parses in the Rust engine test suite.
- CPU determinism: repeated seeded one-million-tick runs produced `ccd396c607ff7b01`.
- Stress invariants: one live organism completed 1,000,000 ticks with 1,000 checks covering SoA lengths, alive/payload agreement, free slots, finite bounded positions, positive live energy, stable tie IDs, and snapshot consistency.
- CPU/GPU differential coverage: motion integration, nearest-neighbor sensing, and fused sensing/motion pass on the available adapter.
- Runtime fallback: deterministic adapter/runtime failure tests cover fallback-enabled and strict-GPU behavior.
- Native ABI: versioned command batches, capability/error JSON, owned buffers, atomic DB3S save/load, and panic containment are covered.
- Desktop: .NET 10 core tests pass; Avalonia Release builds without warnings; production screenshot is `modern/docs/ui-save-load.png`.
- Windows package: `win-x64` publish contains the apphost and native DLL and launches a live `Darwinbots Modern` window.

## Latest performance receipts

- CPU, 100,000 organisms: approximately 15-17 ticks/s across warmed runs.
- GPU, 100,000 organisms, fused sensing/motion with four cooperative lanes: 12.46 ticks/s.
- Required gate: at least 30 simulation ticks/s and 60 FPS at 100,000 organisms on the designated reference discrete GPU.

## Open gates

- Capture authoritative VB6 behavior fixtures for parser, VM, physics, reproduction, mutation, senses, ties, shots, energy, and aggregate statistics.
- Differential-test complete CPU/GPU phase outputs, including interaction-bearing ticks and render-instance generation.
- Reach the 100,000-organism performance target on the designated reference discrete GPU.
- Implement and verify the remaining desktop debugger, editor, species, graph, cost, mutation, physics, environment, GraphJoin, snapshot search, and manual reproduction workflows.
- Obtain successful Linux x64, macOS x64, and macOS arm64 package receipts from `.github/workflows/modern-cross-platform.yml`.
# 2026-07-12 Core ecology, sensing, and visible combat checkpoint

- `cargo test -p darwinbots-engine --test corpses_collisions --test gpu_ffi --test shot_state`: 20 passed, 0 failed.
- `cargo test -p darwinbots-engine --test sensing_compatibility --test gpu_ffi`: 18 passed, 0 failed.
- `dotnet test desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj --no-restore`: 19 passed, 0 failed.
- `dotnet build desktop/src/Darwinbots.Desktop/Darwinbots.Desktop.csproj -c Release --no-restore`: succeeded with 0 warnings and 0 errors.
- Verified persistent decaying corpses, corpse feeding, bounded organism collision separation, nine aim-relative vision sectors, explicit unknown/inert DNA compatibility warnings through the UI import path, persisted visible shot trails, live CPU/GPU switching, and runtime CPU fallback.
# 2026-07-12 Ties, multibots, and historical records checkpoint

- `cargo test -p darwinbots-engine --test ties_multibot --test species_ecology --test corpses_collisions --test sensing_compatibility --test shot_state --test gpu_ffi`: 37 passed, 0 failed.
- `cargo test -p darwinbots-engine --test history_records --test ties_multibot --test gpu_ffi`: 19 passed, 0 failed.
- `dotnet test desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj --no-restore`: 20 passed, 0 failed.
- `dotnet build desktop/src/Darwinbots.Desktop/Darwinbots.Desktop.csproj -c Release --no-restore`: succeeded with 0 warnings and 0 errors.
- Verified bidirectional energy, waste, shell, slime, and chloroplast tie sharing; directed tie memory transfer; slime tie resistance; stable multibot/tie-count sysvars; bounded 100-tick historical sampling; DB3S history round-trip; and live population/energy chart wiring.
- Live Release application window launched as `Darwinbots Modern - New World`. Screenshot evidence remains blocked by Windows Graphics Capture: `SetIsBorderRequired failed: No such interface supported (0x80004002)`.
# 2026-07-12 Playable workflow, package, and stress checkpoint

- Live environment commands: 17 GPU/ABI tests and 3 environment tests passed; obstacle and teleporter add/remove operations execute through the serialized simulation thread.
- Zerobot progression: engine distinguishes self-reproduction from `302` forced reproduction and records successful feeding and DNA-driven movement; automatic and manual progression controller tests passed.
- Desktop core: 24 tests passed. Release desktop build succeeded with 0 warnings and 0 errors.
- Portable package: `dotnet publish ... -r win-x64 --self-contained true` produced `modern/dist/win-x64/Darwinbots.Desktop.exe`; packaged process remained alive and exposed window `Darwinbots Modern - New World` before the smoke process was stopped.
- Long-run CPU stress: 1,000,000 ticks, 1 organism, 1,000 invariant checks, 25.798 seconds, final state hash `4f42679dfb43fce5`.
- A 100-organism million-tick attempt exceeded the bounded 180-second validation window and is not counted as passing evidence.
- 100,000-organism CPU benchmark: 100 ticks in 9.268 seconds, 10.790 ticks/s. Limiting phase was sensing at 37.524 ms.
- 100,000-organism GPU benchmark: 100 ticks in 13.691 seconds, 7.304 ticks/s. Limiting phase was sensing at 83.093 ms.
- CPU is the measured automatic default on this reference machine; GPU remains available for live manual switching and fallback tests remain green.
# 2026-07-12 Acceptance-audit control checkpoint

- Added live reset, `1x/5x/20x/MAX` throttle selection, selected-organism follow mode, and zoom-sensitive high-population render LOD.
- Added two-click obstacle placement, two-click teleporter source/destination placement, viewport feature selection, and stable-ID feature removal while the simulation is live.
- Added automatic/manual Zerobot progression based on self-reproduction, successful feeding, and intentional movement counters; forced `302` reproduction is explicitly excluded from the self-reproduction signal.
- Automatic stages reduce feeder/reproducer DNA to energy-only feeding, remove feeder assistance after evolved feeding, and set Brownian motion to zero after evolved movement.
- Focused environment, Zerobot, GPU/ABI, and desktop gates passed; final Release build succeeded with 0 warnings and 0 errors.
- Remaining GPU audit gap: sensing and physics run on `wgpu`, but packed spatial-grid construction is still CPU-produced before GPU neighborhood queries. Full GPU grid construction/residency is not yet accepted.
# 2026-07-12 Final alpha acceptance gate

- Full Rust workspace: 82 tests passed, 0 failed, including all shipped legacy DNA imports, CPU reproducibility, DB3S round trips, GPU spatial-grid construction, GPU sensing/physics/render preparation, runtime fallback, and gameplay systems.
- Desktop Release tests: 26 passed, 0 failed.
- Final self-contained `win-x64` publish succeeded and `modern/dist/win-x64/Darwinbots.Desktop.exe` launched with window `Darwinbots Modern - New World`.
- GPU spatial indexing now uses on-device clear, count, prefix-offset, and scatter compute passes before neighborhood queries. GPU-derived nearest pairs feed collision candidate resolution.
- 100,000-organism GPU benchmark after GPU grid construction: 100 ticks in 13.861 seconds, 7.214 ticks/s. CPU remains faster on this machine and therefore remains the automatic default.
- Snapshot equality and reproducible stress hashing exclude wall-clock phase telemetry while retaining gameplay state. Paralysis counters are clamped against negative state.
- Live advanced settings now atomically update metabolism, vegetable energy, sunlight, gravity, drag, and Brownian motion while running.
- Snapshot publication rate is reported separately from simulation tick rate. Stable ties and shot trails are parsed and rendered.
- Visual evidence: `modern/docs/evidence-main-window.png` and `modern/docs/evidence-full-desktop.png` show the packaged running world and player UI. The full-desktop capture was used because Windows Graphics Capture is unavailable on this host.
