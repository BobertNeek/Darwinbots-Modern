mod support;

use darwinbots_engine::{Engine, EngineConfig, LegacyDna};
use support::db2_fixtures::{DEFAULT_MAX_VELOCITY, DEFAULT_MOVEMENT_EFFICIENCY};

#[test]
fn movement_command_adds_impulse_and_bot_coasts_without_new_thrust() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        drag: 0.0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let dna = LegacyDna::parse("cond\n*.robage 1 <\nstart\n10 .up store\nstop").unwrap();
    let id = engine.spawn_at(dna, [100.0, 100.0]).unwrap();

    engine.tick().unwrap();
    let first = engine.organism(id).unwrap();
    engine.tick().unwrap();
    let second = engine.organism(id).unwrap();

    assert!(first.velocity[1] > 0.0);
    assert_eq!(second.velocity, first.velocity);
    assert!(second.position[1] > first.position[1]);
}

#[test]
fn voluntary_acceleration_is_clamped_before_efficiency_multiplier() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        drag: 0.0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let id = engine
        .spawn_at(
            LegacyDna::parse("start\n10000 .up store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();

    engine.tick().unwrap();

    let expected = DEFAULT_MAX_VELOCITY * DEFAULT_MOVEMENT_EFFICIENCY;
    assert!((engine.organism(id).unwrap().velocity[1] - expected).abs() < 0.01);
}

#[test]
fn newmove_directive_is_preserved_by_legacy_dna() {
    let dna = LegacyDna::parse("NewMove\nstart\nstop").unwrap();

    assert!(dna.uses_new_move());
    assert!(dna.to_source().starts_with("NewMove\n"));
}
