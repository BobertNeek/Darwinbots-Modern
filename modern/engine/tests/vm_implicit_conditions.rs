use darwinbots_engine::{Engine, EngineConfig, LegacyDna};

#[test]
fn adjacent_conditions_are_implicitly_anded() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let dna = LegacyDna::parse("cond\n1 0 >\n1 2 >\nstart\n10 .up store\nstop").unwrap();
    let id = engine.spawn_at(dna, [500.0, 500.0]).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.organism(id).unwrap().position, [500.0, 500.0]);
}

#[test]
fn explicit_or_reduces_conditions_before_implicit_and() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let dna = LegacyDna::parse("cond\n1 0 >\n1 2 > or\nstart\n10 .up store\nstop").unwrap();
    let id = engine.spawn_at(dna, [500.0, 500.0]).unwrap();

    engine.tick().unwrap();

    assert!(engine.organism(id).unwrap().position[1] > 500.0);
}
