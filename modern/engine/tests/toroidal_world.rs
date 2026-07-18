use darwinbots_engine::{Engine, EngineConfig, LegacyDna, PhysicsSettings};

fn toroidal_test_config() -> EngineConfig {
    EngineConfig {
        world_width: 100.0,
        world_height: 100.0,
        toroidal_world: true,
        metabolism_cost: 0,
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    }
}

#[test]
fn organism_motion_wraps_across_the_world_edge() {
    let mut engine = Engine::new(toroidal_test_config()).unwrap();
    let organism = engine
        .spawn_at(
            LegacyDna::parse("start\n100 .up store\nstop").unwrap(),
            [50.0, 99.0],
        )
        .unwrap();

    engine.tick().unwrap();

    let position = engine
        .snapshot()
        .organisms
        .iter()
        .find(|snapshot| snapshot.id == organism)
        .unwrap()
        .position;
    assert!((0.0..100.0).contains(&position[1]));
    assert!(position[1] < 99.0, "expected y wrap, got {position:?}");
}

#[test]
fn projectile_wrap_preserves_its_travel_segment() {
    let mut engine = Engine::new(toroidal_test_config()).unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n-1 .shoot store\nstop").unwrap(),
            [95.0, 50.0],
        )
        .unwrap();

    engine.tick().unwrap();

    let shot = &engine.snapshot().shots[0];
    let segment_length =
        (shot.end[0] - shot.start[0]).hypot(shot.end[1] - shot.start[1]);
    let velocity_length = shot.velocity[0].hypot(shot.velocity[1]);
    assert!((0.0..100.0).contains(&shot.end[0]));
    assert!((segment_length - velocity_length).abs() < 0.01);
}

#[test]
fn shot_inherits_shortest_toroidal_firer_displacement() {
    let mut engine = Engine::new(toroidal_test_config()).unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n10 .dx store\n-1 .shoot store\nstop").unwrap(),
            [99.0, 50.0],
        )
        .unwrap();

    engine.tick().unwrap();

    let shot = &engine.snapshot().shots[0];
    assert!(shot.velocity[0] > 0.0, "wrapped firer velocity was inverted: {shot:?}");
    assert!(shot.velocity[0] < 100.0, "wrapped firer inherited a world-sized jump: {shot:?}");
}
