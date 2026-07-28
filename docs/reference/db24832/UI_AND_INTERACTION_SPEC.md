# Darwinbots 2.48.32 UI, layout, and interaction specification

## 1. Scope and authority

This document specifies the visible Windows front end of Darwinbots 2.48.32. It covers the MDI shell, arena, menu system, toolbars, dialogs, inspectors, graphs, and visual feedback. It records the shipped interface, including old spelling, hidden controls, and inconsistent conventions. It is a behavioral reconstruction, not a redesign proposal.

The declarative geometry comes from the 26 active VB6 forms in the public [2.48.32 project](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/Iersera.vbp). Event behavior comes from those forms' code. Every control, including hidden controls and indexed control arrays, appears in [the mechanical form inventory](DECLARATIVE_UI_INVENTORY.md). Historical screenshots from the official site and wiki establish the intended visual appearance: [screen shots](http://wiki.darwinbots.com/w/Screen_shots), [archived original 2.1 gallery](https://web.archive.org/web/20031010004035id_/http://digilander.libero.it:80/darwinbots/screenshots.htm).

VB6 form coordinates are twips. At 96 DPI, 15 twips is approximately one pixel. The approximation is not reliable under non-default font or DPI settings because this program predates DPI-aware layout.

## 2. Visual language

Darwinbots uses the native Visual Basic 6 and classic Win32 visual language:

- System-colored gray dialog surfaces, beveled frames, etched group boxes, native buttons, check boxes, radio buttons, list boxes, sliders, and tab strips.
- MS Sans Serif or another system dialog font at roughly 8.25 points for most controls. Dense scientific forms reduce whitespace to fit many parameters.
- A maximizable multiple-document main window. The arena is a borderless MDI child below a native menu and bitmap toolbar and above a multipart status bar.
- Small 16- to 24-pixel bitmap icons with hard edges and era-specific transparency masks. The art is functional: disk, folder, play, pause, fast mode, eye, camera, graph, sun, lock, and internet-state symbols.
- A primarily dark arena. Robot colors and transient shots provide the high-chroma content. Auxiliary dialogs use Windows system colors, not an application theme.
- Meaning is encoded by color in the arena, graph series, robot skins, internet indicators, and the RGB memory monitor. Text and shape sometimes reinforce it, but not consistently.
- Fixed-size tool windows dominate. A few inspectors resize, while most settings forms assume a desktop of at least 800×600 and conventional DPI.

There is no global theme engine, responsive layout, scalable icon set, touch design, or formal accessibility layer. Keyboard accelerators and tab order are the main non-pointer paths. Several important modes are only visible as depressed toolbar buttons, check marks, colored images, or overlay labels.

## 3. Main window composition

