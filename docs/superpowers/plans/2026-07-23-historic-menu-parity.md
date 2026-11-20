# Historic Darwinbots Menu Parity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Recreate every user-facing Darwinbots 2 menu in Avalonia with modern styling and prove that every displayed command invokes working application or engine behavior.

**Architecture:** An immutable catalog defines the complete legacy hierarchy. A typed router converts catalog command IDs into focused desktop services, dialogs, and queued `SimulationSession` engine commands. Catalog, router, engine, and rendered-GUI tests share the same command IDs so missing and inert commands fail automatically.

**Tech Stack:** .NET 10, Avalonia, C# desktop/core libraries, Rust simulation engine, versioned JSON C ABI, xUnit, Rust integration tests, Xvfb/xdotool rendered audit.

## Global Constraints

- VB6 `Darwinbots2/MDIForm1.frm` is the menu and behavioral authority.
- Preserve historical labels and hierarchy; update only color, typography, spacing, icons, and error presentation.
- Every displayed leaf command must have one typed router case and an observable result.
- Mutation enablement and numeric mutation rate are separate state.
- Runtime simulation changes are queued and applied between simulation phases.
- CPU and GPU backends consume the same settings commands with CPU fallback after device failure.
- Do not perform Git operations unless the user explicitly requests them.

---

### Task 1: Historical Menu Catalog and Coverage Gate

**Files:**
- Create: `modern/desktop/src/Darwinbots.Desktop.Core/LegacyMenus/LegacyCommandId.cs`
- Create: `modern/desktop/src/Darwinbots.Desktop.Core/LegacyMenus/LegacyMenuItem.cs`
- Create: `modern/desktop/src/Darwinbots.Desktop.Core/LegacyMenus/LegacyMenuCatalog.cs`
- Create: `modern/desktop/tests/Darwinbots.Desktop.Core.Tests/LegacyMenuCatalogTests.cs`

**Interfaces:**
- Produces: `LegacyCommandId`, `LegacyMenuItem`, and `LegacyMenuCatalog.TopLevel`.
- Consumed by: router, Avalonia menu builder, and parity tests.

- [ ] **Step 1: Write the catalog coverage tests**

```csharp
[Fact]
public void Catalog_matches_historical_top_level_order() =>
    Assert.Equal(
        ["File", "Edit", "View", "Robot", "Objects", "E-Grid", "Internet", "Recording", "Help"],
        LegacyMenuCatalog.TopLevel.Select(item => item.Label));

[Fact]
public void Every_leaf_has_a_unique_command_id()
{
    var leaves = LegacyMenuCatalog.FlattenLeaves().ToArray();
    Assert.Equal(leaves.Length, leaves.Select(item => item.Command).Distinct().Count());
    Assert.DoesNotContain(leaves, item => item.Command is null);
}
```

- [ ] **Step 2: Implement immutable catalog types**

```csharp
public sealed record LegacyMenuItem(
    string Label,
    LegacyCommandId? Command = null,
    string? IconKey = null,
    string? Shortcut = null,
    LegacyMenuCheckKind CheckKind = LegacyMenuCheckKind.None,
    IReadOnlyList<LegacyMenuItem>? Children = null);
```

- [ ] **Step 3: Encode every menu entry from the design specification**

`LegacyMenuCatalog.TopLevel` must include all leaves listed in `docs/superpowers/specs/2026-07-23-historic-menu-parity-design.md`, including nested Shots, Ties, Teleporters, Shapes, Mazes, Grid Size, and dead-snapshot commands.

- [ ] **Step 4: Run the focused test**

Run: `dotnet test modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj --filter LegacyMenuCatalogTests`

Expected: all catalog tests pass and the leaf count is stable.

---

### Task 2: Typed Command State and Router Contract

**Files:**
- Create: `modern/desktop/src/Darwinbots.Desktop.Core/LegacyMenus/LegacyCommandState.cs`
- Create: `modern/desktop/src/Darwinbots.Desktop.Core/LegacyMenus/LegacyCommandResult.cs`
- Create: `modern/desktop/src/Darwinbots.Desktop/Services/LegacyMenuCommandRouter.cs`
- Create: `modern/desktop/src/Darwinbots.Desktop/Services/ILegacyDialogService.cs`
- Create: `modern/desktop/tests/Darwinbots.Desktop.Tests/LegacyMenuCommandRouterTests.cs`

