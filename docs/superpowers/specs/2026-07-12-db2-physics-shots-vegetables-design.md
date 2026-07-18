# DB2 Physics, Shots, and Vegetables Design

Date: 2026-07-12
Status: Implemented; retained as historical design rationale

> Current behavior and verification are recorded in `modern/docs/verification.md`. Version numbers and proposed implementation details below reflect the planning baseline at the time this document was approved.

## Objective

Replace the simplified movement, instantaneous-shot, and flat vegetable-energy systems in Darwinbots Modern with close behavioral ports of the Darwinbots 2 implementations while retaining the modern Rust/Avalonia architecture.

The port must make Animal Minimalis visibly swim, make shots finite moving world objects, and make vegetable survival and repopulation depend on the DB2 chloroplast and light model. It does not require deterministic evolution, identical random sequences, or cycle-identical simulation outcomes.

## Behavioral Authority

The VB6 implementation is authoritative for formulas and phase ordering:

- `Darwinbots2/Physics.bas`: voluntary forces, mass, friction, fluid resistance, gravity, Brownian motion, velocity integration, and collision response.
- `Darwinbots2/Shots.bas`: shot creation, velocity, range, aging, decay, collision, impact, and shot-type effects.
- `Darwinbots2/Vegs.bas`: chloroplast feeding, available light, energy/body allocation, and repopulation cooldown.
- `Darwinbots2/Master.bas`: ordering among robot updates, physics, shots, vegetables, and population maintenance.
- `Darwinbots2/SimOptions.bas` and `Darwinbots2/MDIForm1.frm`: setting meanings and normal defaults.

`Darwin2.48.32.exe` is the black-box reference where source behavior is ambiguous. DarwinbotsC may be consulted for decomposition but cannot override VB6 behavior.

## Scope

### Included

- Impulse-based organism movement and retained momentum.
- DB2 mass effects from body, shell, and chloroplasts.
- Maximum acceleration/velocity safety limits.
- Surface friction, fluid density, viscosity, gravity, Brownian motion, and collision elasticity.
- Persistent shots with position, previous position, velocity, age, range, energy, owner, type, payload, and impact state.
- DB2 shot types already supported by the modern engine, migrated from instant targeting to projectile collision.
- DB2 shot aging, nonlinear decay, impact flashes, and finite removal.
- Chloroplast sysvars at their legacy memory addresses.
- Initial and repopulated vegetable chloroplast reserves.
- DB2 light availability, crowding correction, chloroplast energy production, and configurable energy/body distribution.
- Minimum-vegetable repopulation threshold, batch amount, and cooldown, while retaining the modern hard vegetable cap as a catastrophic-growth guard.
- Normal metabolism for starter simulations; Zerobot sustenance overrides only for Zerobot modes.
- Save/load support for the new physics and projectile state through a save-schema version increase.
- CPU reference behavior and GPU-compatible data layouts.

### Excluded

- Deterministic replay across runs or backends.
- Exact VB6 floating-point rounding at every intermediate operation.
- Online play and legacy save/settings import.
- A general visual redesign.
- Immediate GPU execution of the full DB2 collision and projectile model. The CPU implementation defines behavior first.

## Tick Ordering

Each tick will use explicit current-state inputs and ordered mutations:

1. Publish prior-cycle senses and status sysvars to DNA memory.
2. Execute DNA and collect movement, aim, shot, tie, reproduction, and biology commands.
3. Convert movement commands into voluntary impulses and charge movement energy.
4. Apply gravity, Brownian motion, surface friction, fluid drag, and other environmental forces.
5. Integrate organism velocity and position with DB2 safety clamps.
6. Resolve wall, obstacle, organism, and tie constraints.
7. Rebuild spatial indexes from integrated positions.
8. Spawn requested shots from the organism's post-movement position and actual velocity.
9. Advance existing shots, detect swept collisions, apply impact behavior, age and decay survivors, and remove expired shots.
10. Resolve ties and non-shot interactions.
11. Apply chloroplast/light feeding and vegetable population maintenance.
12. Apply lifecycle, reproduction, mutation, death, corpse, statistics, and snapshot publication.

No phase may observe partially integrated organism buffers. GPU work must preserve this ordering even when several uniform phases are fused.

## Physics Model

### State

The existing organism structure-of-arrays buffers remain the storage foundation. Physics adds or formalizes:

- Current position and prior position.
- Current velocity and actual velocity over the completed tick.
- Independent voluntary, environmental, resistance, and collision impulses.
- Angular aim and optional angular momentum where required by DB2 behavior.
- Derived mass and radius.

