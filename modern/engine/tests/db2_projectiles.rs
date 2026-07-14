mod support;

use darwinbots_engine::{Engine, EngineConfig, LegacyDna, Obstacle, PhysicsSettings};
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

#[test]
fn projectile_moves_once_per_tick_and_expires_after_range() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings { density: 0.0, ..PhysicsSettings::default() },
        ..EngineConfig::testing()
    })
    .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse(
                "cond\n*.robage 1 <\nstart\n-1 .shoot store\nstop",
            )
            .unwrap(),
            [100.0, 100.0],
        )
        .unwrap();

    engine.tick().unwrap();
    let first = engine.snapshot().shots[0].clone();
    engine.tick().unwrap();
    let second = engine.snapshot().shots[0].clone();
    assert_eq!(second.start, first.end);

    engine.tick_many(20).unwrap();
    assert!(engine.snapshot().shots.is_empty());
}

#[test]
fn swept_collision_hits_a_bot_between_projectile_endpoints() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings { density: 0.0, ..PhysicsSettings::default() },
        ..EngineConfig::testing()
    })
    .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n-1 .shoot store\n100 .shootval store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(
            LegacyDna::parse("start\nstop").unwrap(),
            [140.0, 100.0],
        )
        .unwrap();
    let before = engine.organism(target).unwrap().energy;

    engine.tick().unwrap();

    assert!(engine.organism(target).unwrap().energy < before);
    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
    assert_eq!(engine.snapshot().stats.projectile_effects, 1);
}

#[test]
fn newborn_is_immune_to_parent_stream_for_one_tick() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings { density: 0.0, ..PhysicsSettings::default() },
        ..EngineConfig::testing()
    })
    .unwrap();
    let parent = engine
        .spawn_at(
            LegacyDna::parse("start\n-1 .shoot store\n100 .shootval store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let newborn = engine.manual_reproduce(parent, None, [140.0, 100.0]).unwrap();
    let initial_energy = engine.organism(newborn).unwrap().energy;

    engine.tick().unwrap();

    assert_eq!(engine.organism(newborn).unwrap().energy, initial_energy);
}

#[test]
fn positive_memory_shot_writes_the_target_address() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings { density: 0.0, ..PhysicsSettings::default() },
        ..EngineConfig::testing()
    })
    .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n500 .shoot store\n37 .shootval store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [140.0, 100.0])
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory_at(target, 500).unwrap(), 37);
}

#[test]
fn obstacle_intercepts_projectile_before_target() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings { density: 0.0, ..PhysicsSettings::default() },
        ..EngineConfig::testing()
    })
    .unwrap();
    engine
        .add_obstacle(Obstacle {
            id: 1,
            minimum: [125.0, 90.0],
            maximum: [130.0, 110.0],
        })
        .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n-1 .shoot store\n100 .shootval store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [140.0, 100.0])
        .unwrap();
    let before = engine.organism(target).unwrap().energy;

    engine.tick().unwrap();

    assert_eq!(engine.organism(target).unwrap().energy, before);
    assert!(engine.snapshot().shots[0].impact_flash);
}

#[test]
fn virus_shot_transfers_the_firers_dna() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings { density: 0.0, ..PhysicsSettings::default() },
        ..EngineConfig::testing()
    })
    .unwrap();
    let virus = LegacyDna::parse("start\n-7 .shoot store\nstop").unwrap();
    let owner = engine.spawn_at(virus.clone(), [100.0, 100.0]).unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [140.0, 100.0])
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.dna(target).unwrap(), engine.dna(owner).unwrap());
}

#[test]
fn sperm_shot_forces_target_reproduction() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings { density: 0.0, ..PhysicsSettings::default() },
        ..EngineConfig::testing()
    })
    .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n-8 .shoot store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [140.0, 100.0])
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.population(), 3);
    assert_eq!(engine.snapshot().stats.births, 1);
}

fn segment_length(start: [f32; 2], end: [f32; 2]) -> f32 {
    (end[0] - start[0]).hypot(end[1] - start[1])
}
