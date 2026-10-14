use darwinbots_engine::{Engine, EngineConfig, LegacyDna, PhysicsSettings, SaveFile};

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
    let mut config = toroidal_test_config();
    config.world_width = 1_000.0;
    config.world_height = 1_000.0;
    let mut engine = Engine::new(config).unwrap();
    let organism = engine
        .spawn_at(
            LegacyDna::parse("start\n100 .up store\nstop").unwrap(),
            [500.0, 999.0],
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
    assert!((0.0..1_000.0).contains(&position[1]));
    assert!(position[1] < 999.0, "expected y wrap, got {position:?}");
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
    assert!(
        (bounded_state.position[1] - (1_000.0 - bounded_state.radius + bounded_state.velocity[1]))
            .abs()
            < 0.01
    );
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
    let expected = 1_000.0 - state.radius - 50.0 - 1.0;
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

#[test]
fn an_interior_robot_is_not_moved_into_the_border_smudge_band() {
    let mut engine = Engine::new(EngineConfig {
        world_width: 1_000.0,
        world_height: 1_000.0,
        ..toroidal_test_config()
    })
    .unwrap();
    let id = engine
        .spawn_at(LegacyDna::parse("start stop").unwrap(), [850.0, 500.0])
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.organism(id).unwrap().position, [850.0, 500.0]);
}

#[test]
fn wrapped_motion_overshoots_then_respawns_on_the_next_cycle_after_save_restore() {
    let mut engine = Engine::new(EngineConfig {
        world_width: 1_000.0,
        world_height: 1_000.0,
        fixed_bot_radii: true,
        physics: PhysicsSettings {
            max_velocity: 200.0,
            movement_efficiency: 1.0,
            density: 0.0,
            ..Default::default()
        },
        ..toroidal_test_config()
    })
    .unwrap();
    let id = engine
        .spawn_at(
            LegacyDna::parse("cond *.robage 1 < start 200 .up store stop").unwrap(),
            [930.0, 500.0],
        )
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.organism(id).unwrap().position, [1_130.0, 500.0]);
    let mut restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
    restored.tick().unwrap();
    assert_eq!(restored.organism(id).unwrap().position, [311.0, 500.0]);
    assert_eq!(
        restored.robot_state(id).unwrap().kinematics.actual_velocity,
        [200.0, 0.0]
    );
    restored.validate_invariants().unwrap();
}
