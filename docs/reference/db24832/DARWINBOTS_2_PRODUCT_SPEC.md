# Darwinbots 2 complete product specification

Version 1.0 · Research baseline: Darwinbots 2.48.32 · 28 July 2026

## 1. Product definition

Darwinbots 2 is a Windows artificial-life simulator and programmable robot ecology. It places autonomous circular “bots” in a continuous two-dimensional world. Each bot runs a mutable stack-based genome, reads sensors through 1,000 integer memory cells, writes actions into that same memory, consumes energy, interacts through physics, shots, ties, and shared resources, and may reproduce. Population change is therefore the combined result of programmed behavior, random mutation, inheritance, competition, cooperation, and operator-selected world rules.

The product serves four overlapping uses:

- Write and test robot DNA.
- Run open-ended evolution simulations.
- Stage rule-governed robot leagues and contests.
- Observe and analyze artificial ecologies through live inspection, graphs, snapshots, and saved simulations.

The official wiki states the concise premise: bots “fight each other for food in a physically simulated 2D universe,” and successful bots reproduce and evolve. Its news identifies 2.48.32, released 4 November 2016, as the latest published version. The public project metadata independently builds `Darwin2.48.32.exe`. [Official main page](http://wiki.darwinbots.com/w/Main_Page), [downloads](http://wiki.darwinbots.com/w/Main_FTP), [public source project](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/Iersera.vbp).

This specification treats that source snapshot as behavioral authority. Older documentation is used where it explains intent and does not conflict with 2.48.32. Historical pages are sometimes aspirational or tied to 2.3x behavior; conflicts are called out rather than silently blended.

## 2. Product boundary and component model

The main application is a Visual Basic 6 MDI executable. Major runtime modules are:

- Shell, viewport, settings, dialogs, graphs, and inspectors.
- Simulation scheduler and cycle orchestration.
- Robot storage, updating, reproduction, death, and genealogy.
- DNA parser, detokenizer, interpreter, operators, and system variables.
- Mutation engine.
- Physics, spatial buckets, collisions, and vector math.
- Senses and vision.
- Shots, viruses, sperm, poison, venom, and waste particles.
- Ties, multibots, sharing, feeding, communication, and structural forces.
- Chloroplast feeding, light, day/night, tides, and repopulation.
- Obstacles, mazes, and teleporters.
- Leagues, contests, survival, Zerobot, and ecosystem-evolution modes.
- Persistence, snapshots, graph data, internet exchange, and safemode recovery.

The public tree also contains helper programs, notably an internet-mode helper, graph joining, a launcher, and recovery/restart tools. They extend the application but do not replace the VB6 simulation authority. [Source tree](https://github.com/darwinbots/Darwinbots2/tree/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c).

## 3. Domain objects and limits

### 3.1 Simulation

A simulation owns field dimensions and topology, species definitions, robot and shot arrays, obstacle and teleporter collections, physics constants, energy/light policy, mutation policy, cost policy, restart/league state, elapsed cycles, display state, and recording policy. [`SimOptions.bas`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/SimOptions.bas) is the central settings schema.

The robot array can address up to 32,000 entries. Runtime loops use the highest occupied index and reuse free slots. The shot array begins small, grows by roughly ten percent when full, compacts below 70-percent occupancy, and shrinks toward 100 slots when sparse. This design makes array indices operational handles, not durable identities.

### 3.2 Species

A species definition binds a DNA file and display name to startup and inherited attributes:

- Initial count, energy, placement, color, and skin.
- Repopulating/autotroph classification.
- Fixed state.
- Permission to see, execute DNA, move voluntarily, reproduce, mutate, use chloroplasts, or receive viruses.
- League/evolution disqualification and fatal-restriction policy.
- Population counters and optional comments/metadata.

Species name is a practical identity boundary used for conspecific immunity to poison and venom and for many bot strategies. It is not a genomic species test. Automatic forking can assign a new prefixed species name when accumulated mutation distance crosses the configured proportion of DNA length, subject to a 49-species working limit in that code path.

### 3.3 Robot

A robot record in [`Robots.bas`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/Robots.bas) contains:

- Existence, corpse, dead, autotroph, fixed, and restriction flags.
- Species file name, owner, tag, color, and 13-part skin.
- Array index plus simulation, absolute robot, and parent identifiers.
- Position, prior position, velocity, actual velocity, independent/resistive impulses, aim vector, and angular momentum.
- Energy, body, mass, radius, chloroplasts, shell, slime, venom, poison, waste, and permanent waste.
- Age, generation, offspring, descendants, kills, birth cycle, and multibot state.
- 1,000 memory cells and a 1,000-element private-variable area.
- Tokenized DNA, DNA length, gene count, used variables, mutation parameters, mutation totals, detailed log, and subspecies value.
- Up to ten reciprocal ties.
- Vision/reference occurrence arrays, focused opponent, touch/taste state, and eye configuration.
- Poisoning/paralysis locations, values, and countdowns.
- A stored virus shot and build timer.
- Stored sperm DNA and fertilization timer.
- Fifteen staged epigenetic values and twenty hereditary-memory addresses.
- Debug text, activation array, console reference, RGB-monitor state, and miscellaneous mode data.

Robot body is both reserve material and physical scale. Normal radius is derived from body unless fixed-radius compatibility is enabled. Mass is:

```text
body / 1000 + shell / 200 + (chloroplasts / 32000) × 31680
```

clamped to 1–32,000. A large chloroplast load is deliberately very heavy. Fluid “added mass” adds half the displaced-fluid mass of a sphere to acceleration calculations.

Fixed-radius mode uses radius 60. Otherwise, with body floored at one, base radius is:

```text
(ln(body) × body × 905 × 3 / (4π))^(1/3)
```

Chloroplasts then interpolate that result toward radius 415 by `chloroplasts / 32000`; final radius is at least one. The logarithm is an intentional visual/physical size exaggeration over pure volume scaling.

### 3.4 Shot

A shot owns existence/flash/stored flags, position and previous position, velocity, parent index, age, energy, range, value, color, type, source species/autotroph status, poison/venom memory payload, and optional virus or sperm DNA. Shot types are described in section 10.

### 3.5 Tie

Each end of a tie records the other robot, reciprocal tie index, port, current and requested angle/length, countdown, information marker, sharing/energy visual markers, and damped-spring state. A robot has at most ten ties. Reciprocal records make most tie operations symmetrical while preserving creator/back-tie distinctions.

### 3.6 Obstacle and teleporter

An obstacle is an axis-aligned rectangular shape with position, dimensions, color, opacity, drift, vision pass/block rules, and shot absorption behavior. A teleporter is a positioned circular gateway with local/inbound/outbound/internet mode, organism filters, capacity/poll settings, backflow rules, paths, and statistics.

## 4. Canonical simulation cycle

The full cycle is the most important behavioral contract. UI “Single cycle” executes this whole transaction. [`Master.bas`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/Master.bas) and `UpdateBots()` define the order.

### 4.1 Outer cycle

1. Honor pause/F12 and execution mode.
2. Increment cycle counters.
3. Run survival, Zerobot, ecosystem evolution, restart, and mutation-oscillation stage logic where enabled.
4. Update total-energy samples, dynamic costs, population handicap, and mode bookkeeping.
5. Execute the complete DNA of every existing, non-corpse, DNA-enabled robot in ascending array order.
6. Give the Robot Details debugger its optional post-DNA/pre-action memory view.
7. Clear transient touch, taste, edge, and tie-reference senses.
8. Apply PlayerBot key-state overwrites to the selected robot.
9. Update every shot: detect moving-segment collisions, resolve payloads, write taste, move, age, expire, and compact.
10. Record old robot positions.
11. Run the complete robot-update pipeline below.
12. Derive actual velocities from new and old positions.
13. Move obstacles and update teleporters.
14. Recompute total chloroplast units; if below the configured repopulation threshold and cooldown permits, insert a batch of repopulating species.
15. Apply sunlight/chloroplast feeding to robots.
16. Update RGB monitor colors, low-memory safety behavior, safemode checkpoints, autosave, and restart handoff as required.

### 4.2 Robot-update pipeline

Before the main per-robot phase:

1. Test every robot against teleporters.
2. Compute fluid added mass if density is nonzero.
3. Advance tide state. Tides alter buoyancy scaling, vertical gravity, Brownian motion, and later photosynthetic yield.

First per-robot pass:

1. Charge upkeep to living robots.
2. Enforce ongoing poison and venom memory overwrites.
3. Apply `.fixpos` unless globally prohibited.
4. Recalculate mass.
5. Resolve obstacle and border collisions.
6. Apply tie spring force, hardening, breaking, and torque.
7. Sum gravity, mutual “Planet Eaters” gravity, friction, fluid drag, Brownian impulse, and voluntary movement.
8. Resolve robot collisions through spatial buckets.
9. Apply static-friction impulse cancellation.
10. Transfer ordinary data through addressed ties and populate tied-reference values.

Between passes, species population counters reset and all existing robots are counted. Corpses decay here or are removed when body is exhausted.

Second per-robot pass:

1. Execute tie exchanges, sharing, deletion, stiffness, length, and angle commands.
2. Transfer one staged hereditary memory value through a young robot's birth tie.
3. Update aim and angular state.
4. Build/fire viruses and delete requested genes.
5. Integrate position and reset force/movement state.
6. Clamp energy and material stores.

The Big-Bertha restriction then kills oversized robots that fail the configured chloroplast/kills exception.

Final per-robot pass:

1. Refresh tie angle and length readbacks after movement.
2. Apply non-reproductive point/rate mutations.
3. Construct slime, shell, venom, and poison.
4. Metabolize or suffer waste.
5. Fire ordinary shots.
6. Add/remove and decay chloroplasts.
7. Store energy into body or feed body back to energy.
8. Change buoyancy.
9. Queue asexual or sexual reproduction.
10. Apply shock logic.
11. Write vision, reference, position, pain/pleasure, body-change, and other senses.
12. Fire requested ties.
13. Increment age and timer.
14. Convert dying robots to corpses or queue removal.
15. Add energy and body-equivalent energy to total-simulation accounting.

After all robots complete the phase, queued births and deaths execute. Extinct non-native species entries are removed. If all heterotrophs are gone and ordinary restart is enabled outside league mode, another round is requested.

### 4.3 Consequences of order

- DNA generally reads the previous cycle's completed senses.
- Shot impacts occur after DNA, so their taste/poison/venom effects can influence later work in the same cycle and DNA next cycle.
- Tie data transfer happens after DNA but before the recipient fires shots or reproduces. A tie attacker can overwrite the victim's action register after the victim chose it.
- All robots finish a subsystem pass before the next subsystem starts, but array order still matters within a pass.
- Birth and death are deferred, preventing array mutation during most loops.
- Rendering and execution pacing are observational. Video-off must not change the logical cycle.

## 5. DNA programming system

Each living eligible robot executes its entire genome every cycle. The genome is not a neural controller; it is a mutable program with integer and Boolean stacks, immediate memory writes, conditional genes, and action registers.

Key rules:

- Memory addresses 1–1000 persist, except where the engine writes sensors or clears command registers.
- `.name` is an address literal; `*.name` reads it.
- Values and addresses outside normal bounds are normalized rather than rejected, preserving mutated-code viability.
- `cond/start/else/stop` controls gene bodies. An empty condition is true.
- Each token category and store/action may cost energy.
- Store operations are immediate. Physical effects are mostly consumed later in cycle order.
- The interpreter records gene activations only when required by the focus/console UI.
- DNA and memory are inherited; mutation may change either token type or value, insert/delete/reverse/move/copy sequences, or change mutation rates.

The complete parser, operator, flow, numeric, and failure contract is [DNA runtime specification](reference/DNA_RUNTIME_SPEC.md). All 255 compiled names and aliases across 247 distinct addresses are in [the canonical sysvar reference](reference/SYSVARS_REFERENCE.md). The official [Robot DNA portal](http://wiki.darwinbots.com/w/Robot_DNA) and [stack explanation](http://wiki.darwinbots.com/w/Stack) remain useful introductions, but their old 20-entry stack note is superseded by the 100-entry 2.48.32 source.

## 6. Perception and memory

[`Senses.bas`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/Senses.bas) turns world events and focused targets into memory.

### 6.1 Vision

Each robot has nine eyes. Default eyes divide a roughly 90-degree forward field, and DNA may customize direction and width. Vision range is approximately 12 standard robot diameters (`12 × RobSize`, 1,440 world units in the old constant system).

The engine uses spatial buckets and sorted robot order to find candidates. For each target:

1. Compute relative position and absolute target angle.
2. Normalize angle across the 0/2π seam.
3. Reject targets outside a view half-angle widened by apparent robot radius.
4. Approximate distance.
5. Project apparent size across eye cells.
6. Write a value proportional to `RobSize / distance`, preserving the stronger/nearer projection already present.

When center eye 5 owns a target, `lastopp` identifies it for reference-variable copying. The observer receives target behavior occurrence counts, energy, age, outgoing communication cells, aim, tie presence, shell, body, position, relative velocities, poison, venom, kills, multibot/fixed flags, and an optionally addressed target memory value.

Shapes can be sensed according to their visibility flags and can occlude what lies beyond them. “Disable Vision” prevents this work. The UI's vision-grid toggle only changes drawing.

### 6.2 Touch and edge sense

Robot collision writes a general hit flag plus one of forward, backward, right, or left hit cells. Direction is relative to the robot's aim and divided into four quadrants. Border collision sets an edge flag when that axis is not wrapped.

### 6.3 Shot taste

A shot collision writes its type into one directional taste cell, plus an angle-only readback and a flavor-only readback. The direction uses the incoming shot's prior position relative to the robot's aim.

### 6.4 Internal state senses

At late-cycle sense writing, the engine exposes:

- Current energy and body.
- Energy loss (`pain`) and gain (`pleas`) since the prior sense sample.
- Body loss and gain.
- Age and epigenetic timer.
- Absolute and relative velocity components.
- Aim, mass, maximum velocity, fixed state, position, and day/night state.
- Current material stores and counters where their subsystems update memory.
- Tie presence, number, angle, length, and tied-target references.

Position values are written only when they fit signed memory. The sensor system clamps several velocities and energy values to avoid VB overflow. Historical wiki code describes the older algorithm and known bugs; the current source above is authoritative. [Historical Senses page](http://wiki.darwinbots.com/w/Senses).

## 7. Physics and movement

Darwinbots uses a one-cycle explicit/Euler-style integration with impulses. It aims for plausible and evolvable dynamics, not strict real-world units. [`Physics.bas`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/Physics.bas) is the implementation.

### 7.1 Voluntary motion

`.up`, `.dn`, `.sx`, and `.dx` form a bot-relative vector. Up minus down is the forward component; left minus right is the lateral component. The vector rotates through the aim vector, is clipped to maximum velocity magnitude, scaled by movement efficiency, and added to independent impulse.

Legacy movement multiplies requested force by mass; NewMove mode does not. Energy cost equals request-vector magnitude × movement cost × global cost multiplier and cannot exceed current energy in the normal positive-cost case. Movement restrictions, corpses, and disabled DNA suppress voluntary force.

Aim changes use `.aimdx`/`.aimsx` or absolute `.setaim`. Rotation is applied before movement. Angular momentum also receives friction, fluid damping, Brownian noise, and tie torque.

### 7.2 Fluid

Fluid density enables displaced added mass and spherical drag. Reynolds number is:

```text
2 × radius × speed × density / viscosity
```

The drag coefficient is a piecewise approximation spanning Stokes-like low Reynolds behavior, the drag crisis, and high-Reynolds regimes. Drag impulse is:

```text
0.5 × Cd × density × speed² × π × radius²
```

and is capped below current speed to prevent reversal. Fluid density also damps angular momentum. The code contains an unused cylinder-drag implementation and disabled tie-drag experiments; they are not active 2.48.32 behavior.

### 7.3 Solid friction

Z-gravity presses robots against an implicit plane. Static impulse capacity is `mass × Zgravity × coefficientStatic`. Kinetic friction magnitude is `mass × Zgravity × coefficientKinetic`, capped at current speed and applied opposite velocity. The same impulse dampens angular momentum.

### 7.4 Brownian, gravity, and buoyancy

Brownian motion adds a random-direction impulse up to half the configured strength and a small random angular change.

Y-gravity normally adds downward impulse proportional to mass. In pond mode with a nonwrapped vertical axis, buoyancy can reverse gravity above or below a target depth derived from robot buoyancy and tide scaling. Maintaining buoyancy costs energy through movement-cost logic. `.setboy` incrementally changes a 0–1 buoyancy value and exposes a 0–32,000 readback.

Planet Eaters adds pairwise inverse-square attraction. Each robot's mass contribution is capped at 192 before multiplying by the configured `G`, limiting extreme bodies.

### 7.5 Boundaries and topology

Each axis is independently bounded or wrapped. A wrapped robot respawns on the opposite side with smudge distance. A bounded collision clamps the center inside its radius and adds velocity-proportional resistance; it also sets the edge sense.

### 7.6 Robot collisions

Spatial buckets avoid an all-pairs search. Overlapping circles are separated along their center normal. Separation is weighted by opposite mass unless both are fixed/stationary. Velocity response uses mass, relative velocity, fixed-body substitution, and the configured elasticity coefficient. The UI conceptualizes elasticity from ghost-like overlap toward marble-like bounce.

### 7.7 Ties as mechanics

New ties are damped springs with `b=0.02` and `k=0.01`. They tolerate 20 units of compression/stretch before force. Force follows `-kx-bv` along the tie. A tie breaks if surface-to-surface distance exceeds 1,000. After its hardening countdown, it becomes a high-stiffness anti-rope/structural link (`b=0.1`, `k=0.05`), records natural length and creator-relative angle, and marks the robot as a multibot. Angular error beyond five degrees creates equal/opposite transverse impulses and limited rotation.

## 8. Energy, body, chloroplasts, waste, and death

### 8.1 Energy and body

Energy pays DNA, action, upkeep, aging, construction, reproduction, virus, shot, and movement costs. Body stores denser reserve and contributes mass, radius, feeding strength, and shot range.

`.strbody` converts at most 100 energy per cycle into body at 10 energy to 1 body. `.fdbody` converts at most 100 energy-equivalent back, subtracting one body per ten energy. Both registers clear after use. Energy over 32,000 is generally clamped; specific intake paths convert some overflow into body.

Upkeep includes body × Body Upkeep and `(DNA length - 1) × DNA Upkeep`, each multiplied by global cost. Aging adds configurable base, linear, and logarithmic terms after a starting age. All command-class costs are separately configurable.

### 8.2 Chloroplasts and sunlight

Any allowed robot can carry chloroplast units, not only startup vegetables. The legacy “veggy” flag still controls repopulation and contest semantics.

Available light starts from occupied circular robot area divided by unobstructed field area. Obstacles subtract from field area. Density pressure is `(1-lightOccupancy)² × 4`. For a lit robot:

```text
chloroplast fraction = chloroplasts / 16000
gross factor = density pressure × chloroplast fraction × 1.25
maintenance factor = (chloroplasts / 32000)²
net feed = (gross factor - maintenance factor) × local light / 3.5
```

Pond mode attenuates local light by `lightIntensity / depth^gradient`. The sun may cover a moving/ranging horizontal band. Age subtracts `age × chloroplasts / 1,000,000,000`; tides can reduce yield. Configured allocation splits feed between energy and body at the 10:1 conversion.

Chloroplasts naturally decay by:

```text
0.5 / 100^(chloroplasts/16000)
```

per cycle. Adding chloroplasts costs the chloroplast construction cost and may be refused if it would leave energy below 100 or violate autotroph population pressure. Removing them has no construction refund. Chloroplast-bearing robots metabolize waste into small energy/body increments. [`Vegs.bas`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/Vegs.bas), [wiki Chloroplasts overview](http://wiki.darwinbots.com/w/Chloroplasts).

### 8.3 Light timing and population control

Day/night alternates after the configured cycle length. Total-energy thresholds may temporarily suspend the cycle, permanently toggle light between thresholds, or advance phase toward dawn/dusk. Random weather varies the sun band's position and range. Tides have an independent period and phase.

When total chloroplast units fall below maximum/repopulation policy, the cooldown counter permits configured batches of repopulating species. Near the chloroplast cap, only a random fraction of autotroph reproduction attempts succeed to avoid array-order advantage.

### 8.4 Waste and Alzheimer's

Waste is produced by feeding, material transfers, and disposal inefficiency. Some actions convert a fraction into permanent waste. Above the configured combined waste threshold, Alzheimer's writes random values to random memory cells. That can corrupt behavior, including fixing the robot.

Waste can be fired as type -4, moved through ties, shared across hardened ties, or metabolized by chloroplasts. If ordinary waste exceeds 32,000, the robot automatically defecates a particle and converts a small fraction to permanent waste. Permanent waste clamps at 32,000 and cannot be normally cleared. [Waste behavior](http://wiki.darwinbots.com/w/Waste).

### 8.5 Slime, shell, poison, and venom stores

- Slime blocks new ties and viruses, then loses 20 units on a tie attempt. It also decays two percent per cycle.
- Shell adds mass, absorbs body-shot power, and reduces venom power. It does not protect against the ordinary -1 feeding mechanism in the same way.
- Poison is a defensive store. A memory shot or -1 attack can cause the victim to return a -5 poison particle. Tie feeding can poison the attacker when the target has enough poison.
- Venom is an offensive store fired as -3 or injected through ties. It repeatedly forces one chosen memory cell to a chosen value.

Poison also decays two percent per cycle. Construction amounts and costs are capped by their respective routines. The wiki provides the intended distinction: [Shell](http://wiki.darwinbots.com/w/Shell), [Slime](http://wiki.darwinbots.com/w/Slime), [Poison](http://wiki.darwinbots.com/w/Poison), [Venom](http://wiki.darwinbots.com/w/Venom).

### 8.6 Death and corpses

With corpses enabled, a living robot below 15 energy after age zero becomes a white, non-autotroph, nonfixed, blind, DNA/movement-disabled, virus-immune corpse. Its chloroplasts and buoyancy clear, but body remains edible. It is deleted when body drops below 0.5.

Corpse decay is none, waste-shot release, or energy-shot release with configured packet size and period. Without corpses, energy or body below 0.5 queues deletion. The old wiki correctly describes corpses as remaining body stores and the three decay modes. [Corpses](http://wiki.darwinbots.com/w/Corpses).

## 9. Reproduction, heredity, and genealogy

### 9.1 Asexual reproduction

A positive `.repro` or `.mrepro` queues birth. The stored value modulo 100 is the percentage of the parent's energy, body, waste, permanent waste, and chloroplasts assigned to the child. Zero or blocked values do nothing. The parent must have sufficient body and a collision-free position one combined child/parent radius forward.

The child:

- Copies DNA, mutation parameters/history, skin, species identity, restrictions, color, velocity, aim opposite the parent, tag, buoyancy, and relevant mode fields.
- Receives the selected resource share.
- Costs the parent an additional 0.1 percent of transferred energy; the child starts with 99.9 percent of its nominal share.
- Gets a new absolute identity, parent identity, incremented generation, and birth cycle.
- Starts with cleared ordinary memory and ties.
- Immediately receives parent memory 971–975.
- Stages parent memory 976–990 into epigenetic storage. One value transfers per early cycle through the birth tie, if gestation/tie survival allows.
- Receives a temporary birth tie and newborn immunity from shots sharing the parent's parent identity through age one.

`.mrepro` forces extra mutation behavior. Under the older path, the child's rates are temporarily increased; under Delta2, repeated rate changes and reproductive mutations use the advanced algorithm. Global policy can disable asexual reproduction for non-repopulating robots. A “greedy reproduction” restriction can kill attempts assigning under three percent.

### 9.2 Sexual reproduction

A -8 sperm shot copies the male's full DNA. On impact, the recipient remains fertilized for ten cycles. A positive `.sexrepro` during that window queues sexual birth. The mother supplies all resources and most inherited non-DNA state.

Before crossover, both token arrays map to integer “nucleotide” values and are sequence-matched. Genetic distance above 0.6 rejects the cross and blocks sexual reproduction for several cycles. Compatible genomes cross over along matched sequences into one haploid child genome. The system does not model diploid chromosomes, dominance, or meiosis. The wiki explicitly describes this as a simplified crossover algorithm. [Sexual Reproduction](http://wiki.darwinbots.com/w/Sexual_Reproduction).

### 9.3 Identity, generations, and snapshots

Generation saturates at 32,000 for memory/UI compatibility. Parent and absolute IDs support local lineage traversal, descendant counts, and highlights. Saved robot headers preserve generation and cumulative mutation count through the legacy hash. Snapshot and internet transfers can carry organism state, but genealogy is not a permanent global pedigree.

## 10. Shots, feeding, information, defenses, and viruses

[`Shots.bas`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/Shots.bas) implements moving particles and continuous segment/circle collision. A new ordinary shot starts at the firing surface, inherits actual robot velocity plus 40 units along its firing direction, and receives random angular error. `.backshot` reverses direction once; `.aimshoot` applies an offset of 1/200 radian per unit and then clears.

Base range grows logarithmically with virtual body and is quantized into 40-energy/range steps. Large negative `.shootval` extends -1/-6 range through a log2-like transform; positive `.shootval` boosts power similarly. Shot energy decays very little until late in flight through an arctangent curve with constant 40, unless energy- or waste-shot no-decay options apply.

| Type | Name | Impact behavior |
|---:|---|---|
| positive | Information shot | Normalize address to 1–1000 and write `.shootval`; sufficient poison returns -5 instead |
| -1 | Energy-feeding request | Makes target return energy or defensive poison; base power is `20 + body/5`, multiplied by tie count for multibots |
| -2 | Energy packet | Transfers 95% to energy, 0.4% to body, and 1% to waste; some energy overflow becomes body |
| -3 | Venom | Conspecific absorbs it; other species suffer timed forced-memory value after shell absorption |
| -4 | Waste | Adds particle power to target waste |
| -5 | Poison | Conspecific absorbs it; other species suffer timed forced-memory value; normally only emitted defensively |
| -6 | Body-feeding request | Shell absorbs first; highly effective against corpses; base power is `10 + body/2`, multiplied by tie count for multibots |
| -7 | Virus | Carries one copied gene and inserts it at a random gene boundary unless corpse, immune, or stopped by slime |
| -8 | Sperm | Copies full genome into recipient's ten-cycle fertilization store |
| -100 | Poff | Ornamental particle with no collision payload |

For ordinary -1 feeding in proportional mode, requested power is incoming value × remaining shot energy divided by range scale × the configured proportional factor. Fixed mode uses the configured fixed amount. The returned packet has twice the incoming range and half-like relative velocity. Extraction removes about 90% of power from target energy plus 1% from body. The returned -2 packet later gives its receiver 95% energy, 0.4% body, and 1% waste. Corpses halve -1 extraction; -6 multiplies post-shell power by four against corpses.

Venom power is multiplied by 25 for shell comparison, then divided back after shell is consumed. Virus protection uses slime effectiveness 1/20. Poison and venom never directly target `.delgene` address 340; an invalid/zero target becomes a random safe address. Ongoing effects rewrite the chosen memory every robot update until their counters expire.

Virus construction copies the requested gene into a stored cyan shot. Build timer is twice gene length; DNA-copy costs apply. A virus can fire only when timer reaches one. `.vshoot` controls cost/range and sends it in a random direction. Infection inserts the gene, updates length/count, records a mutation/subspecies event, and can trigger league disqualification. Chloroplast-bearing robots cannot build viruses; attempting it removes their chloroplasts in the shipped path. [Virus guide](http://wiki.darwinbots.com/w/Virus), [Shooting overview](http://wiki.darwinbots.com/w/Shooting).

## 11. Ties and multibots

[`Ties.bas`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/Ties.bas) makes ties simultaneously mechanical connections, addressed channels, feeding tools, and multicellular structure.

### 11.1 Formation and addressing

`.tie` fires a tie attempt toward a target within range. The command value becomes the creator's port. Slime probabilistically blocks formation and loses material. The creator and target receive reciprocal tie records with different local ports. `.tiepres` reports the latest relevant port; `.tienum` selects a port for addressed work and resets each cycle.

A tie can be temporary or count down toward hardening. During its first 20 cycles it is a spring. Once hardened it marks the component as multibot, fixes structural angle/length, and enables sharing/advanced control. Ties break at excessive length, explicit deletion, death/removal, or object/teleport operations that cannot preserve them.

### 11.2 Information

Positive `.tieloc` plus `.tieval` writes immediately into the selected tied robot's memory after DNA and before later actions. The tie flashes white. `.readtie` and tied-reference variables expose the selected partner's state and optionally one addressed memory cell.

### 11.3 Resource exchange

Negative `.tieloc` selects a resource protocol:

- -1 energy: give up to 1,000 or take up to 3,000 requested units. Receiver keeps 70% as energy, 2.9% as body, and 1% as waste.
- -3 venom: inject or steal up to 100.
- -4 waste: give or take up to 1,000; one percent becomes permanent waste on the losing/transfer path.
- -6 body: give up to 100 or take up to 300; receiver gets roughly 98.7% body, 3% energy, and 1% waste under the implemented coefficients.

Negative taking is capped by the target's store. A newborn cannot transfer. Poison can cancel hostile energy/body feeding when the target has more than one quarter the requested amount; the attacker becomes poisoned using the target's configured payload. League restrictions can disqualify cross-species giving, taking, information shots, viruses, or sexual reproduction.

### 11.4 Sharing

Hardened multibot ties support `.sharenrg`, `.sharewaste`, `.shareshell`, `.shareslime`, and `.sharechlr`. A value from 1 to 99 requests the executing robot's percentage of the pair's combined store. Thus “share” can be cooperative balancing or theft. Chloroplast sharing has delay/permission rules. Sharing ties flash yellow; resource transfer flashes red. [MB Sharing](http://wiki.darwinbots.com/w/MB_Sharing).

### 11.5 Structural control

`.fixang`, `.fixlen`, `.stifftie`, plus the first four `.tieangN`/`.tielenN` addresses set or read hardened geometry. Stiffness maps the command into damping and spring constants. The selected angle is relative to robot aim. The engine updates geometric readbacks only after movement so they represent the new pose.

### 11.6 Multibot semantics

Program semantics call any robot with a hardened tie a multibot member. Multibots gain sharing, structural control, tie-count amplification for feeding shots, combined virtual body for range/power, and organism-level selection/save/kill/teleport behavior. Community league standards impose stronger expectations such as identity, differentiated structure, cooperation, and reproducibility. [Multibot history and conventions](http://wiki.darwinbots.com/w/Multibot).

## 12. Mutation and evolution

[`NeoMutations.bas`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/NeoMutations.bas) separates mutations that can occur during life from reproduction-time copying/structural mutations.

| Mutation | Time | Effect |
|---|---|---|
| Point | During life | Schedules one location from a geometric waiting time; changes token value or class over Gaussian run length |
| Point2 | During life, Sunline | Biases changes toward useful input/output sysvars and functional store patterns |
| Delta | During life | Mutates one mutation rate itself |
| Copy Error | Reproduction | Mutates copied token runs while scanning DNA |
| Copy Error2 | Reproduction, Sunline | Functional/informational sysvar-biased copy changes |
| Insertion | Reproduction | Inserts a positive Gaussian-length random token run |
| Reversal | Reproduction | Reverses an odd-length sequence around a chosen center |
| Minor Deletion | Reproduction | Deletes a usually short Gaussian-length run |
| Major Deletion | Reproduction | Deletes a usually longer Gaussian-length run |
| Amplification | Reproduction, Sunline | Copies a long centered sequence to another location |
| Translocation | Reproduction, Sunline | Removes a long sequence and reinserts it elsewhere |

Each robot inherits its own “one in N” rates, Gaussian means/deviations, and value-versus-type probabilities. The global current multiplier divides effective N: a larger multiplier increases mutation frequency. The simulation can oscillate that multiplier by sine or sawtooth.

Default unnormalized rates are 1 in 5,000 for initialized mutation types. Default means/deviations are: point 3±1; Delta 500±150; copy error/insertion/minor deletion 1±0; major deletion/reversal 3±1; amplification/translocation 250±75. Default point/copy value-change preference is 80 percent. Normalization instead scales rates by DNA length and configured constants.

Runtime floors prevent expensive structural algorithms from “freezing” on very long DNA. DNA cannot exceed about 32,000 blocks. Mutation never intentionally crosses the master `end`. Every successful change increments counters, appends detail, changes visible color stochastically, recalculates gene/DNA length, and assigns a new subspecies fingerprint.

Delta2 can evolve main rates, distribution parameters, and type/value preference at configured intervals. Advanced epigenetic controls can reset or amplify inherited memory after mutation overload. Automatic forking turns a sufficiently diverged robot into a new named species. The wiki gives useful selection guidance but not the full modern algorithm. [Mutations overview](http://wiki.darwinbots.com/w/Mutations).

## 13. Environment objects and spatial systems

### 13.1 Obstacles and mazes

Obstacles collide with robots and optionally absorb shots. Bots may see the obstacle surface or see through it according to separate flags. Shapes can drift independently along X and Y, pushing/corralling robots. Preset maze commands construct horizontal, vertical, spiral, checkerboard, polar-ice, or trash-compactor arrangements from the same primitives. [`Obstacles.bas`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/Obstacles.bas).

### 13.2 Teleporters

Intrasim teleporters move organisms within one world. Outbound teleporters serialize organisms to a transfer path; inbound teleporters poll and insert them. Internet teleporters bind those paths to the internet helper. Filters decide whether heterotrophs, autotrophs, and corpses pass. A multibot transfers as one organism up to the implementation's bounded connected-component size. Backflow protection prevents immediate recirculation. Shapes may block a teleporter if configured. [`Teleport.bas`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/Teleport.bas).

### 13.3 Background and display-only state

An imported background picture has no physical effect. Vision grids, vectors, gauges, skins, impacts, RGB monitoring, flicker, no-graphics, and video-off states are observational only. Disabling vision in a species is simulation behavior; hiding the vision grid is not.

## 14. Internet mode

Internet mode creates an internet teleporter and exchanges serialized organisms through configured inbound/outbound folders and a helper/server protocol. The UI reports sent/received organisms and remote/source identity. Population graphs can incorporate internet species/simulation data.

The wiki's original FTP-era description is conceptually accurate: outbound collisions remove and queue an organism; periodic updates upload batches and download immigrants; inbound organisms are placed into the simulation. Later releases replaced or repaired helper protocols. The download history says 2.47 introduced a working internet mode, 2.48.09 integrated it, and 2.48.22 restored full function. Current public infrastructure may no longer provide a live megasim, so network availability is external, not a local product guarantee. [Internet Mode](http://wiki.darwinbots.com/w/Internet_Mode), [release archive](http://wiki.darwinbots.com/w/Main_FTP).

## 15. Leagues, contests, and directed evolution

### 15.1 Leagues

Named formats include F1, F2, F3, Multibot, Short Bot, Single Gene/Short Gene, and Single Store. Presets configure costs, physics, startup resources, permitted behavior, population/cycle ceilings, and recognition tolerance. A tournament or stepladder loads competitors from a source directory, runs rounds, samples populations, and determines statistical wins. Contest UI supports manual and instant adjudication. [`F1Mode.bas`](https://github.com/darwinbots/Darwinbots2/blob/467a8a0c1564db2071cf65efe39dcfefc5ab7a0c/Darwinbots2/F1Mode.bas), [League portal](http://wiki.darwinbots.com/w/League).

League restrictions can disqualify information shots, viruses, sexual reproduction, and cross-species tie transfers. Species fatal restrictions can kill or disqualify robots that violate movement, chloroplast, reproduction, or multibot-formation rules.

### 15.2 Restart and ordinary evolution

Ordinary restart can reload a simulation/round when all heterotrophs die. Automatic graphs and snapshots provide observational continuity. Mutation settings, costs, light, repopulation, and restrictions define the selection environment.

### 15.3 Simple Survival

Simple Survival runs staged selection from a chosen source bot. It measures survival/fitness over a configured cycle length, carries a selected lineage forward, can reduce population oscillation, normalize DNA size, open graphs, and start internet mode. The Find Best weighting trades population success against invested energy.

### 15.4 Zerobot

Zerobot begins from random/minimal DNA of a configured length and searches for self-sustaining/effective behavior through repeated mutation and selection stages. Specialized logic prevents cancerous families from winning, backs up from unstable stages, adjusts point rates, and can restart after prolonged failure.

### 15.5 Eco Survival

Eco Survival extends directed evolution to ecosystem criteria. It uses the same stage/restart framework with ecological population and energy behavior. These modes are orchestration layers over the same authoritative robot cycle, not separate simulation engines.

## 16. Persistence, recording, diagnostics, and recovery

### 16.1 Files and directories

The product uses:

- Plain-text `.txt` robot DNA files with optional metadata headers.
- Simulation save files containing world and organism state.
- Settings files containing simulation options and species startup definitions.
- Organism files for whole tied structures and teleport exchange.
- Snapshot files for living/dead DNA records.
- CSV-like graph/database exports for external analysis.
- Key mapping, restriction preset, RGB monitor preset, league, and internet helper files.

The installation expects a main directory with Robots and transfer-related subdirectories. Missing robot paths fall back to the common Robots directory or invoke a file picker. Windows line endings and US numeric formatting are compatibility assumptions documented by the official download page.

### 16.2 Save behaviors

Save Simulation captures the running world. Save Sim Without Mutations removes or disables mutation state for the saved copy. Save Robot's DNA detokenizes the focused genome and writes legacy header information. Save Entire Organism traverses connected ties. Settings save startup/world policy without necessarily capturing live organisms.

Backward compatibility was a continuing product goal. Load routines contain version-aware and fallback behavior, but no schema guarantee covers arbitrary corruption or every retired feature.

### 16.3 Snapshots and graphs

Snapshot of the living records every live genome. Dead snapshots can run continuously and exclude repopulating species. Graphs sample at the configured interval and may save data for later Graph Join processing. The graph set covers population, age, energy, descendants, mutations, DNA length/conditions, total energy, dynamic costs, diversity, chloroplasts, genetic/generational distances, and three custom graphs. [Graphs reference](http://wiki.darwinbots.com/w/Graphs).

### 16.4 Safemode

Safemode checkpoints a last-start settings file and simulation autosave. After a crash, restart logic narrows the fault to roughly 2,000-cycle intervals and can enter diagnostic recovery. The wiki warns not to start/load over an already running simulation during the first checkpoint window and notes limits with multiround contests. The 2.48.29 release repaired the external backup helper. [Safemode procedure](http://wiki.darwinbots.com/w/Safemode).

“Ignore errors” is separate: it lets selected runtime handlers continue after errors, which may preserve a long evolution run but can leave degraded state. Video/off, safe mode, and tray hiding improve long-run practicality but do not make the single-process VB6 engine transactional.

## 17. User interface and front end

The front end is part of the behavioral product, not a generic wrapper. It is a classic Windows VB6 MDI application with an arena, native menus, bitmap toolbar, multipart status bar, fixed scientific dialogs, live robot inspectors, custom graph windows, and bitmap-driven status overlays.

The complete authored UI behavior, exact menu/toolbar maps, layouts, visual design, arena encoding, all settings tabs, and every auxiliary interface are in [UI, layout, and interaction specification](reference/UI_AND_INTERACTION_SPEC.md). The generated [declarative UI inventory](reference/DECLARATIVE_UI_INVENTORY.md) lists every shipped control and its VB6 geometry/state.

Important front-end contracts include:

- One-step means one full canonical cycle.
- New Simulation uses a working options copy; Start new commits and resets; Change applies supported live values.
- Robot commands target current focus and organism commands traverse ties.
- Multiple inspectors and graphs remain open while simulation runs.
- Rendering toggles must not alter simulation results.
- PlayerBot overwrites selected memory at its precise post-sense-clear point.
- Hidden E-Grid, Survival Info, and legacy compatibility controls remain documented even when inaccessible by default.

## 18. Product settings taxonomy

For implementation or test planning, every setting belongs to one of these effect scopes:

| Scope | Examples | Application timing |
|---|---|---|
| Presentation | skins, vectors, gauges, impacts, background, graph interval | Immediate or next redraw/sample |
| Execution pacing | pause, fast, 15 CPS, flicker, video off | Immediate; no logical state change |
| World geometry | field width/height, wrap, obstacles, teleporters | Usually startup; objects mutable live |
| Physics | density, viscosity, friction, gravity, Brownian, elasticity, max velocity | Read each cycle; mutable live with discontinuities |
| Economy | command/action/upkeep/age costs, dynamic cost target | Read during each action/cycle |
| Ecology | light, pond gradient, day/night, tides, repopulation, corpse decay | Read on scheduled cycle phases |
| Genetics | per-robot mutation rates, global multiplier, normalization, Delta2 | Mutation time; inherited per robot |
| Species policy | startup count/resources, autotroph, restrictions | Startup and copied to descendants |
| Competition | presets, rounds, cycles, population cap, disqualification | Round/stage boundaries and prohibited actions |
| Evolution | survival source, stage length, Zerobot size, selection weighting | Controlled restart/stage boundaries |
| Network | user/server/port, paths, filters, polling/backflow | On internet/teleporter update |
| Recovery/recording | safemode, snapshots, autosave, dead exclusions | Scheduled checkpoints/events |

## 19. Nonfunctional and compatibility behavior

- Platform: 32-bit classic Windows/VB6 runtime and ActiveX controls. Wine operation was historically possible with runtime DLL configuration, but slower and less reliable.
- Determinism: a user-specified pseudorandom seed supports repeatability in principle. UI timing, external files/network, true-random builds, event reentrancy, and version differences can break exact replay.
- Performance: arrays, spatial buckets, early exits, displayed-counter copies, shot compaction, optional video suppression, and `DoEvents` yield support hundreds to thousands of robots on period hardware. Several loops remain O(N²) in worst cases, notably mutual gravity.
- Numerical model: mixed Integer, Long, Single, and Double values plus explicit clamps reflect VB6 overflow limits. Exact quirks are behaviorally significant.
- Responsiveness: long loops periodically call `DoEvents`. The application is single-process and not thread-safe by modern standards.
- Regional compatibility: numeric parsing assumes US separators.
- Extensibility: user DNA and data files are primary extension points. There is no stable plugin API.
- Security: robot files are untrusted code only within the DNA VM, but old helper/network/file paths and unsigned executables should be treated as legacy software risk. The metadata hash is not a security boundary.

## 20. Acceptance criteria for a faithful product

A complete compatible implementation must prove:

1. The exact outer and per-robot cycle order in section 4.
2. All robot fields, materials, counters, restrictions, and identity behavior.
3. The parser, 100-entry stacks, operators, flow, costs, address wrapping, and 1,000-cell memory.
4. All compiled sysvar names and aliases.
5. Nine-eye vision, reference values, touch, taste, internal senses, and their timing.
6. Voluntary motion, aim, drag, friction, Brownian, gravity/buoyancy, collisions, borders, and tie mechanics.
7. Every shot type, range/decay/collision rule, poison/venom defense, virus, and sperm path.
8. Tie formation, addressing, information injection, resource exchange, sharing, hardening, structural control, and multibot traversal.
9. Energy/body conversion, costs, light/chloroplast formula, day/night, tides, waste, construction stores, corpses, and death.
10. Asexual and sexual reproduction, inheritance, birth tie, epigenetic memory, and mutation paths.
11. Obstacles, all maze presets, local/file/internet teleporters, backgrounds, and display-only modes.
12. Contest, league, restart, survival, Zerobot, Eco Survival, automatic fork, restriction, and disqualification behavior.
13. Save/load/settings/organism/DNA/snapshot/graph/recovery file behavior.
14. Full UI parity against the two UI appendices, including hidden and disabled states.
15. Regression tests for numeric overflow, stale focus, array compaction, wrap boundaries, missing files, corrupted DNA, and long unattended simulations.

## 21. Known ambiguity and evidence policy

Some public documentation describes older versions, planned features, or obsolete helper protocols. This spec resolves ambiguity by this order:

1. Executed 2.48.32 source path.
2. Declarative 2.48.32 form/project data.
3. Official 2.48 release/download notes.
4. Current wiki concept pages where consistent.
5. Historical readmes, screenshots, forum discussion, and mirrors.

No local Darwinbots-related project or modernization work informed this specification. The evidence set is a clean-room collection of public web, public repository, official wiki, official release archive, and historical public site material.

For history, version chronology, source-quality notes, and a research bibliography, see [history and source provenance](reference/HISTORY_AND_SOURCES.md).
