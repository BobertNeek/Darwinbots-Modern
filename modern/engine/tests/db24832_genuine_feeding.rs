//! Genuine original fresh-load feeding and tie captures at their actual saved cycles.
//! This seeded process protocol does not recover arbitrary Rnd/Gauss from SIM.
use darwinbots_engine::native_binary::{NativeRecordFile, NativeSimulationTail, NativeValue};
use darwinbots_engine::{Engine, SaveFile};

const START: &[u8] = include_bytes!("fixtures/db24832-feeding-start-48032.sim");
const ORIGINAL_C1: &[u8] = include_bytes!("fixtures/db24832-genuine-feeding-cycle1-20261017.sim");
const ORIGINAL_C10: &[u8] = include_bytes!("fixtures/db24832-genuine-feeding-cycle10-20261017.sim");
const TIES_START: &[u8] = include_bytes!("fixtures/db24832-ties-start-48032.sim");
const ORIGINAL_TIES_C1: &[u8] = include_bytes!("fixtures/db24832-genuine-ties-cycle1-20261017.sim");

fn assert_native_value(actual: &NativeValue, expected: &NativeValue, name: &str) {
    match (actual, expected) {
        (NativeValue::Single(actual), NativeValue::Single(expected)) => {
            assert_eq!(f32::from_bits(*actual), f32::from_bits(*expected), "{name}");
        }
        (NativeValue::Double(actual), NativeValue::Double(expected)) => {
            assert_eq!(f64::from_bits(*actual), f64::from_bits(*expected), "{name}");
        }
        _ => assert_eq!(actual, expected, "{name}"),
    }
}

fn assert_original_pair(engine: &Engine, reference: &[u8], cycle: i32) {
    let actual = NativeRecordFile::decode(&engine.export_native_sim().unwrap()).unwrap();
    let expected = NativeRecordFile::decode(reference).unwrap();
    let actual_tail = NativeSimulationTail::decode(&actual.tail).unwrap();
    let expected_tail = NativeSimulationTail::decode(&expected.tail).unwrap();
    assert_eq!(expected_tail.integer("total_cycles").unwrap(), cycle);
    assert_eq!(actual_tail.integer("total_cycles").unwrap(), cycle);
    assert_eq!(expected_tail.integer("seed").unwrap(), 48032);
    assert_eq!(actual_tail.integer("seed").unwrap(), 48032);
    // The original's CInt(Boolean) load guard reset the input's canonical -1
    // global disable flag. Both actual original captures save it as False.
    assert_eq!(expected_tail.integer("disable_mutations").unwrap(), 0);
    assert_eq!(actual_tail.integer("disable_mutations").unwrap(), 0);
    assert!(engine.snapshot().mutations_enabled);
    assert_eq!(expected.robots.len(), 2);
    assert_eq!(actual.robots.len(), 2);
    for expected_robot in &expected.robots {
        let id = expected_robot.long("absolute_id").unwrap();
        let actual_robot = actual
            .robots
            .iter()
            .find(|robot| robot.long("absolute_id").unwrap() == id)
            .unwrap();
        for name in [
            "position_x",
            "position_y",
            "velocity_x",
            "velocity_y",
            "actual_velocity_x",
            "actual_velocity_y",
            "aim",
            "angular_momentum",
            "angular_torque",
            "energy",
            "body",
            "waste",
            "permanent_waste",
            "shell",
            "slime",
            "poison",
            "venom",
            "chloroplasts",
        ] {
            assert_native_value(
                actual_robot.value(name).unwrap(),
                expected_robot.value(name).unwrap(),
                &format!("original pair C{cycle} robot{id}.{name}"),
            );
        }
        assert_eq!(
            actual_robot.long("age").unwrap(),
            expected_robot.long("age").unwrap()
        );
        for name in [
            "memory",
            "mutation_rates",
            "mutation_shapes",
            "owner",
            "dna",
        ] {
            assert_eq!(
                actual_robot.bytes(name).unwrap(),
                expected_robot.bytes(name).unwrap(),
                "original pair C{cycle} robot{id}.{name}"
            );
        }
    }
    let active = |tail: &NativeSimulationTail| {
        (0..tail.integer("shot_count").unwrap())
            .filter(|index| tail.integer(&format!("shot[{index}].exists")).unwrap() != 0)
            .collect::<Vec<_>>()
    };
    assert_eq!(active(&actual_tail), active(&expected_tail));
    for index in active(&expected_tail) {
        let prefix = format!("shot[{index}].");
        for field in expected_tail
            .fields()
            .iter()
            .filter(|field| field.name.starts_with(&prefix) && !field.name.ends_with(".end"))
        {
            assert_native_value(
                actual_tail.value(&field.name).unwrap(),
                &field.value,
                &format!("original pair C{cycle} {}", field.name),
            );
        }
    }
}

