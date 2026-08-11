# Darwinbots Modern Verification Ledger

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
