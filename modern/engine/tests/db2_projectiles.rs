mod support;

use darwinbots_engine::{Engine, EngineConfig, LegacyDna, PhysicsSettings};
use support::db2_fixtures::SHOT_SPEED;

#[test]
fn firing_creates_a_moving_projectile_instead_of_an_instant_hit_line() {
    let mut engine = Engine::new(EngineConfig {
        physics: PhysicsSettings { density: 0.0, ..PhysicsSettings::default() },
        ..EngineConfig::testing()
    })
    .unwrap();
    let attacker = engine
        .spawn_at(
            LegacyDna::parse("start\n-1 .shoot store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\nstop").unwrap(),
            [300.0, 100.0],
        )
        .unwrap();

    engine.tick().unwrap();

    let shot = &engine.snapshot().shots[0];
    assert_eq!(shot.owner, attacker);
    assert!((segment_length(shot.start, shot.end) - SHOT_SPEED).abs() < 0.01);
    assert_ne!(shot.end, [300.0, 100.0]);
}

#[test]
fn shot_velocity_inherits_the_firers_actual_velocity() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings { density: 0.0, ..PhysicsSettings::default() },
        ..EngineConfig::testing()
    })
    .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n10 .dx store\n-1 .shoot store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();

    engine.tick().unwrap();

    let shot = &engine.snapshot().shots[0];
    assert!(shot.velocity[0] > SHOT_SPEED);
}

fn segment_length(start: [f32; 2], end: [f32; 2]) -> f32 {
    (end[0] - start[0]).hypot(end[1] - start[1])
}
