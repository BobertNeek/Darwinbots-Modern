# Darwinbots Modern Verification Ledger

Last updated: 2026-07-18

This ledger records measured evidence for the merged application. A passing test proves the stated boundary only; it does not imply complete Darwinbots 2 parity.

## Current merged gate

| Area | Evidence | Result |
| --- | --- | --- |
| Rust workspace | `cargo test --workspace` in the cross-platform workflow | Pass on Windows, Linux, Intel macOS, and Apple silicon macOS |
| Desktop core | `Darwinbots.Desktop.Core.Tests` | 47/47 pass |
| Rendered desktop | `Darwinbots.Desktop.Tests` using Avalonia Headless with Skia | 17/17 pass |
| Control inventory | `docs/verification/control-surface-audit/validate_matrix.py` | 109/109 controls present and passing |
| Packaging | Framework-dependent publish for `win-x64`, `linux-x64`, `osx-x64`, and `osx-arm64` | Pass on all four targets |

The current cross-platform receipt is [GitHub Actions run 29640693121](https://github.com/BobertNeek/Darwinbots-Modern/actions/runs/29640693121). The rendered audit and production fixes were merged in [PR #3](https://github.com/BobertNeek/Darwinbots-Modern/pull/3).

## Compatibility evidence

- **Legacy reference:** Darwinbots 2.48.32 executable SHA-256 `7F580D9CE8F1095A01FD162E9BCAC03FE437599E080786FD670B676671C31990` is recorded as a reference artifact. The Linux cloud runner did not execute its 32-bit GUI successfully.
- **DNA import:** every robot shipped under `Installer/bots` parses in the Rust engine suite. Plain-text DNA remains the supported legacy interchange format.
- **Sysvars:** all 255 active names from DB2 2.48 `DNATokenizing.bas` resolve to their VB6 addresses. See the [defaults and sysvars audit](../../docs/verification/db2-defaults-and-sysvars-audit.md).
- **Modern persistence:** versioned `DB3S` world saves, history, organism phenotype, ties, shots, and settings have round-trip coverage. Legacy DB2 save and settings formats are intentionally unsupported.
- **Native ABI:** command batches, capability and structured-error JSON, owned buffers, atomic saves, and panic containment have focused tests.
- **Fallback:** adapter creation, shader/runtime failure, and strict-GPU behavior are covered; automatic mode can fall back to CPU.

## Simulation evidence

- Repeated seeded CPU stress runs completed 1,000,000 ticks with invariant checks for structure-of-arrays lengths, alive/payload agreement, free slots, finite bounded positions, positive live energy, stable tie IDs, and snapshot consistency.
- CPU/GPU differential tests cover spatial indexing, nearest-neighbor sensing, motion integration, and fused sensing/motion paths.
- Integrated tests cover persistent shots, toroidal projectile inheritance, collisions, corpses, feeding, ties, multibot state, chloroplast vegetation, reproduction, mutation, historical samples, and live environment updates.
- Snapshot hashing excludes wall-clock phase telemetry while retaining gameplay state, allowing reproducible seeded CPU comparisons without claiming deterministic evolution across ordinary runs.

## Rendered GUI evidence

The current desktop exposes 109 audited controls across setup, runtime, advanced settings, and DNA editing. Each has a stable audit ID, production handler or binding, state/effect description, DB2 intent classification, and rendered interaction result.

Run:

```powershell
pwsh docs/verification/control-surface-audit/run-rendered-audit.ps1
```

See the [GUI parity report](../../docs/parity/gui-parity-report.md) for exact scope and limitations. The audit proves current modern controls and their effects; it does not prove that every historical DB2 window or auxiliary tool has been recreated.

## Performance receipts

Performance varies with hardware, backend, DNA, world density, and enabled phases. Recorded 100,000-organism receipts on the development machine span:

- CPU: approximately 10.79 to 17 simulation ticks/s.
- GPU: approximately 7.21 to 12.46 simulation ticks/s.

CPU was faster on that machine and remains the measured automatic default there. GPU acceleration is available for compatible spatial, sensing, physics, and render-preparation work, but it is not assumed to outperform CPU on every adapter.

The product target remains at least 30 simulation ticks/s while presenting at 60 FPS for 100,000 representative organisms on a designated reference discrete GPU. That target has not yet been met. Snapshot frequency may be reduced independently of headless simulation speed on lower-end systems.

## Open acceptance gaps

- Capture authoritative VB6 fixtures for parser, VM, physics, reproduction, mutation, senses, ties, shots, energy, and aggregate statistics.
- Extend CPU/GPU differential coverage to every interaction-bearing phase and long ecological scenarios.
- Meet and publish the 100,000-organism reference-GPU performance target.
- Perform successful black-box comparison of the exact legacy GUI on a working 32-bit Windows or Wine environment.
- Recreate only the remaining legacy auxiliary workflows that are still judged useful, such as a dedicated DNA debugger, GraphJoin, and Snapshot Search.

Online leagues, ActiveX/VB6 runtime infrastructure, Python 2 networking, and legacy save/settings migration are outside the current scope.
