use darwinbots_engine::{Engine, EngineConfig, LegacyDna};

#[test]
fn vision_routes_nearest_target_into_directional_eye_sectors() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let observer = engine.spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0]).unwrap();
    engine.spawn_at(LegacyDna::parse("start\nstop").unwrap(), [600.0, 500.0]).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory(observer, "eye5").unwrap(), 0);
    assert!((6..=9).any(|eye| engine.memory(observer, &format!("eye{eye}")).unwrap() > 0));

    engine.replace_dna(observer, LegacyDna::parse("start\n314 .setaim store\nstop").unwrap()).unwrap();
    engine.tick().unwrap();
    assert!(engine.memory(observer, "eye5").unwrap() > 0);
}

#[test]
fn parser_reports_unknown_and_inert_legacy_sysvars_without_rejecting_dna() {
    let dna = LegacyDna::parse("start\n*.thisdoesnotexist 1 add\n10 .mkvirus store\nstop").unwrap();

    let warnings = dna.compatibility_warnings();
    assert!(warnings.iter().any(|warning| warning.contains("thisdoesnotexist") && warning.contains("unknown")));
    assert!(warnings.iter().any(|warning| warning.contains("mkvirus") && warning.contains("inert")));
}
