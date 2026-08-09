use darwinbots_engine::{Engine, EngineConfig, LegacyDna, SaveFile, StandardMutationKind};
use serde_json::Value;

const V2_FIXTURE: &[u8] = include_bytes!("fixtures/db3s_v2_phase1.bin");

fn framed_payload(bytes: &[u8]) -> Value {
    let length = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
    serde_json::from_slice(&bytes[10..10 + length]).expect("fixture payload is JSON")
}

fn frame_payload(version: u16, payload: Value) -> Vec<u8> {
    let payload = serde_json::to_vec(&payload).expect("payload serializes");
    let mut bytes = Vec::with_capacity(10 + payload.len());
    bytes.extend_from_slice(b"DB3S");
    bytes.extend_from_slice(&version.to_le_bytes());
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&payload);
    bytes
}

fn v3_fixture_payload() -> Value {
    let engine = SaveFile::decode(V2_FIXTURE).expect("legacy fixture loads");
    framed_payload(&SaveFile::encode(&engine).expect("fixture promotes to v3"))
}

fn first_organism_mut(payload: &mut Value) -> &mut Value {
    payload["slots"]
        .as_array_mut()
        .expect("save contains slots")
        .iter_mut()
        .find_map(|slot| {
            let organism = &mut slot["organism"];
            (!organism.is_null()).then_some(organism)
        })
        .expect("fixture contains a robot")
}

fn first_organism(payload: &Value) -> &Value {
    payload["slots"]
        .as_array()
        .expect("save contains slots")
        .iter()
        .find_map(|slot| {
            let organism = &slot["organism"];
            (!organism.is_null()).then_some(organism)
        })
        .expect("fixture contains a robot")
}

#[test]
fn core_02_persists_remaining_robot_flags_counters_private_diagnostics_and_occurrences() {
    let mut payload = v3_fixture_payload();
    let robot = first_organism_mut(&mut payload);
    robot["flags"]["wall"] = Value::Bool(true);
    robot["flags"]["no_chloroplasts"] = Value::Bool(true);
    robot["counters"]["simulation_age"] = serde_json::json!(17);
    robot["counters"]["old_mutations"] = serde_json::json!(23);
    robot["program"]["private_variable_count"] = serde_json::json!(3);
    robot["program"]["used_variables"] = serde_json::json!([12, 340, 721]);
    robot["senses"]["occurrence_counts"] = serde_json::json!([
        0, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987, 1597, 2584,
        4181, 6765, 10946
    ]);

    let restored = SaveFile::decode(&frame_payload(3, payload)).expect("v3 state loads");
    let round_trip = framed_payload(&SaveFile::encode(&restored).expect("v3 state saves"));
    let robot = first_organism(&round_trip);

    assert_eq!(robot["flags"]["wall"], true);
    assert_eq!(robot["flags"]["no_chloroplasts"], true);
    assert_eq!(robot["counters"]["simulation_age"], 17);
    assert_eq!(robot["counters"]["old_mutations"], 23);
    assert_eq!(robot["program"]["private_variable_count"], 3);
    assert_eq!(robot["program"]["used_variables"], serde_json::json!([12, 340, 721]));
    assert_eq!(
        robot["senses"]["occurrence_counts"],
        serde_json::json!([
            0, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987, 1597, 2584,
            4181, 6765, 10946
        ])
    );
}

#[test]
fn mut_02_point_and_delta_mutations_run_through_an_ordinary_cycle() {
    let mut point_engine = Engine::new(EngineConfig::testing()).expect("point engine");
    point_engine
        .update_mutation_settings(true, 1.0, &[])
        .expect("point policy");
    let point_id = point_engine
        .spawn_at(
            LegacyDna::parse("start\n10 .up store\nstop").expect("point DNA"),
            [500.0, 500.0],
        )
        .expect("point robot");
    let mut point_profile = point_engine.mutation_profile(point_id).unwrap().clone();
    point_profile.point.one_in = 1.0;
    point_profile.delta.one_in = f64::MAX;
    point_engine
        .set_mutation_profile(point_id, point_profile)
        .expect("point profile");
    let point_before = point_engine.dna(point_id).unwrap().clone();
    point_engine.tick().expect("point cycle");
    let point_robot = point_engine.robot_state(point_id).unwrap();
    assert_ne!(point_engine.dna(point_id).unwrap(), &point_before);
    assert_eq!(
        point_robot.program.mutation_metadata.last_mutation_kind,
        Some(StandardMutationKind::Point)
    );

    let mut delta_engine = Engine::new(EngineConfig::testing()).expect("delta engine");
    delta_engine
        .update_mutation_settings(true, 1.0, &[])
        .expect("delta policy");
    let delta_id = delta_engine
        .spawn_at(
            LegacyDna::parse("start\n10 .up store\nstop").expect("delta DNA"),
            [500.0, 500.0],
        )
        .expect("delta robot");
    let mut delta_profile = delta_engine.mutation_profile(delta_id).unwrap().clone();
    delta_profile.point.one_in = f64::MAX;
    delta_profile.delta.one_in = 1.0;
    delta_engine
        .set_mutation_profile(delta_id, delta_profile.clone())
        .expect("delta profile");
    delta_engine.tick().expect("delta cycle");
    let delta_robot = delta_engine.robot_state(delta_id).unwrap();
    assert_eq!(
        delta_robot.program.mutation_metadata.last_mutation_kind,
        Some(StandardMutationKind::Delta)
    );
    assert_ne!(
        &delta_robot.program.mutation_profile,
        &delta_profile,
        "Delta must update a live mutation rate"
    );
}

#[test]
fn per_01_loads_legacy_simulation_state_and_promotes_it_to_db3s_v3() {
    let mut legacy_payload = framed_payload(V2_FIXTURE);
    let robot = first_organism_mut(&mut legacy_payload);
    robot["flags"]["wall"] = Value::Bool(true);
    robot["flags"]["no_chloroplasts"] = Value::Bool(true);
    robot["counters"]["simulation_age"] = serde_json::json!(29);

    let restored = SaveFile::decode(&frame_payload(2, legacy_payload)).expect("legacy save loads");
    let promoted = framed_payload(&SaveFile::encode(&restored).expect("v3 save writes"));
    let robot = first_organism(&promoted);

    assert_eq!(robot["flags"]["wall"], true);
    assert_eq!(robot["flags"]["no_chloroplasts"], true);
    assert_eq!(robot["counters"]["simulation_age"], 29);
}
