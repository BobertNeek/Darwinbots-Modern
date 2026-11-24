# Darwinbots Modern Verification Ledger

## November 24 integration status

Full offline acceptance remains open. The first follow-up merged to `master` as `45e19d22e639980d7545aace2ede8948d8825dc3`. Its tree matches candidate `32cb60847a1de99c8f6b892e092714bf0c25c298`. Windows, Linux, Intel macOS, and Apple silicon passed tests, publish, and package smoke. Local Windows ran 334 desktop-core tests, 240 rendered checks, and 28 regular FFI cases. The physical FFI case also passed on an NVIDIA RTX 3050 using Vulkan, driver 610.88. The Windows package smoke advanced original-format SIM and round-tripped SIM and DBO. [Merged changes](https://github.com/BobertNeek/Darwinbots-Modern/pull/265) · [CI and artifacts](https://github.com/BobertNeek/Darwinbots-Modern/actions/runs/45349321978)

| Package | Artifact ID | GitHub ZIP SHA-256 |
| --- | ---: | --- |
| Windows x64 | 14902060697 | `c64696f1e36129b41946ff4e92297a935e7c860d823ea2a6daeb09d7ca611111` |
| Linux x64 | 14900726213 | `35aa634fcf42033793f9233cc74d754213668048174dd81d64e76f4ff84b1926` |
| Intel macOS | 14902270479 | `b04e235cd6cc5a77c0050368129e8076fd2470e44d8354c1e10e57f79dbd9100` |
| Apple silicon | 14900647291 | `89bef58b9e5bf881aa2292eae2be4d82181acd19c3135391c4b0ab6009219de0` |

The earlier CI package at `da3581e05903ea5315b8903835519d86a5c7f99d` also passed native Windows F2 load, F7 step, and F3 save. A copied original SIM advanced from cycle 281 to 282 and the DB3S readback retained cycle 282 and 194 robots. SHA-256 of the Windows package ZIP was `38684f367d7feed03763e158b5f1a4d24293e041017474e71847ba197cd964cf`. SHA-256 of the DB3S was `b022b5594058f81d30eff15511945744efb8f93563180aa5a5fec64b8a4306f6`.

The later local Windows build at `e465a1ae539ccf6589a55a283010058140d1ff04` opened a positional SIM at cycle 281. F12 ran and paused it twice. The Population graph exported two species at cycle 400 to a CSV with SHA-256 `78ba1ce49f981598729c831e9b7fab995a36d9dc37617975403c71444a5d1203`. Paused Species Apply kept cycle 513, 453 robots, and selected robot `16:0`. Reopening showed the edited name, initial count zero, and energy 777. Starting with that roster kept two species and spawned five organisms from the other species only. Global Settings saved Hide system tray icon, showed the checked value after reopening and prevented the tray button from hiding the simulation. Restoring the option persisted the original unchecked value. Interactive tray restore remains open.

The isolated Windows build with the source tree at `2d27cce7b68efbb30f7b33e5848232d95ed4295b` saved a living snapshot and a mutation-history sidecar at cycle 281. The SNP SHA-256 was `f184a26d47f33d8403d19904b1dcf215e0d68d528db985411b0cfc128897ac13`. The sidecar SHA-256 was `dc17701e6f5dc89234f02b722150deac533e342249c04eb4ca693b0074018b03`. Recording → Recover latest autosave restored a hash-verified copy of the earlier cycle-282 DB3S with 194 organisms. The desktop displayed the recovered cycle, population, and checkpoint filename. The same build completed a one-round direct league between The One and Saber. With a configured limit of 20 cycles, it adjudicated at the cycle-30 sample and displayed a 1–1 result. Restart opened a fresh match with the same entrants and limit. A separate Simple Survival run showed Complete at 20 of 20 mode cycles. Saving at world cycle 450 and reopening restored both counters and the completed mode in the desktop. The survival DB3S SHA-256 was `32cf2c9e169e68dd5e5324a3f75dd1be35fb050b36bb5e30fe11875b92232db8`. The native Robot Console opened for robot `5:2` and returned `mem[310] = 1889`. With dead recording enabled, Ctrl+K on the selected robot produced a `DeadRobots.snp` record and mutation-history file under the disposable profile. The SNP SHA-256 was `c8e6c1a7942c376d3ddfef7d4814741f3b0afbdbddbdaac2d8e5f15d293aa82a`.

The same Windows build loaded a native SIM prepared with one inbound and one outbound file teleporter in a disposable directory. F2 showed native cycle 281 and both portals on the field. Two F7 steps removed the inbound DBO and wrote a native DBO to the outbound directory. F3 saved a DB3S at cycle 283 with inbound `received: 1` and outbound `sent: 1`. Both persisted rows had `internet: false`. The input SIM SHA-256 was `db854250155722d0fc7067c4b50dc6f31a763eab0a162cf8afb9bc8d50739eeb`. The outbound DBO SHA-256 was `463e328f60eee587818f413513a47b1a59a7ae0d8fd06101c4e5b69f5009c253`. The DB3S SHA-256 was `e85eae721d40bcc23b8e146b32102735b8ba9ac419edad80391f48c8207d8939`. This run prepared the portal definitions through the Rust API and exercised transfer through the native desktop. Interactive portal creation remains open.

Original Windows directed-tie checkpoints confirm separate import and first-cycle behavior for reciprocal, one-way, and dormant input rows. The port now preserves their import pointers and native round-trip and tie-creation behavior in automated Windows tests. [Original executable receipts](windows-offline-acceptance.md#november-24-original-windows-results)

The Windows development build at source `cab0cb1c6bc985404c7a88e416d6bc379bd97248` opened the original SIM at cycle 281. In a disposable profile with the tray icon enabled, keyboard activation of Stealth Tray Mode hid the main window while the process stayed alive. The app host SHA-256 was `ee67baf3590d6aa4dcfd13eb67d657283c2d839fa25deec48ec1d76b4e65a000`. Notification-area icon visibility, live tray menu data, and restore remain open in the native walkthrough.

The compatibility integration merged at `b68cd911646c979d4f9389c0ddeb022aaba5de5c` from `6bb2d9c8c2ed09a5b2bb9400de69df6842607b0d`. Its Git tree `e40df83cc05857dc7f051d780d3703686d4960fd` passed Windows, Linux and Intel macOS on the push run. Apple silicon passed on the pull-request run with the same tree. [Merge](https://github.com/BobertNeek/Darwinbots-Modern/pull/266) · [Push run](https://github.com/BobertNeek/Darwinbots-Modern/actions/runs/45377220033) · [Pull-request run](https://github.com/BobertNeek/Darwinbots-Modern/actions/runs/45377229003)

The tools and runtime integration merged at `dad4115b304e0b021af861ee85563e20f268117b` from `e72b8153c3e810a3562a86124c3282ab02bf3e12`. All four jobs passed on the exact push run, including package smoke. [Merge](https://github.com/BobertNeek/Darwinbots-Modern/pull/267) · [CI and packages](https://github.com/BobertNeek/Darwinbots-Modern/actions/runs/45377264169)

The native viewport and physics integration merged at `bd9ace8852a68778108e42073ee66d3c2dd04685` from `b85e086081dcc397e94428acc114d6381cd9fcc1`. Windows, Linux and Apple silicon passed on the push run. Intel macOS passed on the pull-request run with the same tree. The package artifact IDs and SHA-256 hashes are Windows `14914084066` (`84a0254631528a5e015999c94ca29b4082a689fb8085c8efe958cd2f5892a2e9`), Linux `14913498440` (`137fdaaf82fa7af4c6640bd863259128dd90e31722fe8859ebfbc42039a385b3`), Intel macOS `14918837229` (`53b0e5cbc97a4a4c91bf5093aa8ac04fcae6f8987bddb4e1b0e1edd328db2702`) and Apple silicon `14916187131` (`ac192354cf62e196f84ab4fa022bad619cbb1eda42883883ca6c567c4e002274`). [Merge](https://github.com/BobertNeek/Darwinbots-Modern/pull/268) · [Push run](https://github.com/BobertNeek/Darwinbots-Modern/actions/runs/45377272621) · [Pull-request run](https://github.com/BobertNeek/Darwinbots-Modern/actions/runs/45377285725)

On November 24, the NVIDIA GeForce RTX 3050 physical Vulkan run at `b9a3873e88350df8f57d56fb7709b26115b23c33` matched the CPU reference and reloaded GPU simulation at each checkpoint across 10,000 additional original cycles. It began at cycle 281 and ended at cycle 10,281 with 2,035 robots, 966 shots, 9,149 recorded births and a 31,290,470-byte checkpoint. The native test completed successfully in 7,244.81 seconds. Seven strict engine cases, one CLI case, three integration cases and a backend switch test also passed. The continuation log SHA-256 is `d3780fca134f00f260055416f16451fb30e099f310dab5c05373fabe767d2c10`. The merge candidate changed the Windows CI test scheduling and a rendered test readiness guard after that hardware build.

| Platform | Compatibility ZIP artifact and SHA-256 | Tools and runtime ZIP artifact and SHA-256 |
| --- | --- | --- |
| Windows x64 | 14914091795 `442958b5872e070d8935ec290bf7a2604c775cb857172ad7429b2ade5763bc57` | 14914973247 `0b322fb0032a27e198bb2a017639a384ddf321b97d699b3a21705fbaa1273be5` |
| Linux x64 | 14914655755 `31f31cd03f5ee52141e5da85cde3f7d8bdc8fce1f681e9be70d6c4245895576d` | 14913621610 `c6d3d015e6e9c6fe4acc3704d1c5c03b4e23ff8acf695daa9aa497b044b3e5b9` |
| Intel macOS | 14915617013 `2e80548020cf36abc5a2e8455a8bd2fbac32c35db4e1d60e0fcdc07861309135` | 14916364057 `57522ae0988af396d7216bc95a18e60465034ca692199caf034e7d2a4e64d2a4` |
| Apple silicon | 14916436764 `149335a1898eb67872c0d14db4a19303167d4b5d609df025aebab7de3175c01c` | 14913009088 `722fa787e508dcbe279adf0c191d6948944957dee52aeba1e5e30533e8960ab6` |

The isolated native Windows build at `42b8a6abbf4ac2a170df89befc513b0b9d865225` opened the pinned original SIM at cycle 281. Ctrl+R opened Robot Info for `16:0` and displayed its literal founder filename. Descendants returned three. Open Console opened that same robot and `showdna` switched Robot Info to the native DNA text with its marker control. Reproduce reported `OFFSPRING BORN` with the cycle still at 281. The tested desktop DLL SHA-256 was `9993789d22c1b9f2bc6ff80dbace21a6f8d1bb1a322a2a3785912dfd3f8d4a8f` and the Rust engine DLL SHA-256 was `c359484e391a48e0e75a772a93967f2d2050294272233866a2192a2f5e0f2a5f`. Thirty-nine focused desktop tests passed on that build in a disposable profile.

The combined Windows desktop build at `5d16beae46a581a70a5bd5843c2b3c8bfd01a677` passed 430 rendered tests with one optional skip. Its native open and save dialogs opened from F2 and F3. The F3 dialog offered `simulation.sim` and selected the legacy format. Canceling each picker retained cycle 281 and 196 organisms. This walkthrough canceled both pickers. The separate 16-case rendered suite exercised writes and accepted loads, old and current formats, seed capture on cancel, the toolbar seed, and picker pause and resume. The inspected desktop DLL SHA-256 is `91ca7d0e2a9ef659360ae59b2ab8269a0625916fb2cb7f27feb4c81b5a397856`. [Integration](https://github.com/BobertNeek/Darwinbots-Modern/pull/276)

The Windows scene build at `b9ec9e9f6983322e78bcc226f6aa6bf59d9939c3` passed 69 focused rendered tests for original scene colors, background images, graph reset, global profiles and setup appearance. The live Windows Robot Info window opened on the copied native SIM. The computer-use accessibility service did not expose its controls in this launch. The desktop DLL SHA-256 is `52fe3b3777c5926fa03d7fdbab432006ffd48b149fa181ac1dbb4934439de931` and the engine DLL SHA-256 is `ad86175f18c134decb937153def889627e66522b172f3c13c544e879ada2d880`.

## November 4 offline assessment

Target: functional Darwinbots 2.48.32 offline behavior on native Windows, with Windows/Linux/Intel macOS/Apple silicon CI. Internet Mode is removed from menus, toolbar, setup, advanced/global settings and teleporter choices. Local and file teleporters remain supported. Historical VB6/helper sources remain reference material.

Baseline: `9dc4c91`, plus documentation corrections from `181821d`. The current work also fixes a real route gap: **Edit → Species previously opened mutation settings**. The repaired editor applies an atomic startup/repopulation roster without replacing living robots, preserves stable IDs, supports cancellation and restart with the edited roster, and retains source DNA, comments and placement bounds. Removed species retain their living identities and export source while repopulation is disabled. Their retirement survives modern save/resume.

Other current repairs: Internet-marked legacy gateways cannot become active local gateways even when an old local flag remains set; hidden Internet metadata remains readable and round-trippable; event diagnostics scroll and wrap at normal window sizes; corrupted status text and stale six-tab expectations are corrected. Zero-copy startup species stay at zero. Ctrl+Enter creates a world even when Load Settings has focus, rather than reopening the picker. Surviving menu command identifiers keep their numeric values; retired values 15, 67 and 68 are not reused. No ABI or snapshot version is changed; the JSON command protocol gains `update_species_settings`, and configuration entries expose their world-local `species_id`. Optional template metadata uses backward-compatible defaults when reading older modern saves.

### Fresh feature checklist

The checklist maps the original source at `467a8a0c1564db2071cf65efe39dcfefc5ab7a0c` to production paths and executable checks. The retained VB6 source matches that pin, and the six canonical reference documents pass their hash verifier. A passing domain means its covered scenarios passed; it does not imply universal historical-file or arbitrary-program equivalence. Required native Windows acceptance is still incomplete. The [current structured receipt](../../docs/parity/evidence/offline-feature-completion-20261104/qualification.json) records passing checks and the remaining work separately.

| Feature contract | Production boundary | Behavioral checks required for closure |
| --- | --- | --- |
| Cycle/state timing | Canonical scheduler and slot-ordered native robot pipeline | `canonical_cycle_248`, `core_state_contracts`, `native_continuation_qualification` |
| DNA, stacks, flow, costs and sysvars | DNA parser/VM and native sysvar lifecycle | `dna_runtime_248`, `sysvar_lifecycle_248`, `vm`, shipped DNA corpus probes |
| Physics, topology and nine-eye/touch/taste senses | Native physics, sensing, collisions and vision | `db2_physics`, `native_reference_senses_248`, `touch_taste_248`, `toroidal_world` |
| Shots, feeding, defenses, viruses and sperm | Native projectile and toxin pipelines | `db2_projectiles`, `shot_state`, native genuine-feeding controls, virus/sexual cases |
| Ties, multibots and resource/information sharing | Native tie creation, physics, sharing and readback | `ties_multibot`, ordered cycle fixtures, native organism interchange |
| Reproduction and genealogy | Native birth/heredity and lineage bookkeeping | `reproduction_heredity_248`, `robot_state_248`, rendered phylogeny/details tools |
| Mutation and speciation | Standard/advanced mutation, lifelong scheduling and native speciation | `db2_mutation_248`, `mutation_advanced_248`, mutation-enabled save/continuation |
| Ecology, corpses, waste and energy | Native biology, feeding, vegetation and corpse decay | `db2_vegetation`, `corpses_collisions`, genuine chloroplast/body/feeding fixtures, ecological soaks |
| World objects and local exchange | Obstacles/mazes, local relocation and transactional file gateways | `environment_world`, `legacy_transport`, native file-transfer tests and rendered teleporter workflows |
| Modern and original file boundaries | DB3S migrations, native `.sim`/`.dbo`, `.set`/`.gset`, DNA export | `final_state_persistence_248`, `LegacySettingsNativeTests`, native robot/organism round trips |
| Species working copy and policy | Dedicated species mode, stable identities and atomic roster command | Species-menu apply/cancel/reopen test, `SpeciesSettingsNativeTests`, native atomicity/save-resume tests |
| Leagues, contests and evolution | Offline mode authority, classic tournament orchestration and staged evolution | `local_modes`, `zerobot_progression`, community league and rendered mode workflows |
| Graphs, recording and recovery | Native graph metrics/history, snapshots, safe-mode and autosave controllers | `history_records`, rendered graph/export and recovery tests, native Windows walkthrough |
| Robot tools and display controls | Live details/console/activations/phylogeny, PlayerBot, eye/RGB/color tools | Dedicated rendered/causal tool tests and display-only state invariants |
| Offline interface and accessibility | Menu catalog/router, settings, viewport, keyboard and tray | Internet-absence tests, 139-ID inventory, layout checks and native Windows tray checks |
| CPU/GPU execution and continuity | Physical backend, explicit failure/fallback policy and complete state saves | Six strict physical GPU groups, including 10,000-cycle native-world CPU/GPU/save-resume comparison |
| Distribution | Published application folder and bundled robot assets | Four-platform tests/publish/smoke and local Windows package smoke, including species editing |

### Current qualification

Automated qualification passed. On November 20, 2026, the maintainer directed the merge to `master` and a cloud handoff with the remaining native Windows checks still open. Full feature acceptance remains pending.

| Boundary | Qualified revision and result |
| --- | --- |
| Local Rust workspace | `0f60376`: 1,071 passed across 68 targets; 12 physical-only cases ignored here and subsequently executed by the hardware runner. Engine/CLI source is unchanged through `da3581e`. |
| Local Desktop.Core | `0f60376`: 328 passed; Core source is unchanged through `da3581e`. |
| Local rendered desktop | `d9f9339`: 236 passed, one optional viewport benchmark skipped; the 139-ID inventory passed. Product/test source is unchanged at `da3581e`. |
| Physical GPU | `d9f9339`: all six guarded groups passed on RTX 3050 / Vulkan / DiscreteGpu, driver 610.88. The four hardware-only groups passed 7 + 1 + 3 + 1 cases; broader GPU and switch groups passed 14 and 1 cases. These groups overlap; do not add them as unique cases. |
| Original-world long continuation | 10,000 additional cycles, ten matching CPU/GPU gameplay and byte-identical continuous/resumed GPU save checkpoints. Final cycle 10,281: 2,161 robots, 869 shots and 8,999 births. The test took 4,513.66 seconds and retained a physical GPU after every reload. |
| Four-platform CI | `da3581e`, [run 41835933113](https://github.com/BobertNeek/Darwinbots-Modern/actions/runs/41835933113): Windows, Linux, Intel macOS and Apple silicon each passed 1,071 Rust, 328 Core and 236 Desktop cases, with one optional benchmark skipped and 12 hardware-only cases ignored on hosted machines. All four publish and explicit process-wait JSON smoke gates passed. |
| Packages | All four existing CI package artifacts have recorded SHA-256 digests. The downloaded Windows ZIP matches GitHub's digest; its local smoke passed and all 48 DNA files match source. Native observations use this CI package, not a substituted local DLL. |
| Offline routes/layout | All 75 surviving menu actions have production handlers. Internet-absence checks cover menus, setup, advanced/global settings, toolbar and teleporter choices. Latest rendered layouts were inspected; route inventory alone is not behavioral closure. |

The native CI package showed five offline tabs and corrected onboarding. Default Auto creation produced 400 robots and reported CPU; a native Step advanced the cycle display and added 3,600 plant energy. Loading the earlier native `.set` restored the 3+3 CPU setup, and Ctrl+Enter from the focused Load Settings button created the six-robot world without reopening the picker. Ctrl+X opened the dedicated species roster, where zero initial count, starting energy 777 and a rename could be staged. Apply/cancel/reopen and preservation of living robots have automated evidence but are not yet observed in this native walkthrough.

The native walkthrough stopped after the window helper could not recover a minimized window and the user ended Computer Use. Remaining native checks are live species apply/cancel/reopen, live settings, world and organism save/load and robot tools, leagues/evolution/restart, graphs/export, recording/recovery, and tray hide/restore. Tests used an isolated profile. On November 20, all seven original profile files matched their backup hashes and there were no additional files.

The [Windows acceptance walkthrough](windows-offline-acceptance.md) gives the isolated test profile, steps and observable pass conditions for the open scenarios.

Earlier failures remain dated failures: `7a3db6b` stale desktop expectations; `f19c6ce` test-helper compile error; local `0f60376`/`4c646d4` cost-dialog timeouts; and two `518007a` organism-route timeouts. Repairs preserve the assertions and use bounded CPU owners for UI fixtures. Current native Auto creation passed separately. Full logs and private profiles remain local; hashes, package identity and observation limits are in the structured receipt.

### Scope and evidence limits

- Imported DB2 local teleporters choose a random field destination for each transfer. Modern DB3S saves preserve the selected policy. Older DB3S saves use their stored fixed destination. The native `.sim` record stores gateway geometry and mode without destination coordinates, so an exported fixed local route uses random destinations when reopened as `.sim`. The port uses its simulation RNG and current group relocation rules.

The original source and pinned genuine executable captures define behavior. Current tests can recheck retained original oracles without claiming new original-GUI observations. Identical arbitrary random trajectories, every historical file/version, public hosting and a public release are outside this target. Native Linux/macOS tray and window-manager interaction is outside the selected acceptance boundary. Raw user saves, profile backups and private desktop diagnostics remain local.

---

## Historical October 2026 qualification

Last reconciled: 2026-10-25, against completion checkpoint `0225101343367cbe297d4d23afda598cfa9f679a` and its saved qualification. This update changes documentation only; it is not a new engine, GPU or desktop qualification.

The agreed functional offline work was completed on October 21. No known required code repair remains in that scope. The [completion report](../../docs/parity/db24832-completion-report-2026-10-21.html) and [structured receipt](../../output/native-ui65-20261021/completion.json) separate actual native observations, automated coverage and remaining limits. The July Phase 0 ledger below is retained as history, not the current backlog.

## Qualified revisions

| Revision | What its evidence establishes |
| --- | --- |
| `73e350297f54e88a7a690c435517ebb13615f953` | Qualified engine, CLI and Desktop.Core source: CPU65, strict physical GPU65 and sustained GPU capacity. Those source trees are unchanged at the completion checkpoint. |
| `0e57519e809e38736ce5090cae84e13c24951b44` | Managed UI repair for clipped controls and local-mode round summaries; critical native Windows walkthrough and four-platform hosted suite. |
| `5f0f7d800adc2439e0dd09bc2bd913f0bca4b494` | Test-only display/turbo race repair. Three-platform diagnostics passed afterward; product source and the native package did not change. |
| `0225101343367cbe297d4d23afda598cfa9f679a` | Final report/evidence checkpoint. It is not a fresh hardware qualification. |

## Completed evidence

| Area | Recorded result | Evidence |
| --- | --- | --- |
| CPU engine and integration | 94 ordinary steps passed; 1,049 Rust cases across 68 targets, 314 Core cases and 215 Desktop cases before the later UI repair. 25 genuine original controls / 55,849 projected fields matched. | [CPU65 qualification](../../docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass65/qualification-summary.json) |
| Physical GPU | All six strict groups passed on RTX 3050 / Vulkan / DiscreteGpu, including the unchanged 10,000-cycle original-world CPU/GPU comparison and repeated save/resume. Groups overlap; do not add them as unique test counts. | [GPU65 receipt](../../output/gpu65-collected.json) |
| Sustained GPU capacity | Fixed populations of 1,000, 4,000, 8,000 and 32,000 passed exact-count, movement and at least 60-second measurement requirements. These are headless controlled probes, not rendered frame rates or ecological throughput. | [Capacity receipt](../../output/capacity65-controlled-gpu-collected.json) |
| Final UI hosted matrix | Windows x64, Linux x64, Intel macOS and ARM macOS each passed 1,064 Rust cases across 68 targets, 314 Core cases, 221 Desktop cases and package publish/smoke steps. The 12 hardware-only Rust cases were ignored on hosted machines. | [Run 39572329105](https://github.com/BobertNeek/Darwinbots-Modern/actions/runs/39572329105), [collected results](../../output/native-ui65-20261021/hosted-initial-collected.json) |
| Repaired diagnostics | Linux, Intel macOS and ARM macOS each passed 221 Desktop cases. The earlier Intel diagnostic race remains recorded as a failure at its original revision. The repair pauses through the real Pause control and retains the strict assertions. | [Run 39579819052](https://github.com/BobertNeek/Darwinbots-Modern/actions/runs/39579819052), [fixture receipt](../../output/native-ui65-20261021/hosted-fixture-collected.json) |
| Native Windows workflows | Settings file save/change/load; global/live settings reopen; small CPU world transport; simulation save/advance/load; SimpleSurvival progression/restart; graphs and CSV export. The isolated official original Restarter also opened the normal original shell and New Simulation dialog. | [Native walkthrough receipt](../../output/native-ui65-20261021/completion.json) |
| Final local package | Focused UI checks passed, CPU movement/save-resume smoke passed and 48 bundled DNA files matched their source bytes. Seven original profile hashes were restored. | [Package and focused-check receipt](../../output/native-ui65-20261021/completion.json) |

The October 25 recovery rechecked the existing package's engine/desktop hashes and all 48 bot assets, confirmed unchanged engine/CLI/Core source, and read the successful hosted run metadata at the exact UI/test commits. It did not rerun the historical suites or launch an application. A green hosted run alone is not the basis for the completion claim.

## Compatibility and observation limits

- DB2 2.48.32 native `.sim` import/export and `.dbo` organism interchange are implemented. This supersedes the old blanket statement that only modern saves and text DNA exist. See [native simulation export](../engine/src/world/native_sim_export.rs), [native organism interchange](../engine/src/world/native_organism_io.rs) and [desktop native integration tests](../desktop/tests/Darwinbots.Desktop.Tests/LegacySettingsNativeTests.cs).
- The integration tests load a pinned genuine original simulation and an original-executable continuation of a port export, step the loaded worlds, and compare controlled modern checkpoint continuation. Native organism evidence includes an original simulation saved after `.dbo` import. This is finite fixture coverage, not a guarantee for every historical save, DB2 version or unknown record layout.
- Modern `DB3S` saves retain their own versioned continuation contract. Native DB2 export and modern save/resume are distinct paths. Identical arbitrary future random trajectories and incidental VB6 bugs are outside the agreed functional scope.
- At the October checkpoint, Internet Mode appeared as disabled controls. The later offline product removed those controls and kept local exchange. The October report distinguishes automated interactions from critical workflows observed natively.
- Native tray hide was observed; tray restore has automated coverage but was not observed natively. Linux/macOS native window-manager walkthroughs and a manual click of every menu action were not performed. No corresponding product failure was established; these remain explicit verification limits.

## Existing Windows package

The original completion worktree contains `artifacts/win-x64-native-ui66-final/Darwinbots.Desktop.exe`. This framework-dependent Windows x64 package uses .NET 10. Keep it with its adjacent DLLs and `Bots` directory; it is generated local output and is not stored in Git.

| File | SHA-256 |
| --- | --- |
| `darwinbots_engine.dll` | `EB13AA54BC6456A30F7F3CD5F8AA56CDC4ECE43CECD27B67066F19E4FFEF6C1E` |
| `Darwinbots.Desktop.dll` | `3555F7BE066C88247AE7316E133A5C21BB5162C374D48CA6BCE0BB4BB366D729` |

The native DLL is the unchanged qualified engine; the managed DLL contains the final UI repair. A different `target/` build must not be silently substituted for this package. Source builds and hosted packages have their own provenance.

## Delivery and additional acceptance

The completed work remains on `codex/db24832-completion`, separate from `master`. As checked on October 25, the only published release is [legacy reference material](https://github.com/BobertNeek/Darwinbots-Modern/releases/tag/legacy-db2-reference-v1), not a modern application distribution. A merge or modern release is a separate delivery decision; the completion evidence does not imply one has happened.

No required code change remains in the recovered offline checklist. Additional native tray-restore, non-Windows desktop interaction or broader historical-file corpus checks would extend the evidence. They have not been run by this documentation update. New physical GPU/graphics qualification on the training machine requires a coordinated interval; preserve the active A-Life run.

<details>
<summary>Historical Phase 0 ledger — July 30, 2026</summary>

The following checkpoint is preserved with its original commands, results and then-open findings. Its references to current/next work apply to that July revision only.

Last updated: 2026-07-30

This ledger records a measured DB2 2.48.32 Phase 0 checkpoint through `95818f6af58bcb2365ec21f6a49e707ae15ed343`. A passing test proves only its stated boundary. Catalog registration and green unit counts are not behavioral parity.

## Current checkpoint gate

| Area | Command or evidence | Result |
|---|---|---|
| Reference package | `pwsh -NoProfile -File docs/reference/db24832/verify.ps1` | 6/6 verified |
| Selected Rust engine | `cargo test -p darwinbots-engine --test contracts --test manual_tools --test live_legacy_menu_settings` | 23 passed; 0 failed |
| Desktop core | `dotnet test modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj --no-restore --nologo` | 62 passed; 0 failed; 0 skipped |
| Rendered desktop | `dotnet test modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj --no-restore --nologo` | 26 passed; 0 failed; 0 skipped |
| Control inventory | `pwsh -NoProfile -File docs/verification/control-surface-audit/run-rendered-audit.ps1` | 109/109 passing rows validated; 26 rendered tests passed |
| Whitespace/error scan | `git diff --check` | Pass; exit 0 with line-ending warnings only |

Run these commands serially from `D:\Darwinbots 2\.worktrees\db24832-parity`. Do not start competing Cargo, GPU, or desktop builds.

## Phase 0 deterministic evidence

- Typed catalog commands route through explicit production handlers.
- Live mutation controls queue authoritative engine mutations for enabled state, global multiplier, and per-species rates.
- Mutation-free save transforms only the serialized copy; the running state is unchanged.
- Permanent-tie whole-organism removal uses stable `slot:generation` capture and an atomic native `remove_many` command.
- Successful remove-many execution produces exactly one publication for the batch and runs off the caller thread.
- Native command parsing accepts `remove_many` JSON.
- End-to-end organism picker JSON contains only captured group members and ties internal to that group.
- Default E-Grid and Survival Info visibility matches the checkpoint contract.
- Retained shortcut strings are applied as exact input gestures and tested through keyboard input.
- The three online commands are disabled and interaction tests show no network work.
- Current view toggles and object tools have bounded rendered interaction coverage.

## Compatibility and simulation limits

- The recorded 2.48.32 executable hash remains a reference artifact, but the original GUI lacks successful black-box execution evidence.
- Modern `DB3S` persistence and plain-text DNA interchange do not provide DB2 simulation, settings, organism, snapshot, graph, or recovery compatibility.
- Current engine tests cover useful modern kernels and selected integrated behavior. They do not close the P0/P1 cycle ordering, DNA/VM, shots, ties, reproduction, senses, physics, ecology, mutation, or persistence findings.
- CPU/GPU differential coverage remains incomplete across interaction-bearing phases and long ecological scenarios.
- Online networking is excluded, not complete.

## Rendered evidence

The audit writes ignored evidence under `docs/verification/control-surface-audit/modern/`. See `docs/parity/historic-menu-parity-report.md` for the inspected runtime screenshot paths, exact dimensions, and visible findings.

## Next acceptance work

Use `docs/parity/db24832-gap-register.md` as the prioritized non-online backlog. It preserves generic tool substitutes, collapsed settings, shell and viewport differences, engine/runtime gaps, persistence/offline gaps, GPU differentials, and missing original-GUI evidence as open or partial.

</details>
