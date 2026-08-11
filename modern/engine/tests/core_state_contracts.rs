use std::slice;

use darwinbots_engine::ffi::{
    db_buffer_free, db_engine_command_batch, db_engine_create, db_engine_destroy, db_engine_save,
    DbBuffer, DbStatus,
};
use darwinbots_engine::{
    Engine, EngineConfig, LegacyDna, LocalModeConfig, LocalModeKind, RestrictionOptions,
    SaveFile, ShotSnapshot, Teleporter,
};
use serde_json::{json, Value};

fn save_payload(engine: &Engine) -> Value {
    let bytes = SaveFile::encode(engine).expect("save should encode");
    serde_json::from_slice(&bytes[10..]).expect("save payload should be JSON")
}

#[test]
fn core_policy_state_round_trips_through_native_save() {
    let mut config = serde_json::to_value(EngineConfig::testing()).expect("config should encode");
    config["dna_costs"]["dna_copy"] = json!(4.25);
    config["vegetation"]["sun_on_random"] = json!(true);
    config["recording"] = json!({
        "safemode_backup": true,
        "dead_snapshot_enabled": true,
        "exclude_repopulating_robots": true,
        "autosave_interval": 200
    });

    let mut engine = Engine::new(
        serde_json::from_value(config).expect("config should deserialize"),
    )
    .expect("engine should create");

    let mut mode = LocalModeConfig::default();
    mode.kind = LocalModeKind::EcoSurvival;
    mode.maximum_cycles = 321;
    engine
        .configure_local_mode(mode)
        .expect("mode should configure");

    let mut restrictions = RestrictionOptions::default();
    restrictions.non_repopulating.disable_vision = true;
    engine
        .update_restriction_options(restrictions)
        .expect("restrictions should update");

    let payload = save_payload(&engine);
    assert_eq!(payload["config"]["dna_costs"]["dna_copy"], json!(4.25));
    assert_eq!(payload["config"]["vegetation"]["sun_on_random"], json!(true));
    assert_eq!(
        payload["config"]["recording"]["dead_snapshot_enabled"],
        json!(true)
    );

    let restored = SaveFile::decode(
        &SaveFile::encode(&engine).expect("save should encode"),
    )
    .expect("save should decode");
    let restored_mode = restored
        .snapshot()
        .local_mode
        .config
        .as_ref()
        .expect("mode should persist");
    assert_eq!(restored_mode.kind, LocalModeKind::EcoSurvival);
    assert_eq!(restored_mode.maximum_cycles, 321);
    assert!(
        restored
            .snapshot()
            .restriction_options
            .non_repopulating
            .disable_vision
    );
}

#[test]
fn native_ffi_updates_cost_and_recording_authority() {
    let config = serde_json::to_vec(&EngineConfig::testing()).expect("config should encode");
    let commands = br#"{
        "version": 1,
        "commands": [
            {
                "type": "update_cost_settings",
                "costs": {"dna_copy": 6.5, "global_multiplier": 1.25}
            },
            {
                "type": "update_recording_settings",
                "settings": {
                    "safemode_backup": true,
                    "dead_snapshot_enabled": true,
                    "exclude_repopulating_robots": true,
                    "autosave_interval": 90
                }
            },
            {
                "type": "add_teleporter",
                "id": 7,
                "center": [10.0, 20.0],
                "radius": 4.0,
                "destination": [100.0, 200.0],
                "outbound": true,
                "outbound_path": "out.db",
                "respect_shapes": true,
                "backflow_limit": 3
            }
        ]
    }"#;

    let mut handle = std::ptr::null_mut();
    assert_eq!(
        db_engine_create(config.as_ptr(), config.len(), &mut handle),
        DbStatus::Ok
    );
    assert!(!handle.is_null());

    let mut result = DbBuffer::default();
    assert_eq!(
        db_engine_command_batch(handle, commands.as_ptr(), commands.len(), &mut result),
        DbStatus::Ok
    );
    assert_eq!(db_buffer_free(&mut result), DbStatus::Ok);

    let mut save = DbBuffer::default();
    assert_eq!(db_engine_save(handle, &mut save), DbStatus::Ok);
    let saved = unsafe { slice::from_raw_parts(save.data, save.len) }.to_vec();
    assert_eq!(db_buffer_free(&mut save), DbStatus::Ok);
    assert_eq!(db_engine_destroy(handle), DbStatus::Ok);

    let payload: Value = serde_json::from_slice(&saved[10..]).expect("save payload should be JSON");
    assert_eq!(payload["config"]["dna_costs"]["dna_copy"], json!(6.5));
    assert_eq!(
        payload["config"]["recording"]["autosave_interval"],
        json!(90)
    );
    assert_eq!(payload["snapshot"]["teleporters"][0]["outbound"], json!(true));
    assert_eq!(
        payload["snapshot"]["teleporters"][0]["outbound_path"],
        json!("out.db")
    );
}

