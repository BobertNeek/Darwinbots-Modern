//! Actual original 2.48.32 fresh-load captures, not source-simulated oracle output.
//! The fixtures prove listed live fields for these seeded deterministic load cases;
//! native SIM does not contain arbitrary historical uniform/Gaussian state.
use darwinbots_engine::native_binary::{NativeRecordFile, NativeSimulationTail};
use darwinbots_engine::{Engine, EngineConfig, LegacyDna, SaveFile};

const START: &[u8] = include_bytes!("fixtures/db24832-movement-start-48032.sim");
const ORIGINAL_C1: &[u8] = include_bytes!("fixtures/db24832-genuine-movement-cycle1-20261016.sim");
const ORIGINAL_C10: &[u8] =
    include_bytes!("fixtures/db24832-genuine-movement-cycle10-20261016.sim");
const ORIGINAL_C102: &[u8] =
    include_bytes!("fixtures/db24832-genuine-movement-cycle102-20261016.sim");
const FIXED_START: &[u8] = include_bytes!("fixtures/db24832-fixed-start-48032.sim");
const ORIGINAL_FIXED_C1: &[u8] =
    include_bytes!("fixtures/db24832-genuine-fixed-cycle1-20261016.sim");
const BODY_START: &[u8] = include_bytes!("fixtures/db24832-body-start-48032.sim");
const ORIGINAL_BODY_C1: &[u8] = include_bytes!("fixtures/db24832-genuine-body-cycle1-20261016.sim");
const CHLOROPLAST_START: &[u8] = include_bytes!("fixtures/db24832-chloroplast-start-48032.sim");
const ORIGINAL_CHLOROPLAST_C1: &[u8] =
    include_bytes!("fixtures/db24832-genuine-chloroplast-cycle1-20261016.sim");
const ORIGINAL_CHLOROPLAST_C10: &[u8] =
    include_bytes!("fixtures/db24832-genuine-chloroplast-cycle10-20261016.sim");

fn assert_genuine_movement_observation(engine: &Engine, reference: &[u8], cycle: i32) {
    let expected = NativeRecordFile::decode(reference).unwrap();
    let actual = NativeRecordFile::decode(&engine.export_native_sim().unwrap()).unwrap();
    let expected_tail = NativeSimulationTail::decode(&expected.tail).unwrap();
    let actual_tail = NativeSimulationTail::decode(&actual.tail).unwrap();
    assert_eq!(expected_tail.integer("total_cycles").unwrap(), cycle);
    assert_eq!(actual_tail.integer("total_cycles").unwrap(), cycle);
    assert_eq!(expected_tail.integer("seed").unwrap(), 48032);
    assert_eq!(actual_tail.integer("seed").unwrap(), 48032);
    assert_eq!(expected.robots.len(), 1);
    assert_eq!(actual.robots.len(), 1);
    let expected = &expected.robots[0];
    let actual = &actual.robots[0];
    for name in [
        "position_x",
        "position_y",
        "velocity_x",
        "velocity_y",
        "actual_velocity_x",
        "actual_velocity_y",
        "aim",
        "energy",
        "body",
        "angular_momentum",
        "angular_torque",
    ] {
        assert_eq!(
            actual.single(name).unwrap(),
            expected.single(name).unwrap(),
            "original C{cycle} {name}"
        );
    }
    for name in ["absolute_id", "age", "birth_cycle"] {
        assert_eq!(
            actual.long(name).unwrap(),
            expected.long(name).unwrap(),
            "original C{cycle} {name}"
        );
    }
    for name in ["sunbelt", "new_move", "mutations_enabled"] {
        assert_eq!(
            actual.word(name).unwrap(),
            expected.word(name).unwrap(),
            "original C{cycle} {name}"
        );
    }
    // All memory cells include mass, pain/pleasure, body deltas, velocity,
    // coordinates, population and the source's previous FeedVeg light cache.
    // Shapes remain intact when pre-sunbelt rates are normalized on import.
    for name in [
        "memory",
        "mutation_rates",
        "mutation_shapes",
        "owner",
        "dna",
    ] {
        assert_eq!(
            actual.bytes(name).unwrap(),
            expected.bytes(name).unwrap(),
            "original C{cycle} {name}"
        );
    }
}

