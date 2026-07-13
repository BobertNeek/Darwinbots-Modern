mod support;

use darwinbots_engine::{Engine, EngineConfig, LegacyDna, PhysicsSettings};
use support::db2_fixtures::{DEFAULT_MAX_VELOCITY, DEFAULT_MOVEMENT_EFFICIENCY};

#[test]
fn movement_command_adds_impulse_and_bot_coasts_without_new_thrust() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        drag: 0.0,
        physics: PhysicsSettings { density: 0.0, ..PhysicsSettings::default() },
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
        physics: PhysicsSettings { density: 0.0, ..PhysicsSettings::default() },
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

#[test]
fn drag_reduces_retained_momentum_instead_of_replacing_it() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        drag: 0.25,
        physics: PhysicsSettings { density: 0.0, ..PhysicsSettings::default() },
        ..EngineConfig::testing()
    })
    .unwrap();
    let dna = LegacyDna::parse("cond\n*.robage 1 <\nstart\n10 .up store\nstop").unwrap();
    let id = engine.spawn_at(dna, [100.0, 100.0]).unwrap();

    engine.tick().unwrap();
    let first = engine.organism(id).unwrap().velocity[1];
    engine.tick().unwrap();
    let second = engine.organism(id).unwrap().velocity[1];

    assert!(second > 0.0);
    assert!(second < first);
}

#[test]
fn elasticity_rebounds_colliding_bots_with_finite_velocity() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings {
            elasticity: 0.5,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    engine
        .spawn_batch([
            (
                LegacyDna::parse("start\n10 .sx store\nstop").unwrap(),
                [490.0, 500.0],
            ),
            (
                LegacyDna::parse("start\n10 .dx store\nstop").unwrap(),
                [510.0, 500.0],
            ),
        ])
        .unwrap();

    engine.tick().unwrap();

    let organisms = &engine.snapshot().organisms;
    assert!(organisms.iter().flat_map(|bot| bot.position).all(f32::is_finite));
    assert!(organisms.iter().flat_map(|bot| bot.velocity).all(f32::is_finite));
    assert!(organisms[0].velocity[0] < 0.0);
    assert!(organisms[1].velocity[0] > 0.0);
}