**Interfaces:**
- Consumes: `LegacyCommandId`, `SimulationSession`, `IDesktopStorageService`, selected organism state.
- Produces: `Task<LegacyCommandResult> ExecuteAsync(LegacyCommandId command)` and `LegacyCommandState GetState(LegacyCommandId command)`.

- [ ] **Step 1: Write exhaustive routing tests**

```csharp
[Theory]
[MemberData(nameof(AllCommands))]
public void Every_catalog_command_has_a_router_case(LegacyCommandId command) =>
    Assert.True(LegacyMenuCommandRouter.IsRoutable(command), $"Missing route for {command}");
```

- [ ] **Step 2: Define authoritative state and result records**

```csharp
public sealed record LegacyCommandState(bool Enabled, bool Checked, string? DisabledReason = null);
public sealed record LegacyCommandResult(bool Succeeded, bool Canceled, string Status, Exception? Error = null);
```

- [ ] **Step 3: Implement an exhaustive router switch**

The switch must contain every enum member. Initial routes may delegate to focused services introduced by later tasks, but tests must reject `NotImplementedException`, empty success, and status-only placeholders.

- [ ] **Step 4: Run router tests**

Run: `dotnet test modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj --filter LegacyMenuCommandRouterTests`

Expected: catalog and router have a one-to-one command mapping.

---

### Task 3: Avalonia Menu Bar and Modern Historical Styling

**Files:**
- Modify: `modern/desktop/src/Darwinbots.Desktop/Views/MainWindow.axaml`
- Create: `modern/desktop/src/Darwinbots.Desktop/Views/MainWindow.Menu.cs`
- Create: `modern/desktop/src/Darwinbots.Desktop/Services/LegacyMenuBuilder.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop/App.axaml`
- Modify: `modern/desktop/tests/Darwinbots.Desktop.Tests/ControlSurfaceBindingTests.cs`

**Interfaces:**
- Consumes: `LegacyMenuCatalog`, router state, router execution.
- Produces: one menu item per catalog entry with `Tag=LegacyCommandId` and stable automation IDs.

- [ ] **Step 1: Add rendered binding tests**

```csharp
[Fact]
public void Main_window_contains_all_historical_top_level_menus()
{
    var window = new MainWindow(TestSetup.Options);
    foreach (var label in LegacyMenuCatalog.TopLevel.Select(item => item.Label))
        Assert.NotNull(window.FindControl<MenuItem>($"Menu{Sanitize(label)}"));
}
```

- [ ] **Step 2: Add a dedicated menu row above the transport toolbar**

Change the root rows from `58,*,170,28` to `30,58,*,170,28`; shift existing runtime rows by one; place the generated menu in row zero.

- [ ] **Step 3: Implement compact styling**

Use ivory `#FBF9F2`, ink `#111713`, divider `#BEBBB2`, selected wash `#E1EAD8`, active green `#3F8E34`, and amber caution `#BC6B18`. Menu icons are 16 px monochrome path icons and labels use Bahnschrift.

- [ ] **Step 4: Bind click, checked, enabled, shortcut, and selection state**

`LegacyMenuBuilder` creates `MenuItem` objects recursively and updates states whenever snapshots or selections change.

- [ ] **Step 5: Run control-surface tests**

Run: `dotnet test modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj --filter ControlSurfaceBindingTests`

Expected: all top-level and leaf automation IDs resolve uniquely.

---

### Task 4: Wire Existing File, Simulation, Settings, and Robot Actions

**Files:**
- Modify: `modern/desktop/src/Darwinbots.Desktop/Services/LegacyMenuCommandRouter.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Views/MainWindow.Menu.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Views/MainWindow.axaml.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Services/AvaloniaDesktopStorageService.cs`
- Modify: `modern/desktop/tests/Darwinbots.Desktop.Tests/LegacyMenuCommandRouterTests.cs`

**Interfaces:**
- Reuses current import, save, load, pause, kill, clone, DNA export, feature CRUD, and advanced-settings paths.

