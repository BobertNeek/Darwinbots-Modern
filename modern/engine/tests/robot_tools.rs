use darwinbots_engine::{
    CycleEvent, CyclePhase, Engine, EngineConfig, LegacyDna, PlayerBotBinding, RestrictionColumn,
    RestrictionOptions, SaveFile, SpeciesDefinition, SpeciesSettings,
    ffi::{
        DbBuffer, DbStatus, db_buffer_free, db_engine_command_batch, db_engine_create,
        db_engine_destroy, db_engine_snapshot_json,
    },
    native_binary::NativeRecordFile,
};

#[test]
fn auto_tag_uses_exact_founder_names_and_persists_the_native_tag() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let founder = engine.register_species(SpeciesDefinition {
        name: "Founder.txt".into(),
        ..SpeciesDefinition::default()
    });
    let lowercase = engine.register_species(SpeciesDefinition {
        name: "founder.txt".into(),
        ..SpeciesDefinition::default()
    });
    let dna = LegacyDna::parse("start stop\n'#tag: Old tag").unwrap();
    let first = engine
        .spawn_species_at(dna.clone(), founder, [100.0; 2])
        .unwrap();
    let second = engine
        .spawn_species_at(dna.clone(), founder, [200.0; 2])
        .unwrap();
    let unrelated = engine.spawn_species_at(dna, lowercase, [300.0; 2]).unwrap();

    engine
        .update_species_settings(vec![SpeciesSettings {
            species_id: Some(founder),
            definition: SpeciesDefinition {
                name: "Edited roster name".into(),
                ..SpeciesDefinition::default()
            },
            dna: "start stop".into(),
            initial_count: 2,
            initial_energy: 1000,
            comment: String::new(),
        }])
        .unwrap();
    let before = SaveFile::encode(&engine).unwrap();
    assert!(engine.tag_organisms_by_name("", "Other tag").is_err());
    assert_eq!(
        engine
            .tag_organisms_by_name(" Founder.txt", "Other tag")
            .unwrap(),
        0
    );
    assert_eq!(
        engine
            .tag_organisms_by_name("Edited roster name", "Other tag")
            .unwrap(),
        0
    );
    assert_eq!(SaveFile::encode(&engine).unwrap(), before);

    let tag = format!("\0Alpha\x1f\x7fé€ÿ{}", "z".repeat(60));
    let expected = format!("?Alpha????ÿ{}", "z".repeat(34));
    assert_eq!(
        engine.tag_organisms_by_name("Founder.txt", &tag).unwrap(),
        2
    );
    assert_eq!(engine.snapshot().tick, 0);

    let mut resumed = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
    let exported =
        NativeRecordFile::decode(&resumed.export_native_organisms(&[first, second]).unwrap())
            .unwrap();
    let mut expected_native = b"?Alpha????\xff".to_vec();
    expected_native.extend([b'z'; 34]);
    expected_native.resize(50, b' ');
    for (id, record) in [first, second].into_iter().zip(exported.robots) {
        assert_eq!(record.bytes("species_name").unwrap(), b"Founder.txt");
        assert_eq!(record.bytes("tag").unwrap(), expected_native);
        assert!(
            resumed
                .export_dna(id)
                .unwrap()
                .ends_with(&format!("'#tag: {expected}"))
        );
    }
    assert!(
        resumed
            .export_dna(unrelated)
            .unwrap()
            .ends_with("'#tag: Old tag")
    );
    assert_eq!(resumed.tag_organisms_by_name("Founder.txt", "").unwrap(), 2);
    for id in [first, second] {
        assert!(!resumed.export_dna(id).unwrap().contains("'#tag:"));
    }
}

fn binding(key: u32, memory_address: u16, value: i32, invert: bool) -> PlayerBotBinding {
    PlayerBotBinding {
        key,
        memory_address,
        value,
        invert,
    }
}

#[test]
fn player_bot_writes_after_dna_clear_in_the_same_full_cycle() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let id = engine
        .spawn(LegacyDna::parse("start\n1 50 store\nstop").unwrap())
        .unwrap();

    engine
        .configure_player_bot(true, Some(id), vec![binding(65, 50, 77, false)])
        .unwrap();
    engine.set_player_bot_key(id, 65, true).unwrap();

    let trace = engine.tick_with_trace().unwrap();
    let phases: Vec<_> = trace
        .events
        .iter()
        .filter_map(|event| match event {
            CycleEvent::PhaseStarted { phase, .. } => Some(*phase),
            _ => None,
        })
        .collect();
    assert!(
        phases
            .iter()
            .position(|phase| *phase == CyclePhase::AfterDna)
            < phases
                .iter()
                .position(|phase| *phase == CyclePhase::PlayerBot)
    );
    assert!(
        phases
            .iter()
            .position(|phase| *phase == CyclePhase::PlayerBot)
            < phases
                .iter()
                .position(|phase| *phase == CyclePhase::ShotUpdate)
    );
    assert_eq!(engine.memory_at(id, 50).unwrap(), 77);
}

