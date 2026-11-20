use darwinbots_engine::{
    Engine, EngineConfig, GenomeMutator, LegacyDna, MAX_DNA_BLOCKS, MutationKind, PhysicsSettings,
    PointMutator, StandardMutationKind, TokenClass,
};

fn assert_single_terminal_end(dna: &LegacyDna) {
    let end_positions = dna
        .blocks()
        .iter()
        .enumerate()
        .filter_map(|(index, block)| (block.class == TokenClass::End).then_some(index))
        .collect::<Vec<_>>();

    assert_eq!(end_positions, vec![dna.blocks().len() - 1]);
}

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
    assert_eq!(left.blocks().len(), original.blocks().len());
}

#[test]
fn structural_mutations_can_grow_shrink_duplicate_and_replace_zerobot_dna() {
    let original = LegacyDna::parse("0 0 0 0 0 0 0 0").unwrap();

    let mut inserted = original.clone();
    GenomeMutator::new(11).mutate_kind(&mut inserted, MutationKind::Insertion);
    assert_eq!(inserted.blocks().len(), original.blocks().len() + 1);

    let mut deleted = original.clone();
    GenomeMutator::new(12).mutate_kind(&mut deleted, MutationKind::Deletion);
    assert_eq!(deleted.blocks().len(), original.blocks().len() - 1);

    let mut duplicated = original.clone();
    GenomeMutator::new(13).mutate_kind(&mut duplicated, MutationKind::Duplication);
    assert!(duplicated.blocks().len() > original.blocks().len());

    let mut replaced = original.clone();
    let report = GenomeMutator::new(14).mutate_kind(&mut replaced, MutationKind::Replacement);
    assert_eq!(replaced.blocks().len(), original.blocks().len());
    assert_ne!(replaced, original);
    assert_eq!(report.kind, Some(MutationKind::Replacement));
}

#[test]
fn structural_mutations_preserve_single_terminal_end() {
    let original = LegacyDna::parse("0 1 2 3 4 5 6 7").unwrap();
    let boundary_seeds = [3, 9, 10, 11, 21, 22];

    for kind in [
        MutationKind::Insertion,
        MutationKind::Deletion,
        MutationKind::Duplication,
        MutationKind::Replacement,
    ] {
        for seed in boundary_seeds {
            let mut mutated = original.clone();
            let report = GenomeMutator::new(seed).mutate_kind(&mut mutated, kind);

            assert_eq!(report.kind, Some(kind), "kind {kind:?}, seed {seed}");
            match kind {
                MutationKind::Insertion => {
                    assert_eq!(mutated.blocks().len(), original.blocks().len() + 1)
                }
                MutationKind::Deletion => {
                    assert_eq!(mutated.blocks().len(), original.blocks().len() - 1)
                }
                MutationKind::Duplication => {
                    assert!(mutated.blocks().len() > original.blocks().len())
                }
                MutationKind::Replacement => {
                    assert_eq!(mutated.blocks().len(), original.blocks().len())
                }
                MutationKind::Point => unreachable!(),
            }
            assert!(mutated.blocks().len() <= MAX_DNA_BLOCKS);
            assert_single_terminal_end(&mutated);
        }
    }
}

#[test]
fn structural_mutations_respect_32000_block_limit() {
    let original = LegacyDna::parse(&"1 ".repeat(MAX_DNA_BLOCKS)).unwrap();
    assert_eq!(original.blocks().len(), MAX_DNA_BLOCKS);

    for kind in [
        MutationKind::Insertion,
        MutationKind::Duplication,
        MutationKind::Deletion,
        MutationKind::Replacement,
    ] {
        for seed in [29_349, 100_844] {
            let mut mutated = original.clone();
            GenomeMutator::new(seed).mutate_kind(&mut mutated, kind);

            let expected_len = match kind {
                MutationKind::Insertion | MutationKind::Duplication => MAX_DNA_BLOCKS,
                MutationKind::Deletion => MAX_DNA_BLOCKS - 1,
                MutationKind::Replacement => MAX_DNA_BLOCKS,
                MutationKind::Point => unreachable!(),
            };
            assert_eq!(
                mutated.blocks().len(),
                expected_len,
                "kind {kind:?}, seed {seed}"
            );
            assert!(mutated.blocks().len() <= MAX_DNA_BLOCKS);
            assert_single_terminal_end(&mutated);
        }
    }
}

