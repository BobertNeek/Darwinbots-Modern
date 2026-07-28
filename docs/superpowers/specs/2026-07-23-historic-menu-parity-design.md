# Darwinbots Historic Menu Parity Design

## Objective

Replace the narrow Live Settings proposal with the complete Darwinbots 2 menu model. Preserve the historical menu names, grouping, command intent, keyboard behavior, and contextual Robot actions while updating only colors, typography, spacing, and icons to the Darwinbots Modern visual system.

Completion requires every displayed command to invoke a typed handler and produce a visible, testable result. Disabled placeholders, empty click handlers, and controls that only update UI state are not acceptable.

## Authority

- Behavioral and menu authority: `Darwinbots2/MDIForm1.frm` from the VB6 application.
- Modern application shell: `modern/desktop/src/Darwinbots.Desktop/Views/MainWindow.axaml` and its code-behind.
- Existing behavior should be reused through `SimulationSession` and `IEngineClient` rather than reimplemented in view code.
- Legacy Internet and Recording commands remain in scope because the requested surface is the complete historical menu set.

## Visual Thesis

Darwinbots Modern is a living laboratory instrument: warm ivory working surfaces, dense technical typography, thin structural rules, restrained green action state, cyan lineage identity, amber pending state, and compact monochrome icons. The simulation world remains the dominant visual plane.

## Content Plan

1. A conventional desktop menu bar immediately below the window title area contains the historical top-level groups.
2. The transport toolbar remains directly below the menu bar for frequent Run, Pause, Step, Turbo, import, reset, backend, save, and load actions.
3. Existing left simulation shortcuts remain shortcuts only; they do not replace menu commands.
4. Settings commands open focused modeless or modal windows that match DB2 grouping instead of a single all-purpose drawer.
5. Robot commands are available both from the `Robot` menu and from the world viewport context menu when an organism is selected.

## Interaction Thesis

- Menu opening is immediate and conventional; no animated mega-menu or card navigation is introduced.
- Checked commands expose live engine state and update only after the corresponding engine command succeeds.
- Commands that need parameters open a focused dialog; Apply writes through a command batch at the next safe simulation phase boundary.
- Destructive commands require a compact confirmation only when the action cannot be undone.
- Numeric mutation controls always expose the actual value. Mutation can be enabled or disabled, but the rate is independently editable per species and globally.

## Historical Menu Inventory

### File

- New Simulation
- Load Simulation
- Save Simulation
- Save Sim Without Mutations
- Pause Sim (`F12`)
- Quit

### Edit

- Insert Organism...
- Species...
- General Settings...
- Physics and Costs...
- Global Mutation Rates...
- Restart and Leagues...
- Internet Options...
- Disallow robot from fixing or unfixing
- Disable asexual reproduction for non-repopulating robots
- Automatically tag by name...
- Enable Automatic Forking
- Settings for PlayerBot Mode...
- Enable PlayerBot Mode
- Go to eye designer...
- Restriction Overwrites

### View

- Import Background Picture
- Remove Background Picture
- Show Vision Grid
- Display Movement Vectors
- Display Shot Impacts
- Display Resource Gauges
- Settings for RGB Memory Monitor...
- RGB Memory Monitor

### Robot

- Show Robot Info
- Show Phylogeny
- Mutation Rates
- Change Color
- Open Console
- View Gene Activations
- Kill Robot
- Save Robot's DNA
- Make New Species
- Select Entire Organism
- Save Entire Organism
- Kill Entire Organism
- Find Best

### Objects

- Shots / Don't decay energy shots
- Shots / Don't decay waste shots
- Ties / Disable Ties
- Teleporters / New Teleporter...
- Teleporters / Highlight Teleporters
- Teleporters / Delete Teleporter
- Teleporters / Delete All Teleporters
- Shapes / New Shape...
- Shapes / Add Ten Random Shapes
- Shapes / Delete 10 Random Shapes
- Shapes / Delete Shape
- Shapes / Delete All Shapes
- Mazes / Simple Horizontal Maze
- Mazes / Simple Vertical Maze
- Mazes / Spiral
- Mazes / Checkerboard
- Mazes / Polar Ice
- Mazes / Trash Compactor

### E-Grid

- Enable
- Grid Size / Large
- Grid Size / Medium
- Grid Size / Small

### Internet

- Enabled
- Internet Teleporter...
- Internet Options... remains reachable from Edit for historical parity.

### Recording

- Safemode Backup
- Snapshot of the Living
- Snapshot of the Dead / Enable Recording
- Snapshot of the Dead / Exclude Repopulating Robots

### Help

- DNA Help
- Robot Tag Information
- Survival Info
- About

### System Tray

- Show Information, including cycles, global mutation multiplier, total population, animal population, vegetable population, and births
- Show Darwinbots

## Menu Architecture

### Catalog

Create a pure `LegacyMenuCatalog` containing immutable menu definitions. Each leaf has:

- stable `LegacyCommandId`
- historical label
- optional icon key
- optional shortcut
- checkable/radio metadata
- required selection or engine capability
- help text used by tests and accessibility

The catalog is the single inventory used to construct the menu, generate acceptance tests, and validate that no historical command is omitted.

### Command Router

