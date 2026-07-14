use darwinbots_engine::{Engine, EngineConfig, LegacyDna, Obstacle, PhysicsSettings, SpeciesDefinition, Teleporter};

#[test]
fn gravity_and_drag_affect_motion() {
    let mut engine = Engine::new(EngineConfig {
        gravity: [0.0, 2.0],
        drag: 0.5,
        physics: PhysicsSettings { density: 0.0, ..PhysicsSettings::default() },
        ..EngineConfig::testing()
    }).unwrap();
    let id = engine.spawn_at(LegacyDna::parse("start\n10 .up store\nstop").unwrap(), [100.0, 100.0]).unwrap();

    engine.tick().unwrap();

    let position = engine.organism(id).unwrap().position;
    assert!((position[0] - 100.0).abs() < 0.01);
    assert!((position[1] - 104.3).abs() < 0.01);
}

#[test]
fn obstacles_reject_organisms_and_teleporters_relocate_them() {
    let mut obstacle_engine = Engine::new(EngineConfig::testing()).unwrap();
    obstacle_engine.add_obstacle(Obstacle { id: 1, minimum: [120.0, 80.0], maximum: [180.0, 160.0] }).unwrap();
    let mover = obstacle_engine.spawn_at(LegacyDna::parse("start\n50 .dx store\nstop").unwrap(), [100.0, 100.0]).unwrap();
    obstacle_engine.tick().unwrap();
    let obstacle_position = obstacle_engine.organism(mover).unwrap().position;
    assert!(obstacle_position[0] <= 120.0 || obstacle_position[0] >= 180.0
        || obstacle_position[1] <= 80.0 || obstacle_position[1] >= 160.0,
        "position: {obstacle_position:?}");

    let mut teleporter_engine = Engine::new(EngineConfig::testing()).unwrap();
    teleporter_engine.add_teleporter(Teleporter { id: 1, center: [150.0, 100.0], radius: 30.0, destination: [800.0, 700.0] }).unwrap();
    let traveler = teleporter_engine.spawn_at(LegacyDna::parse("start\n50 .dx store\nstop").unwrap(), [100.0, 100.0]).unwrap();
    teleporter_engine.tick().unwrap();
    assert_eq!(teleporter_engine.organism(traveler).unwrap().position, [800.0, 700.0]);
    assert_eq!(teleporter_engine.snapshot().obstacles.len(), 0);
    assert_eq!(teleporter_engine.snapshot().teleporters.len(), 1);
}

#[test]
fn brownian_motion_can_be_disabled_while_running() {
    let mut engine = Engine::new(EngineConfig { brownian_motion: 25.0, ..EngineConfig::testing() }).unwrap();
    assert_eq!(engine.brownian_motion(), 25.0);

    engine.set_brownian_motion(0.0).unwrap();

    assert_eq!(engine.brownian_motion(), 0.0);
}

#[test]
fn complete_environment_settings_can_change_while_running() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine.update_environment(0, 10, 200, [0.0, 2.0], 0.5, 3.0).unwrap();
    let vegetable = engine.register_species(SpeciesDefinition { vegetable: true, ..SpeciesDefinition::default() });
    let id = engine.spawn_species_at(LegacyDna::parse("start\nstop").unwrap(), vegetable, [100.0, 100.0]).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.brownian_motion(), 3.0);
    assert_eq!(engine.organism(id).unwrap().energy, 1_029);
    assert_eq!(engine.snapshot().stats.plant_energy_generated, 19);
}
