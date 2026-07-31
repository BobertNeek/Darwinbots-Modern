# DB2 2.48.32 Non-Online Gap Register

Last updated: 2026-07-30

This register maps every P0/P1 finding from the four systemic audits to a bounded status. `complete` is reserved for the Phase 0 B1-B4 repairs and their deterministic evidence closures. `partial` means useful behavior exists but the audit finding remains open. Catalog registration and test counts never make a subsystem complete. Online networking is `excluded`, not complete.

Audit P0 summary: 20 P0 findings remain `open` or `partial`; 2 P0 findings are `complete`. B2 and B3 are complete evidence closures inside broader audit findings, so they do not erase those broader gaps.

## Cycle/state

| Priority | Finding | Status | Evidence or limit | Dependency / next phase |
|---|---|---|---|---|
| P0 | Canonical cycle and observable state timing differ | open | Modern phase ordering, parallel DNA execution, and queued birth/death timing do not match the audit contract. | Build the three-cycle trace fixture, then repair orchestration order. |
| P0 | DNA execution safety, accounting, and observability are absent | open | No 32,000-block cap, full gene/condition/activation/debug report, or per-token cost accounting. | DNA runtime phase after canonical cycle tracing. |

## DNA/sysvars

| Priority | Finding | Status | Evidence or limit | Dependency / next phase |
|---|---|---|---|---|
| P0 | Tokenizer failure semantics reject valid or mutated legacy input | open | Slash headers, implicit `end`, and unknown-token-to-zero behavior differ. | Golden tokenizer fixtures from authoritative samples. |
| P0 | Accepted advanced/debug tokens can execute as no-ops | open | Parser coverage exceeds VM behavior. | Table-driven VM semantics and negative tests. |
| P0 | Full alias lookup is not full sysvar behavior | partial | The name/address table exists, but selected accepted sysvars remain inert. | Command/sense lifecycle fixtures for all active addresses. |
| P1 | DNA text export loses metadata and source identity | open | Export omits aliases, generation, mutation/tag data, and legacy hash. | Define lossless DNA source/export contract. |

## Physics/senses

| Priority | Finding | Status | Evidence or limit | Dependency / next phase |
|---|---|---|---|---|
| P0 | Core senses are incomplete and occur at the wrong time | open | Touch, edge, taste, occlusion, focus, tie, body-change, and environmental senses remain incomplete. | Ordered sense-lifecycle fixtures after cycle repair. |
| P1 | Physics and topology retain kernels but not DB2 behavior | partial | Drag/friction kernels exist; mass, pond/tide, angular motion, wrap/borders, cluster collisions, and obstacle collision semantics differ. | Canonical physics fixtures and topology split. |

## Shots/defenses

| Priority | Finding | Status | Evidence or limit | Dependency / next phase |
|---|---|---|---|---|
| P0 | Shot, defense, virus, and sperm rules differ | open | Feeding, shell/body, poison/venom, virus insertion, sperm storage, range, and decay semantics differ. | Per-shot-type golden impact suite. |

## Ties/multibots

| Priority | Finding | Status | Evidence or limit | Dependency / next phase |
|---|---|---|---|---|
| P0 | Ties and multibots are simple links | partial | Ports, capacity, hardening, breakage, torque, targeting, and resource protocols remain absent. | Reciprocal-port and multibot mechanics phase. |
| P0/B2 | Whole-organism destructive routing | complete | Stable `slot:generation` capture queues atomic native `remove_many`; the permanent-tie group is removed with one publication. | Retain deterministic engine, native JSON, and desktop regression tests. |
| P1 | Whole-organism export is modern JSON without an import partner | partial | Picker JSON now contains only the captured group and internal ties; legacy interchange/import remains absent. | Organism interchange design in persistence phase. |

## Reproduction/genealogy

| Priority | Finding | Status | Evidence or limit | Dependency / next phase |
|---|---|---|---|---|
| P0 | Robot state cannot represent DB2 heredity and reproduction | open | Required private state, transfer rules, birth placement/tie, sexual state, and staged fields are absent. | Expand authoritative organism state, then golden parent/child fixture. |
| P1 | Genealogy and console are summaries, not diagnostics | open | Current lineage, console, and activation views lack live diagnostic contracts. | Engine observability first, then dedicated nonmodal tools. |

## Ecology/corpses

| Priority | Finding | Status | Evidence or limit | Dependency / next phase |
|---|---|---|---|---|
| P1 | Ecology, materials, corpses, and species policy are simplified | partial | Some construction/photosynthesis exists; waste, defenses, decay, dynamic costs, corpse modes, and policy fields differ. | Ecology/materials phase after canonical timing. |

## Mutation/speciation

| Priority | Finding | Status | Evidence or limit | Dependency / next phase |
|---|---|---|---|---|
| P1 | Mutation and speciation are generic | partial | Live authoritative settings are evidenced, but DB2 mutation classes, distributions, logs, limits, and named speciation remain open. | Per-robot mutation model and deterministic mutation fixtures. |
| P0/B1 | Save Sim Without Mutations copy semantics | complete | The serialized copy disables mutation while authoritative live state remains unchanged. | Retain save/load and live-state regression tests. |
| P0 | Directed-evolution and restriction enforcement are incomplete | open | Competition restrictions, disqualification, fitness, rollback, and restart policy are absent. | Competition authority and durable stage checkpoint phase. |

## Persistence/offline modes

