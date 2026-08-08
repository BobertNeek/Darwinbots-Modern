use std::fs;

use darwinbots_engine::{
    Engine, EngineConfig, FileTeleporter, LegacyDna, LegacySettingsFile, LegacySimulationFile,
    SaveFile, SpeciesDefinition, Teleporter, TeleporterFilters,
};
use tempfile::tempdir;

#[test]
fn legacy_codecs_round_trip_the_authoritative_fixture_fields() {
    let mut engine = Engine::new(EngineConfig::testing()).expect("fixture engine");
    engine
        .spawn_at(LegacyDna::parse("stop").expect("fixture dna"), [12.5, 34.5])
        .expect("fixture organism");

    let installed_settings = include_bytes!("../../../Installer/sets/Simple Simulation.set");
    assert_eq!(
        LegacySettingsFile::decode(installed_settings)
            .expect("installed DB2 settings fixture")
            .encode()
            .expect("installed DB2 settings re-encoding"),
        installed_settings,
    );
    let parsed_settings =
        LegacySettingsFile::decode(installed_settings).expect("installed DB2 settings fields");
    assert_eq!(parsed_settings.species.len(), 2);
    assert_eq!(
        parsed_settings.species[0].robot_path,
        r#"C:\Users\Owner\Desktop\Robots"#
    );
    assert_eq!(parsed_settings.species[0].robot_count, 5);
    assert_eq!(
        parsed_settings.species[0].dna_name,
        "Alga_Minimalis_Chloroplastus.txt"
    );
    assert_eq!(parsed_settings.simulation_name, "");
    assert_eq!(parsed_settings.max_population, 100);

    let mut edited_settings = parsed_settings.clone();
    edited_settings.simulation_name = "Edited fixture".to_owned();
    edited_settings.max_population = 321;
    edited_settings.species[0].robot_path = r#"D:\Darwinbots\EditedRobots"#.to_owned();
    edited_settings.species[0].robot_count = 17;
    let edited_bytes = edited_settings
        .encode()
        .expect("edited settings should encode");
    assert_ne!(edited_bytes, installed_settings);
    let reparsed_settings =
        LegacySettingsFile::decode(&edited_bytes).expect("edited settings should decode");
    assert_eq!(reparsed_settings.simulation_name, "Edited fixture");
    assert_eq!(reparsed_settings.max_population, 321);
    assert_eq!(
        reparsed_settings.species[0].robot_path,
        r#"D:\Darwinbots\EditedRobots"#
    );
    assert_eq!(reparsed_settings.species[0].robot_count, 17);

    let fixtures = ["settings", "simulation"];
    for fixture in fixtures {
        let encoded = match fixture {
            "settings" => LegacySettingsFile::from_engine(&engine)
                .expect("legacy settings projection")
                .encode()
                .expect("legacy settings encoding"),
            "simulation" => LegacySimulationFile::from_engine(&engine)
                .expect("legacy simulation projection")
                .encode()
                .expect("legacy simulation encoding"),
            _ => unreachable!("table only contains known fixtures"),
        };

        let reencoded = match fixture {
            "settings" => LegacySettingsFile::decode(&encoded)
                .expect("legacy settings decoding")
                .encode()
                .expect("legacy settings re-encoding"),
            "simulation" => LegacySimulationFile::decode(&encoded)
                .expect("legacy simulation decoding")
                .encode()
                .expect("legacy simulation re-encoding"),
            _ => unreachable!("table only contains known fixtures"),
        };

        assert_eq!(
            reencoded, encoded,
            "legacy {fixture} fixture changed on round trip"
        );
    }
}