#[test]
fn genuine_original_first_load_tick_matches_with_or_without_a_modern_checkpoint() {
    let input = NativeRecordFile::decode(START).unwrap();
    let input_tail = NativeSimulationTail::decode(&input.tail).unwrap();
    assert_eq!(input_tail.integer("total_cycles").unwrap(), 0);
    assert_eq!(input_tail.integer("seed").unwrap(), 48032);
    for checkpoint_before_first_tick in [false, true] {
        let mut engine = Engine::from_native_sim(START).unwrap();
        if checkpoint_before_first_tick {
            engine = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        }
        engine.tick().unwrap();
        assert_genuine_movement_observation(&engine, ORIGINAL_C1, 1);
        // Age and timer are genuine one-update evidence, not just a cycle label.
        let id = engine.snapshot().organisms[0].id;
        assert_eq!(engine.organism(id).unwrap().age, 1);
        assert_eq!(engine.memory_at(id, 12).unwrap(), 1);
        assert_eq!(engine.organism(id).unwrap().position, [8000.0, 6000.0]);
    }
}

#[test]
fn genuine_original_later_ticks_match_across_the_first_aim_cache_checkpoint() {
    // These are actual saved counters: C100 was not captured.
    for (cycle, reference) in [(10, ORIGINAL_C10), (102, ORIGINAL_C102)] {
        let mut continuous = Engine::from_native_sim(START).unwrap();
        continuous.tick_many(cycle as u32).unwrap();
        assert_genuine_movement_observation(&continuous, reference, cycle);

        let mut resumed = Engine::from_native_sim(START).unwrap();
        resumed.tick().unwrap();
        assert_genuine_movement_observation(&resumed, ORIGINAL_C1, 1);
        resumed = SaveFile::decode(&SaveFile::encode(&resumed).unwrap()).unwrap();
        resumed.tick_many((cycle - 1) as u32).unwrap();
        assert_genuine_movement_observation(&resumed, reference, cycle);
        assert_eq!(
            SaveFile::encode(&resumed).unwrap(),
            SaveFile::encode(&continuous).unwrap()
        );
    }
}

#[test]
fn bare_typed_spawn_keeps_its_existing_first_tick_movement_contract() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let id = engine
        .spawn_at(
            LegacyDna::parse("start 10 .up store stop").unwrap(),
            [500.0; 2],
        )
        .unwrap();
    engine.tick().unwrap();
    assert!(engine.organism(id).unwrap().position[0] > 500.0);
}

#[test]
fn genuine_original_dna_fix_first_tick_matches_with_or_without_a_modern_checkpoint() {
    let input = NativeRecordFile::decode(FIXED_START).unwrap();
    let input_tail = NativeSimulationTail::decode(&input.tail).unwrap();
    assert_eq!(input_tail.integer("total_cycles").unwrap(), 0);
    assert_eq!(input_tail.integer("seed").unwrap(), 48032);
    assert_eq!(input.robots[0].word("fixed").unwrap(), 0);
    for checkpoint_before_first_tick in [false, true] {
        let mut engine = Engine::from_native_sim(FIXED_START).unwrap();
        if checkpoint_before_first_tick {
            engine = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        }
        engine.tick().unwrap();
        assert_genuine_movement_observation(&engine, ORIGINAL_FIXED_C1, 1);
        let actual = NativeRecordFile::decode(&engine.export_native_sim().unwrap()).unwrap();
        // This fixture starts unfixed; its DNA sets the original fixed latch.
        assert_eq!(actual.robots[0].word("fixed").unwrap(), -1);
        let id = engine.snapshot().organisms[0].id;
        assert_eq!(engine.organism(id).unwrap().age, 1);
        assert_eq!(engine.memory_at(id, 12).unwrap(), 1);
        assert_eq!(engine.organism(id).unwrap().position, [8000.0, 6000.0]);
    }
}

#[test]
fn genuine_original_body_store_first_tick_matches_with_or_without_a_modern_checkpoint() {
    let input = NativeRecordFile::decode(BODY_START).unwrap();
    let input_tail = NativeSimulationTail::decode(&input.tail).unwrap();
    assert_eq!(input_tail.integer("total_cycles").unwrap(), 0);
    assert_eq!(input_tail.integer("seed").unwrap(), 48032);
    for checkpoint_before_first_tick in [false, true] {
        let mut engine = Engine::from_native_sim(BODY_START).unwrap();
        if checkpoint_before_first_tick {
            engine = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        }
        engine.tick().unwrap();
        assert_genuine_movement_observation(&engine, ORIGINAL_BODY_C1, 1);
        let actual = NativeRecordFile::decode(&engine.export_native_sim().unwrap()).unwrap();
        assert_eq!(actual.robots[0].single("energy").unwrap(), 19_995.0);
        assert_eq!(actual.robots[0].single("body").unwrap(), 1_000.5);
        let id = engine.snapshot().organisms[0].id;
        assert_eq!(engine.organism(id).unwrap().age, 1);
        assert_eq!(engine.memory_at(id, 12).unwrap(), 1);
        assert_eq!(engine.organism(id).unwrap().position, [8000.0, 6000.0]);
    }
}

