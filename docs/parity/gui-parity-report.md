# Rendered GUI and DB2 Control Phase 0 Audit

Last verified: 2026-07-30

## Outcome

The rendered audit measures the current Avalonia application. It does not establish complete DB2 GUI or behavioral parity. The 109-row baseline inventory and 74-entry historical menu catalog are registration and interaction surfaces; green counts do not prove that each control has DB2 semantics.

| Surface | Measured boundary | Current result |
|---|---:|---:|
| Baseline control matrix | 109 controls | 109/109 passing rows validated |
| Historical menu catalog | 74 typed leaf commands | 74 registered and routed; semantics vary |
| Desktop core suite | Current project | 62 passed; 0 failed; 0 skipped |
| Rendered desktop suite | Current project | 26 passed; 0 failed; 0 skipped |

The machine-readable inventory is `docs/parity/control-surface-matrix.json`. The review table is `docs/parity/control-surface-matrix.md`. Menu-specific evidence is recorded in `docs/parity/historic-menu-parity-report.md`.

## Evidenced interaction boundary

- Typed menu commands route to explicit handlers and enabled-state policy.
- Live mutation settings execute through the authoritative simulation command path.
- Mutation-free save writes a transformed copy and leaves the running world unchanged.
- Whole-organism removal captures stable `slot:generation` identities, queues native `remove_many` JSON, executes off the caller thread, and publishes one batch update.
- The organism export picker serializes only the captured permanent-tie group and its internal ties.
- E-Grid and default Survival Info hidden states are enforced.
- Retained shortcut strings become exact input gestures and are tested as keyboard input.
- The three Internet commands remain visible for discoverability but disabled. The interaction suite, not screenshot appearance alone, proves no network work occurs.
- Current view toggles and object tools are exercised within the modern viewport contract.

## Known GUI gaps

- The MDI arena, bitmap toolbar, 13-panel status instrument, graph windows, and other desktop-shell details are replaced by a dashboard layout.
- Distinct settings workflows are collapsed into setup and live-settings surfaces.
- Robot Info, Console, Gene Activations, Philogeny, Color, PlayerBot, RGB monitor, Eye Designer, help, and related tools remain generic document/value-window substitutes or summaries.
- Parameter dialogs for physics, costs, weather, mutations, restrictions, shapes, and teleporters remain incomplete.
- Viewport rendering and interaction do not yet reproduce all DB2 eyes, gauges, object properties, click modes, cursor feedback, and zoom-lock behavior.
- Graph, status, onboarding, refresh, modal/nonmodal, and offline workflow behavior remains incomplete.
- The exact original 2.48.32 GUI has not been black-box exercised in a working compatible environment.

## Reproducible checkpoint gate

Run serially from the worktree root:

```powershell
pwsh -NoProfile -File docs/reference/db24832/verify.ps1
cargo test -p darwinbots-engine --test contracts --test manual_tools --test live_legacy_menu_settings
dotnet test modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj --no-restore --nologo
dotnet test modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj --no-restore --nologo
pwsh -NoProfile -File docs/verification/control-surface-audit/run-rendered-audit.ps1
git diff --check
```

The rendered audit writes ignored screenshots under `docs/verification/control-surface-audit/modern/`. The checkpoint covers commits through `95818f6af58bcb2365ec21f6a49e707ae15ed343`.