#[test]
fn legacy_object_payload_fields_survive_native_serde() {
    let teleporter_value = json!({
        "id": 7,
        "center": [10.0, 20.0],
        "radius": 4.0,
        "destination": [100.0, 200.0],
        "width": 8.0,
        "height": 9.0,
        "color": 4278255360u32,
        "velocity": [1.0, -2.0],
        "path": "local.db",
        "inbound_path": "in.db",
        "outbound_path": "out.db",
        "inbound": true,
        "outbound": true,
        "local": true,
        "internet": false,
        "drift_horizontal": true,
        "drift_vertical": false,
        "highlight": true,
        "filters": {
            "teleport_vegetables": true,
            "teleport_corpses": true,
            "teleport_heterotrophs": false
        },
        "respect_shapes": true,
        "sent": 12,
        "received": 13,
        "poll_interval": 5,
        "capacity": 6,
        "poll_countdown": 2,
        "backflow_limit": 3
    });
    let teleporter: Teleporter = serde_json::from_value(teleporter_value).expect("teleporter JSON");
    let teleporter_round_trip = serde_json::to_value(teleporter).expect("teleporter output");
    assert_eq!(teleporter_round_trip["outbound"], json!(true));
    assert_eq!(teleporter_round_trip["outbound_path"], json!("out.db"));
    assert_eq!(teleporter_round_trip["respect_shapes"], json!(true));
    assert_eq!(teleporter_round_trip["backflow_limit"], json!(3));

    let shot_value = json!({
        "owner": {"slot": 1, "generation": 2},
        "start": [1.0, 2.0],
        "end": [3.0, 4.0],
        "velocity": [5.0, 6.0],
        "age": 7,
        "range": 8,
        "energy": 9.0,
        "kind": -7,
        "value": 10,
        "impact_flash": false,
        "color": 4278190335u32,
        "source_vegetable": true,
        "source_species_name": "legacy",
        "memory_location": 835,
        "memory_value": 836,
        "stored": true,
        "gene_number": 4,
        "dna_length": 12
    });
    let shot: ShotSnapshot = serde_json::from_value(shot_value).expect("shot JSON");
    let shot_round_trip = serde_json::to_value(shot).expect("shot output");
    assert_eq!(shot_round_trip["color"], json!(4278190335u32));
    assert_eq!(shot_round_trip["source_vegetable"], json!(true));
    assert_eq!(shot_round_trip["memory_location"], json!(835));
    assert_eq!(shot_round_trip["stored"], json!(true));
    assert_eq!(shot_round_trip["dna_length"], json!(12));
}

#[test]
fn robot_limit_rejects_configurations_above_db2_maximum() {
    let mut config = EngineConfig::testing();
    config.organism_capacity = 32_001;
    let error = match Engine::new(config) {
        Ok(_) => panic!("DB2 must reject over-limit capacity"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("32,000"));
}

#[test]
fn per_06_v3_save_preserves_one_living_and_one_dead_snapshot_atomically() {
    let mut engine = Engine::new(EngineConfig::testing()).expect("engine should create");
    engine
        .spawn_at(LegacyDna::parse("stop").expect("test DNA should parse"), [12.5, 34.5])
        .expect("living organism should spawn");

    let mut payload = save_payload(&engine);
    payload["slots"][0]["organism"]["kinematics"]["position"] = json!([12.5, 34.5]);
    payload["slots"][0]["organism"]["kinematics"]["velocity"] = json!([1.25, -2.5]);
    payload["slots"][0]["organism"]["materials"]["energy"] = json!(4321);
    payload["slots"][0]["organism"]["materials"]["body"] = json!(77);
    payload["slots"][0]["organism"]["materials"]["waste"] = json!(13);
    payload["slots"][0]["organism"]["counters"]["age"] = json!(9);

    let dead = json!({
        "position": [90.5, 80.25],
        "velocity": [-1.25, 2.5],
        "energy": 321,
        "body": 45,
        "age": 6,
        "species": 0,
        "repopulating": false,
        "dna": "dead-dna",
        "generation": 4,
        "birth_cycle": 17,
        "parent": {"slot": 0, "generation": 0},
        "offspring": 8,
        "kills": 3,
        "new_mutations": 2,
        "total_mutations": 11,
        "dna_length": 19,
        "unique_id": 9001,
        "chloroplasts": 14
    });
    payload["corpses"] = json!([dead.clone()]);

    let encoded_payload = serde_json::to_vec(&payload).expect("payload should encode");
    let mut frame = b"DB3S".to_vec();
    frame.extend_from_slice(&3u16.to_le_bytes());
    frame.extend_from_slice(&(encoded_payload.len() as u32).to_le_bytes());
    frame.extend_from_slice(&encoded_payload);

    let restored = SaveFile::decode(&frame).expect("V3 save should load");
    assert_eq!(restored.snapshot().organisms.len(), 1);
    assert_eq!(restored.snapshot().corpses.len(), 1);
    assert_eq!(restored.snapshot().organisms[0].position, [12.5, 34.5]);
    assert_eq!(restored.snapshot().organisms[0].velocity, [1.25, -2.5]);
    assert_eq!(restored.snapshot().organisms[0].energy, 4321);
    assert_eq!(restored.snapshot().organisms[0].body, 77);
    assert_eq!(restored.snapshot().organisms[0].waste, 13);
    assert_eq!(restored.snapshot().organisms[0].age, 9);
    assert_ne!(restored.snapshot().organisms[0].unique_id, 9001);
    assert_eq!(serde_json::to_value(&restored.snapshot().corpses[0]).unwrap(), dead);

    let stable = SaveFile::encode(&restored).expect("restored save should encode");
    let stable_again = SaveFile::encode(
        &SaveFile::decode(&stable).expect("re-encoded save should load"),
    )
    .expect("reloaded save should re-encode");
    assert_eq!(stable, stable_again);

    let before_invalid = serde_json::to_value(restored.snapshot()).unwrap();
    let mut invalid = payload;
    invalid["corpses"][0]["body"] = json!({"invalid": true});
    let invalid_payload = serde_json::to_vec(&invalid).unwrap();
    let mut invalid_frame = b"DB3S".to_vec();
    invalid_frame.extend_from_slice(&3u16.to_le_bytes());
    invalid_frame.extend_from_slice(&(invalid_payload.len() as u32).to_le_bytes());
    invalid_frame.extend_from_slice(&invalid_payload);
    assert!(SaveFile::decode(&invalid_frame).is_err());
    assert_eq!(serde_json::to_value(restored.snapshot()).unwrap(), before_invalid);
}
