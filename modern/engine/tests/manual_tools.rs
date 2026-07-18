use darwinbots_engine::{Engine, EngineConfig, LegacyDna, PhysicsSettings, SpeciesDefinition};

#[test]
fn move_clone_and_replace_dna_preserve_stable_world_state() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let species = engine.register_species(SpeciesDefinition {
        name: "Editable".to_owned(),
        ..SpeciesDefinition::default()
    });
    let original = LegacyDna::parse("start\n10 .up store\nstop").unwrap();
    let source = engine.spawn_species_at(original.clone(), species, [100.0, 100.0]).unwrap();

    engine.move_organism(source, [2_000.0, -50.0]).unwrap();
    let clone = engine.clone_organism(source, [500.0, 500.0]).unwrap();
    let replacement = LegacyDna::parse("start\n20 .dx store\nstop").unwrap();
    engine.replace_dna(source, replacement).unwrap();

    assert_eq!(engine.organism(source).unwrap().position, [1_000.0, 0.0]);
    assert_eq!(engine.organism(clone).unwrap().position, [500.0, 500.0]);
    assert_eq!(engine.organism(clone).unwrap().species, species);
    assert_eq!(engine.dna(clone).unwrap(), &original);
    assert!(engine.export_dna(source).unwrap().contains(".dx"));
}

#[test]
fn manual_two_parent_reproduction_records_parentage() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let left_dna = LegacyDna::parse("start\n10 .up store\nstop").unwrap();
    let right_dna = LegacyDna::parse("start\n20 .dx store\nstop").unwrap();
    let left = engine.spawn_at(left_dna, [100.0, 100.0]).unwrap();
    let right = engine.spawn_at(right_dna, [200.0, 100.0]).unwrap();

    let child = engine.manual_reproduce(left, Some(right), [150.0, 100.0]).unwrap();

    let snapshot = engine.snapshot().organisms.iter().find(|value| value.id == child).unwrap();
    assert_eq!(snapshot.parents, [Some(left), Some(right)]);
    assert_eq!(engine.snapshot().stats.births, 1);
    assert_ne!(engine.dna(child).unwrap(), engine.dna(left).unwrap());
    assert_ne!(engine.dna(child).unwrap(), engine.dna(right).unwrap());
}

#[test]
fn clone_preserves_live_energy_biology_and_velocity() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        drag: 0.0,
        physics: PhysicsSettings { density: 0.0, ..PhysicsSettings::default() },
        ..EngineConfig::testing()
    })
    .unwrap();
    let source = engine
        .spawn_at(
            LegacyDna::parse("start\n25 .up store\n200 .mkshell store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    engine.tick().unwrap();
    let source_state = engine.organism(source).unwrap();

    let clone = engine.clone_organism(source, [500.0, 500.0]).unwrap();
    let clone_state = engine.organism(clone).unwrap();

    assert_eq!(clone_state.energy, source_state.energy);
    assert_eq!(clone_state.body, source_state.body);
    assert_eq!(clone_state.shell, source_state.shell);
    assert_eq!(clone_state.slime, source_state.slime);
    assert_eq!(clone_state.waste, source_state.waste);
    assert_eq!(clone_state.velocity, source_state.velocity);
    assert_eq!(engine.dna(clone).unwrap(), engine.dna(source).unwrap());
}

#[test]
fn manual_one_parent_reproduction_splits_energy_and_body_fifty_fifty() {
    let mut engine = Engine::new(EngineConfig { metabolism_cost: 0, ..EngineConfig::testing() }).unwrap();
    let parent = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [100.0, 100.0])
        .unwrap();
    let before = engine.organism(parent).unwrap();

    let child = engine.manual_reproduce(parent, None, [150.0, 100.0]).unwrap();
    let parent_after = engine.organism(parent).unwrap();
    let child_after = engine.organism(child).unwrap();

    assert_eq!(parent_after.energy, before.energy / 2);
    assert_eq!(child_after.energy, before.energy - parent_after.energy);
    assert_eq!(parent_after.body, before.body / 2);
    assert_eq!(child_after.body, before.body - parent_after.body);
    assert_eq!(child_after.parents, [Some(parent), None]);
}

