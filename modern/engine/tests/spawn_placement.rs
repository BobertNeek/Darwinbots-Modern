use darwinbots_engine::{Engine, EngineConfig, LegacyDna};

#[test]
fn reproduction_places_children_at_seeded_random_offsets() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    }).unwrap();
    let parent = engine.spawn_at(
        LegacyDna::parse("start\n50 .repro store\nstop").unwrap(),
        [500.0, 500.0],
    ).unwrap();

    engine.tick().unwrap();

    let child = engine.snapshot().organisms.iter().find(|organism| organism.id != parent).unwrap();
    let dx = child.position[0] - 500.0;
    let dy = child.position[1] - 500.0;
    let distance = (dx * dx + dy * dy).sqrt();
    assert!((12.0..=32.0).contains(&distance));
    assert_ne!(child.position, [508.0, 500.0]);
}