#[test]
fn player_bot_release_and_invert_follow_pressed_xor_invert() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let id = engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();

    engine
        .configure_player_bot(
            true,
            Some(id),
            vec![binding(65, 50, 11, false), binding(66, 51, 22, true)],
        )
        .unwrap();
    engine.set_player_bot_key(id, 65, true).unwrap();
    engine.set_player_bot_key(id, 66, true).unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.memory_at(id, 50).unwrap(), 11);
    assert_eq!(engine.memory_at(id, 51).unwrap(), 0);

    engine.set_player_bot_key(id, 65, false).unwrap();
    engine.set_player_bot_key(id, 66, false).unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.memory_at(id, 50).unwrap(), 11);
    assert_eq!(engine.memory_at(id, 51).unwrap(), 22);
}

#[test]
fn player_bot_aims_at_current_pointer_for_focus_and_stable_highlights_only() {
    let mut engine = Engine::new_native(EngineConfig::testing()).unwrap();
    let dna = LegacyDna::parse("start\nstop").unwrap();
    let focus = engine.spawn_at(dna.clone(), [500.0, 500.0]).unwrap();
    let highlighted = engine.spawn_at(dna.clone(), [900.0, 500.0]).unwrap();
    let stale = engine.spawn_at(dna.clone(), [1300.0, 500.0]).unwrap();
    engine.remove(stale).unwrap();
    let replacement = engine.spawn_at(dna, [1300.0, 500.0]).unwrap();
    assert_eq!(stale.slot(), replacement.slot());
    engine
        .configure_player_bot(true, Some(focus), vec![binding(65, 19, 77, false)])
        .unwrap();
    engine.set_player_bot_key(focus, 65, true).unwrap();
    let baseline = SaveFile::encode(&engine).unwrap();

    engine.set_player_bot_targets(Some(focus), vec![highlighted, focus, stale]);
    engine.set_player_bot_pointer(Some([500.0, 300.0])).unwrap();
    assert_eq!(SaveFile::encode(&engine).unwrap(), baseline);

    let mut restored = SaveFile::decode(&baseline).unwrap();
    assert_eq!(player_bot_aim_writes(&mut restored), [(focus, 77)]);
    // The older candidate path must see pointer aim before the user's key
    // mapping overwrites the same address in the pre-shot PlayerBot phase.
    assert_eq!(
        player_bot_aim_writes(&mut engine),
        [
            (focus, 314),
            (focus, 77),
            (highlighted, 536),
            (highlighted, 77)
        ],
    );

    engine.set_player_bot_key(focus, 65, false).unwrap();
    engine.set_player_bot_pointer(None).unwrap();
    assert!(player_bot_aim_writes(&mut engine).is_empty());
    engine.set_player_bot_pointer(Some([0.0, 0.0])).unwrap();
    assert!(player_bot_aim_writes(&mut engine).is_empty());
}

fn player_bot_aim_writes(engine: &mut Engine) -> Vec<(darwinbots_engine::OrganismId, i32)> {
    engine
        .tick_with_trace()
        .unwrap()
        .events
        .into_iter()
        .filter_map(|event| match event {
            CycleEvent::MemoryWrite {
                phase: CyclePhase::PlayerBot,
                id,
                address: 19,
                after,
                ..
            } => Some((id, after)),
            _ => None,
        })
        .collect()
}

#[test]
fn restriction_options_apply_editable_column_to_live_robot_and_snapshot() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let id = engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    let non_repopulating = RestrictionColumn {
        kill_multibot: true,
        kill_disqualified: true,
        disable_chloroplasts: true,
        fixed_in_place: true,
        disable_vision: true,
        disable_dna: true,
        disable_reproduction: true,
        virus_immune: true,
        disable_mutations: true,
        disable_movement: true,
    };
    let options = RestrictionOptions {
        non_repopulating: non_repopulating.clone(),
        ..RestrictionOptions::default()
    };

    engine.update_restriction_options(options.clone()).unwrap();

    let state = engine.robot_state(id).unwrap();
    assert!(state.flags.fixed);
    assert!(!state.flags.can_see);
    assert!(!state.flags.can_execute_dna);
    assert!(!state.flags.can_reproduce);
    assert!(!state.flags.can_receive_viruses);
    assert!(!state.flags.can_mutate);
    assert!(!state.flags.can_move);
    assert!(!state.flags.can_use_chloroplasts);
    assert_eq!(engine.memory_at(id, 216).unwrap(), 1);
    assert_eq!(engine.snapshot().restriction_options, options);
}

#[test]
fn console_memory_and_energy_mutations_are_engine_authoritative() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let id = engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();

    engine.set_organism_memory(id, 50, 123).unwrap();
    engine.set_organism_energy(id, 4_321).unwrap();

    assert_eq!(engine.memory_at(id, 50).unwrap(), 123);
    assert_eq!(engine.robot_state(id).unwrap().materials.energy, 4_321.0);
    assert_eq!(engine.memory_at(id, 310).unwrap(), 4_321);
}

