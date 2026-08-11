mod support;

use darwinbots_engine::{
    DnaCosts, Engine, EngineConfig, LegacyDna, PhysicsSettings, SaveFile, VegetationSettings,
};
use support::db2_fixtures::{DEFAULT_MAX_VELOCITY, DEFAULT_MOVEMENT_EFFICIENCY};

#[test]
fn newly_loaded_bots_start_with_db2_body_reserve() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let id = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
        .unwrap();

    assert_eq!(engine.organism(id).unwrap().body, 1_000);
}

#[test]
fn physical_radius_is_derived_from_body_like_db2() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
        .unwrap();

    let radius = engine.snapshot().render_instances[0].radius;
    assert!(
        (radius - 114.28).abs() < 0.1,
        "expected DB2 radius 114.28, got {radius}"
    );
}

#[test]
fn fixed_bot_radii_config_survives_save_and_uses_db2_half_radius() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
        .unwrap();

    let encoded = SaveFile::encode(&engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    payload["config"]["fixed_bot_radii"] = serde_json::json!(true);
    let payload = serde_json::to_vec(&payload).unwrap();

    let mut fixed_radius_save = Vec::with_capacity(10 + payload.len());
    fixed_radius_save.extend_from_slice(&encoded[..6]);
    fixed_radius_save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    fixed_radius_save.extend_from_slice(&payload);

    let restored = SaveFile::decode(&fixed_radius_save).unwrap();
    let radius = restored.snapshot().render_instances[0].radius;
    assert_eq!(radius, 60.0);
}

#[test]
fn movement_command_adds_impulse_and_bot_coasts_without_new_thrust() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        drag: 0.0,
        physics: PhysicsSettings {
            surface_gravity: 0.0,
            density: 0.0,
            ..PhysicsSettings::default()
        },
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
        physics: PhysicsSettings {
            surface_gravity: 0.0,
            density: 0.0,
            ..PhysicsSettings::default()
        },
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
    assert!(LegacyDna::parse(&dna.to_source()).unwrap().uses_new_move());
}

#[test]
fn voluntary_motion_cost_uses_the_clipped_acceleration_magnitude() {
    let mut config = EngineConfig::testing();
    config.metabolism_cost = 0;
    config.gravity = [0.0, 0.0];
    config.drag = 0.0;
    config.physics.movement_efficiency = 1.0;
    config.physics.surface_gravity = 0.0;
    config.physics.static_friction = 0.0;
    config.physics.kinetic_friction = 0.0;
    config.dna_costs = DnaCosts {
        movement: 1.0,
        ..DnaCosts::default()
    };
    let mut engine = Engine::new(config).unwrap();
    let id = engine
        .spawn_at(
            LegacyDna::parse("start\n10000 .up store\nstop").unwrap(),
            [500.0, 500.0],
        )
        .unwrap();
    engine.set_organism_energy(id, 32_000).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.organism(id).unwrap().energy, 31_820);
    assert!((engine.organism(id).unwrap().velocity[1] - 180.0).abs() < 0.01);
}

#[test]
fn fixed_and_corpse_flags_suppress_voluntary_motion_and_energy_cost() {
    let mut config = EngineConfig::testing();
    config.metabolism_cost = 0;
    config.gravity = [0.0, 0.0];
    config.drag = 0.0;
    config.physics.movement_efficiency = 1.0;
    config.physics.surface_gravity = 0.0;
    config.physics.static_friction = 0.0;
    config.physics.kinetic_friction = 0.0;
    config.dna_costs = DnaCosts {
        movement: 1.0,
        ..DnaCosts::default()
    };
    let mut engine = Engine::new(config).unwrap();
    let fixed = engine
        .spawn_at(
            LegacyDna::parse("start\n1 .fixpos store\n100 .up store\nstop").unwrap(),
            [500.0, 500.0],
        )
        .unwrap();
    let corpse = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [1_000.0, 500.0])
        .unwrap();
    let encoded = SaveFile::encode(&engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    let slots = payload["slots"].as_array_mut().unwrap();
    slots[corpse.slot() as usize]["organism"]["flags"]["corpse"] = true.into();
    slots[corpse.slot() as usize]["organism"]["flags"]["can_execute_dna"] = false.into();
    slots[corpse.slot() as usize]["organism"]["flags"]["can_move"] = true.into();
    let payload = serde_json::to_vec(&payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(&encoded[..6]);
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    let mut engine = SaveFile::decode(&save).unwrap();
    engine.set_organism_memory(corpse, 1, 100).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.organism(fixed).unwrap().energy, 1_000);
    assert_eq!(engine.organism(fixed).unwrap().position, [500.0, 500.0]);
    assert_eq!(engine.organism(corpse).unwrap().energy, 1_000);
    assert_eq!(engine.organism(corpse).unwrap().position, [1_000.0, 500.0]);
}

