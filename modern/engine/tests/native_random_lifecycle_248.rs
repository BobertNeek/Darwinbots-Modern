use darwinbots_engine::ffi::{
    DbBuffer, DbStatus, db_buffer_free, db_engine_command_batch, db_engine_create,
    db_engine_destroy, db_engine_import_dna, db_engine_load,
};
use darwinbots_engine::native_binary::{NativeRecordFile, NativeSimulationTail};
use darwinbots_engine::{Engine, EngineConfig, LegacyDna, SaveFile};
use serde_json::{Value, json};

const RANDOM_DNA: &str = "start\n32000 rnd 900 store\nstop";

fn config(seed: u64) -> EngineConfig {
    EngineConfig {
        seed,
        ..EngineConfig::testing()
    }
}

fn native_fixture() -> Vec<u8> {
    let mut engine = Engine::new(config(42)).unwrap();
    engine.update_mutation_settings(false, 1.0, &[]).unwrap();
    engine
        .spawn_at(LegacyDna::parse(RANDOM_DNA).unwrap(), [500.0; 2])
        .unwrap();
    engine.export_native_sim().unwrap()
}

fn payload(engine: &Engine) -> Value {
    let save = SaveFile::encode(engine).unwrap();
    serde_json::from_slice(&save[10..]).unwrap()
}

fn save_with_stream(engine: &Engine, state: u32, spare: Option<f32>) -> Vec<u8> {
    let mut data = payload(engine);
    data["native_random"] = json!({
        "state": state,
        "gaussian_spare": spare,
    });
    save_with_payload(engine, &data)
}

fn save_with_payload(engine: &Engine, data: &Value) -> Vec<u8> {
    let save = SaveFile::encode(engine).unwrap();
    let bytes = serde_json::to_vec(data).unwrap();
    let mut framed = save[..6].to_vec();
    framed.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    framed.extend_from_slice(&bytes);
    framed
}

fn next_random_word(engine: &mut Engine) -> i32 {
    assert_eq!(engine.snapshot().organisms.len(), 1);
    let id = engine.snapshot().organisms[0].id;
    engine.execute_robots().unwrap();
    engine.memory_at(id, 900).unwrap()
}

struct FfiEngine(*mut Engine);

impl FfiEngine {
    fn new(seed: u64) -> Self {
        let bytes = serde_json::to_vec(&config(seed)).unwrap();
        let mut pointer = std::ptr::null_mut();
        assert_eq!(
            db_engine_create(bytes.as_ptr(), bytes.len(), &mut pointer),
            DbStatus::Ok
        );
        assert!(!pointer.is_null());
        Self(pointer)
    }

    fn engine(&self) -> &Engine {
        // This fixture owns the live handle until Drop and never aliases a mutation.
        unsafe { &*self.0 }
    }

    fn engine_mut(&mut self) -> &mut Engine {
        unsafe { &mut *self.0 }
    }

    fn load(&mut self, bytes: &[u8]) {
        assert_eq!(
            db_engine_load(self.0, bytes.as_ptr(), bytes.len()),
            DbStatus::Ok
        );
    }

    fn import_random_bot(&mut self) {
        assert_eq!(
            db_engine_import_dna(self.0, RANDOM_DNA.as_ptr(), RANDOM_DNA.len()),
            DbStatus::Ok
        );
    }

    fn command(&mut self, value: Value) -> Value {
        let bytes = serde_json::to_vec(&value).unwrap();
        let mut buffer = DbBuffer::default();
        assert_eq!(
            db_engine_command_batch(self.0, bytes.as_ptr(), bytes.len(), &mut buffer),
            DbStatus::Ok
        );
        assert!(!buffer.data.is_null());
        let output = unsafe { std::slice::from_raw_parts(buffer.data, buffer.len) }.to_vec();
        assert_eq!(db_buffer_free(&mut buffer), DbStatus::Ok);
        serde_json::from_slice(&output).unwrap()
    }

    fn reject_command(&mut self, text: &str) {
        let before = SaveFile::encode(self.engine()).unwrap();
        let mut buffer = DbBuffer::default();
        let status = db_engine_command_batch(self.0, text.as_ptr(), text.len(), &mut buffer);
        let allocated_output = !buffer.data.is_null();
        if allocated_output {
            assert_eq!(db_buffer_free(&mut buffer), DbStatus::Ok);
        }
        assert_ne!(status, DbStatus::Ok, "unexpectedly accepted {text}");
        assert!(!allocated_output, "rejected command published a response");
        assert_eq!(SaveFile::encode(self.engine()).unwrap(), before);
    }
}

