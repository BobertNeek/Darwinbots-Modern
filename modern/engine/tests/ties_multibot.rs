use darwinbots_engine::{Engine, EngineConfig, LegacyDna};

#[test]
fn ties_share_resources_transfer_memory_and_publish_multibot_state() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let source = engine
        .spawn_at(
            LegacyDna::parse("start\n100 .mkshell store\n40 .mkslime store\n1 .tie store\nstop")
                .unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [120.0, 100.0])
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().ties.len(), 1);
    let source_before = engine.organism(source).unwrap();
    let target_before = engine.organism(target).unwrap();
    engine.replace_dna(source, LegacyDna::parse(
        "start\n50 .sharenrg store\n50 .shareshell store\n50 .shareslime store\n50 .sharewaste store\n1 .tienum store\n500 .tieloc store\n123 .tieval store\nstop",
    ).unwrap()).unwrap();

    engine.tick().unwrap();

    let source_after = engine.organism(source).unwrap();
    let target_after = engine.organism(target).unwrap();
    assert!(source_after.energy < source_before.energy);
    assert!(target_after.energy > target_before.energy);
    assert_eq!(source_after.shell, 50);
    assert_eq!(target_after.shell, 50);
    assert_eq!(source_after.slime, 20);
    assert_eq!(target_after.slime, 20);
    assert!(target_after.waste > target_before.waste);
    assert_eq!(engine.memory_at(target, 500).unwrap(), 123);
    assert_eq!(engine.memory(source, "multi").unwrap(), 1);
    assert_eq!(engine.memory(target, "multi").unwrap(), 1);
    assert_eq!(engine.memory(source, "numties").unwrap(), 1);
    assert_eq!(engine.memory(target, "numties").unwrap(), 1);
}

#[test]
fn slime_is_consumed_to_resist_new_ties() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let source = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [100.0, 100.0])
        .unwrap();
    let target = engine
        .spawn_at(
            LegacyDna::parse("start\n50 .mkslime store\nstop").unwrap(),
            [120.0, 100.0],
        )
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n1 .tie store\nstop").unwrap(),
        )
        .unwrap();
    engine
        .replace_dna(target, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();

    engine.tick().unwrap();

    assert!(engine.snapshot().ties.is_empty());
    assert!(engine.organism(target).unwrap().slime < 50);
}

#[test]
fn dragging_one_member_moves_the_tied_organism_and_clears_momentum() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let source = engine
        .spawn_at(
            LegacyDna::parse("start\n1 .tie store\n25 .up store\nstop").unwrap(),
            [250.0, 300.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(
            LegacyDna::parse("start\n25 .up store\nstop").unwrap(),
            [550.0, 300.0],
        )
        .unwrap();
    engine.tick().unwrap();
    let snapshot_before = engine.snapshot();
    assert_eq!(snapshot_before.ties.len(), 1);
    assert_eq!(snapshot_before.ties[0].first, source);
    assert_eq!(snapshot_before.ties[0].second, target);
    let source_before = engine.organism(source).unwrap();
    let target_before = engine.organism(target).unwrap();
    let destination = [400.0, 450.0];
    let source_radius = snapshot_before
        .render_instances
        .iter()
        .find(|instance| instance.slot == source.slot())
        .unwrap()
        .radius;
    let target_radius = snapshot_before
        .render_instances
        .iter()
        .find(|instance| instance.slot == target.slot())
        .unwrap()
        .radius;
    let offset_before = [
        target_before.position[0] - source_before.position[0],
        target_before.position[1] - source_before.position[1],
    ];
    let separation_squared =
        offset_before[0] * offset_before[0] + offset_before[1] * offset_before[1];
    let minimum_separation = source_radius + target_radius;
    assert!(separation_squared > minimum_separation * minimum_separation);
    assert_ne!(source_before.velocity, [0.0, 0.0]);
    assert_ne!(target_before.velocity, [0.0, 0.0]);

    engine.move_organism(source, destination).unwrap();

    let source_after = engine.organism(source).unwrap();
    let target_after = engine.organism(target).unwrap();
    let source_translation = [
        source_after.position[0] - source_before.position[0],
        source_after.position[1] - source_before.position[1],
    ];
    let target_translation = [
        target_after.position[0] - target_before.position[0],
        target_after.position[1] - target_before.position[1],
    ];
    let offset_after = [
        target_after.position[0] - source_after.position[0],
        target_after.position[1] - source_after.position[1],
    ];

    // Mutation guard: removing connected-graph translation or either momentum reset
    // from Engine::move_organism must fail this test.
    assert_eq!(source_after.position, destination);
    assert_eq!(target_translation, source_translation);
    assert_eq!(offset_after, offset_before);
    assert_eq!(source_after.velocity, [0.0, 0.0]);
    assert_eq!(target_after.velocity, [0.0, 0.0]);
}
