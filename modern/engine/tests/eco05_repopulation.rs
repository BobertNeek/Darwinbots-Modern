use darwinbots_engine::{
    Engine, EngineConfig, LegacyDna, RestrictionColumn, RestrictionOptions, SaveFile,
    SpeciesDefinition,
};

#[test]
fn eco05_seeded_repopulation_caps_vegetables_and_survives_save_load() {
    let mut config = EngineConfig::testing();
    config.seed = 0x00EC_0592;
    config.organism_capacity = 8;
    config.vegetable_population_cap = 2;
    config.vegetation.start_chloroplasts = 0;
    config.vegetation.minimum_chloroplast_equivalents = 1;
    config.vegetation.repopulation_amount = 4;
    config.vegetation.repopulation_cooldown = 2;

    let dna = LegacyDna::parse("start\nstop").unwrap();
    let mut engine = Engine::new(config).unwrap();
    let wild = engine.register_species(SpeciesDefinition {
        name: "wild".to_owned(),
        ..SpeciesDefinition::default()
    });
    let reseeded = engine.register_species(SpeciesDefinition {
        name: "reseeded".to_owned(),
        minimum_population: 2,
        reseed: true,
        ..SpeciesDefinition::default()
    });
    let vegetables = engine.register_species(SpeciesDefinition {
        name: "vegetables".to_owned(),
        vegetable: true,
        reseed: true,
        ..SpeciesDefinition::default()
    });
    engine
        .update_restriction_options(RestrictionOptions {
            non_repopulating: RestrictionColumn {
                fixed_in_place: true,
                disable_dna: true,
                ..RestrictionColumn::default()
            },
            repopulating: RestrictionColumn::default(),
        })
        .unwrap();

    let seed = engine
        .spawn_species_batch(dna.clone(), reseeded, [[10.0, 10.0]], 100)
        .unwrap()[0];
    engine.remove(seed).unwrap();
    let wild_id = engine
        .spawn_species_at(dna.clone(), wild, [20.0, 20.0])
        .unwrap();
    assert!(engine.organism(wild_id).unwrap().flags.fixed);
    assert!(!engine.organism(wild_id).unwrap().flags.can_execute_dna);

    let vegetable_ids = engine
        .spawn_species_batch(
            dna,
            vegetables,
            [[100.0, 100.0], [200.0, 200.0], [300.0, 300.0]],
            100,
        )
        .unwrap();
    assert_eq!(vegetable_ids.len(), 2);
    assert_eq!(engine.vegetable_population(), 2);
    engine.remove_many(&vegetable_ids).unwrap();
    assert_eq!(engine.vegetable_population(), 0);

    engine.tick().unwrap();
    assert_eq!(engine.population(), 3);
    assert_eq!(
        engine
            .snapshot()
            .organisms
            .iter()
            .filter(|organism| organism.species == reseeded)
            .count(),
        2
    );

    let save = SaveFile::encode(&engine).unwrap();
    let mut loaded = SaveFile::decode(&save).unwrap();
    engine.tick().unwrap();
    loaded.tick().unwrap();

    assert_eq!(engine.population(), 5);
    assert_eq!(engine.vegetable_population(), 2);
    assert_eq!(loaded.snapshot().organisms, engine.snapshot().organisms);
    assert_eq!(loaded.snapshot().stats, engine.snapshot().stats);
    assert!(
        loaded
            .snapshot()
            .organisms
            .iter()
            .all(|organism| organism.position[0] >= 0.0
                && organism.position[0] <= 1_000.0
                && organism.position[1] >= 0.0
                && organism.position[1] <= 1_000.0)
    );
    assert!(engine.vegetable_population() <= 2);
}