#[test]
fn serde_deserialization_caps_dna_before_structural_mutation() {
    let serialized = LegacyDna::parse("1").unwrap();
    let mut value = serde_json::to_value(&serialized).unwrap();
    let instructions = value
        .get_mut("instructions")
        .and_then(serde_json::Value::as_array_mut)
        .unwrap();
    let master_end = instructions.pop().unwrap();
    let coding_block = instructions[0].clone();
    *instructions = vec![coding_block; MAX_DNA_BLOCKS + 1];
    instructions.push(master_end);

    let dna: LegacyDna = serde_json::from_value(value).unwrap();

    assert_eq!(dna.blocks().len(), MAX_DNA_BLOCKS);
    assert_single_terminal_end(&dna);

    for kind in [
        MutationKind::Insertion,
        MutationKind::Duplication,
        MutationKind::Deletion,
        MutationKind::Replacement,
    ] {
        let mut mutated = dna.clone();
        GenomeMutator::new(29_349).mutate_kind(&mut mutated, kind);

        let expected_len = match kind {
            MutationKind::Insertion | MutationKind::Duplication => MAX_DNA_BLOCKS,
            MutationKind::Deletion => MAX_DNA_BLOCKS - 1,
            MutationKind::Replacement => MAX_DNA_BLOCKS,
            MutationKind::Point => unreachable!(),
        };
        assert_eq!(mutated.blocks().len(), expected_len, "kind {kind:?}");
        assert!(mutated.blocks().len() <= MAX_DNA_BLOCKS);
        assert_single_terminal_end(&mutated);
    }
}

#[test]
fn mutation_reproduction_creates_mutated_child_and_records_change() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let dna = LegacyDna::parse("start\n10 .up store\n50 .mrepro store\nstop").unwrap();
    let parent = engine.spawn_at(dna, [500.0, 500.0]).unwrap();

    // MREPRO increases trials; it does not guarantee the old one-change fallback.
    let mut profile = engine.mutation_profile(parent).unwrap().clone();
    profile.copy_error.one_in = 1.0;
    profile.copy_error.mean = 1.0;
    profile.copy_error.std_dev = 0.0;
    engine.set_mutation_profile(parent, profile).unwrap();

    engine.tick().unwrap();

    let child = engine
        .snapshot()
        .organisms
        .iter()
        .find(|organism| organism.id != parent)
        .unwrap()
        .id;
    assert_ne!(engine.dna(parent).unwrap(), engine.dna(child).unwrap());
    assert!(engine.snapshot().stats.mutations > 0);
}

#[test]
fn runtime_mutation_publishes_detailed_class_log_and_subspecies_metadata() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let dna = LegacyDna::parse("start\n10 .up store\n50 .mrepro store\nstop").unwrap();
    let parent = engine.spawn_at(dna, [500.0, 500.0]).unwrap();

    let mut profile = engine.mutation_profile(parent).unwrap().clone();
    profile.copy_error.one_in = 1.0;
    profile.copy_error.mean = 1.0;
    profile.copy_error.std_dev = 0.0;
    engine.set_mutation_profile(parent, profile).unwrap();

    engine.tick().unwrap();

    let child = engine
        .snapshot()
        .organisms
        .iter()
        .find(|organism| organism.id != parent)
        .unwrap();
    assert_eq!(
        child.last_mutation_kind,
        Some(StandardMutationKind::CopyError)
    );
    assert!(
        child
            .mutation_log
            .first()
            .is_some_and(|entry| entry.contains("CopyError"))
    );
    assert_ne!(child.subspecies, 0);
}

#[test]
fn depleted_organisms_die_and_increment_death_statistics() {
    let mut engine = Engine::new(EngineConfig {
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let attacker = LegacyDna::parse("start\n-1 .shoot store\n2000 .shootval store\nstop").unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    engine.spawn_at(attacker, [300.0, 500.0]).unwrap();
    let target = engine.spawn_at(idle, [550.0, 500.0]).unwrap();

    engine.tick().unwrap();
    engine.tick().unwrap();

    assert!(engine.organism(target).is_err());
    assert_eq!(engine.snapshot().stats.deaths, 1);
}
