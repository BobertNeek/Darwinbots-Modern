# Organism Visual Phenotypes Implementation Plan

> **Status: Archived after implementation.** The unchecked boxes preserve the original execution plan; they do not indicate current missing product work. Use `modern/docs/verification.md` for current evidence and open gates.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore DB2-style inherited skins, mutation-driven lineage colors, body/chloroplast sizing, and selected-bot nine-eye visualization in the modern engine and Avalonia viewport.

**Architecture:** Add simulation-owned visual phenotype and vision value objects in focused Rust modules, store phenotype on each organism, and publish immutable render/snapshot data through the existing JSON C ABI. Keep GPU shaders unchanged: GPU sensing continues to return slot, position, radius, and temporary color, then `Engine::publish_snapshot` attaches aim, skin, lineage color, and autotroph state before publication. Add pure C# viewport geometry helpers so rotation and eye-sector calculations are testable without constructing Avalonia controls.

**Tech Stack:** Rust 2024, serde/serde_json, Rayon, wgpu, .NET 10, C# 13, Avalonia 11, xUnit.

## Global Constraints

- Windows 10 is the first supported alpha target.
- VB6 Darwinbots 2 is the behavioral authority where the modern engine and DarwinbotsC disagree.
- Bot radius uses DB2 body points and chloroplasts; energy does not directly determine radius.
- Plain-text DNA import/export and existing sysvar numbers remain compatible.
- Color changes only after real DNA changes; autotroph colors remain green.
- Skin changes only at automatic speciation and is inherited by descendants.
- The full nine-eye overlay is rendered only for the selected organism.
- The viewport may omit skins and heading marks at dense-population overview zoom.
- Save compatibility is additive within DB3S version 2 by using serde defaults and restore-time repair.
- Do not move new simulation authority into the Avalonia renderer.
- Do not alter WGSL layouts for this feature.

---

## File Structure

- Create `modern/engine/src/visual_phenotype.rs`: phenotype data, stable skin generation, RGB mutation drift, green-band enforcement, and bounded speciation variation.
- Create `modern/engine/src/vision.rs`: DB2 eye addresses, eye configuration snapshots, angular geometry, sight range, and target-sector matching.
- Create `modern/engine/tests/visual_phenotypes.rs`: focused engine inheritance, mutation, speciation, and persistence tests.
- Create `modern/engine/tests/db2_vision.rs`: movable/variable-width eye behavior and snapshot tests.
- Modify `modern/engine/src/lib.rs`: export phenotype and vision value objects.
- Modify `modern/engine/src/species.rs`: add initial skin and lineage defaults to species definitions.
- Modify `modern/engine/src/config.rs`: add live automatic-speciation settings.
- Modify `modern/engine/src/world.rs`: own phenotype state, apply mutation/speciation, publish vision state, and enrich render instances.
- Modify `modern/engine/src/physics.rs`: extend host-side `RenderInstance` without changing GPU structs or WGSL.
- Modify `modern/engine/src/ffi.rs`: accept automatic-speciation settings through create/update commands.
- Modify `modern/engine/tests/contracts.rs`: prove additive save restoration and DB2 radius behavior.
- Modify `modern/engine/tests/gpu_ffi.rs`: prove phenotype fields survive the native JSON ABI on CPU and GPU-capable paths.
- Create `modern/desktop/src/Darwinbots.Desktop.Core/OrganismVisualGeometry.cs`: pure skin and eye geometry calculations.
- Create `modern/desktop/tests/Darwinbots.Desktop.Core.Tests/OrganismVisualGeometryTests.cs`: pure geometry tests.
- Modify `modern/desktop/src/Darwinbots.Desktop.Core/EngineSnapshot.cs`: C# phenotype, eye, and enriched render records.
- Modify `modern/desktop/src/Darwinbots.Desktop.Core/NativeSnapshotParser.cs`: defensive parsing and old-snapshot defaults.
- Modify `modern/desktop/tests/Darwinbots.Desktop.Core.Tests/NativeSnapshotParserTests.cs`: ABI parsing and fallback tests.
- Modify `modern/desktop/src/Darwinbots.Desktop.Core/EnvironmentUpdate.cs`: automatic-speciation command values.
- Modify `modern/desktop/src/Darwinbots.Desktop.Core/WorldSetupOptions.cs`: carry settings from setup to engine.
- Modify `modern/desktop/src/Darwinbots.Desktop.Core/NativeEngineClient.cs`: include settings in engine creation.
- Modify `modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Db2EnvironmentSettingsTests.cs`: command/default tests.
- Modify `modern/desktop/src/Darwinbots.Desktop/Views/AdvancedSettingsWindow.axaml`: advanced automatic-speciation controls.
- Modify `modern/desktop/src/Darwinbots.Desktop/Views/AdvancedSettingsWindow.axaml.cs`: load and return settings.
- Modify `modern/desktop/src/Darwinbots.Desktop/Controls/WorldViewport.cs`: DB2-style skin, heading mark, dense-population LOD, and selected-eye overlay.
- Create `docs/visual-blueprints/organism-visual-phenotypes.png`: image-generation blueprint used for visual comparison.

