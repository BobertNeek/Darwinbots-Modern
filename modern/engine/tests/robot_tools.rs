use darwinbots_engine::{
    CycleEvent, CyclePhase, Engine, EngineConfig, LegacyDna, PlayerBotBinding, RestrictionColumn,
    RestrictionOptions,
    ffi::{
        DbBuffer, DbStatus, db_buffer_free, db_engine_command_batch, db_engine_create,
        db_engine_destroy, db_engine_snapshot_json,
    },
};

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
    assert_eq!(engine.robot_state(id).unwrap().materials.energy, 4_321);
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
