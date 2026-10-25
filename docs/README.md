# Darwinbots Modern Documentation

This directory separates current product evidence from historical design records. The root [README](../README.md) is the product entry point.

## Current reference documents

| Document | Purpose |
| --- | --- |
| [October 21 offline completion report](parity/db24832-completion-report-2026-10-21.html) | Completed functional offline scope, critical native Windows walkthrough and explicit observation limits |
| [Verification ledger](../modern/docs/verification.md) | Exact engine/UI/test revisions, retained qualification, package identity and delivery status on the completion branch |
| [Completion receipt](../output/native-ui65-20261021/completion.json) | Structured native observations, focused repairs, hosted results and package hashes |
| [Detailed October parity audit](parity/db24832-parity-report-2026-10-14.html) | Detailed feature evidence and preserved earlier checkpoints; follow its link to the October 21 conclusion |

The completed work is on `codex/db24832-completion`, separate from `master`. The only published release currently contains legacy reference inputs. Completion of the agreed offline work is not a claim of universal historical-file compatibility, identical arbitrary trajectories or a manual walkthrough of every menu on every operating system.

## Earlier audits

These records retain their original scope and dates. Their old open findings and inventory counts are not the current completion checklist.

| Document | Purpose |
| --- | --- |
| [DB2 defaults and sysvars audit](verification/db2-defaults-and-sysvars-audit.md) | Source-backed default values and DB2 2.48 sysvar mapping |
| [Rendered GUI parity report](parity/gui-parity-report.md) | Earlier 109-control rendered interaction audit and its limits |
| [Control-surface matrix](parity/control-surface-matrix.md) | Human-readable inventory from that audit baseline |
| [Machine-readable control matrix](parity/control-surface-matrix.json) | Structured audit inventory used by the validator |

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
