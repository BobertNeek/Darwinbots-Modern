use darwinbots_engine::{Engine, EngineConfig, LegacyDna, SaveFile};

#[test]
fn fired_shots_publish_visible_persisted_world_state() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let attacker = engine.spawn_at(
        LegacyDna::parse("start\n-1 .shoot store\n40 .shootval store\nstop").unwrap(),
        [100.0, 100.0],
    ).unwrap();
    engine.spawn_at(LegacyDna::parse("start\nstop").unwrap(), [120.0, 100.0]).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.snapshot().shots.len(), 1);
    let shot = &engine.snapshot().shots[0];
    assert_eq!(shot.owner, attacker);
    assert_eq!(shot.kind, -1);
    assert_ne!(shot.start, [100.0, 100.0]);
    assert_ne!(shot.end, [120.0, 100.0]);
    assert!(shot.start[0].hypot(shot.start[1]).is_finite());
    assert!(!shot.impact_flash);
    let saved_age = shot.age;
    let restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
    assert_eq!(restored.snapshot().shots, engine.snapshot().shots);

    engine.replace_dna(attacker, LegacyDna::parse("start\nstop").unwrap()).unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().shots.len(), 1);
    assert!(engine.snapshot().shots[0].age > saved_age);

    engine.tick_many(20).unwrap();
    assert!(engine.snapshot().shots.is_empty());
}