fn assert_genuine_chloroplast_observation(
    engine: &Engine,
    reference: &[u8],
    cycle: i32,
    energy_bits: u32,
    body_bits: u32,
    snapshot_energy: i32,
    snapshot_body: i32,
) {
    assert_genuine_movement_observation(engine, reference, cycle);
    let id = engine.snapshot().organisms[0].id;
    let snapshot = engine.organism(id).unwrap();
    assert_eq!(snapshot.position, [8000.0, 6000.0]);
    assert_eq!(snapshot.velocity, [0.0; 2]);
    assert_eq!(snapshot.age, cycle as u64);
    assert!(!snapshot.flags.can_execute_dna);
    assert_eq!(snapshot.energy, snapshot_energy);
    assert_eq!(snapshot.body, snapshot_body);
    for (address, expected) in [
        (10, 15_841),
        (12, cycle),
        (218, 1),
        (310, 20_000),
        (311, 1_000),
        (401, 0),
        (402, 0),
        (923, 0),
    ] {
        assert_eq!(
            engine.memory_at(id, address).unwrap(),
            expected,
            "chloroplast C{cycle} memory{address}"
        );
    }
    let exported = NativeRecordFile::decode(&engine.export_native_sim().unwrap()).unwrap();
    let robot = &exported.robots[0];
    assert_eq!(robot.word("disable_dna").unwrap(), -1);
    assert_eq!(robot.single("chloroplasts").unwrap(), 16_000.0);
    assert_eq!(robot.single("energy").unwrap().to_bits(), energy_bits);
    assert_eq!(robot.single("body").unwrap().to_bits(), body_bits);
}

#[test]
fn genuine_original_chloroplast_first_tick_preserves_disabled_dna_memory_across_checkpoints() {
    let input = NativeRecordFile::decode(CHLOROPLAST_START).unwrap();
    let input_tail = NativeSimulationTail::decode(&input.tail).unwrap();
    assert_eq!(input_tail.integer("total_cycles").unwrap(), 0);
    assert_eq!(input_tail.integer("seed").unwrap(), 48032);
    assert_eq!(input.robots[0].word("disable_dna").unwrap(), -1);
    assert_eq!(input.robots[0].single("chloroplasts").unwrap(), 16_000.0);
    for checkpoint_before_first_tick in [false, true] {
        let mut engine = Engine::from_native_sim(CHLOROPLAST_START).unwrap();
        if checkpoint_before_first_tick {
            engine = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        }
        engine.tick().unwrap();
        assert_genuine_chloroplast_observation(
            &engine,
            ORIGINAL_CHLOROPLAST_C1,
            1,
            0x469c_497f,
            0x447a_1e63,
            20_005,
            1_000,
        );
        let restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        assert_genuine_chloroplast_observation(
            &restored,
            ORIGINAL_CHLOROPLAST_C1,
            1,
            0x469c_497f,
            0x447a_1e63,
            20_005,
            1_000,
        );
    }
}

#[test]
fn genuine_original_chloroplast_later_ticks_match_with_the_saved_light_cache() {
    let mut continuous = Engine::from_native_sim(CHLOROPLAST_START).unwrap();
    continuous.tick_many(10).unwrap();
    assert_genuine_chloroplast_observation(
        &continuous,
        ORIGINAL_CHLOROPLAST_C10,
        10,
        0x469c_9ec9,
        0x447b_2f56,
        20_047,
        1_005,
    );
    let mut resumed = Engine::from_native_sim(CHLOROPLAST_START).unwrap();
    resumed.tick().unwrap();
    assert_genuine_chloroplast_observation(
        &resumed,
        ORIGINAL_CHLOROPLAST_C1,
        1,
        0x469c_497f,
        0x447a_1e63,
        20_005,
        1_000,
    );
    resumed = SaveFile::decode(&SaveFile::encode(&resumed).unwrap()).unwrap();
    resumed.tick_many(9).unwrap();
    assert_genuine_chloroplast_observation(
        &resumed,
        ORIGINAL_CHLOROPLAST_C10,
        10,
        0x469c_9ec9,
        0x447b_2f56,
        20_047,
        1_005,
    );
    assert_eq!(
        SaveFile::encode(&resumed).unwrap(),
        SaveFile::encode(&continuous).unwrap()
    );
}
