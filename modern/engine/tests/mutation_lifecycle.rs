use darwinbots_engine::{
    Engine, EngineConfig, GenomeMutator, LegacyDna, MutationKind, PhysicsSettings, PointMutator,
};

#[test]
fn point_mutation_is_seeded_and_preserves_program_structure() {
    let original = LegacyDna::parse("start\n10 .up store\nstop").unwrap();
    let mut left = original.clone();
    let mut right = original.clone();

    let left_report = PointMutator::new(91).mutate(&mut left);
    let right_report = PointMutator::new(91).mutate(&mut right);

    assert_eq!(left_report.changes, 1);
    assert_eq!(left_report, right_report);
    assert_eq!(left, right);
    assert_ne!(left, original);
    assert_eq!(left.instructions().len(), original.instructions().len());
}

#[test]
fn structural_mutations_can_grow_shrink_duplicate_and_replace_zerobot_dna() {
    let original = LegacyDna::parse("0 0 0 0 0 0 0 0").unwrap();

    let mut inserted = original.clone();
    GenomeMutator::new(11).mutate_kind(&mut inserted, MutationKind::Insertion);
    assert_eq!(inserted.instructions().len(), original.instructions().len() + 1);

    let mut deleted = original.clone();
    GenomeMutator::new(12).mutate_kind(&mut deleted, MutationKind::Deletion);
    assert_eq!(deleted.instructions().len(), original.instructions().len() - 1);

    let mut duplicated = original.clone();
    GenomeMutator::new(13).mutate_kind(&mut duplicated, MutationKind::Duplication);
    assert!(duplicated.instructions().len() > original.instructions().len());

    let mut replaced = original.clone();
    let report = GenomeMutator::new(14).mutate_kind(&mut replaced, MutationKind::Replacement);
    assert_eq!(replaced.instructions().len(), original.instructions().len());
    assert_ne!(replaced, original);
    assert_eq!(report.kind, Some(MutationKind::Replacement));
}

#[test]
fn mutation_reproduction_creates_mutated_child_and_records_change() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let dna = LegacyDna::parse("start\n10 .up store\n50 .mrepro store\nstop").unwrap();
    let parent = engine.spawn_at(dna, [500.0, 500.0]).unwrap();

    engine.tick().unwrap();

    let child = engine.snapshot().organisms.iter().find(|organism| organism.id != parent).unwrap().id;
    assert_ne!(engine.dna(parent).unwrap(), engine.dna(child).unwrap());
    assert_eq!(engine.snapshot().stats.mutations, 1);
}

#[test]
fn depleted_organisms_die_and_increment_death_statistics() {
    let mut engine = Engine::new(EngineConfig {
        physics: PhysicsSettings { density: 0.0, ..PhysicsSettings::default() },
        ..EngineConfig::testing()
    })
    .unwrap();
    let attacker = LegacyDna::parse("start\n-1 .shoot store\n2000 .shootval store\nstop").unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    engine.spawn_at(attacker, [100.0, 100.0]).unwrap();
    let target = engine.spawn_at(idle, [350.0, 100.0]).unwrap();

    engine.tick().unwrap();

    assert!(engine.organism(target).is_err());
    assert_eq!(engine.snapshot().stats.deaths, 1);
}
