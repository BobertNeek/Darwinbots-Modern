use darwinbots_engine::{
    Engine, EngineConfig, LeaguePreset, LeagueWorkflow, LegacyDna, LocalModeConfig, SaveFile,
    SpeciesDefinition, SpeciesId,
};

fn add_entrant(engine: &mut Engine, name: &str, dna: &str, vegetable: bool) -> SpeciesId {
    let species = engine.register_species(SpeciesDefinition {
        name: name.into(),
        vegetable,
        ..Default::default()
    });
    engine
        .spawn_species_at(LegacyDna::parse(dna).unwrap(), species, [500.0; 2])
        .unwrap();
    species
}

#[test]
fn short_bot_and_sg_enforce_six_gene_limit_before_changing_the_world() {
    for preset in [LeaguePreset::ShortBot, LeaguePreset::SingleGene] {
        for genes in [6, 7] {
            let mut engine = Engine::new(EngineConfig::testing()).unwrap();
            add_entrant(
                &mut engine,
                "candidate",
                &"start stop\n".repeat(genes),
                false,
            );
            // League eligibility applies to entrants, not their food.
            add_entrant(&mut engine, "food", &"start stop\n".repeat(8), true);
            let before = SaveFile::encode(&engine).unwrap();
            let result = engine.configure_local_mode(LocalModeConfig {
                preset,
                ..Default::default()
            });
            if genes == 7 {
                assert!(result.unwrap_err().to_string().contains("at most 6 genes"));
                assert_eq!(before, SaveFile::encode(&engine).unwrap());
            } else {
                result.unwrap();
                assert_eq!(engine.snapshot().world_size, [16000.0, 12000.0]);
                let mut resumed = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
                resumed.start_local_mode().unwrap();
                assert_eq!(
                    resumed
                        .snapshot()
                        .local_mode
                        .config
                        .as_ref()
                        .unwrap()
                        .preset,
                    preset
                );
                assert_eq!(resumed.snapshot().world_size, [16000.0, 12000.0]);
            }
        }
    }
}

#[test]
fn single_store_counts_instruction_sites_including_inc_dec_and_arithmetic_stores() {
    for (source, eligible) in [
        ("start 1 .up store stop", true),
        ("start .up inc stop", true),
        ("start .up dec stop", true),
        ("start 1 .up addstore stop", true),
        ("start stop", false),
        ("start 1 .up store 2 .dn store stop", false),
        ("start .up inc .dn dec stop", false),
    ] {
        let mut engine = Engine::new(EngineConfig::testing()).unwrap();
        add_entrant(&mut engine, "candidate", source, false);
        add_entrant(
            &mut engine,
            "food",
            "start 1 .up store 2 .dn store stop",
            true,
        );
        let before = SaveFile::encode(&engine).unwrap();
        let result = engine.configure_local_mode(LocalModeConfig {
            preset: LeaguePreset::SingleStore,
            ..Default::default()
        });
        assert_eq!(result.is_ok(), eligible, "{source}: {result:?}");
        if !eligible {
            assert_eq!(before, SaveFile::encode(&engine).unwrap());
        }
    }
}

#[test]
fn scheduled_admission_checks_actual_entrants_and_seeding_opponent() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let first = add_entrant(&mut engine, "first", "start .up inc stop", false);
    let second = add_entrant(&mut engine, "second", "start .up dec stop", false);
    let excluded = add_entrant(&mut engine, "excluded", "start .up inc .dn dec stop", false);
    engine
        .configure_local_mode(LocalModeConfig {
            preset: LeaguePreset::SingleStore,
            workflow: LeagueWorkflow::Tournament,
            entrant_order: vec![first, second],
            ..Default::default()
        })
        .unwrap();
    let before = SaveFile::encode(&engine).unwrap();
    let result = engine.configure_local_mode(LocalModeConfig {
        preset: LeaguePreset::SingleStore,
        workflow: LeagueWorkflow::Seeding,
        entrant_order: vec![first],
        seeding_opponent: Some(excluded),
        ..Default::default()
    });
    assert!(result.unwrap_err().to_string().contains("exactly 1 store"));
    assert_eq!(before, SaveFile::encode(&engine).unwrap());
    engine.start_local_mode().unwrap();
    assert!(
        engine
            .snapshot()
            .organisms
            .iter()
            .all(|robot| robot.species != excluded)
    );
}

#[test]
fn configured_save_rechecks_new_entrants_before_starting() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    add_entrant(&mut engine, "first", "start .up inc stop", false);
    engine
        .configure_local_mode(LocalModeConfig {
            preset: LeaguePreset::SingleStore,
            ..Default::default()
        })
        .unwrap();
    add_entrant(
        &mut engine,
        "new invalid entrant",
        "start .up inc .dn dec stop",
        false,
    );
    let before = SaveFile::encode(&engine).unwrap();
    let mut resumed = SaveFile::decode(&before).unwrap();
    assert!(
        resumed
            .start_local_mode()
            .unwrap_err()
            .to_string()
            .contains("exactly 1 store")
    );
    assert_eq!(before, SaveFile::encode(&resumed).unwrap());
}