#[test]
fn robot_tools_commands_round_trip_through_native_batch_and_snapshot() {
    let mut engine = std::ptr::null_mut();
    assert_eq!(
        db_engine_create(std::ptr::null(), 0, &mut engine),
        DbStatus::Ok
    );
    let batch = br#"{"version":1,"commands":[
      {"type":"import_dna","dna":"start\nstop"},
      {"type":"configure_player_bot","enabled":true,"focus":{"slot":0,"generation":0},"bindings":[{"key":65,"memory_address":50,"value":77,"invert":false}]},
      {"type":"set_player_bot_key","slot":0,"generation":0,"key":65,"pressed":true},
      {"type":"set_organism_memory","slot":0,"generation":0,"address":51,"value":12},
      {"type":"set_organism_energy","slot":0,"generation":0,"energy":4321},
      {"type":"update_restriction_options","options":{"non_repopulating":{"fixed_in_place":true},"repopulating":{"disable_dna":true}}},
      {"type":"tick","count":1}
    ]}"#;
    let mut result = DbBuffer::default();
    assert_eq!(
        db_engine_command_batch(engine, batch.as_ptr(), batch.len(), &mut result),
        DbStatus::Ok
    );
    assert_eq!(db_buffer_free(&mut result), DbStatus::Ok);

    let mut snapshot = DbBuffer::default();
    assert_eq!(db_engine_snapshot_json(engine, &mut snapshot), DbStatus::Ok);
    let json: serde_json::Value =
        serde_json::from_slice(unsafe { std::slice::from_raw_parts(snapshot.data, snapshot.len) })
            .unwrap();
    assert_eq!(json["player_bot"]["bindings"][0]["memory_address"], 50);
    assert_eq!(json["organisms"][0]["energy"], 4_321);
    assert!(
        json["organisms"][0]["memory"]
            .as_array()
            .unwrap()
            .iter()
            .any(|cell| cell["address"] == 50 && cell["value"] == 77)
    );
    assert_eq!(
        json["restriction_options"]["non_repopulating"]["fixed_in_place"],
        true
    );
    assert_eq!(db_buffer_free(&mut snapshot), DbStatus::Ok);
    assert_eq!(db_engine_destroy(engine), DbStatus::Ok);
}

#[test]
fn robot_tag_report_copies_distinct_native_rows_in_live_slot_order() {
    use darwinbots_engine::{
        SaveFile,
        native_binary::{NativeRecordFile, NativeValue},
    };

    let mut file =
        NativeRecordFile::decode(include_bytes!("fixtures/db24832-movement-start-48032.sim"))
            .unwrap();
    let template = file.robots.pop().unwrap();
    let founder = std::str::from_utf8(template.bytes("species_name").unwrap()).unwrap();
    let full_tag = "A".repeat(45);
    for (index, (tag, owner, exists, is_corpse)) in [
        (format!("{full_tag}12345"), "Original user", true, false),
        (format!("{full_tag}67890"), "Original user", true, false),
        (String::new(), "Corpse keeper", true, true),
        ("Inactive slot".into(), "Other user", false, false),
        (format!("{full_tag}12345"), "Other user", true, false),
    ]
    .into_iter()
    .enumerate()
    {
        let mut record = template.clone();
        let slot = (index + 1) as i16;
        record
            .set_value("old_slot", NativeValue::Word(slot))
            .unwrap();
        record
            .set_value("absolute_id", NativeValue::Long(i32::from(slot)))
            .unwrap();
        record
            .set_value("exists", NativeValue::Word(if exists { -1 } else { 0 }))
            .unwrap();
        record
            .set_value("owner_length", NativeValue::Word(owner.len() as i16))
            .unwrap();
        record
            .set_value("owner", NativeValue::Bytes(owner.as_bytes().to_vec()))
            .unwrap();
        let mut tag = tag.into_bytes();
        tag.resize(50, b' ');
        record.set_value("tag", NativeValue::Bytes(tag)).unwrap();
        if is_corpse {
            record.set_value("corpse", NativeValue::Word(-1)).unwrap();
            record
                .set_value("species_name_length", NativeValue::Word(6))
                .unwrap();
            record
                .set_value("species_name", NativeValue::Bytes(b"Corpse".to_vec()))
                .unwrap();
        }
        file.robots.push(record);
    }

    let engine = Engine::from_native_sim(&file.encode().unwrap()).unwrap();
    let before = SaveFile::encode(&engine).unwrap();
    assert_eq!(
        engine.robot_tag_report(),
        format!(
            "Tag:FileName:User\r\n~~~\r\n\r\n{full_tag}:{founder}:Original user\r\n{}:Corpse:Corpse keeper\r\n{full_tag}:{founder}:Other user",
            " ".repeat(45),
        )
    );
    assert_eq!(SaveFile::encode(&engine).unwrap(), before);
    assert_eq!(
        Engine::new(EngineConfig::testing())
            .unwrap()
            .robot_tag_report(),
        "Tag:FileName:User\r\n~~~\r\n"
    );
}