impl Drop for FfiEngine {
    fn drop(&mut self) {
        let _ = db_engine_destroy(self.0);
    }
}

#[test]
fn native_sim_load_executes_the_literal_next_random_word_after_its_prologue() {
    let mut engine = Engine::from_native_sim(&native_fixture()).unwrap();
    // Original load reseeds from 42 and consumes exactly its three prologue draws.
    assert_eq!(next_random_word(&mut engine), 1105);
}

#[test]
fn fresh_native_constructor_and_ffi_create_share_the_fresh_prologue() {
    let mut engine = Engine::new_native(config(42)).unwrap();
    engine
        .spawn_at(LegacyDna::parse(RANDOM_DNA).unwrap(), [500.0; 2])
        .unwrap();
    assert_eq!(next_random_word(&mut engine), 18780);

    let mut ffi = FfiEngine::new(42);
    ffi.import_random_bot();
    assert_eq!(next_random_word(ffi.engine_mut()), 18780);
}

#[test]
fn native_ffi_load_reseeds_uniform_state_but_retains_only_the_previous_gaussian_spare() {
    let fixture = native_fixture();
    let direct = Engine::from_native_sim(&fixture).unwrap();
    let mut ffi = FfiEngine::new(77);
    let previous = save_with_stream(ffi.engine(), 0x123456, Some(4.25));
    ffi.load(&previous);
    assert_eq!(
        payload(ffi.engine())["native_random"],
        json!({"state": 0x123456, "gaussian_spare": 4.25})
    );

    ffi.load(&fixture);
    let loaded = payload(ffi.engine());
    assert_eq!(
        loaded["native_random"]["state"],
        payload(&direct)["native_random"]["state"]
    );
    assert_eq!(loaded["native_random"]["gaussian_spare"], 4.25);
    assert_eq!(next_random_word(ffi.engine_mut()), 1105);
    assert_eq!(
        payload(ffi.engine())["native_random"]["gaussian_spare"],
        4.25
    );
}

#[test]
fn modern_rust_and_ffi_load_restore_the_saved_uniform_state_and_spare_without_inheriting() {
    let imported = Engine::from_native_sim(&native_fixture()).unwrap();
    for spare in [None, Some(-2.5)] {
        let saved = save_with_stream(&imported, 0x2468ac, spare);
        let expected_stream = json!({"state": 0x2468ac, "gaussian_spare": spare});
        let mut direct = SaveFile::decode(&saved).unwrap();
        assert_eq!(payload(&direct)["native_random"], expected_stream);
        let canonical = SaveFile::encode(&direct).unwrap();
        let round_trip = SaveFile::decode(&canonical).unwrap();
        assert_eq!(SaveFile::encode(&round_trip).unwrap(), canonical);

        let mut ffi = FfiEngine::new(99);
        let previous = save_with_stream(ffi.engine(), 0x654321, Some(4.25));
        ffi.load(&previous);
        ffi.load(&saved);
        assert_eq!(payload(ffi.engine())["native_random"], expected_stream);
        assert_eq!(SaveFile::encode(ffi.engine()).unwrap(), canonical);
        assert_eq!(
            next_random_word(ffi.engine_mut()),
            next_random_word(&mut direct)
        );
        assert_eq!(
            SaveFile::encode(ffi.engine()).unwrap(),
            SaveFile::encode(&direct).unwrap()
        );
    }
}

#[test]
fn native_restart_seed_command_is_replayable_and_does_not_reseed_the_live_stream() {
    let mut ffi = FfiEngine::new(42);
    let before = payload(ffi.engine())["native_random"].clone();
    let response = ffi.command(json!({
        "version": 1,
        "commands": [{"type": "set_native_restart_seed", "seed": 8765}],
    }));
    assert_eq!(response["results"], json!([null]));
    let saved = payload(ffi.engine());
    assert_eq!(saved["config"]["native_restart_seed"], 8765);
    assert_eq!(saved["native_random"], before);
    let resumed = SaveFile::decode(&SaveFile::encode(ffi.engine()).unwrap()).unwrap();
    assert_eq!(payload(&resumed)["config"]["native_restart_seed"], 8765);
    assert_eq!(payload(&resumed)["native_random"], before);
}

