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
    // Shots.NewShotCollision wraps the current point once using strict >/<,
    // then updateshots records that corrected start and performs Euler movement.
    // The new endpoint can remain outside until the next serial visit.
    for (position, first_start, next_start) in [
        (95.0, 95.0, 35.0),
        (100.0, 100.0, 40.0),
        (-5.0, 95.0, 35.0),
        (205.0, 105.0, 45.0),
    ] {
        let mut engine = Engine::new(toroidal_test_config()).unwrap();
        let owner = engine
            .spawn_at(
                LegacyDna::parse("start\n-1 .shoot store\nstop").unwrap(),
                [95.0, 50.0],
            )
            .unwrap();
        engine.tick().unwrap();
        engine
            .replace_dna(owner, LegacyDna::parse("start stop").unwrap())
            .unwrap();
        let encoded = SaveFile::encode(&engine).unwrap();
        let mut payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
        let shots = &mut payload["projectiles"];
        assert_eq!(shots["alive"][0], serde_json::json!(true));
        // Keep the valid native pool and owner; replace only this shot's
        // geometry/age/range so launch scatter cannot obscure border timing.
        shots["positions"][0] = serde_json::json!([position, 50.0]);
        shots["previous_positions"][0] = serde_json::json!([position - 40.0, 50.0]);
        shots["velocities"][0] = serde_json::json!([40.0, 0.0]);
        shots["ages"][0] = serde_json::json!(0);
        shots["ranges"][0] = serde_json::json!(20.0);
        let payload = serde_json::to_vec(&payload).unwrap();
        let mut save = encoded[..6].to_vec();
        save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        save.extend_from_slice(&payload);
        let mut engine = SaveFile::decode(&save).unwrap();
        engine.validate_invariants().unwrap();

        engine.tick().unwrap();
        assert_eq!(engine.snapshot().shots.len(), 1);
        let shot = &engine.snapshot().shots[0];
        assert_eq!(shot.start, [first_start, 50.0]);
        assert_eq!(shot.end, [first_start + 40.0, 50.0]);
        assert_eq!(shot.velocity, [40.0, 0.0]);
        assert_eq!(shot.age, 1);
        assert!(!shot.impact_flash);
        let segment_length = (shot.end[0] - shot.start[0]).hypot(shot.end[1] - shot.start[1]);
        let velocity_length = shot.velocity[0].hypot(shot.velocity[1]);
        assert_eq!(segment_length, velocity_length);
        assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
        let mut restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        engine.tick().unwrap();
        restored.tick().unwrap();
        let shot = &engine.snapshot().shots[0];
        assert_eq!(shot.start, [next_start, 50.0]);
        assert_eq!(shot.end, [next_start + 40.0, 50.0]);
        assert_eq!(shot.velocity, [40.0, 0.0]);
        assert_eq!(shot.age, 2);
        assert_eq!(engine.snapshot().shots, restored.snapshot().shots);
        restored.validate_invariants().unwrap();
    }
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
