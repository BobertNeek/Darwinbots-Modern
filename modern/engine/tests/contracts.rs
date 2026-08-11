use darwinbots_engine::{
    BackendKind, BackendPreference, Engine, EngineConfig, EngineError, LegacyDna, OrganismId,
    SaveFile, SpeciesDefinition, StandardMutationProfile, TokenClass,
};

const TOXIN_SOURCE_POSITION: [f32; 2] = [100.0, 100.0];
const TOXIN_TARGET_POSITION: [f32; 2] = [350.0, 100.0];
const DEFAULT_ROBOT_RADII_SUM: f32 = 228.56;

fn frame_save(version: u16, payload: &serde_json::Value) -> Vec<u8> {
    let payload = serde_json::to_vec(payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(b"DB3S");
    save.extend_from_slice(&version.to_le_bytes());
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    save
}

fn engine_with_in_flight_venom() -> (Engine, OrganismId, OrganismId) {
    let center_gap = TOXIN_TARGET_POSITION[0] - TOXIN_SOURCE_POSITION[0];
    assert_eq!(center_gap, 250.0);
    assert!(center_gap - DEFAULT_ROBOT_RADII_SUM > 0.0);
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let source_species = engine.register_species(SpeciesDefinition {
        name: "Saved venom source".to_owned(),
        ..SpeciesDefinition::default()
    });
    let target_species = engine.register_species(SpeciesDefinition {
        name: "Saved venom target".to_owned(),
        ..SpeciesDefinition::default()
    });
    let source = engine
        .spawn_species_at(
            LegacyDna::parse("start\n8 .strvenom store\nstop").unwrap(),
            source_species,
            TOXIN_SOURCE_POSITION,
        )
        .unwrap();
    let target = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            target_species,
            TOXIN_TARGET_POSITION,
        )
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(
            source,
            LegacyDna::parse(
                "start\n-3 .shoot store\n8 .shootval store\n500 .vloc store\n77 .venval store\nstop",
            )
            .unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
    (engine, source, target)
}

#[test]
fn review_projection_db3s_v3_resumes_exact_mid_soft_tie_lifecycle() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        world_width: 3_000.0,
        world_height: 2_000.0,
        toroidal_world: false,
        ..EngineConfig::testing()
    })
    .unwrap();
    let source = engine
        .spawn_at(
            LegacyDna::parse("start\n41 .tie store\nstop").unwrap(),
            [400.0, 1_000.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [700.0, 1_000.0])
        .unwrap();
    engine.tick().unwrap();
    engine.tick_many(7).unwrap();

    let before = &engine.snapshot().ties[0];
    assert_eq!(engine.snapshot().tick, 8);
    assert_eq!(before.last, -13);
    assert!((before.b - 0.02).abs() < 0.000_001);
    assert!((before.k - 0.01).abs() < 0.000_001);
    assert!(!before.hardened);

    let save = SaveFile::encode(&engine).unwrap();
    let mut restored = SaveFile::decode(&save).unwrap();
    let after = &restored.snapshot().ties[0];
    assert_eq!(after.last, -13);
    assert!((after.b - 0.02).abs() < 0.000_001);
    assert!((after.k - 0.01).abs() < 0.000_001);
    assert!(!after.hardened);
    assert_eq!(
        (
            restored
                .robot_state(source)
                .unwrap()
                .ties
                .iter()
                .flatten()
                .count(),
            restored
                .robot_state(target)
                .unwrap()
                .ties
                .iter()
                .flatten()
                .count(),
        ),
        (1, 1)
    );
    assert_eq!(
        (
            restored.robot_state(source).unwrap().ties[0]
                .unwrap()
                .countdown,
            restored.robot_state(target).unwrap().ties[0]
                .unwrap()
                .countdown,
        ),
        (-13, -13)
    );

    restored.tick_many(12).unwrap();
    let hardened = &restored.snapshot().ties[0];
    assert_eq!(restored.snapshot().tick, 20);
    assert_eq!(hardened.last, -1);
    assert!((hardened.b - 0.1).abs() < 0.000_001);
    assert!((hardened.k - 0.05).abs() < 0.000_001);
    assert!(hardened.hardened);
    assert_eq!(restored.memory(source, "multi").unwrap(), 1);
    assert_eq!(restored.memory(target, "multi").unwrap(), 1);

    let mut old_v3: serde_json::Value = serde_json::from_slice(&save[10..]).unwrap();
    let strip_lifecycle_fields = |tie: &mut serde_json::Value| {
        let tie = tie.as_object_mut().unwrap();
        tie.remove("last");
        tie.remove("b");
        tie.remove("k");
        tie.remove("hardened");
    };
    strip_lifecycle_fields(&mut old_v3["ties"][0]);
    strip_lifecycle_fields(&mut old_v3["snapshot"]["ties"][0]);
    let migrated = SaveFile::decode(&frame_save(3, &old_v3)).unwrap();
    let migrated_tie = &migrated.snapshot().ties[0];
    assert_eq!(migrated_tie.last, 0);
    assert!((migrated_tie.b - 0.1).abs() < 0.000_001);
    assert!((migrated_tie.k - 0.05).abs() < 0.000_001);
    assert!(migrated_tie.hardened);
}

const SIMPLE_BOT: &str = r#"
' Legacy comments remain legal.
def target 50
cond
  *.robage 0 =
start
  10 .up store
stop
"#;

#[test]
fn imports_legacy_dna_and_preserves_user_variables() {
    let program = LegacyDna::parse(SIMPLE_BOT).expect("legacy DNA should parse");

    assert_eq!(program.user_variable("target"), Some(50));
    assert!(program.blocks().len() >= 8);
}

#[test]
fn parser_compiles_symbolic_memory_operands_to_numeric_addresses() {
    let program = LegacyDna::parse("def custom 777\nstart\n*.custom .up store\nstop").unwrap();
    assert!(
        program
            .blocks()
            .iter()
            .any(|block| block.class == TokenClass::Dereference && block.value == 777)
    );
    assert!(
        program
            .blocks()
            .iter()
            .any(|block| block.class == TokenClass::Number && block.value == 1)
    );
}

#[test]
fn unknown_legacy_instruction_becomes_zero_with_location_warning() {
    let program = LegacyDna::parse("start\nnot-a-command\nstop").unwrap();

    assert!(
        program
            .blocks()
            .iter()
            .any(|block| block.class == TokenClass::Number && block.value == 0)
    );
    assert!(program.compatibility_warnings()[0].contains("line 2"));
    assert!(program.compatibility_warnings()[0].contains("not-a-command"));
}

#[test]
fn batch_spawn_publishes_one_complete_population_snapshot() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let dna = LegacyDna::parse("start\nstop").unwrap();
    let ids = engine
        .spawn_batch((0..16).map(|index| (dna.clone(), [index as f32, 5.0])))
        .unwrap();

    assert_eq!(ids.len(), 16);
    assert_eq!(engine.population(), 16);
    assert_eq!(engine.snapshot().organisms.len(), 16);
    assert_eq!(engine.snapshot().render_instances.len(), 16);
    assert_eq!(engine.snapshot().render_instances[7].slot, 7);
    assert_eq!(engine.snapshot().render_instances[7].position, [7.0, 5.0]);
}