[`MDIForm1.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/MDIForm1.frm) defines a `14295 × 6495`-twip client area, approximately `953 × 433` pixels at 96 DPI. It starts centered and may resize with the desktop.

The vertical stack is:

1. The native Windows title bar and menu bar.
2. A 420-twip bitmap toolbar aligned to the top.
3. The MDI work area, normally occupied by the borderless arena child.
4. A 270-twip multipart status bar aligned to the bottom.

The toolbar contains both standard toolbar buttons and ordinary VB controls positioned over it. The species selector, zoom controls, zoom lock, sun toggle, internet toggle, and contest indicator are overlays rather than toolbar items. Consequently, wrapping and high-DPI behavior are fragile and their horizontal positions are fixed.

Closing the main window exits the application after the normal VB shutdown path. Stealth or tray behavior hides the shell and leaves a tray icon with a small popup menu. The tray popup can show current information or restore Darwinbots.

## 4. Complete menu model

Separators are omitted from the table but remain part of the visual grouping. Ellipses indicate a dialog or picker. Checkable commands preserve state across their corresponding runtime session or saved settings where implemented.

| Menu | Command hierarchy | Shortcut and state |
|---|---|---|
| File | New Simulation | F1 |
| File | Load Simulation | F2 |
| File | Save Simulation | F3 |
| File | Save Sim Without Mutations | Ctrl+F3 |
| File | Pause Sim (F12) | F12 handled in simulation loop |
| File | Quit | Ctrl+Q |
| Edit | Insert Organism... | Ctrl+I |
| Edit | Species... | Ctrl+X |
| Edit | General Settings... | Ctrl+P |
| Edit | Physics and Costs... | F5 |
| Edit | Global Mutation Rates... | Ctrl+G |
| Edit | Restart and Leagues... | Ctrl+F |
| Edit | Internet Options... | — |
| Edit | Disallow robot from fixing or unfixing | Checkable global restriction |
| Edit | Disable asexual reproduction for non-repopulating robots | Checkable restriction |
| Edit | Automatically tag by name... | Opens tag rule input |
| Edit | Enable Automatic Forking | Checkable |
| Edit | Settings for PlayerBot Mode... | Opens key map |
| Edit | Enable PlayerBot Mode | F11, checkable |
| Edit | Go to eye designer... | Opens eye geometry editor |
| Edit | Restriction Overwrites | F8 |
| View | Import Background Picture | File picker |
| View | Remove Background Picture | Removes imported arena image |
| View | Show Vision Grid | Checked by default |
| View | Display Movement Vectors | Checked by default |
| View | Display Shot Impacts | Checked by default |
| View | Display Resource Guages | Checked by default; shipped spelling preserved |
| View | Settings for RGB Memory Monitor... | Opens channel mapping |
| View | RGB Memory Monitor | Checkable |
| Robot | Show Robot Info | Ctrl+R |
| Robot | Show Philogeny | Ctrl+T; shipped spelling preserved |
| Robot | Mutation Rates | Ctrl+M |
| Robot | Change Color | Ctrl+C |
| Robot | Open Console | Ctrl+Y |
| Robot | View Gene Activations | Ctrl+Z |
| Robot | Kill Robot | Ctrl+K |
| Robot | Save Robot's DNA | Ctrl+S |
| Robot | Make New Species | Forks selected robot as species |
| Robot | Select Entire Organism | Ctrl+F1 |
| Robot | Save Entire Organism | Ctrl+F2 |
| Robot | Kill Entire Organism | Ctrl+E |
| Robot | Find Best | Ctrl+B |
| Objects › Shots | Don't decay nrg shots | Checkable |
| Objects › Shots | Don't decay waste shots | Checkable |
| Objects › Ties | Disable Ties | Checkable |
| Objects › Teleporters | New; Hilight; Delete; Delete All | Operates on arena teleporters; shipped “Hilight” spelling |
| Objects › Shapes | New; Add Ten Random; Delete 10 Random; Delete Shape; Delete All | Shape construction and deletion |
| Objects › Mazes | Simple Horizontal; Simple Vertical; Sprial; Checkerboard; Polar Ice; Trash Compactor | Creates obstacle layouts; shipped “Sprial” spelling |
| E-Grid | Enable; Grid Size › Large, Medium, Small | Whole top-level menu is hidden in 2.48.32 |
| Internet | Enabled | Checkable internet mode |
| Internet | Internet Teleporter... | Disabled until internet mode supplies a teleporter |
| Recording | Safemode Backup | Checkable |
| Recording | Snapshot of the living | Immediate snapshot |
| Recording › Snapshot of the dead | Enable recording; Exclude repopulating robots | Persistent snapshot policy |
| Help | DNA Help | Ctrl+D |
| Help | Robot Tag Information | — |
| Help | Survival Info | Hidden unless a relevant mode exposes it |
| Help | About | Ctrl+A |

Robot commands act on the focused robot. When no valid focus exists, handlers either do nothing or ask the user to select a robot. Organism commands traverse permanent ties, not merely species membership.

## 5. Toolbar and direct manipulation

The standard toolbar contains 24 ordered slots. Separators occupy slots 4, 8, 15, 20, and 22.

| Slot | Function | Interaction |
|---:|---|---|
| 1 | New simulation | Same path as File › New Simulation |
| 2 | Load simulation | Opens a simulation file |
| 3 | Save simulation | Saves current simulation |
| 5 | Play | Runs continuously |
| 6 | Single cycle | Advances one complete simulation cycle |
| 7 | Pause | Stops continuous execution |
| 9 | 15 cycles per second | Toggle speed limiter |
| 10 | Fast mode | Toggle accelerated execution |
| 11 | Flicker mode | Toggle reduced/alternating draw behavior |
| 12 | No skin | Toggle robot skin rendering; no shipped tooltip |
| 13 | Disable poffs | Suppress ornamental death particles |
| 14 | Disable graphics | Run without arena drawing |
| 16 | Insertion mode | Toggle arena-click organism insertion |
| 17 | Species selector | Host slot for the overlaid drop-down |
| 18 | Find best | Select a high-fitness robot using current global weighting |
| 19 | Graphs | Split/drop-down command described below |
| 21 | Snapshot living | Records all live robot DNA |
| 23 | Stealth | Hides to the tray; no shipped tooltip |
| 24 | Ignore errors | Enables permissive error handling |

Overlaid controls add:

- A drop-down-only species list used by insertion mode.
- Zoom in and zoom out bitmap buttons.
- A lock toggle whose tooltip says it controls looking outside the arena. In practice it constrains the pan/zoom viewport.
- A sun day/night toggle.
- An internet-mode toggle.
- An F1/contest picture used as a mode indicator.

The graph drop-down has this exact sequence: Population; Average mutations; Average mutations/DNA len; Average Age; Average Descendents; Average Energy; Average DNA length; Average DNA conditions; Total Energy; Dynamic Cost Stats; Species Diversity; Average Chloroplasts; Selective Genetic Distance; Generational Distance; Slow Genetic Distance; Customizable Graphs 1, 2, and 3; Reset all graphs; List all running graphs. Visual separators divide the resource/statistical graphs, genetic graphs, custom graphs, and management commands.

## 6. Arena viewport and visual encoding

[`main.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/main.frm) is a borderless MDI child. The engine renders directly with VB drawing primitives. The camera maps simulation coordinates to form coordinates through zoom and pan state.