#[test]
fn genuine_feeding_c1_proves_native_mutation_draws_mass_order_and_zero_shot_gene() {
    let input = NativeRecordFile::decode(START).unwrap();
    let tail = NativeSimulationTail::decode(&input.tail).unwrap();
    assert_eq!(tail.integer("disable_mutations").unwrap(), -1);
    assert_eq!(tail.integer("seed").unwrap(), 48032);
    for checkpoint_before_first_tick in [false, true] {
        let mut engine = Engine::from_native_sim(START).unwrap();
        if checkpoint_before_first_tick {
            engine = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        }
        engine.tick().unwrap();
        assert_original_pair(&engine, ORIGINAL_C1, 1);
        assert_eq!(engine.snapshot().shots.len(), 2);
        assert!(
            engine
                .snapshot()
                .shots
                .iter()
                .all(|shot| shot.gene_number == 0)
        );
        // Source mutation consumes a Point clock and Delta trial before each
        // newshot's two draws: fresh seed 48032 scatters -15 then -2 units.
        // These velocities come from the genuine executable save, not a fake oracle.
        assert_eq!(
            engine.snapshot().shots[0].velocity,
            [f32::from_bits(0x421f_8cda), f32::from_bits(0x403f_d1ef)]
        );
        assert_eq!(
            engine.snapshot().shots[1].velocity,
            [f32::from_bits(0x421f_fdf4), f32::from_bits(0x3ecc_cbed)]
        );
    }
}

#[test]
fn genuine_feeding_c10_matches_after_preserving_post_c1_runtime_caches() {
    let mut continuous = Engine::from_native_sim(START).unwrap();
    continuous.tick_many(10).unwrap();
    assert_original_pair(&continuous, ORIGINAL_C10, 10);
    let mut resumed = Engine::from_native_sim(START).unwrap();
    resumed.tick().unwrap();
    assert_original_pair(&resumed, ORIGINAL_C1, 1);
    resumed = SaveFile::decode(&SaveFile::encode(&resumed).unwrap()).unwrap();
    resumed.tick_many(9).unwrap();
    assert_original_pair(&resumed, ORIGINAL_C10, 10);
    assert_eq!(
        SaveFile::encode(&resumed).unwrap(),
        SaveFile::encode(&continuous).unwrap()
    );
}

#[test]
fn genuine_ties_c1_preserves_the_original_pair_and_both_endpoint_records() {
    let mut engine = Engine::from_native_sim(TIES_START).unwrap();
    engine.tick().unwrap();
    assert_original_pair(&engine, ORIGINAL_TIES_C1, 1);
    assert!(!engine.snapshot().ties.is_empty());
    let actual = NativeRecordFile::decode(&engine.export_native_sim().unwrap()).unwrap();
    let expected = NativeRecordFile::decode(ORIGINAL_TIES_C1).unwrap();
    for expected_robot in &expected.robots {
        let id = expected_robot.long("absolute_id").unwrap();
        let actual_robot = actual
            .robots
            .iter()
            .find(|robot| robot.long("absolute_id").unwrap() == id)
            .unwrap();
        for name in ["ties_core", "ties_extension"] {
            assert_eq!(
                actual_robot.bytes(name).unwrap(),
                expected_robot.bytes(name).unwrap(),
                "original ties C1 robot{id}.{name}"
            );
        }
    }
}