#[test]
fn seeded_cpu_ticks_are_reproducible_and_publish_snapshots() {
    let config = EngineConfig {
        seed: 42,
        ..EngineConfig::testing()
    };
    let mut left = Engine::new(config.clone()).unwrap();
    let mut right = Engine::new(config).unwrap();
    let dna = LegacyDna::parse("start\n10 .up store\nstop").unwrap();
    left.spawn(dna.clone()).unwrap();
    right.spawn(dna).unwrap();

    for _ in 0..20 {
        left.tick().unwrap();
        right.tick().unwrap();
    }

    assert_eq!(left.snapshot(), right.snapshot());
    assert_eq!(left.snapshot().tick, 20);
    assert_eq!(left.snapshot().organisms.len(), 1);
}

#[test]
fn core_06_seeded_repeatability_save_resume_and_atomicity_contract() {
    let make_engine = |seed| {
        let mut engine = Engine::new(EngineConfig {
            seed,
            metabolism_cost: 0,
            brownian_motion: 1.0,
            ..EngineConfig::testing()
        })
        .unwrap();
        engine
            .spawn_at(
                LegacyDna::parse("start\n50 .repro store\nstop").unwrap(),
                [500.0, 500.0],
            )
            .unwrap();
        engine
    };

    let mut left = make_engine(42);
    let mut right = make_engine(42);
    let mut divergent = make_engine(43);
    let assert_authoritative_match = |left: &Engine, right: &Engine| {
        let mut left = left.snapshot().clone();
        let mut right = right.snapshot().clone();
        for organism in &mut left.organisms {
            organism.activation_bitmap.clear();
            organism.activation_history.clear();
        }
        for organism in &mut right.organisms {
            organism.activation_bitmap.clear();
            organism.activation_history.clear();
        }
        left.phase_timings = Default::default();
        right.phase_timings = Default::default();
        assert_eq!(left, right);
    };

    for (phase, cycles) in [1_u32, 2, 3].into_iter().enumerate() {
        left.tick_many(cycles).unwrap();
        right.tick_many(cycles).unwrap();
        divergent.tick_many(cycles).unwrap();

        assert_authoritative_match(&left, &right);
        if phase == 0 {
            assert_ne!(left.snapshot(), divergent.snapshot());
        }

        left.validate_invariants().unwrap();
        right.validate_invariants().unwrap();
        divergent.validate_invariants().unwrap();

        if phase == 1 {
            let checkpoint = SaveFile::encode(&left).unwrap();
            left = SaveFile::decode(&checkpoint).unwrap();
            assert_authoritative_match(&left, &right);
        }
    }

    let before_failed_load = SaveFile::encode(&left).unwrap();
    let mut truncated = before_failed_load.clone();
    truncated.pop();
    assert!(SaveFile::decode(&truncated).is_err());
    assert_eq!(SaveFile::encode(&left).unwrap(), before_failed_load);
    left.validate_invariants().unwrap();
}

#[test]
fn tick_many_advances_a_batch_and_publishes_the_final_state() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();

    engine.tick_many(25).unwrap();

    assert_eq!(engine.snapshot().tick, 25);
    assert_eq!(engine.snapshot().organisms[0].age, 25);
}

#[test]
fn invariant_checker_accepts_slot_reuse_and_seeded_ticks() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let first = engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    engine.remove(first).unwrap();
    engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    engine.validate_invariants().unwrap();
}

#[test]
fn engine_uses_vm_flow_semantics_for_movement_intents() {
    let mut engine = Engine::new(EngineConfig {
        physics: darwinbots_engine::PhysicsSettings {
            density: 0.0,
            surface_gravity: 0.0,
            ..darwinbots_engine::PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let dna =
        LegacyDna::parse("cond\n2 1 >\nstart\n10 .up store\nelse\n25 .up store\nstop").unwrap();
    let id = engine.spawn(dna).unwrap();

    engine.tick().unwrap();

    assert!((engine.organism(id).unwrap().position[1] - 6.6).abs() < 0.01);
}

#[test]
fn save_files_are_versioned_and_round_trip_reference_state() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();

    let bytes = SaveFile::encode(&engine).unwrap();
    let restored = SaveFile::decode(&bytes).unwrap();

    assert_eq!(&bytes[..4], b"DB3S");
    assert_eq!(restored.snapshot(), engine.snapshot());
}

#[test]
fn db3s_v3_configuration_groups_default_missing_fields_round_trip_and_reject_invalid_atomically() {
    fn field<'a>(root: &'a serde_json::Value, path: &[&str]) -> &'a serde_json::Value {
        let mut current = root;
        for key in path {
            current = match key.parse::<usize>() {
                Ok(index) => current.get(index).unwrap(),
                Err(_) => current.get(*key).unwrap(),
            };
        }
        current
    }

    fn field_mut<'a>(root: &'a mut serde_json::Value, path: &[&str]) -> &'a mut serde_json::Value {
        let mut current = root;
        for key in path {
            current = match key.parse::<usize>() {
                Ok(index) => current.get_mut(index).unwrap(),
                Err(_) => current.get_mut(*key).unwrap(),
            };
        }
        current
    }

    fn remove_field(root: &mut serde_json::Value, path: &[&str]) {
        let (parent, key) = path.split_at(path.len() - 1);
        field_mut(root, parent).as_object_mut().unwrap().remove(key[0]);
    }

    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    let save = SaveFile::encode(&engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&save[10..]).unwrap();
    payload["obstacles"] = serde_json::json!([{
        "id": 7,
        "minimum": [10.0, 20.0],
        "maximum": [30.0, 40.0],
        "color": 4279312947_u32,
        "opacity": 0.5,
        "drift": [1.0, -2.0],
        "vision_policy": "see_through",
        "shot_policy": "pass_through"
    }]);
    payload["teleporters"] = serde_json::json!([{
        "id": 8,
        "center": [100.0, 200.0],
        "radius": 14.0,
        "destination": [900.0, 800.0]
    }]);

    let present = vec![
        (vec!["config", "physics", "max_velocity"], serde_json::json!(99.0)),
        (vec!["config", "shots", "speed"], serde_json::json!(55.0)),
        (vec!["config", "vegetation", "cycle_length"], serde_json::json!(88)),
        (vec!["config", "corpse_decay", "packet_size"], serde_json::json!(7)),
        (vec!["config", "waste", "defecation_threshold"], serde_json::json!(1234)),
        (vec!["config", "recording", "sampling_interval"], serde_json::json!(17)),
        (vec!["config", "auto_speciation"], serde_json::json!(true)),
        (
            vec!["config", "speciation_genetic_distance_percent"],
            serde_json::json!(12.5),
        ),
        (vec!["config", "toroidal_world"], serde_json::json!(false)),
        (vec!["config", "axis_topology"], serde_json::json!([true, false])),
        (
            vec!["config", "dna_costs", "global_multiplier"],
            serde_json::json!(11.0),
        ),
        (
            vec!["config", "advanced_mutation", "delta2", "interval"],
            serde_json::json!(24),
        ),
        (
            vec!["restriction_options", "non_repopulating", "fixed_in_place"],
            serde_json::json!(true),
        ),
        (vec!["player_bot", "enabled"], serde_json::json!(true)),
        (vec!["obstacles", "0", "id"], serde_json::json!(7)),
        (vec!["teleporters", "0", "id"], serde_json::json!(8)),
    ];
    let mut present_payload = payload.clone();
    for (path, value) in &present {
        *field_mut(&mut present_payload, path) = value.clone();
    }

    let restored = SaveFile::decode(&frame_save(3, &present_payload)).unwrap();
    let round_trip = SaveFile::encode(&restored).unwrap();
    let round_trip_payload: serde_json::Value = serde_json::from_slice(&round_trip[10..]).unwrap();
    for (path, value) in &present {
        assert_eq!(field(&round_trip_payload, path), value, "present field {path:?}");
    }
    let stable = SaveFile::encode(&SaveFile::decode(&round_trip).unwrap()).unwrap();
    assert_eq!(stable, round_trip);

    let missing = vec![
        (vec!["config", "physics", "max_velocity"], vec!["config", "physics", "max_velocity"], serde_json::json!(60.0)),
        (vec!["config", "shots", "speed"], vec!["config", "shots", "speed"], serde_json::json!(40.0)),
        (vec!["config", "vegetation", "cycle_length"], vec!["config", "vegetation", "cycle_length"], serde_json::json!(10_000)),
        (vec!["config", "corpse_decay", "packet_size"], vec!["config", "corpse_decay", "packet_size"], serde_json::json!(0)),
        (vec!["config", "waste", "defecation_threshold"], vec!["config", "waste", "defecation_threshold"], serde_json::json!(32_000)),
        (vec!["config", "recording", "sampling_interval"], vec!["config", "recording", "sampling_interval"], serde_json::json!(200)),
        (vec!["config", "auto_speciation"], vec!["config", "auto_speciation"], serde_json::json!(false)),
        (vec!["config", "speciation_genetic_distance_percent"], vec!["config", "speciation_genetic_distance_percent"], serde_json::json!(20.0)),
        (vec!["config", "toroidal_world"], vec!["config", "toroidal_world"], serde_json::json!(true)),
        (vec!["config", "axis_topology"], vec!["config", "axis_topology"], serde_json::Value::Null),
        (vec!["config", "dna_costs", "global_multiplier"], vec!["config", "dna_costs", "global_multiplier"], serde_json::json!(1.0)),
        (vec!["config", "advanced_mutation", "delta2", "interval"], vec!["config", "advanced_mutation", "delta2", "interval"], serde_json::json!(100)),
        (vec!["restriction_options"], vec!["restriction_options", "non_repopulating", "fixed_in_place"], serde_json::json!(false)),
        (vec!["player_bot"], vec!["player_bot", "enabled"], serde_json::json!(false)),
        (vec!["obstacles"], vec!["obstacles"], serde_json::json!([])),
        (vec!["teleporters"], vec!["teleporters"], serde_json::json!([])),
    ];
    for (remove_path, assert_path, default) in &missing {
        let mut candidate = present_payload.clone();
        remove_field(&mut candidate, remove_path);
        let restored = SaveFile::decode(&frame_save(3, &candidate)).unwrap();
        let migrated = SaveFile::encode(&restored).unwrap();
        let migrated_payload: serde_json::Value = serde_json::from_slice(&migrated[10..]).unwrap();
        assert_eq!(field(&migrated_payload, assert_path), default, "missing field {remove_path:?}");
    }

    let baseline = SaveFile::decode(&frame_save(3, &present_payload)).unwrap();
    let baseline_snapshot = baseline.snapshot().clone();
    let mut invalid = present_payload;
    invalid["slots"][0]["organism"]["program"]["mutation_profile"]["point"]["one_in"] =
        serde_json::json!(0.0);
    assert!(SaveFile::decode(&frame_save(3, &invalid)).is_err());
    assert_eq!(baseline.snapshot(), &baseline_snapshot);
}