#[test]
fn gaussian_spare_commands_change_only_the_cache_and_reads_do_not_consume_it() {
    let mut ffi = FfiEngine::new(42);
    for spare in [None, Some(0.0_f32), Some(-0.0), Some(1.25), Some(-2.5)] {
        let mut expected = payload(ffi.engine());
        expected["native_random"]["gaussian_spare"] = json!(spare);
        let response = ffi.command(json!({
            "version": 1,
            "commands": [
                {"type": "set_native_gaussian_spare", "spare": spare},
                {"type": "read_native_gaussian_spare"},
                {"type": "read_native_gaussian_spare"}
            ],
        }));
        assert_eq!(response["results"], json!([null, spare, spare]));
        assert_eq!(payload(ffi.engine()), expected);
        assert_eq!(
            ffi.engine().read_native_gaussian_spare().map(f32::to_bits),
            spare.map(f32::to_bits)
        );

        let before_read = SaveFile::encode(ffi.engine()).unwrap();
        let response = ffi.command(json!({
            "version": 1,
            "commands": [
                {"type": "read_native_gaussian_spare"},
                {"type": "read_native_gaussian_spare"}
            ],
        }));
        assert_eq!(response["results"], json!([spare, spare]));
        assert_eq!(response["tick"], 0);
        assert_eq!(response["population"], 0);
        assert_eq!(response["backend"], "Cpu");
        assert_eq!(SaveFile::encode(ffi.engine()).unwrap(), before_read);
        assert_eq!(
            ffi.engine().read_native_gaussian_spare().map(f32::to_bits),
            spare.map(f32::to_bits)
        );
    }

    let mut independent = FfiEngine::new(77);
    let before = SaveFile::encode(independent.engine()).unwrap();
    let response = independent.command(json!({
        "version": 1,
        "commands": [{"type": "read_native_gaussian_spare"}],
    }));
    assert_eq!(response["results"], json!([null]));
    assert_eq!(SaveFile::encode(independent.engine()).unwrap(), before);
}

