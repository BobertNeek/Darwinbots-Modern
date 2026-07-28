# Rendered GUI and DB2 Control-Parity Audit

Last verified: 2026-07-23

## Outcome

The original rendered audit baseline contains 109 interactive controls. The historical-menu restoration adds 74 DB2 menu leaf commands across nine top-level menus. Every historical command has a typed command ID, catalog entry, routed handler, enabled-state policy, and automated coverage proving that no command remains on the fail-closed pending implementation handler.

| Surface | Controls | Rendered result |
|---|---:|---:|
| SetupWindow | 32 | 32 pass |
| MainWindow | 39 | 39 pass |
| AdvancedSettingsWindow | 34 | 34 pass |
| DnaEditorWindow | 4 | 4 pass |
| Historical menu leaf commands | 74 | 74 routed |
| Original rendered baseline | 109 | 109 pass |

The machine-readable baseline inventory is `docs/parity/control-surface-matrix.json`. The review table is `docs/parity/control-surface-matrix.md`. The current menu-specific result is `docs/parity/historic-menu-parity-report.md`.

## Behavioral authority and limits

Control intent was compared with the original VB6 handlers in `Darwinbots2/MDIForm1.frm` and `Darwinbots2/OptionsForm.frm`. The comparison covers setup, simulation transport, robot commands, world-feature tools, settings, DNA inspection, DNA export, and new-species workflows.

The exact DB2 2.48.32 GUI could not be executed in the Linux cloud runner because its 32-bit Wine environment remained unavailable. This report therefore claims source-intent parity and rendered modern behavior, not pixel parity or exhaustive runtime comparison with the VB6 executable.

The menu audit claims functional coverage for the restored historical menu catalog. It does not claim pixel-identical VB6 dialogs, online play, or restoration of obsolete external utilities.

## Rendered interaction coverage

- Setup menus, starting modes, species fields, mutation slider, file-service workflows, advanced settings, world creation, autosave recovery, defaults reset, and exit.
- Runtime transport, throttle, turbo, backend switching, import, save, load, environment toggles, all nine expanders, live settings, viewport selection, click-to-move, drag-to-move, obstacle placement/removal, teleporter placement, clone, reproduce, follow, DNA editor launch, kill, and reset.
- Every advanced numeric setting, every advanced boolean setting, Apply, and Cancel.
- DNA text input, Apply DNA, Apply and Clone, and Save Bot.
- All nine historical top-level menus opening through the rendered Avalonia visual tree.
- Every one of the 74 historical leaf commands resolving to an explicit production handler and enabled-state policy.
- Live mutation enable, global multiplier, and per-species mutation rates applying through the simulation thread.

Inputs are delivered through Avalonia's rendered headless window with pointer and keyboard events. Asynchronous engine and storage effects are asserted through injected production interfaces, so file pickers and the engine thread are tested without bypassing their window handlers.

## Defects fixed by this audit

- Added injectable engine creation and desktop storage boundaries so runtime reset, import, save, load, autosave, and DNA export can be exercised without replacing production handlers.
- Added stable names to every audited menu, button, field, toggle, expander, and editor action.
- Corrected drag-to-move so it resolves the selected organism's stable `slot:generation` identity from the current UI snapshot before falling back to the session snapshot.
- Corrected the runtime tool rail so nested environment controls and action labels remain readable at the 1500 x 900 audit viewport.
- Added a Skia-backed test application and serialized rendered workflow coverage on Windows, Linux, and macOS CI jobs.

## Intentional modern differences

- CPU/GPU backend selection has no DB2 equivalent.
- Turbo suspends world rendering while simulation throughput continues; DB2 instead ran as fast as its UI loop allowed.
- Render Waste is a display-only control requested for the modern viewport.
- Autosave recovery replaces broad legacy settings/save migration.
- Expanders integrate DB2's separate dialogs and menu surfaces into the live simulation rail.

These rows are marked `intentional-difference` rather than being misreported as DB2 parity.

## Reproducible evidence

Run:

```powershell
pwsh docs/verification/control-surface-audit/run-rendered-audit.ps1
```

The command runs the complete desktop test project, validates the 109-row matrix, and writes these generated screenshots under `docs/verification/control-surface-audit/modern/`:

- `setup-initial.png`
- `setup-exercised.png`
- `advanced-initial.png`
- `dna-initial.png`
- `dna-exercised.png`
- `runtime-initial.png`
- `runtime-exercised.png`

Screenshots are intentionally ignored by Git. CI writes the same evidence to its control-surface audit artifact directory.