---

### Task 1: Visual Phenotype Value Object

**Files:**
- Create: `modern/engine/src/visual_phenotype.rs`
- Create: `modern/engine/tests/visual_phenotypes.rs`
- Modify: `modern/engine/src/lib.rs`
- Modify: `modern/engine/src/species.rs`

**Interfaces:**
- Consumes: packed colors in `0xAARRGGBB` form and the organism's seeded `u64` random state.
- Produces: `SkinPoint`, `VisualPhenotype`, `generated_skin`, `apply_color_mutation`, and `apply_speciation`.

- [ ] **Step 1: Write failing phenotype unit tests**

Add tests that express stable generation, inherited values, local mutation, green autotroph clamping, and bounded skin variation:

```rust
use darwinbots_engine::{SkinPoint, VisualPhenotype, generated_skin};

#[test]
fn generated_skin_is_stable_and_inside_the_bot() {
    let first = generated_skin("Animal Minimalis", 7);
    let second = generated_skin("Animal Minimalis", 7);
    assert_eq!(first, second);
    assert!(first.iter().all(|point| (0.15..=0.82).contains(&point.radius)));
}

#[test]
fn real_mutation_drifts_color_but_zero_changes_do_not() {
    let mut phenotype = VisualPhenotype::new(3, 0xff239ac0, generated_skin("Animal", 3));
    let original = phenotype.clone();
    let mut random = 91;
    phenotype.apply_color_mutation(0, false, &mut random);
    assert_eq!(phenotype, original);
    phenotype.apply_color_mutation(1, false, &mut random);
    assert_ne!(phenotype.color, original.color);
}

#[test]
fn autotroph_mutations_remain_green() {
    let mut phenotype = VisualPhenotype::new(4, 0xff4c963b, generated_skin("Alga", 4));
    let mut random = 44;
    for _ in 0..200 { phenotype.apply_color_mutation(1, true, &mut random); }
    let [red, green, blue] = phenotype.rgb();
    assert!(green >= red && green >= blue);
}

#[test]
fn speciation_changes_one_skin_point_and_resets_distance() {
    let mut phenotype = VisualPhenotype::new(9, 0xff239ac0, generated_skin("Animal", 9));
    phenotype.accumulated_mutations = 12;
    let before = phenotype.skin;
    let mut random = 17;
    phenotype.apply_speciation(10, &mut random);
    assert_eq!(phenotype.lineage_id, 10);
    assert_eq!(phenotype.accumulated_mutations, 0);
    assert_eq!(before.iter().zip(phenotype.skin).filter(|(a, b)| a != &b).count(), 1);
}
```

- [ ] **Step 2: Run the focused test and confirm the missing API failure**

Run:

```powershell
cargo test --manifest-path modern/engine/Cargo.toml --test visual_phenotypes
```

Expected: compilation fails because `SkinPoint`, `VisualPhenotype`, and `generated_skin` do not exist.

- [ ] **Step 3: Implement the phenotype module**

Create serializable value objects with these public shapes:

```rust
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SkinPoint {
    pub radius: f32,
    pub angle: i32,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VisualPhenotype {
    pub lineage_id: u64,
    pub color: u32,
    pub skin: [SkinPoint; 4],
    pub accumulated_mutations: u32,
}
```

Use a stable FNV-1a hash of species name plus seed for `generated_skin`. Generate four radius fractions in `0.15..=0.82` and DB2 aim-unit angles in `0..1257`. `apply_color_mutation` adjusts one RGB channel by `20` per reported change. For autotrophs, convert to HSV and clamp hue to `85..155` degrees, saturation to `0.45..0.90`, and value to `0.40..0.85`. `apply_speciation` varies exactly one point by at most `0.12` radius or `63` aim units and clamps it back inside the allowed bounds.

