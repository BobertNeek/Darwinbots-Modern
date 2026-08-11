use std::path::PathBuf;

use darwinbots_engine::{
    Engine, EngineConfig, LegacyDna, Obstacle, SaveFile, ShapeShotPolicy, ShapeVisionPolicy,
    StandardMutationKind, Teleporter, TeleporterFilters,
};
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

#[test]
fn core_04_round_trips_mutated_shot_tie_and_environment_objects_atomically() {
    let mut engine = Engine::new(EngineConfig::testing()).expect("engine");
    let source = engine
        .spawn_at(
            LegacyDna::parse(
                "start\n1 .tie store\n-1 .shoot store\n40 .shootval store\nstop",
            )
            .expect("source DNA"),
            [100.0, 100.0],
        )
        .expect("source robot");
    engine
        .spawn_at(LegacyDna::parse("start\nstop").expect("target DNA"), [500.0, 100.0])
        .expect("target robot");
    engine.tick().expect("shot and tie cycle");
    assert_eq!(engine.snapshot().shots.len(), 1);
    assert_eq!(engine.snapshot().ties.len(), 1);

    let mut obstacle = Obstacle::new(7, [200.0, 200.0], [260.0, 280.0]);
    engine
        .add_obstacle(obstacle.clone())
        .expect("initial obstacle");
    obstacle.color = 0x1122_3344;
    obstacle.opacity = 0.375;
    obstacle.drift = [1.25, -2.5];
    obstacle.vision_policy = ShapeVisionPolicy::SeeThrough;
    obstacle.shot_policy = ShapeShotPolicy::PassThrough;
    engine
        .update_obstacle(obstacle.clone())
        .expect("mutated obstacle");

    let mut teleporter = Teleporter::new(9, [300.0, 300.0], 24.0, [700.0, 700.0]);
    engine
        .add_teleporter(teleporter.clone())
        .expect("initial teleporter");
    teleporter.width = 48.0;
    teleporter.height = 52.0;
    teleporter.color = 0xaabb_ccdd;
    teleporter.velocity = [1.25, -0.5];
    teleporter.path = PathBuf::from("shared/teleporter");
    teleporter.inbound_path = PathBuf::from("shared/inbound");
    teleporter.outbound_path = PathBuf::from("shared/outbound");
    teleporter.inbound = false;
    teleporter.outbound = true;
    teleporter.local = false;
    teleporter.internet = true;
    teleporter.drift_horizontal = true;
    teleporter.drift_vertical = true;
    teleporter.fixed_position = false;
    teleporter.highlight = true;
    teleporter.filters = TeleporterFilters {
        teleport_vegetables: false,
        teleport_corpses: true,
        teleport_heterotrophs: false,
    };
    teleporter.respect_shapes = true;
    teleporter.sent = 12;
    teleporter.received = 34;
    teleporter.poll_interval = 7;
    teleporter.capacity = 3;
    teleporter.poll_countdown = 5;
    teleporter.backflow_limit = 9;
    engine
        .update_teleporter(teleporter.clone())
        .expect("mutated teleporter");

    let mut expected_shot = engine.snapshot().shots[0].clone();
    expected_shot.owner = source;
    expected_shot.start = [141.0, 142.0];
    expected_shot.end = [151.0, 152.0];
    expected_shot.velocity = [7.0, -8.0];
    expected_shot.age = 17;
    expected_shot.range = 23;
    expected_shot.energy = 345.5;
    expected_shot.kind = -3;
    expected_shot.value = 321;
    expected_shot.impact_flash = false;
    expected_shot.color = 0x5566_7788;
    expected_shot.source_vegetable = true;
    expected_shot.source_species_name = Some("mutated-species".to_owned());
    expected_shot.memory_location = 835;
    expected_shot.memory_value = 1234;
    expected_shot.stored = true;
    expected_shot.gene_number = 9;
    expected_shot.dna_length = 99;

    let mut expected_tie = engine.snapshot().ties[0].clone();
    expected_tie.rest_length = 42.5;
    expected_tie.first_port = 2;
    expected_tie.second_port = 3;
    expected_tie.last = 17;
    expected_tie.b = 0.25;
    expected_tie.k = 0.75;
    expected_tie.hardened = true;
    expected_tie.requested_angle = 1.25;
    expected_tie.angle_fixed = true;
    expected_tie.angle_initialized = true;

    let mut payload = framed_payload(&SaveFile::encode(&engine).expect("save"));
    let projectiles = payload["projectiles"]
        .as_object_mut()
        .expect("projectile pool");
    projectiles.insert("positions".to_owned(), serde_json::json!([[151.0, 152.0]]));
    projectiles.insert(
        "previous_positions".to_owned(),
        serde_json::json!([[141.0, 142.0]]),
    );
    projectiles.insert("velocities".to_owned(), serde_json::json!([[7.0, -8.0]]));
    projectiles.insert("ages".to_owned(), serde_json::json!([17]));
    projectiles.insert("ranges".to_owned(), serde_json::json!([23]));
    projectiles.insert("energies".to_owned(), serde_json::json!([345.5]));
    projectiles.insert("kinds".to_owned(), serde_json::json!([-3]));
    projectiles.insert("values".to_owned(), serde_json::json!([321]));
    projectiles.insert("impact_flash".to_owned(), serde_json::json!([false]));
    projectiles.insert("source_species_names".to_owned(), serde_json::json!(["mutated-species"]));
    projectiles.insert("toxin_memory_locations".to_owned(), serde_json::json!([835]));
    projectiles.insert("toxin_memory_values".to_owned(), serde_json::json!([1234]));
    projectiles.insert("colors".to_owned(), serde_json::json!([0x5566_7788_u32]));
    projectiles.insert("source_vegetables".to_owned(), serde_json::json!([true]));
    projectiles.insert("stored".to_owned(), serde_json::json!([true]));
    projectiles.insert("gene_numbers".to_owned(), serde_json::json!([9]));
    projectiles.insert("dna_lengths".to_owned(), serde_json::json!([99]));
    projectiles.insert("pool_capacity".to_owned(), serde_json::json!(101));

    let tie = payload["ties"]
        .as_array_mut()
        .and_then(|ties| ties.first_mut())
        .and_then(Value::as_object_mut)
        .expect("tie state");
    tie.insert("rest_length".to_owned(), serde_json::json!(42.5));
    tie.insert("first_port".to_owned(), serde_json::json!(2));
    tie.insert("second_port".to_owned(), serde_json::json!(3));
    tie.insert("last".to_owned(), serde_json::json!(17));
    tie.insert("b".to_owned(), serde_json::json!(0.25));
    tie.insert("k".to_owned(), serde_json::json!(0.75));
    tie.insert("hardened".to_owned(), serde_json::json!(true));
    tie.insert("information".to_owned(), serde_json::json!(true));
    tie.insert("sharing_first".to_owned(), serde_json::json!(true));
    tie.insert("sharing_second".to_owned(), serde_json::json!(true));
    tie.insert("requested_angle".to_owned(), serde_json::json!(1.25));
    tie.insert("angle_fixed".to_owned(), serde_json::json!(true));
    tie.insert("angle_initialized".to_owned(), serde_json::json!(true));

    let mut restored = SaveFile::decode(&frame_payload(3, payload.clone())).expect("load");
    assert_eq!(restored.snapshot().shots, vec![expected_shot]);
    assert_eq!(restored.snapshot().ties, vec![expected_tie]);
    assert_eq!(restored.snapshot().obstacles, vec![obstacle.clone()]);
    assert_eq!(restored.snapshot().teleporters, vec![teleporter.clone()]);

    let round_trip = framed_payload(&SaveFile::encode(&restored).expect("re-save"));
    for key in ["projectiles", "ties", "obstacles", "teleporters"] {
        assert_eq!(round_trip[key], payload[key], "authoritative {key} changed");
    }
    let reloaded = SaveFile::decode(&SaveFile::encode(&restored).expect("stable save"))
        .expect("stable load");
    let stable = framed_payload(&SaveFile::encode(&reloaded).expect("stable re-save"));
    for key in ["projectiles", "ties", "obstacles", "teleporters"] {
        assert_eq!(stable[key], round_trip[key], "authoritative {key} is not stable");
    }

    let before_invalid = restored.snapshot().obstacles[0].clone();
    let mut invalid = before_invalid.clone();
    invalid.maximum[0] = invalid.minimum[0];
    assert!(restored.update_obstacle(invalid).is_err());
    assert_eq!(restored.snapshot().obstacles, vec![before_invalid]);
}
