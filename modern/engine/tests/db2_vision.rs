use darwinbots_engine::{Engine, EngineConfig, LegacyDna};

fn vision_engine() -> Engine {
    Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap()
}

#[test]
fn eye_direction_moves_detection_into_eye_five() {
    let mut engine = vision_engine();
    let observer = engine
        .spawn_at(
            LegacyDna::parse("start\n314 .eye5dir store\nstop").unwrap(),
            [500.0, 500.0],
        )
        .unwrap();
    engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 150.0])
        .unwrap();

    engine.tick().unwrap();

    assert!(engine.memory_at(observer, 505).unwrap() > 0);
    let observer = engine.organism(observer).unwrap();
    assert_eq!(observer.vision.eyes[4].direction, 314);
    assert!(observer.vision.eyes[4].value > 0);
}

#[test]
fn eye_snapshot_contains_width_range_and_default_focus() {
    let mut engine = vision_engine();
    let observer = engine
        .spawn_at(
            LegacyDna::parse("start\n70 .eye5width store\n0 .focuseye store\nstop").unwrap(),
            [500.0, 500.0],
        )
        .unwrap();

    engine.tick().unwrap();

    let organism = engine.organism(observer).unwrap();
    assert_eq!(organism.vision.focus_eye, 4);
    assert_eq!(organism.vision.eyes[4].width, 70);
    assert!(organism.vision.eyes[4].half_width_radians > 0.0);
    assert!(organism.vision.eyes[4].range > 0.0);
}

#[test]
fn focus_eye_selects_reference_target() {
    let mut engine = vision_engine();
    let observer = engine
        .spawn_at(
            LegacyDna::parse("start\n314 .eye5dir store\n0 .focuseye store\nstop").unwrap(),
            [500.0, 500.0],
        )
        .unwrap();
    engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 150.0])
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory_at(observer, 689).unwrap(), 500);
    assert_eq!(engine.memory_at(observer, 690).unwrap(), 150);
}

#[test]
fn native_vision_uses_actual_positions_and_resets_references_when_target_leaves_focus() {
    let mut config = EngineConfig::testing();
    config.world_width = 3_000.0;
    config.world_height = 3_000.0;
    config.axis_topology = Some([false, true]);
    let mut engine = Engine::new(config).unwrap();
    let observer = engine
        .spawn_at(
            LegacyDna::parse("start 942 .setaim store stop").unwrap(),
            [1500.0, 2700.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start stop").unwrap(), [1500.0, 300.0])
        .unwrap();
    engine.set_organism_memory(target, 950, 111).unwrap();
    engine.tick().unwrap();
    // Quads.CompareRobots3 uses VectorSub of real positions, with no periodic image.
    assert_eq!(engine.memory(observer, "eye5").unwrap(), 0);
    assert_eq!(engine.memory(observer, "refxpos").unwrap(), 0);
    engine
        .replace_dna(
            observer,
            LegacyDna::parse("start 314 .setaim store 950 .memloc store stop").unwrap(),
        )
        .unwrap();
    engine.move_organism(target, [1500.0, 2100.0]).unwrap();
    engine.tick().unwrap();
    assert!(engine.memory(observer, "eye5").unwrap() > 0);
    assert_eq!(engine.memory(observer, "refxpos").unwrap(), 1500);
    assert_eq!(engine.memory(observer, "refypos").unwrap(), 2100);
    assert_eq!(engine.memory(observer, "memval").unwrap(), 111);
    engine.move_organism(target, [2200.0, 2700.0]).unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.memory(observer, "refxpos").unwrap(), 0);
    assert_eq!(engine.memory(observer, "refypos").unwrap(), 0);
    assert_eq!(engine.memory(observer, "memval").unwrap(), 0);
}