The arena must render, subject to view toggles:

- Each robot as a colored circular body with an optional bitmap-like skin pattern.
- A facing/aim indication and nine-eye visual fan when the vision grid is enabled.
- Resource gauges around or near a robot for energy, body, and defensive stores.
- Velocity or voluntary movement vectors.
- Ties as lines between connected robots. New ties are spring-like links; hardened ties are structural links.
- Shots as small moving colored particles. White identifies energy shots, cyan identifies viruses, yellow identifies poison, and other shots inherit the firing robot's color unless overridden.
- Brief impact flashes when shot-impact display is enabled.
- Obstacles as axis-aligned rectangles with configured color, opacity, drift, vision, and shot behavior.
- Teleporters as masked bitmap sprites, with different visual states for local, inbound, outbound, and internet use.
- An optional imported background image beneath simulation objects.
- Optional RGB memory-monitor color, which replaces or modulates a robot's normal visible color using three mapped memory values.

Runtime overlays are ordinary hidden labels and images made visible by mode. They include “PlayerBot Mode,” saving activity, “Safe Mode,” graph-update progress, a buoyancy warning, “Internet Mode,” internet server/start/backpressure/file-problem indicators, and an Italian message that video output is interrupted. These overlays sit above the arena rather than in a separate notification system.

Primary pointer behavior:

- Click a robot to focus it. The selected robot receives a visible highlight and becomes the target of Robot menu commands and detail windows.
- Drag in the arena to pan the view when camera behavior permits.
- Zoom controls change scale around the current viewport. Zoom lock limits invalid or out-of-arena camera positions.
- In insertion mode, clicking attempts to insert the species chosen in the toolbar at the corresponding world position.
- Object modes use arena clicks to place, select, highlight, or remove shapes and teleporters.
- Context and double-click paths open robot details or perform focused actions depending on mode.

The front end deliberately permits simulation execution while nonmodal inspectors are open. Displayed values can change between UI events. Forms therefore refresh from the current focused robot, not a frozen snapshot, unless the user explicitly saves one.

## 7. Status bar

The bottom status bar has 13 panels. Runtime code fills them with:

1. Current cycles per second.
2. A total/performance value.
3. Robot count.
4. Total chloroplast units.
5. Birth count.
6. Total cycles.
7. Elapsed runtime in hours, minutes, and seconds.
8. Current mutation multiplier.
9. Restart count.
10. Shots processed this cycle.
11. Total simulation energy.
12. Energy delta.
13. Dynamic cost multiplier, displayed as `CostX`.

Panels resize imperfectly with the shell. The bar is an operational instrument, not a prose status area. Critical failures still use modal message boxes or safemode artifacts.

## 8. Simulation Settings window

[`OptionsForm.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/OptionsForm.frm) is a centered, fixed tool window of `10560 × 6765` twips, approximately `704 × 451` pixels. Six tabs occupy the body. A shared footer offers Load settings, Save settings, Global settings, Start new, and Change.

“Start new” applies the working copy and resets the simulation. “Change” applies mutable settings to the running simulation. Some startup-only values cannot be retrofitted cleanly. Load and Save use settings files. “Global settings” opens the separate application-level dialog.

### 8.1 Species tab

The left column is a species list. The command strip provides Add, Delete, Duplicate, Rename, and List Non-Native Species. Selecting a row loads its editable properties.

The property area includes:

- DNA file/name and source path.
- Free-form comment.
- Skin preview and Change action.
- Species color/RGB selection.
- Starting placement represented in a small field preview.
- Starting population and starting energy.
- Repopulating/autotroph status and fixed status.
- Disable Vision.
- Disable DNA Execution.
- Disable Voluntary Movement.
- Disable Reproduction.
- Virus Immune.
- Disable Mutations.
- Disable Chloroplasts.
- Fatal restriction choices used by league or evolution modes.

Species restrictions are copied to new descendants. Non-native species are those introduced after startup, such as internet immigrants, mutations forked into species, or manually classified robots.

### 8.2 General tab

The Field Controls group sets field size using a named/slider choice and explicit width and height. Independent check boxes connect left to right and top to bottom, turning each axis into a wraparound topology. The panel also exposes waste threshold and random-seed controls, including Change seed.

The vegetation and energy area contains:

- Pond Mode.
- Light intensity and sediment/gradient controls, accompanied by a stylized layered-water preview.
- Initial light energy.
- Energy-to-body allocation for photosynthetic feeding.
- Repopulation trigger, batch amount, and cooldown.
- Maximum chloroplast population pressure.
- “Force robots to sustain their own chloroplasts.”
- Day/night and threshold behavior through the Energy dialog.

Corpse controls enable corpses and select no decay, waste release, or energy release. Size and period control decay packets and timing. A graph-update interval controls chart sampling. “Fix bot radii” switches radius treatment for compatibility or controlled contests.

### 8.3 Physics and Costs tab

Physics presets are Fluid, Solid, and Custom. Fluid controls expose density/viscosity-derived drag choices. Solid controls expose friction. A Custom Physics button opens the detailed dialog.

The tab also contains movement efficiency, Brownian motion, vertical gravity, maximum velocity, and collision elasticity. The elasticity scale is labeled from Ghosts toward Marbles, giving a conceptual rather than SI-unit interpretation.

Cost presets are No Costs, F1 Default, and Custom. Custom Simulation Costs opens the full cost editor. Shot energy exchange can use Fixed Nrg or Proportional mode, with a corresponding amount or factor.

### 8.4 Mutations tab

This tab controls the simulation-wide modifier, not the complete per-species mutation matrix. It includes Disable all, a Current Multiplier slider marked low/1/high, oscillation frequency from roughly 16× through 1 to 1/16× periods, and sine versus sawtooth oscillation. The per-species mutation dialog remains available through Robot › Mutation Rates or species setup.

### 8.5 Restart and League tab

Preset commands configure F1/SS, F2/F3/MB, and SB environments. Contest settings include Use contest mode, minimum and maximum rounds, maximum cycles, and maximum population. A general restart option restarts after all heterotrophs die. League logic can override ordinary restart handling and display the Contest Results window.

### 8.6 Internet tab

The internet tab configures inbound and outbound transfer paths, user name, server address, and port. These values feed the internet helper/teleporter system. Path controls are local file-system contracts; changing them does not itself enable internet mode.

## 9. Global Settings

[`frmGset.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/frmGset.frm) is a centered fixed dialog of `10770 × 5700` twips. Its tabs are Main settings, Mutations, Leagues, and Evolution.

