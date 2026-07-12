use darwinbots_engine::{Engine, EngineConfig, LegacyDna, SaveFile};

#[test]
fn bounded_historical_records_are_sampled_and_preserved_by_saves() {
    let mut engine = Engine::new(EngineConfig { metabolism_cost: 0, ..EngineConfig::testing() }).unwrap();
    engine.spawn_at(LegacyDna::parse("start\nstop").unwrap(), [100.0, 100.0]).unwrap();

    engine.tick_many(250).unwrap();

    assert_eq!(engine.snapshot().history.len(), 2);
    assert_eq!(engine.snapshot().history[0].tick, 100);
    assert_eq!(engine.snapshot().history[1].tick, 200);
    assert_eq!(engine.snapshot().history[1].population, 1);
    assert!(engine.snapshot().history[1].total_energy > 0);

    let restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
    assert_eq!(restored.snapshot().history, engine.snapshot().history);
}