#[test]
fn drag_reduces_retained_momentum_instead_of_replacing_it() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        drag: 0.25,
        physics: PhysicsSettings {
            surface_gravity: 0.0,
            density: 0.0,
            ..PhysicsSettings::default()
        },
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
fn brownian_motion_rotates_aim_on_the_next_cycle() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        brownian_motion: 100.0,
        drag: 0.0,
        physics: PhysicsSettings {
            surface_gravity: 0.0,
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let id = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
        .unwrap();

    engine.tick().unwrap();
    engine.tick().unwrap();

    assert_ne!(engine.organism(id).unwrap().aim, 0);
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
    assert!(
        organisms
            .iter()
            .flat_map(|bot| bot.position)
            .all(f32::is_finite)
    );
    assert!(
        organisms
            .iter()
            .flat_map(|bot| bot.velocity)
            .all(f32::is_finite)
    );
    assert!(organisms[0].velocity[0] < 0.0);
    assert!(organisms[1].velocity[0] > 0.0);
}

#[test]
fn static_friction_blocks_impulses_below_the_breakaway_threshold() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        drag: 0.0,
        physics: PhysicsSettings {
            max_velocity: 180.0,
            movement_efficiency: 1.0,
            surface_gravity: 2.0,
            static_friction: 0.6,
            kinetic_friction: 0.0,
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let id = engine
        .spawn_at(
            LegacyDna::parse("cond\n*.robage 1 <\nstart\n1 .up store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.organism(id).unwrap().velocity, [0.0, 0.0]);
}

#[test]
fn static_friction_is_applied_after_collisions_to_cancel_the_post_contact_impulse() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        gravity: [0.0, 0.0],
        brownian_motion: 0.0,
        drag: 0.0,
        physics: PhysicsSettings {
            max_velocity: 200.0,
            movement_efficiency: 1.0,
            surface_gravity: 100.0,
            static_friction: 0.6,
            kinetic_friction: 0.0,
            density: 0.0,
            elasticity: 0.25,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    engine
        .spawn_batch([
            (
                LegacyDna::parse("start\n100 .dx store\nstop").unwrap(),
                [500.0, 500.0],
            ),
            (
                LegacyDna::parse("start\n100 .sx store\nstop").unwrap(),
                [700.0, 500.0],
            ),
        ])
        .unwrap();

    engine.tick().unwrap();

    let velocities: Vec<_> = engine
        .snapshot()
        .organisms
        .iter()
        .map(|organism| organism.velocity)
        .collect();
    assert!(
        velocities.iter().all(|velocity| *velocity == [0.0, 0.0]),
        "post-collision static friction must cancel the residual contact impulse; actual velocities: {velocities:?}"
    );
}

#[test]
fn maximum_velocity_caps_total_velocity_after_environment_impulses() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        gravity: [0.0, 100.0],
        drag: 0.0,
        physics: PhysicsSettings {
            max_velocity: 10.0,
            movement_efficiency: 1.0,
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let id = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [100.0, 100.0])
        .unwrap();

    engine.tick().unwrap();

    let velocity = engine.organism(id).unwrap().velocity;
    assert!(
        velocity[0].hypot(velocity[1]) <= 10.001,
        "velocity: {velocity:?}"
    );
}

#[test]
fn pond_buoyancy_reverses_vertical_gravity_at_the_bottom() {
    let config = EngineConfig {
        gravity: [0.0, 6.2],
        toroidal_world: false,
        physics: PhysicsSettings {
            density: 0.0,
            surface_gravity: 0.0,
            static_friction: 0.0,
            kinetic_friction: 0.0,
            ..PhysicsSettings::default()
        },
        vegetation: VegetationSettings {
            pond_mode: true,
            ..VegetationSettings::default()
        },
        ..EngineConfig::testing()
    };
    let mut sinking = Engine::new(config.clone()).unwrap();
    let sinker = sinking
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 900.0])
        .unwrap();
    sinking.tick().unwrap();
    sinking.tick().unwrap();

    let mut floating = Engine::new(config).unwrap();
    let floater = floating
        .spawn_at(
            LegacyDna::parse("start\n32000 .setboy store\nstop").unwrap(),
            [500.0, 900.0],
        )
        .unwrap();
    floating.tick().unwrap();
    floating.tick().unwrap();
    floating.tick().unwrap();

    assert!(sinking.organism(sinker).unwrap().velocity[1] > 0.0);
    assert!(
        floating.organism(floater).unwrap().velocity[1] < 0.0,
        "floating velocity: {:?}",
        floating.organism(floater).unwrap().velocity
    );
}

