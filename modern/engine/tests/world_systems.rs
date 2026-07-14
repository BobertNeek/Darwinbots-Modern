use darwinbots_engine::{Engine, EngineConfig, LegacyDna, SpatialIndex};

#[test]
fn uniform_spatial_index_returns_sorted_nearby_slots() {
    let index = SpatialIndex::build(&[[10.0, 10.0], [22.0, 10.0], [200.0, 200.0]], 32.0);

    assert_eq!(index.neighbors([10.0, 10.0], 40.0), vec![0, 1]);
    assert_eq!(index.nearest([10.0, 10.0], Some(0), 40.0), Some(1));
    assert_eq!(index.segment_candidates([0.0, 10.0], [40.0, 10.0], 4.0), vec![0, 1]);
}

#[test]
fn sensing_writes_nearest_organism_reference_sysvars() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let observer = engine.spawn_at(LegacyDna::parse("start\n314 .setaim store\nstop").unwrap(), [100.0, 100.0]).unwrap();
    engine.spawn_at(idle, [400.0, 100.0]).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory(observer, "refxpos").unwrap(), 400);
    assert_eq!(engine.memory(observer, "refypos").unwrap(), 100);
    assert!(engine.memory(observer, "eye5").unwrap() > 0);
}

#[test]
fn negative_shot_damages_nearest_target_and_records_event() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let attacker = LegacyDna::parse("start\n-1 .shoot store\n100 .shootval store\nstop").unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    engine.spawn_at(attacker, [100.0, 100.0]).unwrap();
    let target = engine.spawn_at(idle, [120.0, 100.0]).unwrap();
    let before = engine.organism(target).unwrap().energy;

    engine.tick().unwrap();

    assert!(engine.organism(target).unwrap().energy < before);
    assert_eq!(engine.snapshot().stats.shots_fired, 1);
}

#[test]
fn reproduction_creates_child_with_stable_id_and_birth_statistics() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let reproducer = LegacyDna::parse("start\n50 .repro store\nstop").unwrap();
    engine.spawn_at(reproducer, [500.0, 500.0]).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.population(), 2);
    assert_eq!(engine.snapshot().stats.births, 1);
    assert_ne!(engine.snapshot().organisms[0].id, engine.snapshot().organisms[1].id);
}

#[test]
fn tie_commands_create_multibot_link_and_share_energy() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let connector = LegacyDna::parse("start\n1 .tie store\n10 .sharenrg store\nstop").unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let source = engine.spawn_at(connector, [100.0, 100.0]).unwrap();
    let target = engine.spawn_at(idle, [140.0, 100.0]).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.snapshot().ties.len(), 1);
    assert_eq!(engine.snapshot().stats.ties_created, 1);
    assert!(engine.organism(source).unwrap().energy < engine.organism(target).unwrap().energy);
}