- [ ] **Step 1: Add fake-service tests for existing routes**

Test New, Load, Save, Save Without Mutations, Pause, Quit, Insert Organism, General Settings, Physics and Costs, Show Robot Info, Kill Robot, Save Robot DNA, New Teleporter, New Shape, and About.

- [ ] **Step 2: Extract reusable async methods from button handlers**

Button handlers and menu routes must call shared methods such as `ImportOrganismAsync`, `SaveSimulationAsync(bool includeMutationSchedules)`, and `RemoveSelectedRobotAsync`; neither calls the other handler.

- [ ] **Step 3: Add save-without-mutations serialization option**

Expose `SaveAsync(SaveOptions options)` through session and engine. `SaveOptions.WithoutMutationSchedules` clears pending schedules in serialized output without mutating the live world.

- [ ] **Step 4: Run router and persistence tests**

Run: `dotnet test modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj --filter LegacyMenuCommandRouterTests`

Expected: all reused routes produce the same service or engine calls as existing buttons.

---

### Task 5: Numeric Global and Per-Species Mutation Rates

**Files:**
- Create: `modern/desktop/src/Darwinbots.Desktop.Core/MutationPolicy.cs`
- Create: `modern/desktop/src/Darwinbots.Desktop/Views/MutationRatesWindow.axaml`
- Create: `modern/desktop/src/Darwinbots.Desktop/Views/MutationRatesWindow.axaml.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/IEngineClient.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/SimulationSession.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/NativeEngineClient.cs`
- Modify: `modern/engine/src/ffi.rs`
- Modify: `modern/engine/src/world.rs`
- Modify: `modern/engine/src/species.rs`
- Create: `modern/engine/tests/live_mutation_policy.rs`
- Create: `modern/desktop/tests/Darwinbots.Desktop.Tests/MutationRatesWindowTests.cs`

**Interfaces:**
- Produces: `UpdateMutationPoliciesAsync(IReadOnlyList<MutationPolicy> policies)`.

- [ ] **Step 1: Write engine tests for live rate changes**

```rust
#[test]
fn disabling_mutation_preserves_configured_rate_and_reenable_restores_it() {
    let mut engine = fixture_world();
    engine.set_species_mutation_policy(0, false, 2.5, true, true, 20.0).unwrap();
    assert_eq!(engine.species()[0].mutation_rate, 2.5);
    engine.set_species_mutation_policy(0, true, 2.5, true, true, 20.0).unwrap();
    assert!(engine.species()[0].mutations_enabled);
}
```

- [ ] **Step 2: Add the versioned FFI command**

```json
{"type":"set_species_mutation_policy","species":0,"enabled":true,"rate_percent":2.5,"reproduction":true,"continuous":true,"speciation_threshold_percent":20.0}
```

- [ ] **Step 3: Implement atomic session update**

Serialize all edited species policies into one versioned command batch and publish one post-command snapshot.

- [ ] **Step 4: Build the mutation editor**

Each species row has a checkbox, `Slider Minimum="0" Maximum="100"`, numeric `NumericUpDown`, effective probability text, reproduction checkbox, continuous checkbox, event count, speciation checkbox, and threshold field. Slider and numeric field bind to the same decimal value.

- [ ] **Step 5: Verify Apply and Cancel**

Run focused Rust and desktop tests. Expected: Apply changes engine behavior; Cancel sends no command; disabling does not overwrite the numeric rate.

---

### Task 6: Robot, Species, View, and World-Object Tools