The physics implementation will be divided into focused modules rather than expanding `world.rs`:

- `physics/movement.rs`: DNA movement commands and voluntary impulse.
- `physics/environment.rs`: gravity, Brownian motion, friction, density, and viscosity.
- `physics/integration.rs`: velocity/position integration and safety clamps.
- `physics/collision.rs`: organism and boundary response.
- Existing GPU code implements the same phase contract through backend adapters.

### Movement

`.up`, `.dn`, `.sx`, and `.dx` are thrust requests. They do not directly replace velocity. The implementation will preserve DB2's legacy-vs-`NewMove` mass behavior and the normal `PhysMoving` multiplier. Voluntary acceleration is clamped using the configured maximum velocity safeguard before becoming an impulse.

Velocity persists between ticks. Resistance forces reduce it according to environment settings. A bot that stops thrusting therefore coasts instead of stopping instantly. Animal Minimalis can stop thrusting while firing without becoming permanently fixed in place.

### Defaults and Live Settings

The initial normal-simulation defaults are taken from DB2, including maximum velocity `60` and movement efficiency `0.66`. Existing advanced environment controls remain live. The advanced panel gains the DB2 parameters needed by the model:

- Maximum velocity.
- Movement efficiency.
- Surface gravity and static/kinetic friction.
- Fluid density and viscosity.
- Collision elasticity.
- Brownian magnitude.
- X/Y gravity.

The controls stay in the existing advanced settings surface and preserve its layout conventions.

## Projectile Model

### Storage

Shots become persistent engine entities stored in bounded structure-of-arrays buffers with stable internal slots. Each live shot contains:

- Owner stable ID and originating species.
- Current and previous position.
- Velocity.
- Age and maximum range in ticks.
- Shot type and payload value.
- Current shot energy and original range.
- Memory location/value metadata for venom and positive-memory shots.
- Stored DNA payload for virus or sexual shots when those types are active.
- Flags for stored, impact flash, and alive state.

Shot capacity grows in bounded chunks and compacts or reuses dead slots without invalidating organism IDs.

### Creation and Motion

Creation ports `newshot` behavior:

- Spawn at the firing organism's radius along its aim.
- Apply forward/backshot/aimshoot direction and DB2 random angular spread.
- Add the organism's actual velocity to a 40-unit shot impulse.
- Calculate range and shot energy using DB2 body and range-multiplier formulas.
- Clear one-cycle shot command sysvars once the request is accepted.

Each tick advances shots with Euler integration. Collision uses the segment between previous and current position so fast shots do not tunnel through small organisms. Parent/newborn immunity, obstacles, world boundaries, and corpses follow DB2 rules.

### Aging, Decay, and Rendering

Shot age advances once per tick unless the applicable DB2 no-decay setting suppresses it. Energy decays with the nonlinear DB2 formula near the end of range. A shot is removed after impact/flash completion or once age exceeds range.

Snapshots expose current projectile segments, not attacker-to-target beams. The viewport draws the short previous-to-current segment or impact flash. Repeated firing therefore appears as moving particles rather than permanent lines.

### Effects

Existing modern effects for feeding, donation, venom, waste, poison, body, memory, reproduction, virus, and sexual shots move behind a single projectile-impact interface. Effects are applied only after a projectile collision. Statistics distinguish shots created, shots impacting, and effects successfully applied.

## Vegetable and Chloroplast Model

### Sysvars

The parser and engine use the actual DB2 memory addresses:

- `.chlr` = `920`
- `.mkchlr` = `921`
- `.rmchlr` = `922`
- `.light` = `923`

Other chloroplast sysvars, including `.availability` and `.sharechlr`, will be audited from `DNATokenizing.bas` before implementation. Symbolic and numeric legacy DNA must observe the same addresses. Existing modern saves are not required to retain the old incorrect chloroplast aliases.

### Initialization and Inheritance

Vegetables loaded as initial or repopulated species begin with the configured DB2 `StartChlr` reserve, normally `16,000`. Ordinary offspring inherit/split chloroplast and body state according to the DB2 reproduction routine rather than being reset to modern defaults. Non-vegetable bots may acquire and use chloroplasts through DNA or ties.

### Feeding

The feeding phase ports `feedvegs`:

