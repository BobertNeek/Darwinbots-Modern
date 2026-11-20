use darwinbots_engine::{Engine, EngineConfig, LegacyDna, RobotSenseState};

fn engine() -> Engine {
    Engine::new(EngineConfig::default()).expect("CPU engine should initialize")
}

#[test]
fn default_sense_state_keeps_the_stable_eleven_slot_census_shape() {
    assert_eq!(
        RobotSenseState::default().vision_reference_occurrences,
        vec![0; 11]
    );
}

#[test]
fn census_counts_legacy_categories_and_publishes_exact_own_readbacks() {
    let mut engine = engine();
    let dna = LegacyDna::parse(
        "cond\n\
         0 1 =\n\
         start\n\
         1 store\n\
         2 store\n\
         3 store\n\
         4 store\n\
         5 store\n\
         6 store\n\
         7 store\n\
         826 store\n\
         824 store\n\
         .tie 99\n\
         .tie store\n\
         *501 *509\n\
         1 99\n\
         .eye5\n\
         *500 *510\n\
         stop\n\
         end\n\
         7 store 826 store 824 store 330 *505",
    )
    .unwrap();
    let id = engine.spawn(dna).unwrap();

    engine.tick().unwrap();

    assert_eq!(
        engine
            .robot_state(id)
            .unwrap()
            .senses
            .vision_reference_occurrences,
        vec![1, 1, 1, 1, 1, 1, 1, 2, 2, 1, 1]
    );
    let own_readbacks = (721..=731)
        .map(|address| engine.memory_at(id, address).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(own_readbacks, vec![1, 1, 1, 1, 1, 1, 1, 2, 2, 1, 1]);
}

#[test]
fn current_dna_census_is_visible_to_the_next_dna_evaluation_after_spawn_and_replace() {
    let mut engine = engine();
    let initial = LegacyDna::parse(
        "start\n\
         0 1 store\n\
         *721 50 store\n\
         stop\n\
         end",
    )
    .unwrap();
    let id = engine.spawn(initial).unwrap();

    engine.tick().unwrap();
    assert_eq!(engine.memory_at(id, 50).unwrap(), 1);
    assert_eq!(
        engine
            .robot_state(id)
            .unwrap()
            .senses
            .vision_reference_occurrences,
        vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );

    let replacement = LegacyDna::parse(
        "start\n\
         0 1 store\n\
         0 1 store\n\
         0 1 store\n\
         *721 50 store\n\
         stop\n\
         end",
    )
    .unwrap();
    engine.replace_dna(id, replacement).unwrap();

    engine.tick().unwrap();
    assert_eq!(engine.memory_at(id, 50).unwrap(), 3);
    assert_eq!(
        engine
            .robot_state(id)
            .unwrap()
            .senses
            .vision_reference_occurrences,
        vec![3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(engine.memory_at(id, 721).unwrap(), 3);
}

#[test]
fn native_own_memory_survives_editor_refresh_and_both_save_formats() {
    let mut engine =
        Engine::from_native_sim(include_bytes!("fixtures/db24832-original-user.sim")).unwrap();
    let id = engine.snapshot().organisms[0].id;
    for address in 721..=731 {
        engine
            .set_organism_memory(id, address, -i32::from(address))
            .unwrap();
    }
    engine.set_ties_enabled(true);
    for address in 721..=731 {
        assert_eq!(engine.memory_at(id, address).unwrap(), -address);
    }
    let saved = darwinbots_engine::SaveFile::encode(&engine).unwrap();
    let resumed = darwinbots_engine::SaveFile::decode(&saved).unwrap();
    assert_eq!(
        saved,
        darwinbots_engine::SaveFile::encode(&resumed).unwrap()
    );
    let native = Engine::from_native_sim(&resumed.export_native_sim().unwrap()).unwrap();
    let imported_id = native.snapshot().organisms[0].id;
    for address in 721..=731 {
        assert_eq!(resumed.memory_at(id, address).unwrap(), -address);
        assert_eq!(native.memory_at(imported_id, address).unwrap(), -address);
    }
}

#[test]
fn native_own_memory_survives_cycles_until_explicit_dna_replacement() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let id = engine
        .spawn_at(
            LegacyDna::parse("start *721 50 store stop").unwrap(),
            [500.0; 2],
        )
        .unwrap();
    engine.update_mutation_settings(false, 1.0, &[]).unwrap();
    engine.set_organism_memory(id, 721, 123).unwrap();
    engine.set_organism_memory(id, 729, 987).unwrap();
    engine.tick_many(3).unwrap();
    assert_eq!(engine.memory_at(id, 50).unwrap(), 123);
    assert_eq!(engine.memory_at(id, 729).unwrap(), 987);
    assert_eq!(engine.memory_at(id, 721).unwrap(), 123);
    engine
        .replace_dna(
            id,
            LegacyDna::parse("start 0 .up store *721 50 store stop").unwrap(),
        )
        .unwrap();
    assert_eq!(engine.memory_at(id, 721).unwrap(), 1);
    engine.tick().unwrap();
    assert_eq!(engine.memory_at(id, 50).unwrap(), 1);
}
