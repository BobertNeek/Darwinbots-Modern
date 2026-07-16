# Rendered GUI Parity Audit Report

## Scope and authority

This audit resumes the requested control-surface work by creating durable evidence, a machine-readable matrix, a markdown matrix, and repeatable rendered-GUI automation. Behavioral authority remains DB2 runtime first, then VB6 source. In this run, DB2 runtime execution was blocked by Wine before the application window could render, so original runtime observations are marked blocked and source/reference parity is used where available.

## Evidence locations

- Modern screenshots: `docs/verification/control-surface-audit/modern/`
- Original runtime logs/failure: `docs/verification/control-surface-audit/logs/wine-db2-attempt.log`
- Automation: `docs/verification/control-surface-audit/run-gui-audit.sh`
- Matrix validation: `docs/verification/control-surface-audit/validate_matrix.py`


## PR-compatible evidence policy

Rendered screenshots are generated audit artifacts and are intentionally excluded from the Git diff because Codex Create PR does not support binary files. The matrix and report keep the original screenshot paths as reproducible evidence locations; the omitted files, SHA-256 hashes, and regeneration command are listed in `docs/verification/control-surface-audit/omitted-binary-evidence-manifest.md`. Recreate them with `./docs/verification/control-surface-audit/run-gui-audit.sh`.

## Smoke gate result

| Gate item | Result | Evidence |
|---|---:|---|
| Modern desktop build | Pass after installing/overriding Rust 1.96.0 | `logs/dotnet-build-desktop-after-rustup.log` |
| Modern setup screen renders | Pass | `modern/setup-initial-modern-001.png` |
| Modern setup menus open via injected input | Pass | `modern/setup-menu-file-open-modern-002.png`, `modern/setup-menu-view-open-modern-003.png`, `modern/setup-menu-help-open-modern-004.png` |
| Modern starter world can be created | Pass | `modern/setup-create-world-before-modern-012.png`, `modern/live-initial-modern-013.png` |
| Modern simulation viewport receives mouse input | Pass | `modern/live-viewport-select-modern-025.png`, `modern/live-viewport-drag-modern-026.png` |
| Original DB2 executable renders through Wine | Blocked | `logs/wine-db2-attempt.log` reports `/usr/bin/wine-stable: 40: exec: /usr/lib/wine/wine: Exec format error` |

## Matrix summary

- Total controls inventoried: 100
- blocked: 2
- source-parity: 98

## Confirmed defects and fixes in this run

No application-code defects were corrected in this constrained run because the rendered smoke audit did not produce a safely isolated behavioral defect that could be fixed without deeper parity confirmation. The substantive deliverables are the repeatable rendered automation, evidence tree, source/runtime matrix, and blocked DB2 runtime record.

## Blocked comparisons

- DB2 2.48.32 runtime GUI parity is blocked by the installed Wine wrapper failing with an exec-format error before the DB2 process starts.
- Original screenshots remain blocked and are represented by the Wine log path in the matrix.

## Remaining checklist

- Install a working 32-bit Wine stack and rerun DB2 smoke gate.
- Expand xdotool coverage to every advanced numeric min/mid/max value.
- Exercise file picker save/load/import paths with temporary files and malformed DNA.
- Add deeper assertions against engine state snapshots for each rendered interaction.
- Capture the requested screen recordings with ffmpeg once DB2 runtime starts successfully.
