use darwinbots_engine::{
    Engine, EngineConfig, LegacyDna, LocalModeConfig, LocalModeKind, SpeciesDefinition,
};

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
