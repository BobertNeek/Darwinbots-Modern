use darwinbots_engine::{Engine, EngineConfig, LegacyDna};

#[test]
fn reproduction_places_children_forward_at_native_fixed_radius_separation() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        fixed_bot_radii: true,
        ..EngineConfig::testing()
    })
    .unwrap();
    let parent = engine
        .spawn_at(
            LegacyDna::parse("start\n50 .repro store\nstop").unwrap(),
            [500.0, 500.0],
        )
        .unwrap();

    engine.set_organism_aim(parent, 0).unwrap();
    engine.tick().unwrap();

    let child = engine
        .snapshot()
        .organisms
        .iter()
        .find(|organism| organism.id != parent)
        .unwrap();
    // Robots.Reproduce assigns the summed radii to a Long, then places forward.
    // FindRadius returns 60 for each fixed-radius robot; no random offset is used.
    assert_eq!(child.position, [620.0, 500.0]);
}

#[test]
fn newborn_separation_rounds_the_native_sum_to_a_long_before_placement() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let parent = engine
        .spawn_at(
            LegacyDna::parse("start 50 .repro store stop").unwrap(),
            [500.0, 500.0],
        )
        .unwrap();
    engine.tick().unwrap();
    let child = engine
        .snapshot()
        .organisms
        .iter()
        .find(|organism| organism.parents[0] == Some(parent))
        .unwrap();
    // Source FindRadius(1000 body * 0.5) is Single 87.561775; sondist is Long 175.
    assert_eq!(child.position, [675.0, 500.0]);
}

#[test]
fn newborn_placement_follows_native_forward_bearings_in_all_four_quadrants() {
    for (aim, expected) in [
        (0, [620.0, 500.0]),
        (314, [500.09555, 380.00003]),
        (628, [380.00015, 499.8089]),
        (942, [499.71335, 619.99963]),
    ] {
        let mut engine = Engine::new(EngineConfig {
            metabolism_cost: 0,
            fixed_bot_radii: true,
            ..EngineConfig::testing()
        })
        .unwrap();
        let parent = engine
            .spawn_at(
                LegacyDna::parse(&format!("start {aim} .setaim store 50 .repro store stop"))
                    .unwrap(),
                [500.0, 500.0],
            )
            .unwrap();
        engine.tick().unwrap();
        let child = engine
            .snapshot()
            .organisms
            .iter()
            .find(|organism| organism.parents[0] == Some(parent))
            .unwrap();
        assert!(
            (child.position[0] - expected[0]).abs() < 0.0001,
            "aim={aim}: {:?}",
            child.position
        );
        assert!(
            (child.position[1] - expected[1]).abs() < 0.0001,
            "aim={aim}: {:?}",
            child.position
        );
    }
}
