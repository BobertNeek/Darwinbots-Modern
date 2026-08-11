use std::{fs, path::Path};

use darwinbots_engine::{
    Engine, EngineConfig, FileTeleporter, LegacyDna, LegacyOrganism, LegacyOrganismFile,
    LegacyOrganismKey, LegacyOrganismMember, LegacyOrganismTie, LegacySettingsFile,
    LegacySimulationFile, SaveFile, SpeciesDefinition, Teleporter, TeleporterFilters,
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
fn legacy_settings_corpus_round_trips_all_shipped_assets() {
    let assets_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Installer/sets");
    let mut assets = fs::read_dir(&assets_dir)
        .unwrap_or_else(|error| {
            panic!(
                "{}: failed to enumerate shipped settings assets: {error}",
                assets_dir.display()
            )
        })
        .map(|entry| {
            entry
                .unwrap_or_else(|error| {
                    panic!(
                        "{}: failed to read settings asset entry: {error}",
                        assets_dir.display()
                    )
                })
                .path()
        })
        .filter(|path| path.extension().is_some_and(|extension| extension == "set"))
        .collect::<Vec<_>>();
    assets.sort();

    assert_eq!(
        assets.len(),
        7,
        "{}: expected exactly seven shipped .set assets",
        assets_dir.display()
    );

    for path in assets {
        let path_display = path.display().to_string();
        let bytes = fs::read(&path)
            .unwrap_or_else(|error| panic!("{path_display}: failed to read asset: {error}"));
        let parsed = LegacySettingsFile::decode(&bytes).unwrap_or_else(|error| {
            panic!("{path_display}: failed to parse legacy settings: {error}")
        });
        let encoded = parsed
            .encode()
            .unwrap_or_else(|error| panic!("{path_display}: failed to encode settings: {error}"));

        assert_eq!(
            encoded.as_slice(),
            bytes.as_slice(),
            "{path_display}: no-op encoding changed an unknown or unimplemented legacy field"
        );

        let reparsed = LegacySettingsFile::decode(&encoded).unwrap_or_else(|error| {
            panic!("{path_display}: failed to reparse encoded settings: {error}")
        });
        assert_legacy_settings_semantics(&path, &parsed, &reparsed);
    }
}

fn assert_legacy_settings_semantics(
    path: &Path,
    expected: &LegacySettingsFile,
    actual: &LegacySettingsFile,
) {
    assert_eq!(actual.version, expected.version, "{}: version changed", path.display());
    assert_eq!(
        actual.simulation_name,
        expected.simulation_name,
        "{}: simulation name changed",
        path.display()
    );
    assert_eq!(
        actual.field_size,
        expected.field_size,
        "{}: field size changed",
        path.display()
    );
    assert_eq!(
        actual.world_size,
        expected.world_size,
        "{}: world size changed",
        path.display()
    );
    assert_eq!(
        actual.max_population,
        expected.max_population,
        "{}: maximum population changed",
        path.display()
    );
    assert_eq!(
        actual.blocked_vegetables,
        expected.blocked_vegetables,
        "{}: blocked vegetables flag changed",
        path.display()
    );
    assert_eq!(
        actual.disable_ties,
        expected.disable_ties,
        "{}: disable ties flag changed",
        path.display()
    );
    assert_eq!(
        actual.population_limit_method,
        expected.population_limit_method,
        "{}: population limit method changed",
        path.display()
    );
    assert_eq!(
        actual.toroidal_world,
        expected.toroidal_world,
        "{}: toroidal world flag changed",
        path.display()
    );
    assert_eq!(
        actual.max_energy,
        expected.max_energy,
        "{}: maximum energy changed",
        path.display()
    );
    assert_eq!(
        actual.min_vegetables,
        expected.min_vegetables,
        "{}: minimum vegetables changed",
        path.display()
    );
    assert_eq!(actual.gravity, expected.gravity, "{}: gravity changed", path.display());
    assert_eq!(
        actual.sunlight_energy,
        expected.sunlight_energy,
        "{}: sunlight energy changed",
        path.display()
    );
    assert_eq!(
        actual.species,
        expected.species,
        "{}: species startup records changed",
        path.display()
    );
}

#[test]
fn entire_organism_interchange_restores_members_state_and_ties() {
    let mut source = Engine::new(EngineConfig::testing()).expect("source engine");
    let first = source
        .spawn_at(LegacyDna::parse("stop").expect("first dna"), [12.0, 34.0])
        .expect("first source organism");
    let second = source
        .spawn_at(LegacyDna::parse("stop").expect("second dna"), [50.0, 60.0])
        .expect("second source organism");
    let file = LegacyOrganismFile {
        version: 1,
        tick: 812,
        members: vec![
            LegacyOrganismMember {
                slot: first.slot(),
                generation: first.generation(),
                organism: LegacyOrganism {
                    dna: "stop".to_owned(),
                    species: 0,
                    vegetable: false,
                    corpse: false,
                    repopulating: false,
                    position: [12.0, 34.0],
                    velocity: [1.5, -2.5],
                    energy: 777,
                    age: 91,
                    birth_cycle: 44,
                    body: 321,
                },
            },
            LegacyOrganismMember {
                slot: second.slot(),
                generation: second.generation(),
                organism: LegacyOrganism {
                    dna: "stop".to_owned(),
                    species: 0,
                    vegetable: false,
                    corpse: false,
                    repopulating: false,
                    position: [50.0, 60.0],
                    velocity: [0.0, 3.0],
                    energy: 555,
                    age: 12,
                    birth_cycle: 8,
                    body: 222,
                },
            },
        ],
        ties: vec![LegacyOrganismTie {
            first: LegacyOrganismKey::from(first),
            second: LegacyOrganismKey::from(second),
            rest_length: 42.5,
        }],
    };

    let bytes = file.encode().expect("organism interchange encoding");
    let decoded = LegacyOrganismFile::decode(&bytes).expect("organism interchange decoding");
    assert_eq!(decoded, file);

    let mut engine = Engine::new(EngineConfig::testing()).expect("fixture engine");
    let imported = engine
        .import_legacy_organism_file(&decoded)
        .expect("whole organism import");
    assert_eq!(imported, 2);
    assert_eq!(engine.population(), 2);
    assert_eq!(engine.snapshot().ties.len(), 1);
    assert_eq!(engine.snapshot().ties[0].rest_length, 42.5);
    assert_eq!(engine.snapshot().organisms[0].energy, 777);
    assert_eq!(engine.snapshot().organisms[0].age, 91);
    assert_eq!(engine.snapshot().organisms[0].body, 321);
}

#[test]
fn legacy_vb6_organism_records_import_without_losing_state_or_ties() {
    let file = LegacyOrganismFile::decode(&synthetic_vb6_organism())
        .expect("VB6 organism record decoding");

    assert_eq!(file.members.len(), 1);
    assert_eq!(file.members[0].slot, 7);
    assert_eq!(file.members[0].organism.position, [12.5, 34.5]);
    assert_eq!(file.members[0].organism.velocity, [1.25, -2.5]);
    assert_eq!(file.members[0].organism.energy, 777);
    assert_eq!(file.members[0].organism.age, 91);
    assert_eq!(file.members[0].organism.birth_cycle, 44);
    assert_eq!(file.members[0].organism.body, 321);
    assert_eq!(
        file.ties.len(),
        0,
        "self-referential fixture tie must be ignored"
    );
}

#[test]
fn legacy_organism_fixture_round_trips_and_rejects_unknown_version() {
    let fixture = LegacyOrganismFile::decode(&synthetic_vb6_organism())
        .expect("VB6 organism fixture decoding");
    let encoded = fixture.encode().expect("organism fixture encoding");
    assert_eq!(LegacyOrganismFile::decode(&encoded).unwrap(), fixture);

    let mut unsupported = encoded;
    unsupported[4..6].copy_from_slice(&2_u16.to_le_bytes());
    assert!(LegacyOrganismFile::decode(&unsupported).is_err());
}

fn synthetic_vb6_organism() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_i16(&mut bytes, 1);
    for value in [1, 0, 0] {
        push_i16(&mut bytes, value);
    }
    for value in [12.5, 34.5, 1.25, -2.5, 0.0, 0.0, 0.0] {
        push_f32(&mut bytes, value);
    }
    for tie in 0..11 {
        push_i16(&mut bytes, 1);
        push_i16(&mut bytes, if tie == 0 { 7 } else { 0 });
        push_i16(&mut bytes, 1);
        push_f32(&mut bytes, 0.0);
        push_f32(&mut bytes, 0.0);
        push_i16(&mut bytes, 0);
        push_i32(&mut bytes, 42);
        push_i32(&mut bytes, 0);
        for _ in 0..7 {
            push_i16(&mut bytes, 0);
        }
    }
    push_f32(&mut bytes, 777.0);
    for _ in 0..50 {
        push_i16(&mut bytes, 0);
        push_i16(&mut bytes, 0);
    }
    push_i16(&mut bytes, 0);
    for _ in 0..=1000 {
        push_i16(&mut bytes, 0);
    }
    push_i16(&mut bytes, 2);
    push_i16(&mut bytes, 2);
    push_i16(&mut bytes, 1);
    push_i16(&mut bytes, 10);
    push_i16(&mut bytes, 1);
    for _ in 0..21 {
        push_f32(&mut bytes, 0.0);
    }
    for value in [3, 4, 5] {
        push_i16(&mut bytes, value);
    }
    push_i32(&mut bytes, 6);
    push_i32(&mut bytes, 91);
    push_i32(&mut bytes, 44);
    for value in [1, 2, 2] {
        push_i16(&mut bytes, value);
    }
    for _ in 0..14 {
        push_i16(&mut bytes, 0);
    }
    push_i32(&mut bytes, 0);
    push_f32(&mut bytes, 321.0);
    push_f32(&mut bytes, 0.0);
    push_i16(&mut bytes, 0);
    for _ in 0..4 {
        push_f32(&mut bytes, 0.0);
    }
    push_i16(&mut bytes, 0);
    push_i16(&mut bytes, -1);
    push_i16(&mut bytes, 0);
    push_text(&mut bytes, "");
    push_text(&mut bytes, "");
    push_i16(&mut bytes, 0);
    push_i16(&mut bytes, 0);
    for _ in 0..21 {
        push_f32(&mut bytes, 0.0);
        push_f32(&mut bytes, 0.0);
    }
    push_i16(&mut bytes, 0);
    push_i16(&mut bytes, 0);
    push_i16(&mut bytes, 0);
    push_i16(&mut bytes, 0);
    push_i16(&mut bytes, 7);
    for _ in 0..4 {
        push_i16(&mut bytes, 0);
    }
    push_f32(&mut bytes, 0.0);
    push_f32(&mut bytes, 0.0);
    push_i16(&mut bytes, 0);
    push_i16(&mut bytes, 0);
    push_i16(&mut bytes, 0);
    push_i16(&mut bytes, 0);
    push_i16(&mut bytes, 0);
    for _ in 0..501 {
        for _ in 0..3 {
            push_i32(&mut bytes, 0);
        }
    }
    push_i32(&mut bytes, 0);
    push_i32(&mut bytes, 0);
    push_i16(&mut bytes, 0);
    for _ in 0..11 {
        push_u8(&mut bytes, 0);
        push_f32(&mut bytes, 0.0);
        push_f32(&mut bytes, 0.0);
        push_f32(&mut bytes, 0.0);
    }
    push_f32(&mut bytes, 0.0);
    push_f32(&mut bytes, 0.0);
    for _ in 0..15 {
        push_i16(&mut bytes, 0);
    }
    bytes.extend_from_slice(&[0; 50]);
    push_i16(&mut bytes, 0);
    push_i16(&mut bytes, 0);
    push_u8(&mut bytes, 0);
    push_u8(&mut bytes, 0);
    push_u8(&mut bytes, 0);
    push_i32(&mut bytes, 0);
    push_f32(&mut bytes, 0.0);
    push_f32(&mut bytes, 0.0);
    bytes.extend_from_slice(&[254, 254, 254]);
    bytes
}

