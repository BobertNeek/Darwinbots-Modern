# Organism Visual Phenotypes Design

## Goal

Restore the visual information carried by Darwinbots 2 organisms while keeping the rendering path scalable and leaving room for richer future visuals.

The modern game will show:

- DB2-accurate body size derived from body points and chloroplasts.
- A small inherited species skin inside every organism, rotated with its aim.
- Gradual lineage color drift caused by actual DNA mutations.
- A modest skin variation only when a lineage crosses the automatic-speciation threshold.
- Autotroph colors constrained to green hues.
- A compact heading marker on every visible organism.
- The complete nine-eye field only for the selected organism.

## Behavioral Authority

The VB6 implementation remains authoritative for body radius, mutation color drift, organism aim, and eye geometry.

The modern implementation intentionally clarifies one legacy ambiguity:

- DB2 2.44.1 and 2.48 contain a disabled `MutateSkin` routine.
- The modern engine will not vary skin on ordinary mutations.
- A skin variation will occur only when automatic speciation creates a new lineage, matching the remembered user-facing behavior without making every mutation visually noisy.

## Engine State

Each organism owns a compact visual phenotype:

- `color`: packed RGBA color inherited from its parent.
- `skin`: four polar points defining a three-segment polyline.
- `lineage`: stable lineage identifier used for inspection and persistence.

Species definitions retain their initial color and skin. Initial organisms copy those values into organism state. Children copy their parent's phenotype before mutation and speciation processing.

Visual phenotype belongs to the simulation state, not the desktop renderer. This ensures saves, headless snapshots, CPU execution, and future GPU rendering all agree.

## Color Mutation

When a mutation report contains one or more real DNA changes:

1. Start from the parent's inherited color.
2. Apply one small channel adjustment per reported DNA change.
3. Use the simulation random stream so the operation remains reproducible when the CPU seed is reproducible.
4. Clamp channels to valid display values.
5. Preserve readable saturation and brightness.

For non-autotrophs, RGB channels may drift locally as in DB2.

For autotrophs, convert the result to HSV, constrain hue to the green band, and clamp saturation and value to keep the organism visibly green. Autotroph descendants may still form distinguishable green lineages.

## Automatic Speciation and Skin Variation

Automatic speciation uses accumulated lineage mutation distance relative to DNA length. The threshold remains a simulation setting.

When the threshold is crossed:

1. Allocate a new lineage identifier.
2. Preserve the organism's current color as the new lineage color.
3. Vary one skin point modestly in radius or angle.
4. Clamp the resulting point so the skin remains inside the organism circle.
5. Reset the lineage's accumulated mutation distance.

The changed skin is inherited by descendants. It does not continuously animate or change on ordinary ticks.

## Size and Energy Display

Bot radius follows DB2 `FindRadius` behavior:

- Body points determine the base radius.
- Chloroplasts increase radius toward the chloroplast maximum.
- Stored energy does not directly alter radius.

Body remains a functional energy reserve through `.strbody` and `.fdbody`; one body point represents ten energy points. The existing visual radius therefore changes as bots store or recover body.

Energy may later be shown through the optional DB2-style resource arc, but it is not part of this implementation slice.

## Snapshot and ABI

Render instances will carry everything needed for the normal viewport without a per-frame lookup:

- slot and generation
- position
- radius
- packed color
- aim
- four skin points
- vegetable/autotroph flag

The immutable organism snapshot continues to expose detailed biology and eye state for selection and inspection.

The ABI change must be versioned and parsed defensively by the desktop client. Older snapshots without phenotype fields receive generated defaults.

## Rendering

Normal organisms are drawn in this order:

1. Filled circle using inherited organism color.
2. Selection outline when applicable.
3. Skin polyline in a contrast-adjusted tint of the organism color.
4. Small heading marker at the circumference.

Skin point geometry is scaled by the rendered radius and rotated by aim. At low zoom or very high population, the renderer may omit skin and heading marks while preserving colored circles. This is a presentation optimization only.

The selected organism additionally shows nine eye sectors using its configured eye directions and widths. The overlay is world-space, translucent, and non-interactive. It must not imply that the skin itself is the vision cone.

## Persistence

Autosaves persist organism color, skin, lineage identifier, and accumulated lineage mutation distance. Loading older modern saves generates phenotype values from the species definition and preserves all other organism state.

Imported DNA files receive the chosen species color and a stable generated skin. Exported DNA remains plain legacy-compatible DNA and does not embed visual metadata.

## Testing

Focused tests will cover:

- DB2 radius behavior for body and chloroplast changes.
- Parent-to-child phenotype inheritance.
- No color change when mutation reports zero DNA changes.
- Local color drift after real DNA mutation.
- Green-band enforcement for autotroph descendants.
- No skin change on ordinary mutation.
- One bounded skin change at automatic speciation.
- Save/load phenotype round trips and old-save defaults.
- Snapshot parsing of aim, color, skin, and lineage fields.
- Renderer geometry for aim rotation and nine-eye selection overlay.

Visual acceptance will compare the running desktop viewport against an image blueprint and screenshots at normal and dense population levels.
