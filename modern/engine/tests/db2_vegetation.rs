mod support;

use darwinbots_engine::{Engine, EngineConfig, LegacyDna, SpeciesDefinition, sysvar_address};
use support::db2_fixtures::{
    MEM_AVAILABILITY, MEM_CHLR, MEM_LIGHT, MEM_MAKE_CHLR, MEM_REMOVE_CHLR, MEM_SHARE_CHLR,
    START_CHLR,
};

#[test]
fn chloroplast_sysvars_use_db2_memory_addresses() {
    assert_eq!(sysvar_address("chlr"), Some(MEM_CHLR));
    assert_eq!(sysvar_address("mkchlr"), Some(MEM_MAKE_CHLR));
    assert_eq!(sysvar_address("rmchlr"), Some(MEM_REMOVE_CHLR));
    assert_eq!(sysvar_address("light"), Some(MEM_LIGHT));
    assert_eq!(sysvar_address("availability"), Some(MEM_AVAILABILITY));
    assert_eq!(sysvar_address("sharechlr"), Some(MEM_SHARE_CHLR));
}

#[test]
fn initial_and_reseeded_vegetables_start_with_db2_chloroplasts() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        name: "Fixture vegetables".to_owned(),
        vegetable: true,
        minimum_population: 1,
        reseed: true,
        ..SpeciesDefinition::default()
    });
    let initial = engine
        .spawn_species_batch(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetables,
            [[100.0, 100.0]],
            1_000,
        )
        .unwrap()[0];
    assert_eq!(engine.organism(initial).unwrap().chloroplasts, START_CHLR);

    engine.remove(initial).unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.snapshot().organisms.len(), 1);
    assert_eq!(engine.snapshot().organisms[0].chloroplasts, START_CHLR);
}

#[test]
fn vegetable_reproduction_splits_parent_biology_without_resetting_child() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        name: "Reproducing vegetables".to_owned(),
        vegetable: true,
        ..SpeciesDefinition::default()
    });
    engine
        .spawn_species_batch(
            LegacyDna::parse("start\n50 .repro store\nstop").unwrap(),
            vegetables,
            [[500.0, 500.0]],
            1_000,
        )
        .unwrap();

    engine.tick().unwrap();

    let plants: Vec<_> = engine
        .snapshot()
        .organisms
        .iter()
        .filter(|bot| bot.species == vegetables)
        .collect();
    assert_eq!(plants.len(), 2);
    assert_eq!(plants.iter().map(|bot| bot.chloroplasts).sum::<i32>(), START_CHLR);
    assert!(plants.iter().all(|bot| bot.chloroplasts > 0));
}
