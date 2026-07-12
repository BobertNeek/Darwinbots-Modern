use darwinbots_engine::{Engine, EngineConfig, LegacyDna};

#[test]
fn ties_share_resources_transfer_memory_and_publish_multibot_state() {
    let mut engine = Engine::new(EngineConfig { metabolism_cost: 0, ..EngineConfig::testing() }).unwrap();
    let source = engine.spawn_at(LegacyDna::parse(
        "start\n100 .mkshell store\n40 .mkslime store\n1 .tie store\nstop",
    ).unwrap(), [100.0, 100.0]).unwrap();
    let target = engine.spawn_at(LegacyDna::parse("start\nstop").unwrap(), [120.0, 100.0]).unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().ties.len(), 1);
    let source_before = engine.organism(source).unwrap();
    let target_before = engine.organism(target).unwrap();
    engine.replace_dna(source, LegacyDna::parse(
        "start\n50 .sharenrg store\n50 .shareshell store\n50 .shareslime store\n50 .sharewaste store\n900 .tieloc store\n123 .tieval store\nstop",
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
    assert_eq!(engine.memory_at(target, 900).unwrap(), 123);
    assert_eq!(engine.memory(source, "multi").unwrap(), 1);
    assert_eq!(engine.memory(target, "multi").unwrap(), 1);
    assert_eq!(engine.memory(source, "numties").unwrap(), 1);
    assert_eq!(engine.memory(target, "numties").unwrap(), 1);
}

#[test]
fn slime_is_consumed_to_resist_new_ties() {
    let mut engine = Engine::new(EngineConfig { metabolism_cost: 0, ..EngineConfig::testing() }).unwrap();
    let source = engine.spawn_at(LegacyDna::parse("start\nstop").unwrap(), [100.0, 100.0]).unwrap();
    let target = engine.spawn_at(LegacyDna::parse("start\n50 .mkslime store\nstop").unwrap(), [120.0, 100.0]).unwrap();
    engine.tick().unwrap();
    engine.replace_dna(source, LegacyDna::parse("start\n1 .tie store\nstop").unwrap()).unwrap();
    engine.replace_dna(target, LegacyDna::parse("start\nstop").unwrap()).unwrap();

    engine.tick().unwrap();

    assert!(engine.snapshot().ties.is_empty());
    assert!(engine.organism(target).unwrap().slime < 50);
}