**Files:**
- Create: `modern/desktop/src/Darwinbots.Desktop/Services/RobotCommands.cs`
- Create: `modern/desktop/src/Darwinbots.Desktop/Services/ViewCommands.cs`
- Create: `modern/desktop/src/Darwinbots.Desktop/Services/WorldObjectCommands.cs`
- Create: `modern/desktop/src/Darwinbots.Desktop/Views/RobotInfoWindow.axaml`
- Create: `modern/desktop/src/Darwinbots.Desktop/Views/PhylogenyWindow.axaml`
- Create: `modern/desktop/src/Darwinbots.Desktop/Views/RobotConsoleWindow.axaml`
- Create: `modern/desktop/src/Darwinbots.Desktop/Views/GeneActivationsWindow.axaml`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Controls/WorldViewport.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/IEngineClient.cs`
- Modify: `modern/engine/src/ffi.rs`
- Modify: `modern/engine/src/world.rs`
- Create: `modern/engine/tests/legacy_menu_world_tools.rs`

**Interfaces:**
- Produces robot selection operations, overlays, shot/tie policies, teleporter/shape CRUD, and deterministic maze generation.

- [ ] **Step 1: Add engine tests for each stateful world command**
- [ ] **Step 2: Implement robot inspection and mutation/color/speciation actions**
- [ ] **Step 3: Implement background and overlay rendering switches**
- [ ] **Step 4: Implement shot, tie, teleporter, shape, and maze commands**
- [ ] **Step 5: Add the viewport Robot context menu from the same catalog subtree**
- [ ] **Step 6: Run focused engine and rendered desktop tests**

Every step is accepted only when the command changes snapshot state, opens a populated tool, or changes rendered viewport evidence.

---

### Task 7: E-Grid, Internet, Recording, PlayerBot, and Help

**Files:**
- Create: `modern/desktop/src/Darwinbots.Desktop/Services/EGridCommands.cs`
- Create: `modern/desktop/src/Darwinbots.Desktop/Services/InternetCommands.cs`
- Create: `modern/desktop/src/Darwinbots.Desktop/Services/RecordingCommands.cs`
- Create: `modern/desktop/src/Darwinbots.Desktop/Services/HelpCommands.cs`
- Create: `modern/desktop/src/Darwinbots.Desktop/Views/InternetOptionsWindow.axaml`
- Create: `modern/desktop/src/Darwinbots.Desktop/Views/PlayerBotSettingsWindow.axaml`
- Create: `modern/desktop/src/Darwinbots.Desktop/Views/EyeDesignerWindow.axaml`
- Create: `modern/desktop/src/Darwinbots.Desktop/Views/RgbMemoryMonitorWindow.axaml`
- Create: `modern/desktop/src/Darwinbots.Desktop/Views/AboutWindow.axaml`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Services/LegacyMenuCommandRouter.cs`
- Modify: `modern/engine/src/ffi.rs`
- Modify: `modern/engine/src/world.rs`

**Interfaces:**
- Produces functional E-Grid sizing, optional interchange connection, recording output, PlayerBot state, eye design, memory monitor, and local documentation windows.

- [ ] **Step 1: Implement E-Grid state and radio menu synchronization**
- [ ] **Step 2: Implement Internet options and explicit connect/disconnect state**
- [ ] **Step 3: Implement safe backup plus living/dead snapshot writers and filters**
- [ ] **Step 4: Implement PlayerBot, eye designer, and RGB memory monitor tools**
- [ ] **Step 5: Package DNA, tag, survival, and About documentation locally**
- [ ] **Step 6: Run focused service and router tests**

Expected: each command has a durable observable result and survives save/load where the historical setting is persistent.

---

### Task 8: Exhaustive Rendered and Behavioral Parity Audit

**Files:**
- Modify: `docs/parity/control-surface-matrix.json`
- Modify: `docs/parity/control-surface-matrix.md`
- Modify: `docs/parity/gui-parity-report.md`
- Modify: `docs/verification/control-surface-audit/run-gui-audit.sh`
- Create: `modern/desktop/tests/Darwinbots.Desktop.Tests/HistoricMenuParityTests.cs`

**Interfaces:**
- Consumes all catalog and router entries.
- Produces requirement-by-requirement completion evidence.

- [ ] **Step 1: Generate the matrix from `LegacyMenuCatalog`**
- [ ] **Step 2: Open every menu and submenu in rendered automation**
- [ ] **Step 3: Execute every safe command and inspect every destructive confirmation**
- [ ] **Step 4: Verify checked/radio state after runtime and save/load transitions**
- [ ] **Step 5: Verify all mutation numeric paths against engine snapshots**
- [ ] **Step 6: Run complete desktop and engine test suites**
- [ ] **Step 7: Require zero `unwired`, `placeholder`, `unknown`, or missing rows before completion**

Expected: the generated inventory exactly matches the historical normalized command set and every row has executable evidence.
