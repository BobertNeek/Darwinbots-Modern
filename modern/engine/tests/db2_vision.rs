use darwinbots_engine::{sysvar_address, Engine, EngineConfig, LegacyDna};

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
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [850.0, 500.0])
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
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [850.0, 500.0])
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory_at(observer, 689).unwrap(), 850);
    assert_eq!(engine.memory_at(observer, 690).unwrap(), 500);
}

#[test]
fn focus_eye_publishes_and_resets_references_across_wrapped_targets() {
    let mut config = EngineConfig::testing();
    config.world_width = 1_000.0;
    config.world_height = 1_000.0;
    config.axis_topology = Some([false, true]);
    let mut engine = Engine::new(config).unwrap();
    let observer = engine
        .spawn_at(
            LegacyDna::parse("start\n0 .focuseye store\nstop").unwrap(),
            [500.0, 800.0],
        )
        .unwrap();
    let wrapped_target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 100.0])
        .unwrap();
    engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [900.0, 800.0])
        .unwrap();

    let memloc = sysvar_address("memloc").unwrap() as u16;
    engine.set_organism_memory(wrapped_target, 950, 111).unwrap();
    engine.set_organism_memory(observer, memloc, 0).unwrap();
    assert_eq!(engine.memory(observer, "refxpos").unwrap(), 0);

    engine.tick().unwrap();

    assert!(engine.memory(observer, "eye4").unwrap() > 0);
    let eye5 = engine.memory(observer, "eye5").unwrap();
    assert!(
        eye5 > 0,
        "eye5={eye5} refxpos={} refypos={}",
        engine.memory(observer, "refxpos").unwrap(),
        engine.memory(observer, "refypos").unwrap()
    );
    let wrapped_position = engine.organism(wrapped_target).unwrap().position;
    assert_eq!(engine.memory(observer, "refxpos").unwrap(), wrapped_position[0].round() as i32);
    assert_eq!(engine.memory(observer, "refypos").unwrap(), wrapped_position[1].round() as i32);
    assert_eq!(engine.memory(observer, "refmemval").unwrap(), 0);

    engine.set_organism_memory(observer, memloc, 1001).unwrap();
    engine.tick().unwrap();
    let wrapped_position = engine.organism(wrapped_target).unwrap().position;
    assert_eq!(engine.memory(observer, "refxpos").unwrap(), wrapped_position[0].round() as i32);
    assert_eq!(engine.memory(observer, "refmemval").unwrap(), 0);

    engine.move_organism(wrapped_target, [500.0, 500.0]).unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.memory(observer, "refxpos").unwrap(), 0);
    assert_eq!(engine.memory(observer, "refypos").unwrap(), 0);
    assert_eq!(engine.memory(observer, "refmemval").unwrap(), 0);
}