#[test]
fn invalid_gaussian_spare_batches_roll_back_cache_and_other_commands() {
    let mut ffi = FfiEngine::new(42);
    ffi.command(json!({
        "version": 1,
        "commands": [{"type": "set_native_gaussian_spare", "spare": 1.25}],
    }));
    // Raw numbers distinguish f32 overflow from json!(NaN), which becomes null.
    ffi.reject_command(r#"{"version":1,"commands":[{"type":"set_native_gaussian_spare","spare":9},{"type":"set_native_gaussian_spare","spare":1e100}]}"#);
    ffi.reject_command(r#"{"version":1,"commands":[{"type":"set_native_gaussian_spare","spare":9},{"type":"set_native_gaussian_spare","spare":-1e100}]}"#);
    ffi.reject_command(
        r#"{"version":1,"commands":[{"type":"set_native_gaussian_spare","spare":1e400}]}"#,
    );
    ffi.reject_command(
        r#"{"version":1,"commands":[{"type":"set_native_gaussian_spare","spare":NaN}]}"#,
    );
    ffi.reject_command(r#"{"version":1,"commands":[{"type":"set_native_gaussian_spare","spare":9},{"type":"set_native_sun_change","sun_change":12},{"type":"set_native_restart_seed","seed":8765},{"type":"tick","count":1000001}]}"#);
    assert_eq!(ffi.engine().read_native_gaussian_spare(), Some(1.25));
}

#[test]
fn gaussian_spare_commands_do_not_enable_a_legacy_random_stream() {
    let mut ffi = FfiEngine::new(42);
    let mut legacy = payload(ffi.engine());
    legacy["native_random"] = Value::Null;
    ffi.load(&save_with_payload(ffi.engine(), &legacy));
    let before = SaveFile::encode(ffi.engine()).unwrap();
    let response = ffi.command(json!({
        "version": 1,
        "commands": [{"type": "read_native_gaussian_spare"}],
    }));
    assert_eq!(response["results"], json!([null]));
    ffi.command(json!({
        "version": 1,
        "commands": [{"type": "set_native_gaussian_spare", "spare": null}],
    }));
    assert_eq!(SaveFile::encode(ffi.engine()).unwrap(), before);
    ffi.reject_command(
        r#"{"version":1,"commands":[{"type":"set_native_gaussian_spare","spare":1.25}]}"#,
    );
    assert!(payload(ffi.engine())["native_random"].is_null());
}

#[test]
fn explicit_gaussian_handoff_survives_native_load_and_yields_to_modern_saved_state() {
    let fixture = native_fixture();
    for spare in [None, Some(0.0_f32), Some(-0.0), Some(1.25)] {
        let mut receiver = FfiEngine::new(77);
        let uniform = payload(receiver.engine())["native_random"]["state"].clone();
        receiver.command(json!({
            "version": 1,
            "commands": [
                {"type": "set_native_gaussian_spare", "spare": spare},
                {"type": "set_native_sun_change", "sun_change": 255}
            ],
        }));
        assert_eq!(
            payload(receiver.engine())["native_random"]["state"],
            uniform
        );
        receiver.load(&fixture);
        assert_eq!(
            receiver
                .engine()
                .read_native_gaussian_spare()
                .map(f32::to_bits),
            spare.map(f32::to_bits)
        );
        assert_eq!(
            payload(receiver.engine())["native_random"]["state"],
            0x97c633
        );
        // Native .sim carries its own weather byte, unlike its process Gaussian.
        assert_eq!(receiver.engine().read_native_sun_change(), 10);
        assert_eq!(next_random_word(receiver.engine_mut()), 1105);
    }

    let source = Engine::from_native_sim(&fixture).unwrap();
    for saved_spare in [None, Some(0.0_f32), Some(-2.5)] {
        let saved = save_with_stream(&source, 0x2468ac, saved_spare);
        let expected = SaveFile::encode(&SaveFile::decode(&saved).unwrap()).unwrap();
        let mut receiver = FfiEngine::new(99);
        receiver.command(json!({
            "version": 1,
            "commands": [
                {"type": "set_native_gaussian_spare", "spare": 1.25},
                {"type": "set_native_sun_change", "sun_change": 255}
            ],
        }));
        receiver.load(&saved);
        assert_eq!(SaveFile::encode(receiver.engine()).unwrap(), expected);
        let response = receiver.command(json!({
            "version": 1,
            "commands": [{"type": "read_native_gaussian_spare"}],
        }));
        assert_eq!(response["results"], json!([saved_spare]));
        assert_eq!(SaveFile::encode(receiver.engine()).unwrap(), expected);
    }
}

#[test]
fn sun_change_commands_preserve_uniform_gaussian_and_other_world_state() {
    let mut ffi = FfiEngine::new(42);
    ffi.command(json!({
        "version": 1,
        "commands": [{"type": "set_native_gaussian_spare", "spare": 1.25}],
    }));
    for sun_change in [0_u8, 12, 255] {
        let mut expected = payload(ffi.engine());
        expected["vegetation_runtime"]["sun_change"] = json!(sun_change);
        let response = ffi.command(json!({
            "version": 1,
            "commands": [
                {"type": "set_native_sun_change", "sun_change": sun_change},
                {"type": "read_native_sun_change"},
                {"type": "read_native_gaussian_spare"}
            ],
        }));
        assert_eq!(response["results"], json!([null, sun_change, 1.25]));
        assert_eq!(payload(ffi.engine()), expected);

        let before_read = SaveFile::encode(ffi.engine()).unwrap();
        let response = ffi.command(json!({
            "version": 1,
            "commands": [
                {"type": "read_native_sun_change"},
                {"type": "read_native_gaussian_spare"},
                {"type": "read_native_sun_change"}
            ],
        }));
        assert_eq!(response["results"], json!([sun_change, 1.25, sun_change]));
        assert_eq!(SaveFile::encode(ffi.engine()).unwrap(), before_read);
    }
    // Deserialization must reject values outside original Vegs.SunChange Byte.
    ffi.reject_command(r#"{"version":1,"commands":[{"type":"set_native_gaussian_spare","spare":9},{"type":"set_native_sun_change","sun_change":256}]}"#);
    ffi.reject_command(
        r#"{"version":1,"commands":[{"type":"set_native_sun_change","sun_change":-1}]}"#,
    );
}

#[test]
fn native_cycle_lifecycle_preserves_monotonic_ffi_ticks_and_saved_start_boundaries() {
    let mut ffi = FfiEngine::new(42);
    ffi.import_random_bot();
    assert_eq!(ffi.engine().snapshot().tick, 0);
    assert_eq!(ffi.engine().snapshot().native_cycle, Some(-1));
    let pending = SaveFile::encode(ffi.engine()).unwrap();
    let restored = SaveFile::decode(&pending).unwrap();
    assert_eq!(restored.snapshot().tick, 0);
    assert_eq!(restored.snapshot().native_cycle, Some(-1));

    // The first command changes real world state, but the original cycle is 0.
    let response = ffi.command(json!({"version":1,"commands":[{"type":"tick","count":1}]}));
    assert_eq!(response["tick"], 1);
    assert_eq!(ffi.engine().snapshot().tick, 1);
    assert_eq!(ffi.engine().snapshot().native_cycle, Some(0));
    let native_zero = ffi.engine().export_native_sim().unwrap();
    let file = NativeRecordFile::decode(&native_zero).unwrap();
    assert_eq!(
        NativeSimulationTail::decode(&file.tail)
            .unwrap()
            .integer("total_cycles")
            .unwrap(),
        0
    );
    let first_update = SaveFile::encode(ffi.engine()).unwrap();

    ffi.load(&pending);
    ffi.command(json!({"version":1,"commands":[{"type":"tick","count":1}]}));
    assert_eq!(SaveFile::encode(ffi.engine()).unwrap(), first_update);

    // Loading an actual cycle-zero world must advance to 1, not repeat 0.
    ffi.load(&native_zero);
    assert_eq!(ffi.engine().snapshot().native_cycle, Some(0));
    assert_eq!(ffi.engine().snapshot().tick, 0);
    ffi.command(json!({"version":1,"commands":[{"type":"tick","count":1}]}));
    assert_eq!(ffi.engine().snapshot().native_cycle, Some(1));
    assert_eq!(ffi.engine().snapshot().tick, 1);
}

#[test]
fn failed_native_cycle_batches_restore_both_counters_and_pending_start_state() {
    let mut ffi = FfiEngine::new(42);
    ffi.import_random_bot();
    ffi.reject_command(
        r#"{"version":1,"commands":[{"type":"tick","count":1},{"type":"tick","count":1000001}]}"#,
    );
    assert_eq!(ffi.engine().snapshot().native_cycle, Some(-1));
    assert_eq!(ffi.engine().snapshot().tick, 0);

    let mut saved = payload(ffi.engine());
    saved["native_cycle"] = json!(i32::MAX);
    ffi.load(&save_with_payload(ffi.engine(), &saved));
    ffi.reject_command(r#"{"version":1,"commands":[{"type":"tick","count":1}]}"#);
    assert_eq!(ffi.engine().snapshot().native_cycle, Some(i32::MAX));
    assert_eq!(ffi.engine().snapshot().tick, 0);
}

#[test]
fn extinct_species_template_exports_are_read_only_and_survive_complete_save_reload() {
    let mut ffi = FfiEngine::new(42);
    let source = "start 7 900 store stop";
    let imported = ffi.command(json!({"version":1,"commands":[{
        "type":"import_species", "name":"Test.txt", "dna":source,
        "vegetable":false, "color":0, "minimum_population":0, "reseed":false,
        "initial_energy":20000, "positions":[[500,500]]
    }]}));
    let species = imported["results"][0]["species"].as_u64().unwrap();
    ffi.command(json!({"version":1,"commands":[
        {"type":"configure_local_mode","config":{"workflow":"population_seeding","entrant_order":[species]}},
        {"type":"start_local_mode"}
    ]}));
    let id = ffi.engine().snapshot().organisms[0].id;
    let all_members: Vec<_> = ffi
        .engine()
        .snapshot()
        .organisms
        .iter()
        .map(|robot| json!({"slot":robot.id.slot(),"generation":robot.id.generation()}))
        .collect();
    ffi.command(json!({"version":1,"commands":[
        {"type":"replace_dna","slot":id.slot(),"generation":id.generation(),"dna":"start 999 900 store stop"},
        {"type":"remove_many","organisms":all_members},
        {"type":"tick","count":1}
    ]}));
    assert_eq!(ffi.engine().population(), 0);
    let saved = SaveFile::encode(ffi.engine()).unwrap();

    for reload in [false, true] {
        if reload {
            ffi.load(&saved);
        }
        let before = SaveFile::encode(ffi.engine()).unwrap();
        let response = ffi.command(json!({"version":1,"commands":[
            {"type":"read_native_gaussian_spare"},
            {"type":"export_species_template_dna","species":species},
            {"type":"read_native_sun_change"}
        ]}));
        let dna = response["results"][1]["dna"].as_str().unwrap();
        assert_eq!(
            LegacyDna::parse(dna).unwrap().blocks(),
            LegacyDna::parse(source).unwrap().blocks()
        );
        assert_eq!(SaveFile::encode(ffi.engine()).unwrap(), before);
        // Both the pure query path and transactional mixed path fail atomically.
        ffi.reject_command(
            &json!({"version":1,"commands":[
                {"type":"export_species_template_dna","species":species},
                {"type":"export_species_template_dna","species":u32::MAX}
            ]})
            .to_string(),
        );
        ffi.reject_command(
            &json!({"version":1,"commands":[
                {"type":"tick","count":1},
                {"type":"export_species_template_dna","species":u32::MAX}
            ]})
            .to_string(),
        );
        assert_eq!(SaveFile::encode(ffi.engine()).unwrap(), before);
    }
}