#[test]
fn db3s_v3_old_save_defaults_standard_mutation_profile() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let id = engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    let save = SaveFile::encode(&engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&save[10..]).unwrap();
    let program = payload["slots"][id.slot() as usize]["organism"]["program"]
        .as_object_mut()
        .unwrap();
    program.remove("mutation_profile");
    program.remove("point_mutation_schedule");

    let restored = SaveFile::decode(&frame_save(3, &payload)).unwrap();

    assert_eq!(
        restored.mutation_profile(id).unwrap(),
        &StandardMutationProfile::default()
    );
}

#[test]
fn db3s_v3_round_trip_preserves_nondefault_standard_mutation_profile() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let id = engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    let mut profile = StandardMutationProfile::default();
    profile.point.one_in = 1_250.0;
    profile.delta.mean = 750.0;
    profile.reversal.std_dev = 2.0;
    profile.point_value_change_percent = 79;
    profile.copy_error_value_change_percent = 65;
    engine.set_mutation_profile(id, profile.clone()).unwrap();

    let save = SaveFile::encode(&engine).unwrap();
    assert_eq!(&save[..4], b"DB3S");
    assert_eq!(u16::from_le_bytes([save[4], save[5]]), 3);
    assert_eq!(engine.snapshot().schema_version, 2);
    let restored = SaveFile::decode(&save).unwrap();

    assert_eq!(restored.mutation_profile(id).unwrap(), &profile);
    assert_eq!(restored.snapshot().schema_version, 2);
}

#[test]
fn db3s_v3_round_trip_preserves_asexual_birth_state() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        world_width: 4_000.0,
        world_height: 3_000.0,
        toroidal_world: false,
        ..EngineConfig::testing()
    })
    .unwrap();
    let parent = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [1_000.0, 1_000.0])
        .unwrap();
    let mut profile = StandardMutationProfile::default();
    profile.point.one_in = 1_250.0;
    profile.delta.mean = 750.0;
    engine
        .set_mutation_profile(parent, profile.clone())
        .unwrap();
    let save = SaveFile::encode(&engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&save[10..]).unwrap();
    payload["mutations_enabled"] = serde_json::json!(false);
    let robot = &mut payload["slots"][parent.slot() as usize]["organism"];
    robot["materials"]["energy"] = serde_json::json!(4_000);
    robot["materials"]["body"] = serde_json::json!(400);
    robot["materials"]["waste"] = serde_json::json!(40);
    robot["materials"]["permanent_waste"] = serde_json::json!(20);
    robot["materials"]["chloroplasts"] = serde_json::json!(200);
    robot["materials"]["previous_energy"] = serde_json::json!(4_000);
    robot["counters"]["generation"] = serde_json::json!(7);
    robot["identity"]["absolute_id"] = serde_json::json!(41);
    let cells = robot["program"]["memory"]["cells"].as_array_mut().unwrap();
    cells.extend([
        serde_json::json!([300, 25]),
        serde_json::json!([971, 1_971]),
        serde_json::json!([976, 2_976]),
    ]);
    cells.sort_by_key(|cell| cell[0].as_u64().unwrap());
    let mut engine = SaveFile::decode(&frame_save(3, &payload)).unwrap();
    engine.tick().unwrap();
    let child = engine
        .snapshot()
        .organisms
        .iter()
        .find(|organism| organism.parents == [Some(parent), None])
        .unwrap()
        .id;

    let bytes = SaveFile::encode(&engine).unwrap();
    assert_eq!(&bytes[..4], b"DB3S");
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 3);
    assert_eq!(engine.snapshot().schema_version, 2);
    let current_payload: serde_json::Value = serde_json::from_slice(&bytes[10..]).unwrap();
    assert_eq!(current_payload["next_absolute_id"], 43);
    let restored = SaveFile::decode(&bytes).unwrap();

    let restored_parent = restored.robot_state(parent).unwrap();
    let restored_child = restored.robot_state(child).unwrap();
    assert_eq!(restored_parent.identity.absolute_id, 41);
    assert_eq!(restored_child.identity.absolute_id, 42);
    assert_eq!(restored_child.identity.parents, [Some(parent), None]);
    assert_eq!(restored_child.counters.generation, 8);
    assert_eq!(restored_child.counters.birth_cycle, 1);
    assert_eq!(restored_child.program.mutation_profile, profile);
    assert_eq!(restored_child.program.memory.read(971), 1_971);
    assert_eq!(restored_child.heredity.epigenetic_staging[0], 2_976);
    assert_eq!(restored.snapshot().ties.len(), 1);
    assert_eq!(restored.snapshot().ties[0].first, parent);
    assert_eq!(restored.snapshot().ties[0].second, child);
    assert_eq!(restored.snapshot().ties[0].first_port, 0);
    assert_eq!(restored.snapshot().ties[0].second_port, 1);
    assert_eq!(restored.snapshot().ties[0].last, 100);
}

