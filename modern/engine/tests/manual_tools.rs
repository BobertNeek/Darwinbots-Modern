use darwinbots_engine::{Engine, EngineConfig, LegacyDna, SpeciesDefinition};

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