Add `#[serde(default = "default_skin")] pub skin: [SkinPoint; 4]` and `#[serde(default)] pub lineage_id: u64` to `SpeciesDefinition`. Keep `SpeciesDefinition::default().color` unchanged.

- [ ] **Step 4: Run the focused tests**

Run the Task 1 command again.

Expected: all `visual_phenotypes` tests pass.

- [ ] **Step 5: Commit the phenotype primitives**

```powershell
git add modern/engine/src/visual_phenotype.rs modern/engine/src/lib.rs modern/engine/src/species.rs modern/engine/tests/visual_phenotypes.rs
git commit -m "Add inherited visual phenotype primitives"
```

---

### Task 2: Organism Inheritance, Mutation, Speciation, and Saves

**Files:**
- Modify: `modern/engine/src/config.rs`
- Modify: `modern/engine/src/world.rs`
- Modify: `modern/engine/src/ffi.rs`
- Modify: `modern/engine/src/persistence.rs`
- Modify: `modern/engine/tests/visual_phenotypes.rs`
- Modify: `modern/engine/tests/contracts.rs`

**Interfaces:**
- Consumes: `VisualPhenotype` from Task 1 and `MutationReport.changes` from `GenomeMutator`.
- Produces: organism-owned phenotype, `auto_speciation` configuration, and additive DB3S restoration.

- [ ] **Step 1: Add failing integration tests**

Add tests with these assertions:

```rust
fn phenotype_engine(auto_speciation: bool, threshold: f32) -> Engine {
    Engine::new(EngineConfig {
        metabolism_cost: 0,
        auto_speciation,
        speciation_genetic_distance_percent: threshold,
        ..EngineConfig::testing()
    }).unwrap()
}

fn spawn_mutating_parent(engine: &mut Engine, vegetable: bool) -> OrganismId {
    let species = engine.register_species(SpeciesDefinition {
        name: if vegetable { "Visual Alga" } else { "Visual Animal" }.to_owned(),
        vegetable,
        color: if vegetable { 0xff4c963b } else { 0xff239ac0 },
        mutation_rate: 100.0,
        ..SpeciesDefinition::default()
    });
    engine.spawn_species_batch(
        LegacyDna::parse("start\n10 .up store\n50 .repro store\nstop").unwrap(),
        species,
        [[500.0, 500.0]],
        1_000,
    ).unwrap()[0]
}

#[test]
fn child_inherits_phenotype_then_mutation_drifts_only_the_child() {
    let mut engine = phenotype_engine(false, 25.0);
    let parent = spawn_mutating_parent(&mut engine, false);
    let parent_before = engine.organism(parent).unwrap().phenotype.clone();
    engine.tick().unwrap();
    let child = engine.snapshot().organisms.iter().find(|bot| bot.id != parent).unwrap();
    assert_eq!(child.phenotype.skin, parent_before.skin);
    assert_eq!(child.phenotype.lineage_id, parent_before.lineage_id);
    assert_ne!(child.phenotype.color, parent_before.color);
}

#[test]
fn crossing_speciation_threshold_changes_skin_once() {
    let mut engine = phenotype_engine(true, 1.0);
    spawn_mutating_parent(&mut engine, false);
    engine.tick().unwrap();
    let child = engine.snapshot().organisms.iter().max_by_key(|bot| bot.id.slot()).unwrap();
    assert_ne!(child.phenotype.lineage_id, 0);
    assert_eq!(child.phenotype.accumulated_mutations, 0);
}

#[test]
fn phenotype_round_trips_through_version_two_save() {
    let mut engine = phenotype_engine(true, 1.0);
    spawn_mutating_parent(&mut engine, true);
    engine.tick().unwrap();
    let restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
    assert_eq!(restored.snapshot().organisms[0].phenotype, engine.snapshot().organisms[0].phenotype);
}
```

- [ ] **Step 2: Run focused tests and confirm missing integration**

```powershell
cargo test --manifest-path modern/engine/Cargo.toml --test visual_phenotypes --test contracts
```

Expected: compilation fails because organism snapshots do not expose phenotype and config lacks automatic-speciation fields.

- [ ] **Step 3: Add engine configuration and organism state**

Add these fields to `EngineConfig`:

```rust
#[serde(default)]
pub auto_speciation: bool,
#[serde(default = "default_speciation_genetic_distance_percent")]
pub speciation_genetic_distance_percent: f32,
```