#[test]
fn db3s_v3_older_save_defaults_asexual_heredity_and_identity_fields() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let id = engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    let save = SaveFile::encode(&engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&save[10..]).unwrap();
    payload.as_object_mut().unwrap().remove("next_absolute_id");
    payload["config"]["dna_costs"]
        .as_object_mut()
        .unwrap()
        .remove("dna_copy");
    let robot = &mut payload["slots"][id.slot() as usize]["organism"];
    robot["identity"]
        .as_object_mut()
        .unwrap()
        .remove("absolute_id");
    robot.as_object_mut().unwrap().remove("heredity");
    robot.as_object_mut().unwrap().remove("buoyancy");

    let restored = SaveFile::decode(&frame_save(3, &payload)).unwrap();

    assert_eq!(restored.robot_state(id).unwrap().identity.absolute_id, 0);
    assert_eq!(
        restored.robot_state(id).unwrap().heredity,
        darwinbots_engine::RobotHeredityState::default()
    );
    let migrated = SaveFile::encode(&restored).unwrap();
    assert_eq!(&migrated[..4], b"DB3S");
    assert_eq!(u16::from_le_bytes([migrated[4], migrated[5]]), 3);
    let migrated: serde_json::Value = serde_json::from_slice(&migrated[10..]).unwrap();
    assert_eq!(migrated["config"]["dna_costs"]["dna_copy"], 0.0);
    assert_eq!(migrated["next_absolute_id"], 1);
    assert_eq!(
        migrated["slots"][id.slot() as usize]["organism"]["buoyancy"],
        0.0
    );
    assert_eq!(restored.snapshot().schema_version, 2);
}

#[test]
fn db3s_v3_rejects_malformed_standard_mutation_profile() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let id = engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    let save = SaveFile::encode(&engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&save[10..]).unwrap();
    payload["slots"][id.slot() as usize]["organism"]["program"]["mutation_profile"]["point"]["one_in"] =
        serde_json::json!(0.0);

    assert!(SaveFile::decode(&frame_save(3, &payload)).is_err());
}

#[test]
fn snapshot_v2_exposes_phase1_telemetry() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine
        .spawn(LegacyDna::parse("cond\n1 0 >\nstart\nstop").unwrap())
        .unwrap();

    engine.tick().unwrap();

    let snapshot = engine.snapshot();
    let organism = &snapshot.organisms[0];
    assert_eq!(snapshot.schema_version, 2);
    assert_eq!(organism.current_gene, 1);
    assert_eq!(organism.condition_count, 1);
    assert_eq!(organism.generation, 0);
    assert_eq!(organism.birth_cycle, 0);
}

#[test]
fn save_round_trip_preserves_pre_refactor_robot_state_fields() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let species = SpeciesDefinition {
        name: "save-contract".to_owned(),
        vegetable: true,
        ..Default::default()
    };
    let species = engine.register_species(species);
    let id = engine
        .spawn_species_at(
            LegacyDna::parse("start\n10 .up store\nstop").unwrap(),
            species,
            [125.0, 250.0],
        )
        .unwrap();
    engine.tick().unwrap();
    let before = engine.robot_state(id).unwrap().clone();

    let restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
    let after = restored.robot_state(id).unwrap();

    assert_eq!(after.identity.species, before.identity.species);
    assert_eq!(after.identity.parents, before.identity.parents);
    assert_eq!(after.flags, before.flags);
    assert_eq!(after.kinematics, before.kinematics);
    assert_eq!(after.materials, before.materials);
    assert_eq!(after.counters.age, before.counters.age);
    assert_eq!(after.program.dna, before.program.dna);
    assert_eq!(after.program.memory, before.program.memory);
    assert_eq!(after.program.rng_state, before.program.rng_state);
    assert_eq!(
        after.effects.forced_reproduction,
        before.effects.forced_reproduction
    );
    assert_eq!(after.phenotype, before.phenotype);
}

#[test]
fn db3s_v3_round_trip_preserves_mid_effect_toxin_state() {
    let (mut engine, source, target) = engine_with_in_flight_venom();
    engine
        .replace_dna(source, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    let before = engine.robot_state(target).unwrap().effects.clone();
    assert!(before.paralysis_active);
    assert_eq!(before.paralysis_countdown, 6.9870706);
    assert_eq!(before.paralysis_location, 500);
    assert_eq!(before.paralysis_value, 77);

    let mut restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
    assert_eq!(restored.robot_state(target).unwrap().effects, before);

    engine.tick().unwrap();
    restored.tick().unwrap();
    assert_eq!(restored.snapshot(), engine.snapshot());
    assert_eq!(restored.memory_at(target, 500).unwrap(), 77);
}

#[test]
fn db3s_v3_round_trip_preserves_in_flight_toxin_payload() {
    let (engine, source, target) = engine_with_in_flight_venom();
    let bytes = SaveFile::encode(&engine).unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&bytes[10..]).unwrap();
    assert_eq!(
        payload["projectiles"]["source_species_names"][0],
        "Saved venom source"
    );
    assert_eq!(payload["projectiles"]["toxin_memory_locations"][0], 500);
    assert_eq!(payload["projectiles"]["toxin_memory_values"][0], 77);

    let mut restored = SaveFile::decode(&bytes).unwrap();
    restored.remove(source).unwrap();
    restored.tick().unwrap();

    assert_eq!(restored.organism(target).unwrap().paralyzed, 6);
    assert_eq!(restored.memory_at(target, 500).unwrap(), 77);
}

