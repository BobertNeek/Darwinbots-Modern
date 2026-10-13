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