Main settings includes:

- Fixed screen ratio.
- Old colors compatibility.
- Suppress buoyancy warning.
- Start with video off.
- Automatically update graphs.
- Hide to tray when restarting.
- Body cutoff/“Big Bertha” prevention.
- Greedy parent or child energy-dumping enforcement.
- New random seed on Start new and on Load Simulation.
- Main directory override.
- Safemode enablement.
- Find Best weighting between population success and invested energy.
- Epigenetic reset, amplification, and overload controls.
- Save epigenetic memory as temporary gene data.
- Randomized internet pictures.

The Mutations tab exposes advanced mutation algorithms: Sunline Point2, CopyError2, Amplification, and Translocation switches; Delta2; advanced rate-change parameters; normalization; and the DNA length at which mutation rate becomes slowest.

The Leagues tab selects a source directory and tournament/stepladder behavior. It can filter or add one entrant, configure F1 recognition tolerance, set advanced starting chloroplasts, disqualify F2/F3 behaviors, and open League Restrictions.

The Evolution tab selects Simple Survival, Zerobot, or Eco Survival. Survival controls choose the source bot, stage/cycle length, population-oscillation reduction, automatic graphs, optional internet startup, and DNA-size normalization. Zerobot adds initial DNA length. Evolution Restrictions opens the restriction dialog.

## 10. Detailed parameter dialogs

### 10.1 Custom Physics

[`PhysicsOptions.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/PhysicsOptions.frm) is `9330 × 5850` twips. It separates fluid properties from floor-plane properties and general forces.

- Fluid: viscosity, density, and a live Reynolds-related display.
- Solid plane: Z gravity, static friction, kinetic friction.
- General: Bang Efficiency, Y gravity, Brownian motion, Zero Momentum, and Planet Eaters with gravitational constant `G`.
- Assistance: Help on Units and General Help.
- Commit: Okay.

Sliders and numeric text fields synchronize through change/scroll handlers. Invalid or out-of-range values are clamped or reverted when the settings are applied.

### 10.2 Custom Simulation Costs

[`CostsForm.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/CostsForm.frm) is a dense `10920 × 9255`-twip form.

DNA-command costs cover Number, Dereference (`*number`), Basic, Advanced, Bitwise, Condition, Logic, Stores, and Flow. Morphological/action costs cover voluntary movement, rotation, tie formation, shot formation, DNA upkeep, venom, poison, slime, shell, body upkeep, DNA copy, and chloroplast construction.

Aging costs include a base cost, starting age, linear increase, and logarithmic increase. Dynamic-cost overrides include the cost multiplier, whether all robots count toward the target, lower and upper zero/reinstate thresholds, negative-multiplier permission, target population/range, and sensitivity. F1 Default restores the competition preset. Okay commits.

### 10.3 Energy, sun, and weather

[`EnergyForm.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/EnergyForm.frm) is `5610 × 5025` twips. It sets day/night cycle period, advanced tide period and offset, weather/sun variation, and total-energy thresholds for forcing sun up or down.

Threshold mode is one of temporary suspend, permanent toggle, or advance to dawn/dusk. These choices determine whether the normal day/night counter pauses, is bypassed, or is advanced to the requested phase.

### 10.4 Mutation Rates

[`NeoMutprob.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/NeoMutprob.frm) is `9165 × 8490` twips. The mutation selector covers Point, Delta Mutation, Copy Error, Insertion, Reversal, Minor Deletion, Amplification, Major Deletion, Translocation, Point2, and Copy Error2.

For the selected class, the form shows enablement, “1 in …” probability, Gaussian lower/mean/upper/standard-deviation information, and sliders controlling whether value or token type changes. Category-distribution controls distinguish Number, Dereference, Operator, Logic, Comparison, and Flow outcomes. Summary fields estimate mutation likelihood. Default restores standard rates; Okay applies.

### 10.5 Restriction Options