#[test]
fn db3s_v3_missing_toxin_projectile_fields_default_to_aligned_empty_payloads() {
    let (engine, _source, _target) = engine_with_in_flight_venom();
    let save = SaveFile::encode(&engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&save[10..]).unwrap();
    let projectiles = payload["projectiles"].as_object_mut().unwrap();
    projectiles.remove("source_species_names");
    projectiles.remove("toxin_memory_locations");
    projectiles.remove("toxin_memory_values");

    let restored = SaveFile::decode(&frame_save(3, &payload)).unwrap();
    let migrated = SaveFile::encode(&restored).unwrap();
    let migrated: serde_json::Value = serde_json::from_slice(&migrated[10..]).unwrap();
    assert_eq!(
        migrated["projectiles"]["source_species_names"],
        serde_json::json!([null])
    );
    assert_eq!(
        migrated["projectiles"]["toxin_memory_locations"],
        serde_json::json!([0])
    );
    assert_eq!(
        migrated["projectiles"]["toxin_memory_values"],
        serde_json::json!([0])
    );
}

#[test]
fn in_flight_toxin_uses_frozen_species_after_owner_species_change() {
    let (engine, source, target) = engine_with_in_flight_venom();
    let save = SaveFile::encode(&engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&save[10..]).unwrap();
    let target_species =
        payload["slots"][target.slot() as usize]["organism"]["identity"]["species"].clone();
    payload["slots"][source.slot() as usize]["organism"]["identity"]["species"] = target_species;
    let mut restored = SaveFile::decode(&frame_save(3, &payload)).unwrap();

    restored.tick().unwrap();

    assert_eq!(restored.organism(target).unwrap().venom, 0);
    assert_eq!(restored.organism(target).unwrap().paralyzed, 6);
    assert_eq!(restored.memory_at(target, 500).unwrap(), 77);
}