fn push_i16(bytes: &mut Vec<u8>, value: i16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_i32(bytes: &mut Vec<u8>, value: i32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_f32(bytes: &mut Vec<u8>, value: f32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u8(bytes: &mut Vec<u8>, value: u8) {
    bytes.push(value);
}

fn push_text(bytes: &mut Vec<u8>, value: &str) {
    push_i16(bytes, value.len() as i16);
    bytes.extend_from_slice(value.as_bytes());
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
    assert_eq!(engine.snapshot().file_teleporters[0].sent, 1);
    assert_eq!(engine.snapshot().file_teleporters[0].received, 0);

    let inbound_file = inbound.join(outgoing[0].file_name().expect("transfer file name"));
    fs::copy(&outgoing[0], &inbound_file).expect("stage inbound transfer");
    engine.tick().expect("inbound file transfer");
    assert_eq!(engine.population(), 2, "inbound transfer was not inserted");
    assert_eq!(engine.snapshot().file_teleporters[0].received, 1);
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
    assert_eq!(backflow_engine.snapshot().file_teleporters[0].sent, 2);
    assert!(
        backflow_engine.snapshot().file_teleporters[0].backflow_guard_active(),
        "same-path file teleporter must publish an active backflow guard"
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
            ..Teleporter::new(8, [200.0, 200.0], 30.0, [700.0, 700.0])
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