Use `20.0` as the configurable threshold and keep `auto_speciation` disabled by default to match DB2's opt-in AutoFork behavior.

Add `phenotype: VisualPhenotype` to `Organism`, `phenotype: VisualPhenotype` to `OrganismSnapshot`, and `next_lineage_id: u64` to `Engine`. Initialize species lineage/skin in `register_species`; initialize organism phenotype from species in `spawn_species_at_unpublished`; copy the complete parent phenotype after child spawn in reproduction and clone operations.

- [ ] **Step 4: Connect mutation and speciation**

In `mutation_phase`, immediately after `GenomeMutator::mutate`:

```rust
if report.changes > 0 {
    let vegetable = self.species
        .get(organism.species.0 as usize)
        .is_some_and(|species| species.vegetable);
    organism.phenotype.apply_color_mutation(report.changes, vegetable, &mut organism.random_state);
    organism.phenotype.accumulated_mutations = organism.phenotype.accumulated_mutations
        .saturating_add(report.changes);
    let threshold = ((organism.dna.instructions().len().max(1) as f32)
        * self.config.speciation_genetic_distance_percent.clamp(0.1, 10_000.0)
        / 100.0).ceil() as u32;
    if self.config.auto_speciation && organism.phenotype.accumulated_mutations >= threshold {
        let lineage = self.next_lineage_id;
        self.next_lineage_id = self.next_lineage_id.saturating_add(1);
        organism.phenotype.apply_speciation(lineage, &mut organism.random_state);
    }
}
```

Resolve the mutable-borrow boundary by copying `auto_speciation`, threshold percent, and `next_lineage_id` before borrowing the organism, then writing the incremented lineage counter after the borrow ends.

- [ ] **Step 5: Add old-save repair without changing DB3S version**

Use serde defaults for new fields. In `Engine::restore`, repair an uninitialized phenotype (`lineage_id == 0`) from its species definition and advance `next_lineage_id` above every restored organism/species lineage. Keep `SaveFile::VERSION` at `2` because the payload remains additive JSON under the existing binary envelope.

- [ ] **Step 6: Accept live configuration through the ABI**

Add optional `auto_speciation: Option<bool>` and `speciation_genetic_distance_percent: Option<f32>` values to `EngineCommand::UpdateEnvironment`. Apply present values and leave omitted values unchanged so old desktop clients remain valid.

- [ ] **Step 7: Run focused integration tests**

Run the Task 2 command again.

Expected: phenotype inheritance, speciation, old defaults, and save round trips pass.

- [ ] **Step 8: Commit engine integration**

```powershell
git add modern/engine/src/config.rs modern/engine/src/world.rs modern/engine/src/ffi.rs modern/engine/src/persistence.rs modern/engine/tests/visual_phenotypes.rs modern/engine/tests/contracts.rs
git commit -m "Integrate lineage phenotypes with mutation and saves"
```

---

### Task 3: DB2 Eye Geometry and Sensing

**Files:**
- Create: `modern/engine/src/vision.rs`
- Create: `modern/engine/tests/db2_vision.rs`
- Modify: `modern/engine/src/lib.rs`
- Modify: `modern/engine/src/world.rs`

**Interfaces:**
- Consumes: VM addresses `501..509`, `511`, `521..529`, and `531..539`; aim in DB2 units where `200` units equal one radian.
- Produces: `EyeSnapshot`, `VisionSnapshot`, DB2 eye range calculations, and actual sensing that matches the published overlay.

- [ ] **Step 1: Write failing movable-eye tests**

```rust
#[test]
fn eye_direction_moves_detection_between_sectors() {
    let mut engine = vision_engine();
    let observer = engine.spawn_at(
        LegacyDna::parse("start\n314 .eye5dir store\nstop").unwrap(),
        [500.0, 500.0],
    ).unwrap();
    engine.spawn_at(LegacyDna::parse("start\nstop").unwrap(), [850.0, 500.0]).unwrap();
    engine.tick().unwrap();
    assert!(engine.memory_at(observer, 505).unwrap() > 0);
    assert_eq!(engine.snapshot().organisms[0].vision.eyes[4].direction, 314);
}

#[test]
fn selected_eye_snapshot_contains_width_range_and_focus() {
    let mut engine = vision_engine();
    let observer = engine.spawn_at(
        LegacyDna::parse("start\n70 .eye5width store\n0 .focuseye store\nstop").unwrap(),
        [500.0, 500.0],
    ).unwrap();
    engine.tick().unwrap();
    let bot = engine.snapshot().organisms.iter().find(|bot| bot.id == observer).unwrap();
    assert_eq!(bot.vision.focus_eye, 4);
    assert!(bot.vision.eyes[4].half_width_radians > 0.0);
    assert!(bot.vision.eyes[4].range > 0.0);
}
```

