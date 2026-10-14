use darwinbots_engine::{
    Engine, EngineConfig, LegacyDna, Obstacle, ShapeShotPolicy, ShapeVisionPolicy,
    SpeciesDefinition, Teleporter, TeleporterFilters,
};

#[test]
fn gravity_and_drag_affect_motion() {
    let mut config = EngineConfig::testing();
    config.gravity = [0.0, 2.0];
    config.drag = 0.5;
    config.physics.density = 0.0;
    let mut engine = Engine::new(config).unwrap();
    let id = engine
        .spawn_at(
            LegacyDna::parse("start\n10 .up store\nstop").unwrap(),
            [300.0, 500.0],
        )
        .unwrap();

    engine.tick().unwrap();

    let position = engine.organism(id).unwrap().position;
    assert!((position[0] - 303.3).abs() < 0.01);
    assert!((position[1] - 502.0).abs() < 0.01);
}

#[test]
fn planet_eaters_apply_mutual_db2_mass_scaled_attraction() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        gravity: [0.0, 0.0],
        physics: darwinbots_engine::PhysicsSettings {
            planet_eaters: true,
            planet_eaters_g: 1_000.0,
            surface_gravity: 0.0,
            static_friction: 0.0,
            kinetic_friction: 0.0,
            ..darwinbots_engine::PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let first = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [300.0, 500.0])
        .unwrap();
    let second = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [700.0, 500.0])
        .unwrap();

    engine.tick().unwrap();

    assert!(engine.organism(first).unwrap().velocity[0] > 0.0);
    assert!(engine.organism(second).unwrap().velocity[0] < 0.0);
}

#[test]
fn obstacles_reject_organisms_and_teleporters_relocate_them() {
    let mut obstacle_engine = Engine::new(EngineConfig::testing()).unwrap();
    obstacle_engine
        .add_obstacle(Obstacle::new(1, [320.0, 480.0], [380.0, 560.0]))
        .unwrap();
    let mover = obstacle_engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [333.0, 500.0])
        .unwrap();
    obstacle_engine.tick().unwrap();
    let obstacle_position = obstacle_engine.organism(mover).unwrap().position;
    assert!(
        obstacle_position[0] <= 320.0
            || obstacle_position[0] >= 380.0
            || obstacle_position[1] <= 480.0
            || obstacle_position[1] >= 560.0,
        "position: {obstacle_position:?}"
    );

    let mut teleporter_engine = Engine::new(EngineConfig::testing()).unwrap();
    teleporter_engine
        .add_teleporter(Teleporter {
            id: 1,
            center: [350.0, 500.0],
            radius: 30.0,
            destination: [800.0, 700.0],
            ..Teleporter::new(1, [350.0, 500.0], 30.0, [800.0, 700.0])
        })
        .unwrap();
    let traveler = teleporter_engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [350.0, 500.0])
        .unwrap();
    teleporter_engine.tick().unwrap();
    assert_eq!(
        teleporter_engine.organism(traveler).unwrap().position,
        [800.0, 700.0]
    );
    assert_eq!(teleporter_engine.snapshot().obstacles.len(), 0);
    assert_eq!(teleporter_engine.snapshot().teleporters.len(), 1);
}

#[test]
fn opaque_obstacle_blocks_vision_of_a_target_behind_it() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let observer = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [300.0, 500.0])
        .unwrap();
    engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [650.0, 500.0])
        .unwrap();
    engine
        .add_obstacle(Obstacle::new(1, [450.0, 450.0], [500.0, 550.0]))
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(
        engine.memory_at(observer, 505).unwrap(),
        0,
        "an opaque obstacle must occlude the target from eye five"
    );
}

#[test]
fn shape_drift_moves_the_obstacle_before_publication() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let mut obstacle = Obstacle::new(1, [80.0, 190.0], [120.0, 210.0]);
    obstacle.drift = [3.0, -2.0];
    engine.add_obstacle(obstacle).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.snapshot().obstacles[0].minimum, [83.0, 188.0]);
    assert_eq!(engine.snapshot().obstacles[0].maximum, [123.0, 208.0]);
}

#[test]
fn shape_editor_properties_persist_through_the_authoritative_engine() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine
        .add_obstacle(Obstacle::new(1, [80.0, 190.0], [120.0, 210.0]))
        .unwrap();
    let mut edited = engine.snapshot().obstacles[0].clone();
    edited.color = 0xff112233;
    edited.opacity = 0.35;
    edited.drift = [1.5, -0.5];
    edited.vision_policy = ShapeVisionPolicy::SeeThrough;
    edited.shot_policy = ShapeShotPolicy::PassThrough;

    engine.update_obstacle(edited.clone()).unwrap();

    assert_eq!(&engine.snapshot().obstacles[0], &edited);
}

#[test]
fn see_through_shape_preserves_vision_of_a_target_behind_it() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let observer = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [300.0, 500.0])
        .unwrap();
    engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [650.0, 500.0])
        .unwrap();
    let mut obstacle = Obstacle::new(1, [450.0, 450.0], [500.0, 550.0]);
    obstacle.vision_policy = ShapeVisionPolicy::SeeThrough;
    engine.add_obstacle(obstacle).unwrap();

    engine.tick().unwrap();

    assert!(engine.memory_at(observer, 505).unwrap() > 0);
}

#[test]
fn brownian_motion_can_be_disabled_while_running() {
    let mut engine = Engine::new(EngineConfig {
        brownian_motion: 25.0,
        ..EngineConfig::testing()
    })
    .unwrap();
    assert_eq!(engine.brownian_motion(), 25.0);

    engine.set_brownian_motion(0.0).unwrap();

    assert_eq!(engine.brownian_motion(), 0.0);
}

