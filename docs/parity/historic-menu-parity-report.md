# Historical Menu Parity Report

Last verified: 2026-07-23

## Scope

Darwinbots Modern restores the historic DB2 menu organization while retaining the modern laboratory visual language. The production shell exposes these nine top-level menus:

| Menu | Functional area |
|---|---|
| File | World creation, save/load, autosave recovery, and exit |
| Edit | Runtime policies, automatic speciation, player-bot controls, and diagnostics |
| View | Viewport overlays, background images, and shot-visibility controls |
| Robot | Inspection, lineage, DNA export, color, selection, cloning, killing, and species branching |
| Objects | Shot lifetime policy and tie enablement |
| E-Grid | Historical energy-grid density choices |
| Internet | Historical online-play affordances, intentionally disabled |
| Recording | Living snapshots and dead-organism records |
| Help | DNA, robot-tag, survival, and application reference material |

## Acceptance result

- 74 historical leaf commands are declared in the typed `LegacyMenuCommand` catalog.
- All 74 commands have a production handler and enabled-state policy.
- No historical command resolves to the fail-closed `not implemented` handler.
- All nine top-level menus open in the rendered Avalonia audit.
- Runtime settings are applied through queued engine commands rather than mutating UI-only state.
- Mutation controls persist enabled state, global multiplier, and per-species rates in authoritative snapshots and saves.
- Tie policy, shot decay, robot fixing, asexual reproduction, species color, organism aim, and species branching are engine-backed.
- View overlays and imported backgrounds alter the viewport directly.
- Robot inspection, lineage, console, gene activation, restriction, and RGB-memory commands open populated diagnostic surfaces.
- Living snapshots, dead records, robot DNA, and entire-organism exports write versioned data through user-selected or application-data locations.

## Intentional differences

- Online play was explicitly removed from the modernization scope. `Internet Enabled`, `Internet Teleporter`, and `Internet Options` remain visible for historical discoverability but are disabled with an explanatory tooltip.
- DB2 modal tools are consolidated into focused Avalonia report and value-entry windows instead of reproducing VB6 pixel layouts.
- GPU selection, turbo rendering suspension, autosave recovery, and integrated Live Settings are modern additions.
- RGB memory monitoring is an on-demand diagnostic snapshot rather than a continuously repainting VB6 monitor.

## Verification receipts

| Check | Result |
|---|---|
| Rust live legacy-menu settings tests | 2 passed |
| Desktop core tests | 59 passed |
| Rendered desktop interaction tests | 21 passed |
| Historical command source audit | 74 of 74 routed; 0 missing |
| Rendered top-level menu audit | 9 of 9 opened |
| Intentionally disabled commands | 3 online-only commands |

The latest generated runtime screenshot is written to `%TEMP%\darwinbots-modern-control-audit\runtime-initial.png`. The rendered audit intentionally keeps generated binary evidence out of Git; rerun the desktop test project to regenerate it.