- [ ] **Step 2: Run the focused eye tests**

```powershell
cargo test --manifest-path modern/engine/Cargo.toml --test db2_vision
```

Expected: compilation fails because vision snapshots do not exist.

- [ ] **Step 3: Implement `vision.rs`**

Define:

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EyeSnapshot {
    pub direction: i32,
    pub width: i32,
    pub center_radians: f32,
    pub half_width_radians: f32,
    pub range: f32,
    pub value: i32,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VisionSnapshot {
    pub focus_eye: u8,
    pub eyes: [EyeSnapshot; 9],
}
```

Implement DB2 defaults: nine adjacent `PI/18` sectors centered around aim, direction values as offsets, width `0` representing the standard width, and focus index `abs(focuseye + 4) % 9`. Port `AbsoluteEyeWidth` and `EyeSightDistance` from VB6 `Quads.bas`; clamp non-finite results.

- [ ] **Step 4: Make sensing use the same geometry**

Replace single `eye_sector(...) -> usize` assignment with `matching_eyes(...) -> [bool; 9]`. For each target, test angular inclusion and width-dependent range against every eye, then retain the strongest target independently per eye. Populate refvars from the configured focused eye; the default focus remains eye 5/index 4, preserving current Animal Minimalis behavior.

Publish `VisionSnapshot::from_memory(&organism.memory, biology.aim)` in `snapshot_organism`.

- [ ] **Step 5: Run DB2 vision and starter ecology tests**

```powershell
cargo test --manifest-path modern/engine/Cargo.toml --test db2_vision --test species_ecology --test db2_starter_ecology
```

Expected: all tests pass, including Animal Minimalis target recognition.

- [ ] **Step 6: Commit vision behavior**

```powershell
git add modern/engine/src/vision.rs modern/engine/src/lib.rs modern/engine/src/world.rs modern/engine/tests/db2_vision.rs
git commit -m "Implement DB2 movable eye geometry"
```

---

### Task 4: Render Snapshot and Native ABI

**Files:**
- Modify: `modern/engine/src/physics.rs`
- Modify: `modern/engine/src/world.rs`
- Modify: `modern/engine/tests/gpu_ffi.rs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/EngineSnapshot.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/NativeSnapshotParser.cs`
- Modify: `modern/desktop/tests/Darwinbots.Desktop.Core.Tests/NativeSnapshotParserTests.cs`

**Interfaces:**
- Consumes: organism phenotype and vision from Tasks 2 and 3.
- Produces: enriched immutable render instances and defensive C# records.

- [ ] **Step 1: Write failing Rust ABI assertions**

Extend the FFI snapshot test to assert:

```rust
assert!(json["render_instances"][0]["aim"].is_number());
assert_eq!(json["render_instances"][0]["skin"].as_array().unwrap().len(), 4);
assert!(json["render_instances"][0]["lineage_id"].is_number());
assert_eq!(json["organisms"][0]["vision"]["eyes"].as_array().unwrap().len(), 9);
```

- [ ] **Step 2: Write failing C# parser assertions**

Add JSON containing phenotype, four skin points, and nine eyes, then assert:

```csharp
Assert.Equal(77UL, snapshot.RenderInstances[0].LineageId);
Assert.Equal(4, snapshot.RenderInstances[0].Skin.Length);
Assert.Equal(314, snapshot.RenderInstances[0].Aim);
Assert.Equal(9, snapshot.Organisms[0].Vision.Eyes.Length);
Assert.Equal(4, snapshot.Organisms[0].Vision.FocusEye);
```

Add a second test with the old four-field render instance and assert generated/default skin, aim `0`, lineage `0`, and nine default eyes.

- [ ] **Step 3: Run focused ABI/parser tests**

```powershell
cargo test --manifest-path modern/engine/Cargo.toml --test gpu_ffi
dotnet test modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj --filter NativeSnapshotParserTests
```

Expected: both suites fail on missing fields.

- [ ] **Step 4: Enrich host render instances**

Extend `RenderInstance` with serde-defaulted fields:

```rust
pub generation: u32,
pub aim: i32,
pub skin: [SkinPoint; 4],
pub lineage_id: u64,
pub vegetable: bool,
```

Initialize these fields to defaults in GPU readback construction. In `publish_snapshot`, overwrite every retained instance from the authoritative slot organism, biology, and species. Keep `GpuSenseOutput` and WGSL unchanged.

- [ ] **Step 5: Extend C# records and parser**

Add `SkinPointSnapshot`, `EyeSnapshot`, `VisionSnapshot`, and `VisualPhenotypeSnapshot` records. Extend `RenderInstanceSnapshot` with generation, aim, skin, lineage, and vegetable values using optional/default constructor parameters so existing C# test setup continues compiling. Parse nullable native arrays defensively and normalize to exactly four skin points and nine eyes.

- [ ] **Step 6: Run focused ABI/parser tests again**

Run the Task 4 commands.

Expected: both focused suites pass.

- [ ] **Step 7: Commit snapshot contracts**

```powershell
git add modern/engine/src/physics.rs modern/engine/src/world.rs modern/engine/tests/gpu_ffi.rs modern/desktop/src/Darwinbots.Desktop.Core/EngineSnapshot.cs modern/desktop/src/Darwinbots.Desktop.Core/NativeSnapshotParser.cs modern/desktop/tests/Darwinbots.Desktop.Core.Tests/NativeSnapshotParserTests.cs
git commit -m "Publish phenotype and vision snapshot data"
```

---

### Task 5: Live Automatic-Speciation Settings

**Files:**
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/EnvironmentUpdate.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/WorldSetupOptions.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop.Core/NativeEngineClient.cs`
- Modify: `modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Db2EnvironmentSettingsTests.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Views/AdvancedSettingsWindow.axaml`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Views/AdvancedSettingsWindow.axaml.cs`

**Interfaces:**
- Consumes: `EngineConfig.auto_speciation` and `speciation_genetic_distance_percent` from Task 2.
- Produces: advanced settings that apply while the simulation is running.

- [ ] **Step 1: Write failing command serialization tests**

```csharp
[Fact]
public void AutomaticSpeciationSettingsSerializeIntoEnvironmentCommand()
{
    var update = EnvironmentUpdate.Default with
    {
        AutoSpeciation = true,
        SpeciationGeneticDistancePercent = 12.5f,
    };
    var json = NativeCommandSerializer.SerializeEnvironment(update);
    Assert.Contains("\"auto_speciation\":true", json);
    Assert.Contains("\"speciation_genetic_distance_percent\":12.5", json);
}
```

- [ ] **Step 2: Run the focused settings tests**

```powershell
dotnet test modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj --filter Db2EnvironmentSettingsTests
```

Expected: compilation fails because `EnvironmentUpdate` lacks the new properties.

- [ ] **Step 3: Extend settings and native commands**

Add positional values `bool AutoSpeciation` and `float SpeciationGeneticDistancePercent` to `EnvironmentUpdate`, defaulting to `false` and `20f`. Serialize both snake-case fields in the update command and include them in `NativeEngineClient` creation JSON. Carry them through `WorldSetupOptions.ToEnvironmentUpdate()`.

- [ ] **Step 4: Add advanced UI controls**

Under a new `EVOLUTION` heading in `AdvancedSettingsWindow.axaml`, add:

```xml
<CheckBox x:Name="AutoSpeciation" Content="Automatic speciation"/>
<Grid ColumnDefinitions="*,150">
  <TextBlock Text="Mutation distance (% DNA length)"/>
  <NumericUpDown Grid.Column="1" x:Name="SpeciationDistance" Minimum="0.1" Maximum="10000" Increment="0.5"/>
</Grid>
```

Load values in the constructor and return them in `Update`. Do not add these controls to the main simulation surface.

- [ ] **Step 5: Run focused settings tests**

Run the Task 5 command again.

Expected: all settings tests pass.

- [ ] **Step 6: Commit live settings**

```powershell
git add modern/desktop/src/Darwinbots.Desktop.Core/EnvironmentUpdate.cs modern/desktop/src/Darwinbots.Desktop.Core/WorldSetupOptions.cs modern/desktop/src/Darwinbots.Desktop.Core/NativeEngineClient.cs modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Db2EnvironmentSettingsTests.cs modern/desktop/src/Darwinbots.Desktop/Views/AdvancedSettingsWindow.axaml modern/desktop/src/Darwinbots.Desktop/Views/AdvancedSettingsWindow.axaml.cs
git commit -m "Expose automatic speciation settings"
```

---

### Task 6: Testable Viewport Geometry and DB2-Style Rendering

**Files:**
- Create: `modern/desktop/src/Darwinbots.Desktop.Core/OrganismVisualGeometry.cs`
- Create: `modern/desktop/tests/Darwinbots.Desktop.Core.Tests/OrganismVisualGeometryTests.cs`
- Modify: `modern/desktop/src/Darwinbots.Desktop/Controls/WorldViewport.cs`
- Create: `docs/visual-blueprints/organism-visual-phenotypes.png`

**Interfaces:**
- Consumes: enriched `RenderInstanceSnapshot` and selected `VisionSnapshot` from Task 4.
- Produces: radius-scaled skin points, heading point, eye-sector geometry, and Avalonia drawing.

- [ ] **Step 1: Generate the required visual blueprint before editing the GUI**

Use the image-generation tool with high reasoning and this prompt:

```text
Create a precise desktop software viewport blueprint for Darwinbots Modern, matching the existing warm off-white DB2-style interface. Show a pale gridded world with many circular organisms. Animals use several related cyan, orange, red, and ochre lineage colors; autotrophs use varied green hues only. Every close-enough organism has a thin three-segment angular skin inside its circle and a tiny heading tick at the circumference. Organisms vary in size according to body mass. One selected animal has a black selection ring and a translucent nine-sector fan showing its complete vision field; the focused central eye is slightly stronger. Include a dense distant cluster where LOD hides internal skins but preserves colors. Do not redesign menus, sidebars, typography, or window layout. This is a rendering blueprint, not promotional art. 16:9 desktop screenshot composition.
```

Save the generated result as `docs/visual-blueprints/organism-visual-phenotypes.png`.

- [ ] **Step 2: Write failing pure geometry tests**

```csharp
[Fact]
public void SkinPointsScaleAndRotateWithAim()
{
    var skin = new[]
    {
        new SkinPointSnapshot(0.5f, 0),
        new SkinPointSnapshot(0.5f, 314),
        new SkinPointSnapshot(0.5f, 628),
        new SkinPointSnapshot(0.5f, 942),
    };
    var points = OrganismVisualGeometry.SkinPoints(skin, radius: 10, aim: 314);
    Assert.Equal(4, points.Length);
    Assert.All(points, point => Assert.InRange(MathF.Sqrt(point.X * point.X + point.Y * point.Y), 4.99f, 5.01f));
}

[Fact]
public void EyeSectorsProduceNineArcsAndPreserveFocus()
{
    var sectors = OrganismVisualGeometry.EyeSectors(VisionSnapshot.Default, aim: 0, radius: 8);
    Assert.Equal(9, sectors.Length);
    Assert.Equal(4, Array.FindIndex(sectors, sector => sector.Focused));
}
```

- [ ] **Step 3: Run geometry tests and confirm failure**

```powershell
dotnet test modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj --filter OrganismVisualGeometryTests
```

Expected: compilation fails because `OrganismVisualGeometry` does not exist.

- [ ] **Step 4: Implement pure geometry helpers**

Define dependency-free records `VisualPoint(float X, float Y)` and `EyeSectorGeometry(float StartRadians, float SweepRadians, float Range, bool Focused)`. Convert DB2 aim units with `radians = units / 200f`; invert Y only in the Avalonia drawing layer, not in core geometry.

- [ ] **Step 5: Refactor viewport organism drawing**

Replace the current early-return circle loop with four focused private methods:

```csharp
private void DrawOrganisms(DrawingContext context, double scaleX, double scaleY);
private void DrawOrganism(DrawingContext context, RenderInstanceSnapshot instance, double scaleX, double scaleY, bool details);
private void DrawSkin(DrawingContext context, RenderInstanceSnapshot instance, Point center, double radius);
private void DrawSelectedVision(DrawingContext context, double scaleX, double scaleY);
```

Draw order is fill, selection outline, skin, heading tick. Use a contrast tint derived from luminance: dark skins on light fills and light skins on dark fills. Set `details` false when rendered radius is below `3.25` pixels, population exceeds `20_000` at zoom below `1`, or the existing stride is greater than one.

After all organisms, find only the selected organism snapshot and draw its nine translucent sector wedges. Use cyan for ordinary sectors, warm red-orange for the focused sector, and cap overlay opacity at `0.16` so bots remain readable.

- [ ] **Step 6: Run focused geometry and parser tests**

```powershell
dotnet test modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj --filter "OrganismVisualGeometryTests|NativeSnapshotParserTests"
```

Expected: all focused desktop tests pass.

- [ ] **Step 7: Commit renderer and blueprint**

```powershell
git add docs/visual-blueprints/organism-visual-phenotypes.png modern/desktop/src/Darwinbots.Desktop.Core/OrganismVisualGeometry.cs modern/desktop/tests/Darwinbots.Desktop.Core.Tests/OrganismVisualGeometryTests.cs modern/desktop/src/Darwinbots.Desktop/Controls/WorldViewport.cs
git commit -m "Render inherited skins and selected vision"
```

---

### Task 7: Focused Verification, Visual Comparison, and Windows Folder Build

**Files:**
- Modify only if a verification failure identifies a concrete defect in a file already listed above.
- Output: `modern/dist/win-x64/`
- Evidence: desktop screenshots captured during the comparison loop.

**Interfaces:**
- Consumes: completed engine, ABI, settings, and renderer tasks.
- Produces: a tested Windows 10 runnable folder and screenshot evidence matching the approved blueprint.

- [ ] **Step 1: Run focused Rust feature tests**

```powershell
cargo test --manifest-path modern/engine/Cargo.toml --test visual_phenotypes --test db2_vision --test contracts --test species_ecology --test db2_starter_ecology --test gpu_ffi
```

Expected: all selected Rust tests pass.

- [ ] **Step 2: Run focused desktop tests**

```powershell
dotnet test modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj --filter "NativeSnapshotParserTests|OrganismVisualGeometryTests|Db2EnvironmentSettingsTests"
```

Expected: all selected .NET tests pass.

- [ ] **Step 3: Publish the runnable Windows folder**

Close any running Darwinbots Modern process, build the native engine in release mode, and publish the desktop host:

```powershell
Get-Process Darwinbots.Desktop -ErrorAction SilentlyContinue | Stop-Process
cargo build --release --manifest-path modern/engine/Cargo.toml
dotnet publish modern/desktop/src/Darwinbots.Desktop/Darwinbots.Desktop.csproj -c Release -r win-x64 --self-contained true -o modern/dist/win-x64
Copy-Item modern/engine/target/release/darwinbots_engine.dll modern/dist/win-x64/darwinbots_engine.dll -Force
```

Expected: `modern/dist/win-x64/Darwinbots.Desktop.exe` exists with the matching native DLL.

- [ ] **Step 4: Perform visual comparison with computer use enabled only during interaction**

Launch the published executable, create a starter world, select an Animal Minimalis, zoom until skins are visible, and capture a screenshot using `Alt+PrintScreen`. Compare against `docs/visual-blueprints/organism-visual-phenotypes.png` for:

- varied green autotroph lineages
- non-green animal lineages
- body-size variation
- angular skins rotated with aim
- heading marks
- nine selected-eye sectors with focused-eye emphasis
- dense-population LOD

Turn computer use off immediately after each interaction batch so the blue halo does not remain visible.

- [ ] **Step 5: Exercise live speciation settings without a long tick benchmark**

Open Advanced Settings, enable automatic speciation, set mutation distance to `1%`, apply while running, and confirm a mutating lineage produces a bounded skin change while ordinary descendants retain their parent skin. This is a short feature check, not a tick-count stress test.

- [ ] **Step 6: Commit any concrete visual corrections**

If the screenshot comparison required corrections, stage only the files changed for those corrections and commit:

```powershell
git add modern/desktop/src/Darwinbots.Desktop/Controls/WorldViewport.cs modern/desktop/src/Darwinbots.Desktop.Core/OrganismVisualGeometry.cs
git commit -m "Polish DB2 organism visual readability"
```

If no correction was required, do not create an empty commit.

- [ ] **Step 7: Push and merge safely**

Push the `codex/organism-visual-phenotypes` branch, merge it into `master` only after confirming `master` has no unrelated uncommitted changes, then push `master`. Never reset or overwrite unrelated work.

```powershell
git push -u origin codex/organism-visual-phenotypes
git -C "D:\Darwinbots 2" merge --no-ff codex/organism-visual-phenotypes -m "Merge DB2 organism visual phenotypes"
git -C "D:\Darwinbots 2" push origin master
```

Expected: the feature branch and merge commit are present remotely, and the packaged runnable folder contains the verified build.
