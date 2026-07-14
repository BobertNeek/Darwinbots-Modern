# DB2 Defaults and Sysvars Audit

Date: 2026-07-13

## Authority

- `Darwinbots2/MDIForm1.frm`, `MDIForm_Load`: clean DB2 2.48 defaults.
- `Darwinbots2/SimOptions.bas`, `SimOptions`: setting meanings and zero-initialized fields.
- `Darwinbots2/Master.bas` and `Darwinbots2/Vegs.bas`: vegetable-energy wiring and formulas.
- `Darwinbots2/DNATokenizing.bas`, `LoadSysVars`: all symbolic sysvar names and addresses.

Saved `lastran.set` and `lastexit.set` files are user state, not clean defaults. F1/F2/SB presets are optional profiles, not normal-simulation defaults.

## DB2-backed defaults

| Setting | DB2 2.48 | Modern default |
| --- | ---: | ---: |
| World width | 16000 | 16000 |
| World height | 12000 | 12000 |
| Maximum velocity | 60 | 60 |
| Movement efficiency (`PhysMoving`) | 0.66 | 0.66 |
| Brownian motion (`PhysBrown`) | 0.5 | 0.5 |
| X/Y gravity | 0 / 0 | 0 / 0 |
| Surface gravity | 0 | 0 |
| Static/kinetic friction | 0 / 0 | 0 / 0 |
| Fluid density/viscosity | 0 / 0 | 0 / 0 |
| Collision elasticity | 0 | 0 |
| Shot impulse speed | 40 | 40 |
| Energy-shot no-decay | false | false |
| Waste-shot no-decay | false | false |
| Starting chloroplasts | 16000 | 16000 |
| Vegetable energy allotment (`MaxEnergy`) | 100 | 100 |
| Minimum chloroplast equivalents (`MinVegs`) | 50 | 50 |
| Repopulation amount | 10 | 10 |
| Repopulation cooldown | 10 | 10 |
| Vegetable feeding to body | 0.75 | 0.75 |
| Daytime | true | true |
| Day/night enabled | false | false |

DB2 calls `feedvegs SimOpts.MaxEnergy`; the default `100` is therefore intentional. The engine does not lower this value to compensate for population size. Per-organism energy and body remain capped at `32000`, matching `Vegs.bas`.

## Modern-only defaults

These have no direct clean-DB2 equivalent or intentionally preserve the approved modernization requirements:

| Setting | Modern default | Reason |
| --- | ---: | --- |
| Organism capacity | 25000 in setup | Performance safety control; DB2's `MaxPopulation = 100` is a vegetable population option, not an engine allocation ceiling. |
| Vegetable hard cap | 500 | Catastrophic-growth guard retained by the approved design. |
| Backend | Auto | Select GPU when available with CPU fallback. |
| Flat vegetable compatibility bonus | 0 | DB2 chloroplast feeding is authoritative; the compatibility bonus must not stack with it. |
| Metabolism abstraction | 1 | Temporary aggregate cost until the complete DB2 cost table is ported. |
| Shot range multiplier | 1 | Neutral modern scaling hook. |
| Day/night cycle fallback length | 10000 | Inactive while DB2's default day/night toggle is false. |
| Zerobot sustenance | Disabled metabolism | Applies only to Zerobot setup modes and never overrides normal starter metabolism. |

## Sysvar result

The modern runtime table is generated from the active `sysvar(...)` entries in DB2 2.48's `DNATokenizing.bas`:

- DB2 names audited: 255
- Modern names resolved: 255
- Missing names: 0
- Wrong addresses: 0
- Extra runtime names: 0

The audit corrected `.trefbody` from `472` to DB2 address `437` and restored 56 missing names, including focusable-eye controls, extended input/output channels, total-population references, tie channels, `.fertilized`, `.reftype`, and `.pval`.

The test `every_db2_248_sysvar_resolves_to_its_vb6_address` parses the VB6 source table directly and compares every entry to the Rust resolver, preventing the copied runtime table from silently drifting.
