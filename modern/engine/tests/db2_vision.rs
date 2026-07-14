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
        .spawn_at(
            LegacyDna::parse("start\nstop").unwrap(),
            [850.0, 500.0],
        )
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
            LegacyDna::parse(
                "start\n70 .eye5width store\n0 .focuseye store\nstop",
            )
            .unwrap(),
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
            LegacyDna::parse(
                "start\n314 .eye5dir store\n0 .focuseye store\nstop",
            )
            .unwrap(),
            [500.0, 500.0],
        )
        .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\nstop").unwrap(),
            [850.0, 500.0],
        )
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory_at(observer, 689).unwrap(), 850);
    assert_eq!(engine.memory_at(observer, 690).unwrap(), 500);
}