#[test]
fn pond_full_buoyancy_sinks_shallow_but_floats_deep_when_raw_threshold_exceeds_one() {
    let config = EngineConfig {
        gravity: [0.0, 6.2],
        world_width: 2_000.0,
        world_height: 1_000.0,
        toroidal_world: false,
        brownian_motion: 0.0,
        drag: 0.0,
        physics: PhysicsSettings {
            density: 0.0,
            surface_gravity: 0.0,
            static_friction: 0.0,
            kinetic_friction: 0.0,
            ..PhysicsSettings::default()
        },
        vegetation: VegetationSettings {
            pond_mode: true,
            tides: 2,
            ..VegetationSettings::default()
        },
        ..EngineConfig::testing()
    };
    let mut engine = Engine::new(config).unwrap();
    let dna = LegacyDna::parse("start\n32000 .setboy store\nstop").unwrap();
    let shallow = engine.spawn_at(dna.clone(), [500.0, 150.0]).unwrap();
    let deep = engine.spawn_at(dna, [1_500.0, 850.0]).unwrap();

    engine.tick().unwrap();
    engine.tick().unwrap();
    engine.tick().unwrap();

    let shallow_velocity = engine.organism(shallow).unwrap().velocity[1];
    let deep_velocity = engine.organism(deep).unwrap().velocity[1];
    assert!(
        shallow_velocity > 0.0,
        "raw buoyancy threshold above one must still sink shallow full-buoyancy robot: {shallow_velocity}"
    );
    assert!(
        deep_velocity < 0.0,
        "deeper full-buoyancy robot must float at the same tide phase: {deep_velocity}"
    );
}

#[test]
fn pond_tides_modulate_vertical_gravity_over_the_configured_period() {
    let config = EngineConfig {
        gravity: [0.0, 6.2],
        toroidal_world: false,
        vegetation: VegetationSettings {
            pond_mode: true,
            tides: 4,
            ..VegetationSettings::default()
        },
        physics: PhysicsSettings {
            density: 0.0,
            surface_gravity: 0.0,
            static_friction: 0.0,
            kinetic_friction: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    };
    let mut engine = Engine::new(config).unwrap();
    let id = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 900.0])
        .unwrap();

    engine.tick().unwrap();
    let high_tide_velocity = engine.organism(id).unwrap().velocity[1];
    engine.tick().unwrap();
    let falling_velocity = engine.organism(id).unwrap().velocity[1];

    assert!(
        high_tide_velocity < 1.0 && falling_velocity > high_tide_velocity + 0.5,
        "high tide velocity: {high_tide_velocity}, falling velocity: {falling_velocity}"
    );
}

#[test]
fn non_toroidal_border_collision_writes_edge_sense() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        toroidal_world: false,
        gravity: [0.0, 0.0],
        drag: 0.0,
        brownian_motion: 0.0,
        physics: PhysicsSettings {
            density: 0.0,
            surface_gravity: 0.0,
            static_friction: 0.0,
            kinetic_friction: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let left = engine
        .spawn_at(
            LegacyDna::parse("start\n-100 .dx store\nstop").unwrap(),
            [0.0, 500.0],
        )
        .unwrap();
    let right = engine
        .spawn_at(
            LegacyDna::parse("start\n100 .dx store\nstop").unwrap(),
            [1_000.0, 500.0],
        )
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory_at(left, 214).unwrap(), 1);
    assert_eq!(engine.memory_at(right, 214).unwrap(), 1);
}

#[test]
fn fluid_added_mass_reduces_acceleration_and_angular_damping_is_applied_in_tick() {
    let config = |density: f64, brownian_motion: f32| EngineConfig {
        metabolism_cost: 0,
        gravity: [0.0, 10.0],
        brownian_motion,
        drag: 0.0,
        physics: PhysicsSettings {
            surface_gravity: 0.0,
            density,
            viscosity: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    };

    let mut dry = Engine::new(config(0.0, 0.0)).unwrap();
    let dry_id = dry
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
        .unwrap();
    dry.tick().unwrap();

    let mut wet = Engine::new(config(0.000002, 0.0)).unwrap();
    let wet_id = wet
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
        .unwrap();
    wet.tick().unwrap();

    assert!(
        wet.organism(wet_id).unwrap().velocity[1] < dry.organism(dry_id).unwrap().velocity[1] * 0.5,
        "fluid added mass must reduce gravity acceleration"
    );

    let mut dry_rotation = Engine::new(config(0.0, 100.0)).unwrap();
    let dry_rotation_id = dry_rotation
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
        .unwrap();
    dry_rotation.tick().unwrap();
    dry_rotation.tick().unwrap();

    let mut wet_rotation = Engine::new(config(0.000002, 100.0)).unwrap();
    let wet_rotation_id = wet_rotation
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
        .unwrap();
    wet_rotation.tick().unwrap();
    wet_rotation.tick().unwrap();

    assert_ne!(dry_rotation.organism(dry_rotation_id).unwrap().aim, 0);
    assert_eq!(wet_rotation.organism(wet_rotation_id).unwrap().aim, 0);
}
