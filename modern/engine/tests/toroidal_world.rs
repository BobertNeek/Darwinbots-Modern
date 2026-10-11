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
fn topology_wraps_x_but_bounds_y_inside_robot_radius_and_smudge() {
    let mut engine = Engine::new(EngineConfig {
        world_width: 1_000.0,
        world_height: 1_000.0,
        toroidal_world: false,
        axis_topology: Some([true, false]),
        metabolism_cost: 0,
        gravity: [0.0, 0.0],
        drag: 0.0,
        physics: PhysicsSettings {
            surface_gravity: 0.0,
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let wrapped = engine
        .spawn_at(
            LegacyDna::parse("start\n100 .sx store\nstop").unwrap(),
            [1.0, 500.0],
        )
        .unwrap();
    let bounded = engine
        .spawn_at(
            LegacyDna::parse("start\n100 .up store\nstop").unwrap(),
            [500.0, 999.0],
        )
        .unwrap();

    engine.tick().unwrap();

    let wrapped_state = engine.organism(wrapped).unwrap();
    let bounded_state = engine.organism(bounded).unwrap();
    assert!(wrapped_state.position[0] > wrapped_state.radius);
    assert_eq!(engine.memory_at(wrapped, 214).unwrap(), 0);
    assert!(bounded_state.position[1] <= 1_000.0 - bounded_state.radius + 0.01);
    assert_eq!(engine.memory_at(bounded, 214).unwrap(), 1);
}

#[test]
fn wrapped_axis_applies_border_smudge_before_integration() {
    let mut engine = Engine::new(EngineConfig {
        world_width: 1_000.0,
        world_height: 1_000.0,
        toroidal_world: false,
        axis_topology: Some([true, false]),
        metabolism_cost: 0,
        gravity: [0.0, 0.0],
        drag: 0.0,
        physics: PhysicsSettings {
            surface_gravity: 0.0,
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let organism = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [1.0, 500.0])
        .unwrap();
    engine.tick().unwrap();

    let state = engine.organism(organism).unwrap();
    let expected = (state.radius + 50.0).min(1_000.0 - state.radius);
    let position = state.position;
    assert!(
        (position[0] - expected).abs() < 0.01,
        "expected smudge respawn, got {position:?}"
    );
    assert_eq!(engine.memory_at(organism, 214).unwrap(), 0);
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
    engine.tick().unwrap();

    let shot = &engine.snapshot().shots[0];
    let segment_length = (shot.end[0] - shot.start[0]).hypot(shot.end[1] - shot.start[1]);
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
    assert!(
        shot.velocity[0] > 0.0,
        "wrapped firer velocity was inverted: {shot:?}"
    );
    assert!(
        shot.velocity[0] < 100.0,
        "wrapped firer inherited a world-sized jump: {shot:?}"
    );
}
