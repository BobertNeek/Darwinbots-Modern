# Unified Live Settings and Continuous Mutation Design

Date: 2026-07-19
Status: Approved

## Problem

Darwinbots Modern currently consults a species mutation percentage only when that species reproduces. A zerobot that has not evolved reproduction therefore cannot mutate and can never escape its initial all-zero genome. This differs from DB2, which schedules point mutations on living organisms by age as well as applying copy-time mutations during reproduction.

The running simulation also exposes only a few controls in collapsed left-panel sections. Physics and energy updates technically exist, but they are difficult to discover, mutation rates are setup-only, and world limits cannot be adjusted from one coherent runtime surface.

## Product Decisions

- Add a prominent `SETTINGS` button to the main simulation toolbar beside `RESET`.
- Open one unified Live Settings window using the existing DB2-style typography, spacing, colors, and left-navigation conventions.
- Apply engine-affecting changes through the simulation command queue at a tick boundary.
- Add continuous point mutation for living organisms while retaining structural/copy mutation during reproduction.
- Treat mutation percentages as live species settings inherited by current organisms and their descendants.
- Make every setting with meaningful runtime behavior editable while the simulation is running.
- Keep starting mode and initial species count setup-only because they describe world creation rather than live state.

## Live Settings Surface

The window has five focused sections rather than one long form:

1. **Species + Mutation**
   - Current species name and vegetable status.
   - Mutation enabled toggle.
   - Mutation activity slider from `0%` to `100%`.
   - Reseed toggle and minimum population.
2. **Environment + Energy**
   - Metabolism, sunlight, plant energy, chloroplast feeding, day/night, gravity, drag, and Brownian motion.
3. **Physics + Interactions**
   - Velocity, movement efficiency, friction, density, viscosity, elasticity, shot speed/range/decay, and shot decay exceptions.
4. **World**
   - World width and height, toroidal edges, live population ceiling, and vegetable ceiling.
   - World resizing wraps or clamps existing positions according to the toroidal setting.
   - The live population ceiling may be raised only to the engine capacity reserved when the world was created; lowering it blocks new births but does not delete organisms.
5. **Runtime + Display**
   - Waste rendering, selected-organism vision, automatic speciation, and speciation distance.
   - Speed and backend remain in the main toolbar because they are transport controls and already apply immediately.

The current left-panel quick controls remain as shortcuts and update from the same live settings state. They no longer form the only path to the settings.

`APPLY` validates the entire form, submits one versioned command batch, waits for the engine-thread acknowledgement, then refreshes the window from the published snapshot. A rejected field leaves all live settings unchanged and produces a specific status message. `CANCEL` closes without changing state.

## Continuous Mutation Semantics

Each living organism has a scheduled next point-mutation age. A species mutation activity of `0%` disables scheduling. Nonzero activity uses DB2's point-mutation probability shape:

```text
per-base per-tick probability = mutation_percent / 5,000,000
```

This makes `1%` correspond to DB2's default point-mutation denominator of `5000`, where DB2 evaluates one chance per base pair per 1000 cycles. The engine samples a geometric interval from the organism's seeded random state and DNA length instead of testing every base every tick. At the scheduled age it applies one numeric point mutation, records the mutation, updates the visual phenotype and speciation distance, then schedules the next age.

Reproduction retains the existing percentage chance to apply a structural genome mutation to the child. Thus the same species slider controls both DB2-style lifetime point mutation and reproduction-time mutation without making either path dependent on the other.

Changes to a live species mutation rate invalidate and reschedule the next point-mutation age for all living members of that species. Setting the rate to zero cancels pending continuous mutations immediately.

## Engine and ABI Boundaries

- Extend species snapshots with the current mutation rate, mutation-enabled state, reseed state, and minimum population.
- Add versioned commands for live species settings and live world limits.
- Keep environment/physics updates on the existing `update_environment` path.
- Apply a settings batch atomically on the simulation thread before the next tick phases begin.
- Store the next point-mutation age with organism state so save/load preserves mutation scheduling.
- Share one mutation-result helper between continuous and reproduction mutation so statistics, DNA reference counts, phenotype drift, and auto-speciation stay consistent.

## Runtime Safety

- Clamp percentages to `0..100` and reject non-finite values.
- Reject world dimensions below the minimum viable viewport size.
- Reject a live population ceiling above the reserved engine capacity.
- Never kill organisms merely because a ceiling was lowered.
- If an organism's DNA contains no numeric instruction, a scheduled point mutation records no change and reschedules normally; structural reproduction mutation can still create mutable instructions.
- Existing saves that lack mutation scheduling fields initialize schedules from species settings after load.

## Acceptance Criteria

- A zerobot population with mutation enabled accumulates point mutations without first reproducing.
- A species at `0%` does not mutate continuously or during ordinary reproduction unless DNA explicitly requests mutation reproduction.
- Moving a live mutation slider changes existing organisms and descendants without resetting the world.
- Mutation statistics, DNA editor contents, phenotype color drift, and auto-speciation reflect continuous mutations.
- The main toolbar exposes `SETTINGS` without requiring users to discover a collapsed expander.
- Every runtime-meaningful setup/advanced field is represented in Live Settings and applies without restarting.
- Initial count and starting mode are clearly identified as world-creation-only rather than silently ignored.
- Applying invalid settings is atomic and leaves the running world unchanged.