| Priority | Finding | Status | Evidence or limit | Dependency / next phase |
|---|---|---|---|---|
| P0 | Legacy simulation/settings interchange is absent | open | Modern `.db3s` cannot read or emit DB2 simulation/settings/organism/stats formats. | Versioned compatibility/import-export plan. |
| P1 | Modern persistence is deterministic but not product-compatible | partial | Modern state round-trips; DB2 save, graph, snapshot, genealogy, and recovery contracts do not. | Compatibility fixtures and legacy file boundaries. |
| P1 | Snapshot recording does not meet the DB2 contract | open | Living/dead records use modern JSON behavior rather than the required genome record policy. | Recording contract and repopulation-exclusion tests. |
| P1 | Safemode recovery is rotating autosave | open | No last-start artifact, fault-window narrowing, crash classification, or diagnostic recovery. | Recovery controller and induced-failure test. |
| P1 | Settings persistence is coupled to world saves | open | No standalone settings file workflow. | Separate settings authority and serializer. |
| P0 | Offline league, contest, F1, and restart orchestration is absent | open | Presets, rounds, adjudication, survival layers, results, and deterministic restart are absent. | Offline competition phase. |
| P0 | Graph contract is reduced to two mini charts | open | No full metric set, multiple windows, reset, custom series, or compatible export. | History contract, graph windows, then export. |
| P0 | Safemode is autosave, not fault-window recovery | open | Same systemic gap as the persistence audit; current recovery only selects recent autosave. | Shared recovery phase. |
| P0 | Offline file teleporters and full obstacle behavior are missing | open | Local file transfer, filters, backflow, tied transfer, and obstacle policy are absent. | Persistence boundary before teleporter workflow. |
| P1 | Snapshots and organism persistence use modern JSON | partial | Captured organism JSON is now internally exact, but DB2 record behavior remains absent. | Legacy recording/interchange phase. |
| P1 | Save without mutations was mislabeled | complete | B1 copy-only transform is now deterministic and leaves live state unchanged. | Retain focused regression coverage. |
| P1 | CLI cannot automate offline workflows | open | No CLI save/load, league, contest, graph, recording, or recovery commands. | Add only after workflow authorities exist. |
| Excluded | Online networking | excluded | Internet commands are disabled and inert; no network work is performed. | No online implementation in this program. |

## GPU differentials

| Priority | Finding | Status | Evidence or limit | Dependency / next phase |
|---|---|---|---|---|
| P1 | Interaction-bearing CPU/GPU coverage is incomplete | open | Existing differentials cover selected kernels, not every phase or long ecological scenario. | Extend only after canonical CPU semantics are fixed. |

## Desktop shell/settings/tools/viewport

| Priority | Finding | Status | Evidence or limit | Dependency / next phase |
|---|---|---|---|---|
| P0 | Main shell and status instrument are a redesign | open | Dashboard, toolbar, charts, log, and status differ from the MDI compatibility layout. | Dedicated compatibility shell phase. |
| P0 | Menu completeness is mostly cosmetic | partial | Typed routing, B1-B4, view toggles, and current object tools are evidenced; distinct settings/actions remain collapsed. | Implement distinct authorities one workflow at a time. |
| P0/B4 | Keyboard shortcuts were incompatible and incomplete | complete | Every retained catalog shortcut is applied as its exact input gesture and exercised through keyboard input. | Keep conflict and routing regression tests. |
| P0 | Simulation Settings lacks DB2 working-copy semantics | open | Setup/live settings do not provide the fixed six-tab species-first workflow. | Settings working-copy model and validation phase. |
| P0 | Focused robot tools are summaries | open | Generic document/value-window substitutes lack live specialized behavior. | Engine observability, then dedicated tool windows. |
| P0 | Viewport visual and interaction parity is incomplete | partial | Current toggles and object tools work, but eyes, gauges, sprites, properties, click modes, cursor feedback, and zoom semantics differ. | Viewport compatibility phase. |
| P1 | Parameter-dialog ecosystem is absent | open | Physics, costs, weather, mutation, restriction, shape, and teleporter editors remain incomplete. | Build after authoritative settings models. |
| P1 | Graph and status coverage is skeletal | open | Two embedded charts and one status string do not satisfy the graph/status contract. | Shared history/status authority. |
| P1 | Original visual state controls are not synchronized | open | Many legacy runtime controls and indicators are absent. | Compatibility shell and state projection. |
| P1 | Persistence contracts diverge | partial | B1 is closed and captured organism JSON is narrowed; legacy snapshots and interchange remain open. | Persistence/offline phase. |
| P1 | Modal/nonmodal and refresh behavior differs | open | Generic tools use modal dialogs and lack live refresh. | Dedicated nonmodal tool framework. |
| P1 | Help, onboarding, and About are incomplete | open | Generic text substitutes remain. | Content and first-run phase after core workflows. |
| P0/B3 | E-Grid and Survival Info hidden-state contract | complete | E-Grid is absent by default and Survival Info is mode-gated. | Retain visibility-state tests. |

## Acceptance evidence

| Priority | Finding | Status | Evidence or limit | Dependency / next phase |
|---|---|---|---|---|
| P0/P1 | Catalog and green-count overclaim risk | partial | Phase 0 records exact deterministic and rendered boundaries; counts are explicitly non-parity evidence. | Maintain requirement-to-test receipts per subsystem. |
| P0 | Exact original GUI black-box evidence is missing | open | No successful compatible execution of the 2.48.32 GUI is available. | Capture authoritative fixtures and rendered original behavior on a working environment. |
| P0/B1-B4 | Phase 0 repair evidence | complete | Copy-only mutation-free save, atomic group removal, hidden states, and exact retained input gestures have focused deterministic coverage. | Preserve these gates while later phases proceed. |