#[test]
fn later_toxin_hit_caps_fractional_duration_at_32000() {
    let (engine, source, target) = engine_with_in_flight_venom();
    let save = SaveFile::encode(&engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&save[10..]).unwrap();
    let effects = payload["slots"][target.slot() as usize]["organism"]["effects"]
        .as_object_mut()
        .unwrap();
    effects.insert("paralysis_active".to_owned(), serde_json::json!(true));
    effects.insert(
        "paralysis_countdown".to_owned(),
        serde_json::json!(31_999.0),
    );
    effects.insert("paralysis_location".to_owned(), serde_json::json!(400));
    effects.insert("paralysis_value".to_owned(), serde_json::json!(66));
    payload["slots"][target.slot() as usize]["organism"]["materials"]["paralyzed"] =
        serde_json::json!(31_999);
    payload["projectiles"]["values"][0] = serde_json::json!(20);
    let mut restored = SaveFile::decode(&frame_save(3, &payload)).unwrap();
    restored
        .replace_dna(source, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();

    restored.tick().unwrap();

    let effect = &restored.robot_state(target).unwrap().effects;
    assert!(effect.paralysis_active);
    assert_eq!(effect.paralysis_countdown, 31_999.0);
    assert_eq!(effect.paralysis_location, 500);
    assert_eq!(effect.paralysis_value, 77);
    assert_eq!(restored.organism(target).unwrap().paralyzed, 31_999);
}

#[test]
fn newborn_and_reused_slots_start_without_toxin_effects() {
    let (mut engine, source, parent) = engine_with_in_flight_venom();
    engine
        .replace_dna(source, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    assert!(engine.robot_state(parent).unwrap().effects.paralysis_active);
    engine
        .replace_dna(
            parent,
            LegacyDna::parse("start\n50 .repro store\nstop").unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    let child = engine
        .snapshot()
        .organisms
        .iter()
        .find(|organism| organism.parents.contains(&Some(parent)))
        .unwrap()
        .id;
    assert!(!engine.robot_state(child).unwrap().effects.paralysis_active);
    assert_eq!(
        engine
            .robot_state(child)
            .unwrap()
            .effects
            .paralysis_countdown,
        0.0
    );

    engine.remove(parent).unwrap();
    let replacement = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
        .unwrap();
    assert_eq!(replacement.slot(), parent.slot());
    assert!(
        !engine
            .robot_state(replacement)
            .unwrap()
            .effects
            .paralysis_active
    );
    assert_eq!(
        engine
            .robot_state(replacement)
            .unwrap()
            .effects
            .paralysis_countdown,
        0.0
    );
}

#[test]
fn db3s_v2_migrates_every_existing_robot_field() {
    let fixture = include_bytes!("fixtures/db3s_v2_phase1.bin");
    assert_eq!(u16::from_le_bytes([fixture[4], fixture[5]]), 2);

    let restored = SaveFile::decode(fixture).unwrap();
    assert_eq!(restored.snapshot().tick, 42);
    assert_eq!(restored.snapshot().world_size, [2222.0, 1111.0]);
    let id = restored.snapshot().organisms[0].id;
    let after = restored.robot_state(id).unwrap();
    assert_eq!(after.identity.species, darwinbots_engine::SpeciesId(1));
    assert_eq!(after.identity.parents, [None, None]);
    assert_eq!(after.identity.absolute_id, 0);
    assert!(after.flags.exists);
    assert!(after.flags.vegetable);
    assert!(after.flags.fixed);
    assert!(!after.flags.can_see);
    assert!(!after.flags.can_move);
    assert!(!after.flags.can_mutate);
    assert!(!after.flags.can_receive_viruses);
    assert_eq!(after.kinematics.position, [321.0, 654.0]);
    assert_eq!(after.kinematics.previous_position, [320.0, 650.0]);
    assert_eq!(after.kinematics.velocity, [1.5, -2.5]);
    assert_eq!(after.kinematics.actual_velocity, [1.25, -2.25]);
    assert_eq!(after.kinematics.pending_velocity, [3.5, -4.5]);
    assert_eq!(after.materials.energy, 1234);
    assert_eq!(after.materials.body, 876);
    assert_eq!(after.materials.mass, 9.5);
    assert_eq!(after.materials.radius, 8.5);
    assert_eq!(after.materials.waste, 76);
    assert_eq!(after.materials.permanent_waste, 65);
    assert_eq!(after.materials.shell, 54);
    assert_eq!(after.materials.slime, 43);
    assert_eq!(after.materials.venom, 32);
    assert_eq!(after.materials.poison, 21);
    assert_eq!(after.materials.chloroplasts, 12345);
    assert_eq!(after.materials.aim, 678);
    assert_eq!(after.materials.pain, 17);
    assert_eq!(after.materials.pleasure, 18);
    assert_eq!(after.materials.paralyzed, 19);
    assert_eq!(after.materials.poisoned, 20);
    assert_eq!(after.counters.age, 12);
    assert_eq!(after.counters.generation, 0);
    assert_eq!(after.counters.birth_cycle, 0);
    assert_eq!(after.program.memory.read(50), 4321);
    assert_eq!(after.program.rng_state, 987654321);
    assert_eq!(
        after.program.mutation_profile,
        StandardMutationProfile::default()
    );
    assert_eq!(after.program.point_mutation_schedule, None);
    assert_eq!(
        after.heredity,
        darwinbots_engine::RobotHeredityState::default()
    );
    assert!(after.program.dna.uses_new_move());
    assert_eq!(after.program.dna.user_variable("fixturevar"), Some(777));
    assert_eq!(
        after.program.dna.compatibility_warnings(),
        ["frozen v2 warning"]
    );
    assert_eq!(after.program.dna.metadata().generation, 4);
    assert_eq!(after.program.dna.metadata().mutations, 5);
    assert_eq!(after.program.dna.metadata().tag, "frozen-v2");
    assert_eq!(
        after.program.dna.metadata().legacy_hash.as_deref(),
        Some("legacy-v2-hash")
    );
    assert!(!after.program.dna.metadata().hash_valid);
    assert!(after.effects.forced_reproduction);
    assert!(!after.effects.paralysis_active);
    assert_eq!(after.effects.paralysis_countdown, 0.0);
    assert_eq!(after.effects.paralysis_location, 0);
    assert_eq!(after.effects.paralysis_value, 0);
    assert!(!after.effects.poisoning_active);
    assert_eq!(after.effects.poisoning_countdown, 0.0);
    assert_eq!(after.effects.poisoning_location, 0);
    assert_eq!(after.effects.poisoning_value, 0);
    assert_eq!(after.phenotype.lineage_id, 22);
    assert_eq!(after.phenotype.color, 4281558681);
    assert_eq!(after.phenotype.accumulated_mutations, 6);

    let snapshot = restored.snapshot();
    assert_eq!(snapshot.stats.completed_cycles, 42);
    assert_eq!(snapshot.stats.projectile_effects, 5);
    assert_eq!(snapshot.stats.intentional_movement_events, 15);
    assert_eq!(snapshot.ties.len(), 1);
    assert_eq!(snapshot.ties[0].rest_length, 45.5);
    assert_eq!(snapshot.species[1].minimum_population, 2);
    assert_eq!(snapshot.species[1].mutation_rate, 1.75);
    assert!(!snapshot.mutations_enabled);
    assert_eq!(snapshot.global_mutation_multiplier, 2.5);
    assert!(!snapshot.ties_enabled);
    assert!(!snapshot.robot_fixing_allowed);
    assert!(!snapshot.asexual_reproduction_allowed);
    assert_eq!(snapshot.obstacles[0].id, 31);
    assert_eq!(snapshot.teleporters[0].id, 32);
    assert_eq!(snapshot.corpses[0].age, 111);
    assert_eq!(snapshot.shots[0].value, 444);
    assert_eq!(snapshot.history[0].tick, 40);

    let migrated = SaveFile::encode(&restored).unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&migrated[10..]).unwrap();
    assert_eq!(payload["config"]["seed"], 77);
    assert_eq!(payload["config"]["physics"]["max_velocity"], 99.0);
    assert_eq!(payload["config"]["shots"]["speed"], 55.0);
    assert_eq!(payload["config"]["vegetation"]["cycle_length"], 88);
    assert_eq!(payload["config"]["dna_costs"]["global_multiplier"], 11.0);
    assert_eq!(payload["config"]["dna_costs"]["dna_copy"], 0.0);
    assert_eq!(payload["next_absolute_id"], 1);
    assert_eq!(payload["slots"][0]["organism"]["buoyancy"], 0.0);
    assert_eq!(payload["slots"][1]["generation"], 9);
    assert_eq!(
        payload["slots"][0]["organism"]["materials"]["previous_energy"],
        1222
    );
    assert_eq!(
        payload["slots"][0]["organism"]["materials"]["plant_energy_carry_micros"],
        111
    );
    assert_eq!(
        payload["slots"][0]["organism"]["materials"]["plant_body_carry_micros"],
        222
    );
    assert_eq!(payload["free_slots"], serde_json::json!([1]));
    assert_eq!(
        payload["pending_mutations"],
        serde_json::json!([{"slot": 0, "generation": 0}])
    );
    assert_eq!(payload["species_templates"][1]["initial_energy"], 2222);
    assert_eq!(payload["next_lineage_id"], 44);
    assert_eq!(payload["projectiles"]["ages"], serde_json::json!([2, 0]));
    assert_eq!(payload["projectiles"]["free_slots"], serde_json::json!([1]));
    assert_eq!(
        payload["projectiles"]["source_species_names"],
        serde_json::json!([null, null])
    );
    assert_eq!(
        payload["projectiles"]["toxin_memory_locations"],
        serde_json::json!([0, 0])
    );
    assert_eq!(
        payload["projectiles"]["toxin_memory_values"],
        serde_json::json!([0, 0])
    );
    assert_eq!(payload["vegetation_runtime"]["repopulation_counter"], 16);
    assert_eq!(payload["vegetation_runtime"]["day_night_counter"], 17);
}

#[test]
fn db3s_v2_accepts_parent_defaulted_field_omissions() {
    let fixture = include_bytes!("fixtures/db3s_v2_phase1.bin");
    let frame = |payload: &serde_json::Value| {
        let payload = serde_json::to_vec(payload).unwrap();
        let mut save = Vec::with_capacity(10 + payload.len());
        save.extend_from_slice(b"DB3S");
        save.extend_from_slice(&2_u16.to_le_bytes());
        save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        save.extend_from_slice(&payload);
        save
    };
    let mut payload: serde_json::Value = serde_json::from_slice(&fixture[10..]).unwrap();
    let root = payload.as_object_mut().unwrap();

    let config = root["config"].as_object_mut().unwrap();
    for field in [
        "vegetable_population_cap",
        "toroidal_world",
        "metabolism_cost",
        "vegetable_energy_per_tick",
        "sunlight_energy",
        "gravity",
        "drag",
        "brownian_motion",
        "physics",
        "shots",
        "vegetation",
        "dna_costs",
        "auto_speciation",
        "speciation_genetic_distance_percent",
    ] {
        config.remove(field);
    }

    let organism = root["slots"][0]["organism"].as_object_mut().unwrap();
    for field in ["species", "parents", "phenotype", "flags"] {
        organism.remove(field);
    }
    let dna = organism["dna"].as_object_mut().unwrap();
    for field in [
        "user_variables",
        "new_move",
        "compatibility_warnings",
        "metadata",
    ] {
        dna.remove(field);
    }
    let kinematics = root["kinematics"].as_object_mut().unwrap();
    kinematics.remove("previous_positions");
    kinematics.remove("actual_velocities");
    root.remove("biology");
    root.remove("forced_reproductions");

    let snapshot = root["snapshot"].as_object_mut().unwrap();
    let organism_snapshot = snapshot["organisms"][0].as_object_mut().unwrap();
    organism_snapshot.remove("phenotype");
    organism_snapshot.remove("vision");
    let stats = snapshot["stats"].as_object_mut().unwrap();
    for field in [
        "completed_cycles",
        "projectile_impacts",
        "projectile_effects",
        "energy_harvested",
        "energy_donated",
        "reseeds",
        "self_reproductions",
        "feeding_events",
        "plant_energy_generated",
        "plant_body_produced",
        "intentional_movement_events",
    ] {
        stats.remove(field);
    }
    snapshot["ties"][0]
        .as_object_mut()
        .unwrap()
        .remove("first_port");
    snapshot["ties"][0]
        .as_object_mut()
        .unwrap()
        .remove("second_port");
    for field in [
        "world_size",
        "corpses",
        "shots",
        "history",
        "render_instances",
        "species",
        "mutations_enabled",
        "global_mutation_multiplier",
        "ties_enabled",
        "robot_fixing_allowed",
        "asexual_reproduction_allowed",
        "obstacles",
        "teleporters",
        "phase_timings",
    ] {
        snapshot.remove(field);
    }
    root["ties"][0]
        .as_object_mut()
        .unwrap()
        .remove("first_port");
    root["ties"][0]
        .as_object_mut()
        .unwrap()
        .remove("second_port");

    for field in [
        "species",
        "species_templates",
        "mutations_enabled",
        "global_mutation_multiplier",
        "ties_enabled",
        "robot_fixing_allowed",
        "asexual_reproduction_allowed",
        "next_lineage_id",
        "obstacles",
        "teleporters",
        "corpses",
        "projectiles",
        "vegetation_runtime",
        "history",
    ] {
        root.remove(field);
    }

    let restored = SaveFile::decode(&frame(&payload)).unwrap();
    let id = restored.snapshot().organisms[0].id;
    let robot = restored.robot_state(id).unwrap();
    assert_eq!(robot.identity.species, darwinbots_engine::SpeciesId(0));
    assert_eq!(robot.identity.parents, [None, None]);
    assert!(robot.flags.exists);
    assert!(!robot.flags.corpse);
    assert!(!robot.flags.dead);
    assert!(!robot.flags.vegetable);
    assert!(!robot.flags.fixed);
    assert!(robot.flags.can_see);
    assert!(robot.flags.can_execute_dna);
    assert!(robot.flags.can_move);
    assert!(robot.flags.can_reproduce);
    assert!(robot.flags.can_mutate);
    assert!(robot.flags.can_use_chloroplasts);
    assert!(robot.flags.can_receive_viruses);
    assert_eq!(robot.phenotype.lineage_id, 1);
    assert_eq!(robot.phenotype.color, 0xff62_a844);
    assert_eq!(robot.phenotype.skin[0].radius, 0.77);
    assert_eq!(robot.phenotype.skin[0].angle, 68);
    assert!(!robot.program.dna.uses_new_move());
    assert!(robot.program.dna.compatibility_warnings().is_empty());
    assert_eq!(robot.program.dna.metadata().generation, 0);
    assert_eq!(robot.program.dna.metadata().mutations, 0);
    assert!(robot.program.dna.metadata().tag.is_empty());
    assert!(robot.program.dna.metadata().legacy_hash.is_none());
    assert!(robot.program.dna.metadata().hash_valid);
    assert!(!robot.effects.forced_reproduction);
    assert_eq!(
        robot.kinematics.previous_position,
        robot.kinematics.position
    );
    assert_eq!(robot.kinematics.actual_velocity, [0.0, 0.0]);
    assert_eq!(restored.snapshot().world_size, [2222.0, 1111.0]);
    assert!(restored.snapshot().mutations_enabled);
    assert_eq!(restored.snapshot().global_mutation_multiplier, 1.0);
    assert!(restored.snapshot().ties_enabled);
    assert!(restored.snapshot().robot_fixing_allowed);
    assert!(restored.snapshot().asexual_reproduction_allowed);
    assert_eq!(restored.snapshot().ties[0].first_port, 0);
    assert_eq!(restored.snapshot().ties[0].second_port, 0);
    assert_eq!(restored.snapshot().ties[0].last, 0);
    assert_eq!(restored.snapshot().ties[0].b, 0.1);
    assert_eq!(restored.snapshot().ties[0].k, 0.05);
    assert!(restored.snapshot().ties[0].hardened);

    let migrated = SaveFile::encode(&restored).unwrap();
    let migrated: serde_json::Value = serde_json::from_slice(&migrated[10..]).unwrap();
    assert_eq!(migrated["config"]["vegetable_population_cap"], 500);
    assert_eq!(migrated["config"]["toroidal_world"], true);
    assert_eq!(migrated["config"]["sunlight_energy"], 100);
    assert_eq!(migrated["config"]["physics"]["max_velocity"], 180.0);
    assert_eq!(migrated["config"]["shots"]["speed"], 40.0);
    assert_eq!(
        migrated["config"]["vegetation"]["start_chloroplasts"],
        16_000
    );
    assert_eq!(migrated["config"]["dna_costs"]["global_multiplier"], 1.0);
    assert_eq!(
        migrated["config"]["speciation_genetic_distance_percent"],
        20.0
    );
    assert_eq!(migrated["species"][0]["lineage_id"], 1);
    assert_eq!(
        migrated["species"][0]["skin"][0]["radius"],
        serde_json::json!(0.77_f32)
    );
    assert_eq!(migrated["species"][0]["skin"][0]["angle"], 68);
    assert_eq!(migrated["species_templates"], serde_json::json!([null]));
    assert_eq!(migrated["projectiles"]["owners"], serde_json::json!([]));
    assert_eq!(migrated["vegetation_runtime"]["repopulation_counter"], 0);
    assert_eq!(migrated["vegetation_runtime"]["day_night_counter"], 0);

    let mut nested: serde_json::Value = serde_json::from_slice(&fixture[10..]).unwrap();
    let root = nested.as_object_mut().unwrap();
    let config = root["config"].as_object_mut().unwrap();
    config.insert("physics".to_owned(), serde_json::json!({}));
    config.insert("shots".to_owned(), serde_json::json!({}));
    config.insert("vegetation".to_owned(), serde_json::json!({}));
    config.insert("dna_costs".to_owned(), serde_json::json!({}));

    let biology = root["biology"][0].as_object_mut().unwrap();
    for field in [
        "energy",
        "mass",
        "radius",
        "permanent_waste",
        "plant_energy_carry_micros",
        "plant_body_carry_micros",
    ] {
        biology.remove(field);
    }
    let stats = root["stats"].as_object_mut().unwrap();
    for field in [
        "completed_cycles",
        "projectile_impacts",
        "projectile_effects",
        "energy_harvested",
        "energy_donated",
        "reseeds",
        "self_reproductions",
        "feeding_events",
        "plant_energy_generated",
        "plant_body_produced",
        "intentional_movement_events",
    ] {
        stats.remove(field);
    }
    for field in ["mutation_rate", "skin", "lineage_id"] {
        root["species"][1].as_object_mut().unwrap().remove(field);
    }
    for field in ["species", "repopulating"] {
        root["corpses"][0].as_object_mut().unwrap().remove(field);
    }
    for field in ["projectile_impacts", "plant_energy_generated"] {
        root["history"][0].as_object_mut().unwrap().remove(field);
    }

    let snapshot = root["snapshot"].as_object_mut().unwrap();
    for field in ["species", "repopulating"] {
        snapshot["corpses"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
    }
    for field in ["velocity", "age", "range", "energy", "impact_flash"] {
        snapshot["shots"][0].as_object_mut().unwrap().remove(field);
    }
    for field in ["projectile_impacts", "plant_energy_generated"] {
        snapshot["history"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
    }
    for field in ["generation", "aim", "skin", "lineage_id", "vegetable"] {
        snapshot["render_instances"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
    }
    for field in ["mutation_rate", "skin", "lineage_id"] {
        snapshot["species"][1]
            .as_object_mut()
            .unwrap()
            .remove(field);
    }
    for field in ["cycle", "projectiles", "vegetation"] {
        snapshot["phase_timings"]
            .as_object_mut()
            .unwrap()
            .remove(field);
    }

    let nested = SaveFile::decode(&frame(&nested)).unwrap();
    let id = nested.snapshot().organisms[0].id;
    let robot = nested.robot_state(id).unwrap();
    assert_eq!(robot.materials.mass, 1.0);
    assert_eq!(robot.materials.radius, 1.0);
    assert_eq!(robot.materials.permanent_waste, 0);
    assert_eq!(nested.snapshot().stats.projectile_effects, 0);
    assert_eq!(nested.snapshot().corpses[0].species, 0);
    assert!(!nested.snapshot().corpses[0].repopulating);
    assert_eq!(nested.snapshot().history[0].projectile_impacts, 0);
    assert_eq!(nested.snapshot().history[0].plant_energy_generated, 0);

    let nested = SaveFile::encode(&nested).unwrap();
    let nested: serde_json::Value = serde_json::from_slice(&nested[10..]).unwrap();
    assert_eq!(nested["config"]["physics"]["max_velocity"], 180.0);
    assert_eq!(nested["config"]["shots"]["speed"], 40.0);
    assert_eq!(nested["config"]["vegetation"]["cycle_length"], 10_000);
    assert_eq!(nested["config"]["dna_costs"]["global_multiplier"], 1.0);
    assert_eq!(nested["stats"]["intentional_movement_events"], 0);
    assert_eq!(nested["species"][1]["mutation_rate"], 0.0);
    assert_eq!(
        nested["slots"][0]["organism"]["materials"]["plant_energy_carry_micros"],
        0
    );
    assert_eq!(
        nested["slots"][0]["organism"]["materials"]["plant_body_carry_micros"],
        0
    );
}

#[test]
fn db3s_v3_round_trip_preserves_complete_phase1_state() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let species = SpeciesDefinition {
        name: "db3s-v3-phase1".to_owned(),
        vegetable: true,
        ..Default::default()
    };
    let species = engine.register_species(species);
    let id = engine
        .spawn_species_at(
            LegacyDna::parse("cond\n1 0 >\nstart\n17 .up store\nstop").unwrap(),
            species,
            [321.0, 654.0],
        )
        .unwrap();
    engine.tick().unwrap();
    let before = engine.robot_state(id).unwrap().clone();

    let bytes = SaveFile::encode(&engine).unwrap();
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 3);
    assert!(!String::from_utf8_lossy(&bytes).contains("activation_bitmap"));
    assert!(!String::from_utf8_lossy(&bytes).contains("debug_text"));

    let restored = SaveFile::decode(&bytes).unwrap();
    let after = restored.robot_state(id).unwrap();
    assert_eq!(after.identity, before.identity);
    assert_eq!(after.flags, before.flags);
    assert_eq!(after.kinematics, before.kinematics);
    assert_eq!(after.materials, before.materials);
    assert_eq!(after.counters, before.counters);
    assert_eq!(after.program.dna, before.program.dna);
    assert_eq!(after.program.memory, before.program.memory);
    assert_eq!(
        after.program.private_variables,
        before.program.private_variables
    );
    assert_eq!(after.program.rng_state, before.program.rng_state);
    assert_eq!(
        after.program.mutation_metadata,
        before.program.mutation_metadata
    );
    assert_eq!(
        after.program.fractional_cost_remainder,
        before.program.fractional_cost_remainder
    );
    assert_eq!(after.program.current_gene, before.program.current_gene);
    assert_eq!(
        after.program.condition_count,
        before.program.condition_count
    );
    assert!(after.program.activation_bitmap.is_empty());
    assert!(after.program.debug_text.is_empty());
    assert_eq!(after.senses, before.senses);
    assert_eq!(after.effects, before.effects);
    assert_eq!(after.heredity, before.heredity);
    assert_eq!(after.ties, before.ties);
    assert_eq!(after.phenotype, before.phenotype);
}

#[test]
fn mutation_free_save_disables_saved_mutations_without_changing_live_engine() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine
        .update_mutation_settings(true, 2.5, &[(darwinbots_engine::SpeciesId(0), 3.5)])
        .unwrap();

    let bytes = SaveFile::encode_without_mutations(&engine).unwrap();
    let restored = SaveFile::decode(&bytes).unwrap();

    assert!(!restored.snapshot().mutations_enabled);
    assert_eq!(restored.snapshot().global_mutation_multiplier, 0.0);
    assert!(
        restored
            .snapshot()
            .species
            .iter()
            .all(|species| species.mutation_rate == 0.0)
    );
    assert!(engine.snapshot().mutations_enabled);
    assert_eq!(engine.snapshot().global_mutation_multiplier, 2.5);
    assert_eq!(engine.snapshot().species[0].mutation_rate, 3.5);
}

#[test]
fn unavailable_gpu_falls_back_to_cpu_when_allowed() {
    let config = EngineConfig {
        backend: BackendPreference::Gpu,
        allow_cpu_fallback: true,
        force_gpu_unavailable_for_tests: true,
        ..EngineConfig::testing()
    };

    let engine = Engine::new(config).unwrap();

    assert_eq!(engine.backend(), BackendKind::Cpu);
    assert!(engine.capabilities().fallback_reason.is_some());
}

#[test]
fn unavailable_gpu_is_an_error_when_fallback_is_disabled() {
    let config = EngineConfig {
        backend: BackendPreference::Gpu,
        allow_cpu_fallback: false,
        force_gpu_unavailable_for_tests: true,
        ..EngineConfig::testing()
    };

    assert!(matches!(
        Engine::new(config),
        Err(EngineError::GpuUnavailable(_))
    ));
}
#[test]
fn db2_defaults_are_exposed_by_engine_config() {
    let config = EngineConfig::default();
    assert_eq!(config.world_width, 16_000.0);
    assert_eq!(config.world_height, 12_000.0);
    assert_eq!(config.brownian_motion, 0.0);
    assert_eq!(config.physics.max_velocity, 180.0);
    assert_eq!(config.physics.movement_efficiency, 0.66);
    assert_eq!(config.physics.surface_gravity, 2.0);
    assert_eq!(config.physics.static_friction, 0.6);
    assert_eq!(config.physics.kinetic_friction, 0.4);
    assert_eq!(config.physics.density, 0.0);
    assert_eq!(config.physics.viscosity, 0.0);
    assert_eq!(config.physics.elasticity, 0.0);
    assert_eq!(config.shots.speed, 40.0);
    assert_eq!(config.shots.range_multiplier, 1.0);
    assert_eq!(config.shots.decay, 40.0);
    assert!(!config.shots.energy_shots_do_not_decay);
    assert!(!config.shots.waste_shots_do_not_decay);
    assert_eq!(config.vegetation.start_chloroplasts, 16_000);
    assert_eq!(config.vegetation.max_energy_per_tick, 40);
    assert_eq!(config.vegetation.minimum_chloroplast_equivalents, 10);
    assert_eq!(config.vegetation.repopulation_amount, 10);
    assert_eq!(config.vegetation.repopulation_cooldown, 25);
    assert_eq!(config.vegetation.feeding_to_body, 0.5);
    assert!(config.vegetation.daytime);
    assert!(!config.vegetation.day_night_enabled);
}

#[test]
fn save_version_one_is_rejected_after_db2_state_upgrade() {
    let engine = Engine::new(EngineConfig::testing()).unwrap();
    let mut bytes = SaveFile::encode(&engine).unwrap();
    bytes[4..6].copy_from_slice(&1u16.to_le_bytes());

    let error = match SaveFile::decode(&bytes) {
        Ok(_) => panic!("save version 1 was accepted"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("unsupported version 1"));
}