#[test]
fn file_teleporter_cycles_through_tempfiles_with_filters_and_safe_failures() {
    let root = tempdir().expect("temporary teleporter root");
    let inbound = root.path().join("inbound");
    let outbound = root.path().join("outbound");
    fs::create_dir_all(&inbound).expect("inbound directory");
    fs::create_dir_all(&outbound).expect("outbound directory");

    let mut engine = Engine::new(EngineConfig::testing()).expect("fixture engine");
    let mut vegetable = SpeciesDefinition::default();
    vegetable.name = "Fixture vegetable".to_owned();
    vegetable.vegetable = true;
    let vegetable_species = engine.register_species(vegetable);

    engine
        .add_file_teleporter(
            FileTeleporter::new(7, [100.0, 100.0], 30.0)
                .with_paths(&inbound, &outbound)
                .with_polling(1, 1)
                .with_filters(TeleporterFilters {
                    teleport_vegetables: false,
                    teleport_corpses: false,
                    teleport_heterotrophs: true,
                }),
        )
        .expect("file teleporter registration");
    let saved = SaveFile::encode(&engine).expect("file teleporter save");
    let restored = SaveFile::decode(&saved).expect("file teleporter restore");
    let saved_payload: serde_json::Value = serde_json::from_slice(&saved[10..]).unwrap();
    let restored_payload = SaveFile::encode(&restored).expect("file teleporter re-save");
    let restored_payload: serde_json::Value =
        serde_json::from_slice(&restored_payload[10..]).unwrap();
    assert_eq!(
        saved_payload["file_teleporters"], restored_payload["file_teleporters"],
        "file teleporter state was lost across DB3S round trip"
    );

    engine
        .spawn_at(
            LegacyDna::parse("stop").expect("heterotroph dna"),
            [100.0, 100.0],
        )
        .expect("heterotroph spawn");
    engine
        .spawn_species_at(
            LegacyDna::parse("stop").expect("vegetable dna"),
            vegetable_species,
            [100.0, 100.0],
        )
        .expect("vegetable spawn");

    engine.tick().expect("outbound file transfer");
    let outgoing = fs::read_dir(&outbound)
        .expect("outbound listing")
        .map(|entry| entry.expect("outbound entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "dbo"))
        .collect::<Vec<_>>();
    assert_eq!(
        outgoing.len(),
        1,
        "filtering or outbound serialization changed"
    );
    assert_eq!(
        engine.population(),
        1,
        "blocked vegetable crossed the teleporter"
    );

    let inbound_file = inbound.join(outgoing[0].file_name().expect("transfer file name"));
    fs::copy(&outgoing[0], &inbound_file).expect("stage inbound transfer");
    engine.tick().expect("inbound file transfer");
    assert_eq!(engine.population(), 2, "inbound transfer was not inserted");
    assert!(
        !inbound_file.exists(),
        "successful inbound transfer was not consumed"
    );

    let bad_file = inbound.join("corrupt.dbo");
    fs::write(&bad_file, b"not a Darwinbots organism").expect("corrupt transfer");
    let population_before_failure = engine.population();
    engine.tick().expect("corrupt transfer should fail safely");
    assert_eq!(engine.population(), population_before_failure);
    assert!(bad_file.exists(), "failed inbound transfer was destroyed");

    let shared = root.path().join("shared");
    fs::create_dir_all(&shared).expect("shared teleporter directory");
    let mut backflow_engine = Engine::new(EngineConfig::testing()).expect("backflow engine");
    backflow_engine
        .add_file_teleporter(
            FileTeleporter::new(9, [150.0, 150.0], 30.0)
                .with_paths(&shared, &shared)
                .with_polling(1, 2)
                .with_filters(TeleporterFilters {
                    teleport_vegetables: false,
                    teleport_corpses: false,
                    teleport_heterotrophs: true,
                }),
        )
        .expect("backflow teleporter registration");
    backflow_engine
        .spawn_at(
            LegacyDna::parse("stop").expect("backflow dna"),
            [150.0, 150.0],
        )
        .expect("backflow spawn");
    backflow_engine
        .spawn_at(
            LegacyDna::parse("stop").expect("second backflow dna"),
            [151.0, 150.0],
        )
        .expect("second backflow spawn");
    backflow_engine.tick().expect("backflow outbound cycle");
    assert_eq!(
        backflow_engine.population(),
        0,
        "newly emitted file was immediately re-imported"
    );
    assert_eq!(
        fs::read_dir(&shared)
            .expect("shared listing")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "dbo"))
            .count(),
        2,
        "same-directory transfer was consumed as backflow"
    );

    let corpse_outbound = root.path().join("corpse-outbound");
    fs::create_dir_all(&corpse_outbound).expect("corpse outbound directory");
    let mut corpse_engine = Engine::new(EngineConfig {
        metabolism_cost: 2_000,
        ..EngineConfig::testing()
    })
    .expect("corpse engine");
    corpse_engine
        .add_file_teleporter(
            FileTeleporter::new(10, [250.0, 250.0], 30.0)
                .with_paths(root.path().join("unused-inbound"), &corpse_outbound)
                .with_polling(1, 1)
                .with_filters(TeleporterFilters {
                    teleport_vegetables: false,
                    teleport_corpses: true,
                    teleport_heterotrophs: false,
                }),
        )
        .expect("corpse teleporter registration");
    corpse_engine
        .spawn_at(
            LegacyDna::parse("stop").expect("corpse dna"),
            [250.0, 250.0],
        )
        .expect("corpse spawn");
    corpse_engine.tick().expect("corpse creation cycle");
    assert_eq!(corpse_engine.snapshot().corpses.len(), 1);
    corpse_engine.tick().expect("corpse outbound cycle");
    assert_eq!(
        fs::read_dir(&corpse_outbound)
            .expect("corpse outbound listing")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "dbo"))
            .count(),
        1,
        "corpse filter path never serialized a corpse"
    );

    let mut local_engine = Engine::new(EngineConfig::testing()).expect("local fixture engine");
    local_engine
        .add_teleporter(Teleporter {
            id: 8,
            center: [200.0, 200.0],
            radius: 30.0,
            destination: [700.0, 700.0],
        })
        .expect("local teleporter registration");
    let local_id = local_engine
        .spawn_at(LegacyDna::parse("stop").expect("local dna"), [200.0, 200.0])
        .expect("local spawn");
    local_engine.tick().expect("local transfer");
    assert_eq!(
        local_engine
            .robot_state(local_id)
            .expect("local robot")
            .kinematics
            .position,
        [700.0, 700.0]
    );
}
