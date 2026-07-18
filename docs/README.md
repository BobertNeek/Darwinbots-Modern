# Darwinbots Modern Documentation

This directory separates current product evidence from historical design records. The root [README](../README.md) is the product entry point.

## Current reference documents

| Document | Purpose |
| --- | --- |
| [Verification ledger](../modern/docs/verification.md) | Current merged test, packaging, compatibility, and performance evidence |
| [DB2 defaults and sysvars audit](verification/db2-defaults-and-sysvars-audit.md) | Source-backed default values and the complete DB2 2.48 sysvar mapping result |
| [Rendered GUI parity report](parity/gui-parity-report.md) | Scope, limits, and results of the 109-control rendered interaction audit |
| [Control-surface matrix](parity/control-surface-matrix.md) | Human-readable inventory of every current desktop control |
| [Machine-readable control matrix](parity/control-surface-matrix.json) | Structured source for audit validation and automation |

## Reproducible verification

Run the rendered desktop audit from the repository root:

```powershell
pwsh docs/verification/control-surface-audit/run-rendered-audit.ps1
```

The command runs the Avalonia desktop tests, validates the control matrix, and writes generated screenshots beneath `docs/verification/control-surface-audit/modern/`. Those screenshots are intentionally ignored by Git; CI publishes equivalent evidence as workflow artifacts.

## Behavioral authority

The VB6 source in `Darwinbots2/` is authoritative for legacy formulas, sysvars, phase ordering, and control intent. DarwinbotsC is a secondary implementation reference only. Black-box executable comparisons must identify the exact executable version used and must not be generalized into claims of complete runtime parity.

## Historical design records

Files under `docs/plans/`, `docs/superpowers/specs/`, and `docs/superpowers/plans/` preserve the reasoning and intended implementation sequence for completed feature work. Their checkboxes, dependency versions, filenames, and proposed commands reflect the planning baseline at the time they were written; they are not current task trackers.

Use current source, the verification ledger, and the parity reports when they differ from a historical plan.