Create `LegacyMenuCommandRouter` outside `MainWindow`. It receives a `LegacyCommandContext` containing `SimulationSession`, storage service, current snapshot, selected organism, window owner, and application services. One exhaustive switch routes every `LegacyCommandId` to a focused service or dialog.

The router returns `LegacyCommandResult` with success, cancellation, failure, and user-facing status text. Checked UI state is refreshed from authoritative engine/application state after success; handlers do not optimistically toggle state.

### Focused Services

- `SimulationFileCommands`: new, load, save, save without mutations, autosave, quit.
- `SpeciesCommands`: insert, edit species, mutation rates, tags, forking, restrictions, PlayerBot, eye design.
- `RobotCommands`: inspect, phylogeny, console, gene activity, color, kill, export, speciate, whole-organism actions, find best.
- `WorldObjectCommands`: shots, ties, teleporters, shapes, and maze generators.
- `ViewCommands`: background, overlays, movement vectors, impacts, gauges, and memory monitor.
- `InternetCommands`: connection state, options, and teleporter configuration.
- `RecordingCommands`: backup and living/dead snapshots.
- `HelpCommands`: local documentation and About.

Views may coordinate dialogs but may not contain simulation algorithms.

## Mutation Model and UI

`Global Mutation Rates...` opens a table of species and global controls. Every species row contains:

- mutation enabled checkbox
- numeric mutation rate in percent with a slider and editable numeric field
- current effective probability summary
- reproduction mutation enabled checkbox
- continuous lifetime mutation enabled checkbox
- accumulated mutation event count
- automatic speciation enabled checkbox
- genetic distance threshold

`Robot / Mutation Rates` opens the same editor scoped to the selected robot's species, with inherited and effective values visible.

Changing a checkbox never destroys the stored rate. Re-enabling mutation restores the last numeric rate. Apply sends one atomic settings command to the engine; Cancel leaves engine and UI state unchanged.

The initial compatibility mapping is:

`per-base per-tick probability = mutation percent / 5,000,000`

Therefore `1.0%` approximates one point-mutation opportunity per 5,000 bases per cycle. The engine stores the probability and mutation schedule; the desktop stores only user presentation preferences.

## Engine and ABI Requirements

The engine command surface must support runtime updates for all checkable and parameterized simulation commands. Commands are queued through `SimulationSession` and applied between phases.

Required command families:

- species and global mutation policy
- reproduction/fixing/tie restrictions
- visual overlay state that depends on simulation data
- shot decay policy
- E-Grid enabled state and size
- teleporter CRUD
- shape CRUD and maze generation
- selected robot mutation/color/speciation/kill/whole-organism actions
- recording and snapshot filters
- PlayerBot and eye-design state
- Internet connection and interchange state

The C ABI exposes versioned command batches and structured errors. GPU and CPU backends consume the same settings contract.

## Windows and Dialogs

Historical dialogs are separate focused windows rather than pages in one drawer:

- Species Manager
- General Settings
- Physics and Costs
- Mutation Rates
- Restart and Leagues
- Internet Options
- PlayerBot Settings
- Eye Designer
- Restriction Overwrites
- RGB Memory Monitor Settings
- Robot Info
- Phylogeny
- Robot Console
- Gene Activations
- Teleporter Editor
- Shape Editor
- Internet Teleporter
- DNA Help
- Robot Tag Information
- Survival Info
- About

Existing `SetupWindow`, `AdvancedSettingsWindow`, and `DnaEditorWindow` should be reused or split where their current responsibility matches these windows.

## Error Handling

- Missing selection: Robot commands remain visible but disabled with accessible help explaining that a robot must be selected.
- Unsupported backend capability: the command remains visible, disabled, and reports the missing capability in its tooltip. Release acceptance requires both CPU and GPU paths or a safe CPU fallback for simulation commands.
- Invalid numeric input: Apply remains disabled and the field shows a concise inline reason.
- Engine rejection: retain entered values, do not update check state, and show the structured engine error in the status bar and dialog.
- File cancellation is a normal canceled result, not an error.
- Device loss during a command triggers CPU fallback before the command is retried once.

## Verification Contract

1. Parse `MDIForm1.frm` in a test fixture and compare normalized historical menu labels with `LegacyMenuCatalog`. Every historical leaf command must be represented or explicitly classified as separator/status-only.
2. A catalog test requires every leaf to have a router case and verifies no router case lacks a catalog entry.
3. Handler tests execute every command against fakes and assert a state change, dialog request, file request, documentation request, or structured cancellation.
4. Numeric mutation tests prove rate editing, enable/disable preservation, atomic Apply, Cancel, persistence, and runtime engine effect.
5. Rendered GUI automation opens every top-level menu, submenu, and dialog; exercises each safe command; and captures destructive confirmation surfaces without confirming destructive actions in the shared fixture.
6. CPU and GPU differential tests compare all runtime settings commands and fallback behavior.
7. The final parity matrix contains no `unwired`, `placeholder`, or `unknown` status for any displayed command.

## Delivery Sequence

1. Catalog and router with exhaustive tests.
2. Menu bar and visual styling.
3. Existing command wiring.
4. Mutation editor and engine mutation commands.
5. Robot and species tools.
6. View and world-object tools.
7. E-Grid, Internet, Recording, and Help.
8. Exhaustive rendered audit and parity closure.
