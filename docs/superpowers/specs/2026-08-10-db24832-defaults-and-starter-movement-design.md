# DB2.48.32 Defaults and Starter Movement Design

## Goal

Make every supported clean-new-world default match Darwinbots 2.48.32 while preserving approved modern-only controls. Verify that newly spawned organisms move without manual dragging.

## Authority and scope

The clean-default authority is `Darwinbots2/MDIForm1.frm`, especially `MDIForm_Load`, supplemented by `SimOptions.bas` and the relevant legacy forms. Saved `lastran.set` and `lastexit.set` files are user state and are not defaults. F1, F2, and SB configurations are optional profiles.

This change covers engine defaults, desktop setup models, dialog controls, presentation toggles, starter-species settings, and reset/new-world behavior. It does not alter loaded saves or saved user settings.

## Default classification

Each exposed setting belongs to one class:

1. DB2-backed: use the clean DB2 2.48.32 value.
2. Modern-only: preserve the current value because DB2 has no equivalent.
3. Unsupported legacy setting: retain the documented boundary and do not invent behavior.

DB2-backed values include Brownian motion `0.5`, world size `16000 x 12000`, maximum velocity `60`, movement efficiency `0.66`, chart interval `200`, gradient `1.02`, legacy ecology values, mutation rates, competition values, and presentation toggles initialized by `MDIForm_Load`.

Modern-only values include Auto GPU selection, CPU fallback, deterministic seed controls, organism capacity, the vegetable safety cap, offline-only behavior, and Zerobot assistance.

## Architecture

Rust remains simulation authority. `EngineConfig` and its nested settings define engine behavior. Desktop default objects and XAML controls must serialize matching DB2-backed values into that authority.

Defaults remain idiomatic constants in their existing Rust and C# domains. Focused tests compare the resulting clean setup and serialized engine configuration with an explicit DB2.48.32 expectation table. This avoids adding a runtime manifest or legacy settings-file dependency.

## Starter movement investigation

Animal Minimalis is a reproduction witness, not a special case. The current headless executable records movement on CPU and GPU without dragging. The desktop executable used during testing predates later engine and UI parity commits, so the first diagnostic action is a fresh desktop build.

The fresh starter-world check must prove that untouched, non-fixed organisms generate intentional movement and position changes. It must run once on CPU and once on the physical GPU path. If the engine moves but the viewport does not, the fix belongs in snapshot publication or desktop rendering. If the engine does not move, the fix belongs at the first stale import, sensing, DNA, command, or physics boundary. Dragging cannot initialize or unlock normal movement.

## Error handling

Invalid legacy values continue to use existing validation. Unsupported defaults remain explicit instead of being approximated. GPU failure keeps the approved Auto-to-CPU fallback and must not be reported as a GPU pass.

## Verification

Use RED-first focused tests for every production behavior change. The minimum gates are:

- Rust clean-default contract tests.
- Desktop setup/default serialization tests.
- One fresh starter-world movement regression on CPU.
- The same regression on a physical GPU, with adapter receipt.
- One desktop build proving the current executable contains the changes.

No broad suite is required unless a focused failure exposes a wider dependency.