[`frmRestriOps.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/frmRestriOps.frm) defines separate columns for repopulating and non-repopulating robots. Each can force fixed state or disable vision, DNA, reproduction, virus infection, mutation, movement, or chloroplasts. Fatal restrictions can kill a robot that fails to form a multibot within 150 cycles or disqualify league violations. Presets can be loaded, saved, and applied.

## 11. Object editors

### 11.1 Shape editor

[`ObstacleForm.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/ObstacleForm.frm) configures rectangular shapes. Width and height use sliders. Color is random or black. A shape is transparent or opaque, can be visible to bots or see-through, and can absorb shots. Independent vertical and horizontal drift controls set velocity. Maze controls specify corridor and wall widths. Add Shape creates another object; OK applies to the current object.

### 11.2 Teleporter editor

[`TeleportForm.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/TeleportForm.frm) supports intrasim, outbound, inbound, and internet teleporters.

The form configures transfer path or network fields, accepted organism types (heterotroph, autotroph, corpse), inbound poll interval, maximum organisms per poll, backflow limit, fixed position, whether shapes block transfer, and size. Internet mode supplies separate paths/status. Statistics report transferred organisms and activity. OK commits; Cancel discards.

Object creation changes the cursor/mode in the arena. Deletion commands act on the selected object or all objects after the handler's normal confirmation behavior.

## 12. Robot inspection and manipulation

### 12.1 Robot Details

[`robdata.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/robdata.frm) is a fixed tool window with a narrow facts column and a large DNA/memory pane.

The facts column reports species, last simulation, robot array ID, unique ID, parent ID, age, generation, offspring, energy, body, mass, radius, velocity, waste, permanent waste, venom, shell, slime, poison, virus timer, chloroplasts, new mutations, total mutations, DNA length, gene count, and descendants. Buttons open descendants/phylogeny, console, gene activations, reproduction, DNA detail, memory detail, and mutation detail.

The right pane switches between formatted DNA and memory views. The form can show memory after DNA execution but before the rest of the cycle, making it a debugger for cycle-order effects. Double-clicking the tag permits editing. Values refresh against the currently selected robot.

### 12.2 Console

[`console.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/console.frm) is a compact, custom-skinned debugger resembling a small media player. It has a rich-text transcript, a command/input field, and bitmap controls for running, pausing, stepping cycles, inspecting eyes, and DNA debugging. The console attaches to the focused robot and exposes memory/DNA execution state rather than acting as a general operating-system shell.

### 12.3 Gene Activations

[`ActivForm.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/ActivForm.frm) is a shallow `5715 × 990`-twip tool. It draws a compact bar/timeline representation of genes whose store operations activated during the latest DNA execution. It follows the focused robot.

### 12.4 Philogeny

[`parentele.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/parentele.frm) retains Italian-era labels. It traverses parent/child identifiers and can highlight descendants or the whole known lineage to a selected generation depth. Historical identity data is bounded and not a complete external genealogy database.

### 12.5 Color editor

[`colorform.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/colorform.frm) uses red, green, and blue sliders with a live preview. “Write color as data into DNA file” persists the chosen presentation metadata. “Use This Color” applies it to the robot or species context.

## 13. Specialized monitors and editors

### 13.1 RGB Memory Monitor

[`frmMonitorSet.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/frmMonitorSet.frm) maps one memory location to each red, green, and blue channel. Each channel has a floor and ceiling. Runtime values are normalized into 0–255 display intensity and used to recolor robots. Presets may be loaded and saved. OK applies; Cancel closes without applying the working values.

### 13.2 PlayerBot key map

[`frmPBMode.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/frmPBMode.frm) lists key-to-memory bindings. Add enters a capture state with a “Press any key” overlay; Delete removes a selected binding. Load and Save use a key-map file. During PlayerBot mode, keyboard state overwrites the selected bot's mapped memory after normal sense clearing and before shots and robot updates.

### 13.3 Eye designer

[`frmEYE.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/frmEYE.frm) exposes nine rows for `eye1` through `eye9`. Each row controls direction and width. “Write DNA” produces commands that recreate the design. Optional setup actions turn Brownian motion off, reset aim, and disable costs so a user can test the eye fan predictably.

## 14. Graphs and contest UI

[`grafico.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/grafico.frm) draws charts directly rather than embedding a chart package. It owns axes, series lines, legend arrays, min/max scaling, and resize logic. Update Now samples immediately. Reset discards current history. “Save on d.p. 999” writes graph data at the designated point for later joining/export. Multiple graph windows can run at once and the toolbar menu can list them.