- Determine whether light is available from day/night and threshold controls.
- Calculate usable world area after obstacles.
- Calculate occupied robot area and available-light crowding correction.
- Apply optional sun position/range and pond-depth attenuation.
- Calculate chloroplast gain and maintenance loss from DB2 formulas.
- Apply age and tide corrections where enabled.
- Divide production between energy and body using `VegFeedingToBody`.
- Clamp energy and body to DB2 limits and publish `.light`/`.availability` consistently.

The unconditional modern vegetable bonus is removed from normal DB2 feeding. A compatibility/debug setting may retain a flat bonus, but its default is zero and it cannot stack silently with photosynthesis.

### Population Maintenance

DB2 repopulation uses total chloroplast equivalents rather than only a species count. When equivalents fall below the configured minimum:

- Advance the repopulation cooldown.
- Add the configured batch amount when cooldown elapses.
- Randomly choose an eligible repopulating vegetable species.
- Spawn within that species' permitted world region.
- Initialize it with normal starting chloroplast, body, and energy state.

The modern vegetable cap remains a hard upper bound after DB2's maintenance logic. DNA-driven vegetable reproduction is still permitted below the cap.

## Setup and Runtime Behavior

The Zerobot sustenance selector is applied only when the selected starting mode contains Zerobots. Starter Bots + Vegetables always starts with normal metabolism and DB2 vegetable feeding. The UI will disable or annotate irrelevant Zerobot controls when starter mode is selected.

Physics, shot-decay, and plant-energy settings remain editable while running. Changes affect subsequent phases and do not rebuild or reset the world.

## Persistence and Compatibility

The binary save schema version increases. Saves include all organism physics fields, live projectiles, chloroplast/light state, vegetable repopulation cooldown, and new environment settings. Load validates bounds and discards no live projectile state.

Legacy DNA text remains the required compatibility format. Symbolic sysvars and direct numeric addresses are tested. Compatibility with old modern-alpha saves is optional; if rejected, the error must clearly report the unsupported save version.

## CPU and GPU Boundaries

The CPU implementation is completed and validated first. `SimulationBackend` receives explicit contracts for:

- Force accumulation and integration.
- Collision candidate generation.
- Projectile integration and candidate generation.
- Render-instance preparation.

GPU kernels may accelerate uniform integration and broad-phase candidate generation. Collision effects, variable shot payloads, DNA, and authoritative lifecycle transitions remain on CPU initially. CPU and GPU differential tests compare invariants and bounded floating-point tolerances after each accelerated phase.

## Verification

### Formula Fixtures

Create focused fixtures from the VB6 routines for:

- Movement impulse with legacy and `NewMove` bots.
- Momentum retention with no thrust.
- Maximum-velocity clipping.
- Gravity, Brownian motion, friction, density, viscosity, and elasticity.
- Shot creation position, velocity, body-derived range, aging, decay, and expiry.
- Swept projectile collision and every supported shot effect.
- Chloroplast construction/removal costs and correct addresses.
- Light crowding, day/night, pond depth, energy/body split, and repopulation cooldown.

### Scenario Tests

- Animal Minimalis must show sustained displacement over time rather than only movement-command counters.
- An Animal Minimalis that begins firing must retain momentum and later reacquire new targets.
- Visible shots must move between snapshots and disappear within their calculated range.
- Starter vegetables must initialize with chloroplasts, remain bounded by the hard cap, and repopulate after depletion.
- Starter ecology must sustain deaths, births, feeding, and nonzero moving-animal populations over long runs.
- One-million-tick headless stress runs must preserve invariants and bounded memory.

### Visual Acceptance

Use keyboard-driven `Alt+Print Screen` clipboard capture for Darwinbots because Windows Graphics Capture cannot capture the Avalonia window on this host. Compare fresh-world screenshots at controlled tick intervals with DB2 reference behavior:

- Animals visibly coast and change locations.
- Shots appear as short moving particles or impact flashes, not persistent beams.
- Vegetables form a maintained but nonexplosive food population.
- The simple DB2-style visual language remains unchanged.

## Rollout Sequence

1. Capture VB6 formula fixtures and finalize setting defaults.
2. Correct chloroplast sysvars and add save-schema migration boundary.
3. Port organism impulse physics on CPU.
4. Port persistent projectile storage, motion, collision, and effects.
5. Port chloroplast feeding and repopulation.
6. Fix setup-mode metabolism and expose advanced settings.
7. Run full CPU tests and long ecology scenarios.
8. Update GPU backend contracts and differential tests.
9. Package and conduct timed visual playtests.

The production build is not considered fixed until all three systems pass together in a fresh Starter Bots + Vegetables simulation.
