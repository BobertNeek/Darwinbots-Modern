use darwinbots_engine::{
    CycleEvent, CyclePhase, Engine, EngineConfig, LegacyDna, LocalModeConfig, LocalModeKind,
    QueueChangeKind, SaveFile, SpeciesDefinition,
};

fn restore_with_bodies(engine: &Engine, values: &[(darwinbots_engine::OrganismId, i32)]) -> Engine {
    let encoded = SaveFile::encode(engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    for (id, body) in values {
        let materials = &mut payload["slots"][id.slot() as usize]["organism"]["materials"];
        materials["body"] = serde_json::json!(body);
    }
    let payload = serde_json::to_vec(&payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(&encoded[..6]);
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    SaveFile::decode(&save).unwrap()
}

#[test]
fn survival_handicap_is_applied_during_bookkeeping() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let species = engine.register_species(SpeciesDefinition {
        name: "Handicap entrant".to_owned(),
        minimum_population: 1,
        ..SpeciesDefinition::default()
    });
    let ids = engine
        .spawn_species_batch(
            LegacyDna::parse("start\nstop").unwrap(),
            species,
            [[100.0, 100.0]],
            1_000,
        )
        .unwrap();
    engine
        .configure_local_mode(LocalModeConfig {
            kind: LocalModeKind::SimpleSurvival,
            source_species: Some(species),
            energy_handicap: 25,
            ..LocalModeConfig::default()
        })
        .unwrap();
    engine.start_local_mode().unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.organism(ids[0]).unwrap().energy, 975);
}

#[test]
fn low_memory_housekeeping_removes_the_lowest_energy_genome_over_the_limit() {
    let mut config = EngineConfig::testing();
    config.low_memory_dna_limit = 4;
    config.low_memory_detail_limit = 2;
    let mut engine = Engine::new(config).unwrap();
    let species = engine.register_species(SpeciesDefinition::default());
    engine
        .spawn_species_batch(
            LegacyDna::parse("1 2 3 start\nstop").unwrap(),
            species,
            [[100.0, 100.0]],
            1_000,
        )
        .unwrap();

    engine.tick().unwrap();

    assert!(engine.snapshot().organisms.is_empty());
    assert_eq!(engine.snapshot().stats.deaths, 1);
}

#[test]
fn low_memory_housekeeping_preserves_inclusive_budget_composite_rank_threshold_candidate_safety_and_accounting()
 {
    assert_eq!(Engine::low_memory_deletion_budget(1, 637_500), 2);
    assert_eq!(Engine::low_memory_deletion_budget(1, 425_000), 3);
    assert_eq!(Engine::low_memory_deletion_budget(1, 255_000), 3);

    let threshold_dna = LegacyDna::parse("start\nstop").unwrap();
    let threshold = threshold_dna.blocks().len();
    let mut threshold_config = EngineConfig::testing();
    threshold_config.low_memory_dna_limit = threshold;
    threshold_config.low_memory_detail_limit = threshold;
    let mut threshold_engine = Engine::new(threshold_config).unwrap();
    threshold_engine.spawn(threshold_dna).unwrap();
    let threshold_trace = threshold_engine.tick_with_trace().unwrap();
    assert_eq!(threshold_engine.population(), 1);
    assert_eq!(threshold_engine.snapshot().stats.deaths, 0);
    assert!(threshold_engine.snapshot().corpses.is_empty());
    assert!(!threshold_trace.events.iter().any(|event| {
        matches!(
            event,
            CycleEvent::QueueChange {
                kind: QueueChangeKind::DeathCommitted,
                ..
            }
        )
    }));

    let mut config = EngineConfig::testing();
    config.low_memory_dna_limit = 1;
    config.low_memory_detail_limit = 1;
    let mut engine = Engine::new(config).unwrap();
    let dna = LegacyDna::parse("start\nstop").unwrap();
    let first = engine.spawn(dna.clone()).unwrap();
    let second = engine.spawn(dna.clone()).unwrap();
    let protected = engine.spawn(dna).unwrap();
    engine = restore_with_bodies(&engine, &[(first, 100), (second, 1), (protected, 31_999)]);
    engine.set_organism_energy(first, 1_000).unwrap();
    engine.set_organism_energy(second, 1_500).unwrap();
    engine.set_organism_energy(protected, 10).unwrap();

    let trace = engine.tick_with_trace().unwrap();
    let committed: Vec<_> = trace
        .events
        .iter()
        .filter_map(|event| match event {
            CycleEvent::QueueChange {
                phase: CyclePhase::Restart,
                id,
                kind: QueueChangeKind::DeathCommitted,
                ..
            } => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(committed, vec![second, first]);
    assert_eq!(engine.snapshot().organisms.len(), 1);
    assert_eq!(engine.snapshot().organisms[0].id, protected);
    assert_eq!(engine.snapshot().stats.deaths, 2);
    assert_eq!(engine.snapshot().corpses.len(), 2);
    assert_eq!(engine.snapshot().corpses[0].body, 1);
    assert_eq!(engine.snapshot().corpses[0].energy, 1);
    assert_eq!(engine.snapshot().corpses[1].body, 100);
    assert_eq!(engine.snapshot().corpses[1].energy, 100);
}