#[test]
fn complete_environment_settings_can_change_while_running() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine
        .update_environment(0, 10, 200, [0.0, 2.0], 0.5, 3.0)
        .unwrap();
    let vegetable = engine.register_species(SpeciesDefinition {
        vegetable: true,
        ..SpeciesDefinition::default()
    });
    let id = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetable,
            [300.0, 500.0],
        )
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.brownian_motion(), 3.0);
    assert_eq!(engine.organism(id).unwrap().energy, 1_050);
    assert_eq!(engine.snapshot().stats.plant_energy_generated, 39);
}

#[test]
fn intrasim_teleporter_applies_filters_and_counts_capacity_per_transfer() {
    let mut engine = Engine::new(EngineConfig {
        fixed_bot_radii: true,
        metabolism_cost: 0,
        gravity: [0.0, 0.0],
        drag: 0.0,
        brownian_motion: 0.0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        vegetable: true,
        ..SpeciesDefinition::default()
    });
    let accepted = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [300.0, 500.0])
        .unwrap();
    let over_capacity = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [310.0, 500.0])
        .unwrap();
    let filtered_vegetable = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetables,
            [320.0, 500.0],
        )
        .unwrap();
    let mut teleporter = Teleporter::new(1, [300.0, 500.0], 40.0, [800.0, 800.0]);
    teleporter.capacity = 1;
    teleporter.filters = TeleporterFilters {
        teleport_vegetables: false,
        teleport_corpses: false,
        teleport_heterotrophs: true,
    };
    engine.add_teleporter(teleporter).unwrap();

    engine.tick().unwrap();

    let accepted_position = engine.organism(accepted).unwrap().position;
    assert!(
        accepted_position[0] > 700.0 && accepted_position[1] > 700.0,
        "accepted organism did not reach the destination side: {accepted_position:?}"
    );
    assert!(engine.organism(over_capacity).unwrap().position[0] < 400.0);
    assert!(engine.organism(filtered_vegetable).unwrap().position[0] < 400.0);

    let mut corpse_engine = Engine::new(EngineConfig {
        corpse_decay: darwinbots_engine::CorpseDecaySettings {
            enabled: true,
            packet_size: 0,
            ..Default::default()
        },
        metabolism_cost: 2_000,
        gravity: [0.0, 0.0],
        drag: 0.0,
        brownian_motion: 0.0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let mut corpse_teleporter = Teleporter::new(2, [300.0, 500.0], 40.0, [800.0, 800.0]);
    corpse_teleporter.filters = TeleporterFilters {
        teleport_vegetables: false,
        teleport_corpses: true,
        teleport_heterotrophs: false,
    };
    corpse_engine.add_teleporter(corpse_teleporter).unwrap();
    corpse_engine
        .spawn_at(LegacyDna::parse("stop").unwrap(), [300.0, 500.0])
        .unwrap();
    corpse_engine.tick().unwrap();
    corpse_engine.tick().unwrap();

    assert_eq!(corpse_engine.snapshot().corpses.len(), 1);
    assert_eq!(corpse_engine.snapshot().corpses[0].position, [800.0, 800.0]);
}

#[test]
fn intrasim_teleporter_respects_shape_blocking() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        gravity: [0.0, 0.0],
        drag: 0.0,
        brownian_motion: 0.0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let mut teleporter = Teleporter::new(1, [300.0, 500.0], 40.0, [800.0, 800.0]);
    teleporter.respect_shapes = true;
    engine.add_teleporter(teleporter).unwrap();
    engine
        .add_obstacle(Obstacle::new(1, [280.0, 480.0], [320.0, 520.0]))
        .unwrap();
    let traveler = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [300.0, 500.0])
        .unwrap();

    engine.tick().unwrap();

    assert_ne!(engine.organism(traveler).unwrap().position, [800.0, 800.0]);
}

#[test]
fn intrasim_teleporter_blocks_immediate_paired_backflow() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        gravity: [0.0, 0.0],
        drag: 0.0,
        brownian_motion: 0.0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let mut outbound = Teleporter::new(1, [300.0, 500.0], 30.0, [700.0, 500.0]);
    outbound.backflow_limit = 1;
    let mut inbound = Teleporter::new(2, [700.0, 500.0], 30.0, [300.0, 500.0]);
    inbound.backflow_limit = 1;
    engine.add_teleporter(outbound).unwrap();
    engine.add_teleporter(inbound).unwrap();
    let traveler = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [300.0, 500.0])
        .unwrap();

    engine.tick().unwrap();
    assert_eq!(engine.organism(traveler).unwrap().position, [700.0, 500.0]);

    engine.tick().unwrap();

    assert_eq!(
        engine.organism(traveler).unwrap().position,
        [700.0, 500.0],
        "a paired teleporter must not immediately send a traveler back"
    );
}

#[test]
fn outbound_only_teleporter_does_not_relocate_intrasim_organisms() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        gravity: [0.0, 0.0],
        drag: 0.0,
        brownian_motion: 0.0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let mut teleporter = Teleporter::new(1, [300.0, 500.0], 30.0, [700.0, 500.0]);
    teleporter.local = false;
    teleporter.inbound = false;
    teleporter.outbound = true;
    engine.add_teleporter(teleporter).unwrap();
    let traveler = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [300.0, 500.0])
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(
        engine.organism(traveler).unwrap().position,
        [300.0, 500.0],
        "outbound-only gateways are not intrasim relocation routes"
    );
}
