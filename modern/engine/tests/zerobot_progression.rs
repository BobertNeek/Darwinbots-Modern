use darwinbots_engine::{Engine, EngineConfig, LegacyDna};

#[test]
fn progression_counters_distinguish_evolved_behavior_from_forced_assistance() {
    let mut forced = Engine::new(EngineConfig::testing()).unwrap();
    forced.spawn_at(LegacyDna::parse("start\n302 .shoot store\n50 .shootval store\nstop").unwrap(), [100.0, 100.0]).unwrap();
    forced.spawn_at(LegacyDna::parse("0 0 0 0 0").unwrap(), [120.0, 100.0]).unwrap();
    forced.tick().unwrap();
    assert_eq!(forced.snapshot().stats.births, 1);
    assert_eq!(forced.snapshot().stats.self_reproductions, 0);

    let mut evolved = Engine::new(EngineConfig::testing()).unwrap();
    evolved.spawn_at(LegacyDna::parse("start\n50 .repro store\nstop").unwrap(), [100.0, 100.0]).unwrap();
    evolved.tick().unwrap();
    assert_eq!(evolved.snapshot().stats.self_reproductions, 1);

    let mut behavior = Engine::new(EngineConfig::testing()).unwrap();
    behavior.spawn_at(LegacyDna::parse("start\n10 .up store\n-1 .shoot store\n40 .shootval store\nstop").unwrap(), [100.0, 100.0]).unwrap();
    behavior.spawn_at(LegacyDna::parse("start\nstop").unwrap(), [120.0, 100.0]).unwrap();
    behavior.tick().unwrap();
    assert!(behavior.snapshot().stats.feeding_events > 0);
    assert!(behavior.snapshot().stats.intentional_movement_events > 0);
}