[`Contest_Form.frm`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/Contest_Form.frm) shows competitors, wins, and populations, with five visible entry rows. Manual W and instant-win controls let an operator adjudicate a round. Contest logic may add rounds until its statistical win condition is met.

## 15. Help, onboarding, notifications, and About

- DNA Help is a `10455 × 7335`-twip text viewer with a large help area, Basic Help, and Exit.
- First-time onboarding is a very large `14115 × 11160`-twip form dominated by an embedded instructional bitmap. Its caption is “Getting Started: Please select your first simulation,” and “Got it” dismisses it.
- `NetEvent` is a tiny borderless network-event toast with a Close action.
- About windows display product/version and credits. Version captions derive from VB project metadata rather than a separately maintained UI constant.

## 16. Interaction-state requirements

The shipped front end follows these state rules:

- Play, Pause, and Single Cycle must remain mutually coherent. A single step runs one full cycle, not one internal subsystem.
- Video-off and fast modes affect drawing and pacing, not the canonical simulation state transition.
- Menus and toolbar toggles mirror the same underlying flags where both exist.
- Focus-dependent robot commands must resolve the current robot at invocation time. A dead or removed focus must be rejected safely.
- New Simulation edits a temporary options copy. Start new commits and resets. Change commits supported values to the live simulation.
- Fixed dialogs are generally nonresponsive to DPI and window size. Graph and main windows are the principal resizable surfaces.
- File pickers use the VB common-dialog control and remember or infer Darwinbots directories. Robot DNA uses text files; simulations and settings use program-specific serialized files.
- Long work uses `DoEvents` to keep the classic UI responsive. It is cooperative and can expose partially updated display counters, so the engine keeps separate displayed totals in several places.
- Error handling ranges from normal modal messages through “Ignore errors” to safemode recovery. The latter writes recovery artifacts and narrows a crash to cycle windows.

## 17. Known front-end defects and compatibility traits

These are product behaviors worth preserving in an archival clone but fixing in a modernized product:

- Misspellings include “Guages,” “Philogeny,” “Hilight,” “Sprial,” and several internal Italian labels.
- Some modes have bitmap-only buttons without tooltips.
- Color-only state is common.
- There is no undo for killing robots or deleting all objects.
- Modal and nonmodal patterns are inconsistent.
- Tab order in dense settings forms reflects construction history more than reading order.
- Numeric input assumes United States decimal/group separators. The official download page warns that other regional formats can cause UI errors.
- The toolbar uses absolute overlays, so high DPI, font substitution, or narrow windows can misalign controls.
- First-time help is a raster image and does not reflow, scale cleanly, or expose semantic text.
- Several hidden or partially retired features remain in forms, including E-Grid and Survival Info.
- “Old colors,” fixed screen ratio, fixed radii, and external `sysvars2.21.txt` exist to preserve old simulations and bots.

## 18. Front-end acceptance checklist

A behaviorally faithful implementation must demonstrate:

- All top-level menus and commands above, including enable/disable and hidden states.
- Toolbar parity, tooltips, direct controls, and mode synchronization.
- Selection, organism traversal, insertion, pan, zoom, focus, background, and object-edit interactions.
- Arena rendering for robots, skins, eyes, ties, every shot class, obstacles, teleporters, impact flashes, gauges, vectors, and overlays.
- All six simulation tabs and all four global tabs with reversible editing before commit.
- Every detailed dialog and inspector listed above.
- Live graph updates and simultaneous graph windows.
- Keyboard shortcuts and PlayerBot memory overwrites at the correct point in the cycle.
- Save/load pickers and compatibility paths.
- Visual checks at classic 96-DPI desktop scale, plus documented degradation at other DPI settings.

For field-by-field geometry and default declarative state, use [the full control inventory](DECLARATIVE_UI_INVENTORY.md). That appendix is intentionally exhaustive; this document supplies the semantics the raw form tree cannot express.
