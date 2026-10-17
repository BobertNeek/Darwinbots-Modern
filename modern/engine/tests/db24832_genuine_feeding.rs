//! Genuine original fresh-load feeding and tie captures at their actual saved cycles.
//! This seeded process protocol does not recover arbitrary Rnd/Gauss from SIM.
use darwinbots_engine::native_binary::{NativeRecordFile, NativeSimulationTail, NativeValue};
use darwinbots_engine::{Engine, SaveFile};

const START: &[u8] = include_bytes!("fixtures/db24832-feeding-start-48032.sim");
const ORIGINAL_C1: &[u8] = include_bytes!("fixtures/db24832-genuine-feeding-cycle1-20261017.sim");
const ORIGINAL_C10: &[u8] = include_bytes!("fixtures/db24832-genuine-feeding-cycle10-20261017.sim");
const TIES_START: &[u8] = include_bytes!("fixtures/db24832-ties-start-48032.sim");
const ORIGINAL_TIES_C1: &[u8] = include_bytes!("fixtures/db24832-genuine-ties-cycle1-20261017.sim");
const CONTACTS_START: &[u8] = include_bytes!("fixtures/db24832-contacts-start-48032.sim");
const ORIGINAL_CONTACTS_C1: &[u8] =
    include_bytes!("fixtures/db24832-genuine-contacts-cycle1-20261017.sim");

const REPRODUCTION_START: &[u8] = include_bytes!("fixtures/db24832-reproduction-start-48032.sim");
const ORIGINAL_REPRODUCTION_C1: &[u8] =
    include_bytes!("fixtures/db24832-genuine-reproduction-cycle1-20261017.sim");
const ORIGINAL_REPRODUCTION_C10: &[u8] =
    include_bytes!("fixtures/db24832-genuine-reproduction-cycle10-20261017.sim");

const ORIGINAL_CONTACTS_C11: &[u8] =
    include_bytes!("fixtures/db24832-genuine-contacts-cycle11-20261017.sim");
const ORIGINAL_CONTACTS_C100: &[u8] =
    include_bytes!("fixtures/db24832-genuine-contacts-cycle100-20261017.sim");

const ORIGINAL_FEEDING_C100: &[u8] =
    include_bytes!("fixtures/db24832-genuine-feeding-cycle100-20261017.sim");
const ORIGINAL_TIES_C10: &[u8] =
    include_bytes!("fixtures/db24832-genuine-ties-cycle10-20261017.sim");
const ORIGINAL_TIES_C100: &[u8] =
    include_bytes!("fixtures/db24832-genuine-ties-cycle100-20261017.sim");
const ORIGINAL_CONTACTS_C10: &[u8] =
    include_bytes!("fixtures/db24832-genuine-contacts-cycle10-20261017.sim");
const ORIGINAL_REPRODUCTION_C100: &[u8] =
    include_bytes!("fixtures/db24832-genuine-reproduction-cycle100-20261017.sim");

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
    assert!(!expected.robots.is_empty());
    assert_eq!(actual.robots.len(), expected.robots.len());
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
        let actual_memory = actual_robot.bytes("memory").unwrap();
        let expected_memory = expected_robot.bytes("memory").unwrap();
        assert_eq!(actual_memory.len(), 2002);
        assert_eq!(expected_memory.len(), 2002);
        for address in 0..=1000 {
            let offset = address * 2;
            let actual_word =
                i16::from_le_bytes([actual_memory[offset], actual_memory[offset + 1]]);
            let expected_word =
                i16::from_le_bytes([expected_memory[offset], expected_memory[offset + 1]]);
            assert_eq!(
                actual_word, expected_word,
                "original pair C{cycle} robot{id}.memory{address}"
            );
        }
        for name in ["mutation_rates", "mutation_shapes", "owner", "dna"] {
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
fn genuine_feeding_c10_and_c100_match_after_preserving_runtime_caches() {
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
    let mut resumed_at_c10 = SaveFile::decode(&SaveFile::encode(&continuous).unwrap()).unwrap();
    for observed in [&mut continuous, &mut resumed, &mut resumed_at_c10] {
        observed.tick_many(90).unwrap();
        assert_original_pair(observed, ORIGINAL_FEEDING_C100, 100);
        let restored = SaveFile::decode(&SaveFile::encode(observed).unwrap()).unwrap();
        assert_original_pair(&restored, ORIGINAL_FEEDING_C100, 100);
    }
    assert_eq!(
        SaveFile::encode(&resumed).unwrap(),
        SaveFile::encode(&continuous).unwrap()
    );
    assert_eq!(
        SaveFile::encode(&resumed_at_c10).unwrap(),
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

#[test]
fn genuine_contacts_c1_matches_three_actor_live_fields_and_public_memory_across_checkpoints() {
    let input = NativeRecordFile::decode(CONTACTS_START).unwrap();
    let input_tail = NativeSimulationTail::decode(&input.tail).unwrap();
    assert_eq!(input_tail.integer("total_cycles").unwrap(), 0);
    assert_eq!(input_tail.integer("seed").unwrap(), 48032);
    let expected = NativeRecordFile::decode(ORIGINAL_CONTACTS_C1).unwrap();
    assert_eq!(expected.robots.len(), 3);
    assert!(
        expected
            .robots
            .iter()
            .all(|robot| robot.long("age").unwrap() == 1)
    );
    for checkpoint_before_first_tick in [false, true] {
        let mut engine = Engine::from_native_sim(CONTACTS_START).unwrap();
        if checkpoint_before_first_tick {
            engine = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        }
        engine.tick().unwrap();
        let restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        for observed in [&engine, &restored] {
            assert_original_pair(observed, ORIGINAL_CONTACTS_C1, 1);
            assert_eq!(observed.snapshot().organisms.len(), expected.robots.len());
            let exported =
                NativeRecordFile::decode(&observed.export_native_sim().unwrap()).unwrap();
            for reference in &expected.robots {
                let absolute_id = reference.long("absolute_id").unwrap();
                let actual = exported
                    .robots
                    .iter()
                    .find(|robot| robot.long("absolute_id").unwrap() == absolute_id)
                    .unwrap();
                // Extend the common physical/material/memory/shot comparison
                // with the original's saved live flags, counters and genetics.
                for name in [
                    "vegetable",
                    "wall",
                    "fixed",
                    "offspring",
                    "mutations",
                    "last_mutation",
                    "parent",
                    "birth_cycle",
                    "gene_count",
                    "generation",
                    "stored_dna_length",
                    "skin",
                    "color",
                    "buoyancy",
                    "corpse",
                    "exists",
                    "dead",
                    "species_name",
                    "mutations_enabled",
                    "copy_error_policy",
                    "point_policy",
                    "view",
                    "new_move",
                    "old_slot",
                    "cant_see",
                    "disable_dna",
                    "disable_movement",
                    "cant_reproduce",
                    "virus_immune",
                    "subspecies",
                    "sperm_length",
                    "sperm_dna",
                    "fertilized",
                    "simulation_id",
                    "absolute_id",
                    "multibot",
                    "epigenetic_memory",
                    "tag",
                    "sunbelt",
                    "no_chloroplasts",
                    "multibot_time",
                    "chloroplast_share_delay",
                    "disqualification",
                    "old_mutations",
                ] {
                    assert_native_value(
                        actual.value(name).unwrap(),
                        reference.value(name).unwrap(),
                        &format!("original contacts C1 robot{absolute_id}.{name}"),
                    );
                }
                let snapshot = observed
                    .snapshot()
                    .organisms
                    .iter()
                    .find(|robot| robot.unique_id == absolute_id as u64)
                    .unwrap();
                let public = observed.organism(snapshot.id).unwrap();
                assert_eq!(
                    public.position,
                    [
                        reference.single("position_x").unwrap(),
                        reference.single("position_y").unwrap()
                    ]
                );
                assert_eq!(
                    public.velocity,
                    [
                        reference.single("velocity_x").unwrap(),
                        reference.single("velocity_y").unwrap()
                    ]
                );
                assert_eq!(public.age, 1);
                for (actual_flag, name, inverted) in [
                    (public.flags.exists, "exists", false),
                    (public.flags.corpse, "corpse", false),
                    (public.flags.dead, "dead", false),
                    (public.flags.vegetable, "vegetable", false),
                    (public.flags.wall, "wall", false),
                    (public.flags.fixed, "fixed", false),
                    (public.flags.can_execute_dna, "disable_dna", true),
                    (public.flags.can_move, "disable_movement", true),
                    (public.flags.can_reproduce, "cant_reproduce", true),
                    (public.flags.can_see, "cant_see", true),
                    (public.flags.virus_immune, "virus_immune", false),
                    (public.flags.no_chloroplasts, "no_chloroplasts", false),
                ] {
                    let original = reference.word(name).unwrap() != 0;
                    assert_eq!(
                        actual_flag,
                        original ^ inverted,
                        "public robot{absolute_id}.{name}"
                    );
                }
                let memory = reference.bytes("memory").unwrap();
                for address in 1..=1_000 {
                    let offset = address as usize * 2;
                    let original = i16::from_le_bytes([memory[offset], memory[offset + 1]]);
                    assert_eq!(
                        observed.memory_at(public.id, address).unwrap(),
                        i32::from(original),
                        "public contacts C1 robot{absolute_id}.memory{address}"
                    );
                }
            }
        }
    }
}

fn assert_original_live_capture(engine: &Engine, reference: &[u8], cycle: i32) {
    assert_original_pair(engine, reference, cycle);
    let expected = NativeRecordFile::decode(reference).unwrap();
    let actual = NativeRecordFile::decode(&engine.export_native_sim().unwrap()).unwrap();
    assert_eq!(engine.snapshot().organisms.len(), expected.robots.len());
    for reference in &expected.robots {
        let absolute_id = reference.long("absolute_id").unwrap();
        let actual = actual
            .robots
            .iter()
            .find(|robot| robot.long("absolute_id").unwrap() == absolute_id)
            .unwrap();
        for name in [
            "vegetable",
            "wall",
            "fixed",
            "offspring",
            "mutations",
            "last_mutation",
            "parent",
            "birth_cycle",
            "gene_count",
            "generation",
            "stored_dna_length",
            "skin",
            "color",
            "buoyancy",
            "corpse",
            "exists",
            "dead",
            "species_name",
            "mutations_enabled",
            "copy_error_policy",
            "point_policy",
            "view",
            "new_move",
            "old_slot",
            "cant_see",
            "disable_dna",
            "disable_movement",
            "cant_reproduce",
            "virus_immune",
            "subspecies",
            "sperm_length",
            "sperm_dna",
            "fertilized",
            "simulation_id",
            "absolute_id",
            "multibot",
            "epigenetic_memory",
            "tag",
            "sunbelt",
            "no_chloroplasts",
            "multibot_time",
            "chloroplast_share_delay",
            "disqualification",
            "old_mutations",
        ] {
            assert_native_value(
                actual.value(name).unwrap(),
                reference.value(name).unwrap(),
                &format!("original live C{cycle} robot{absolute_id}.{name}"),
            );
        }
        let snapshot = engine
            .snapshot()
            .organisms
            .iter()
            .find(|robot| robot.unique_id == absolute_id as u64)
            .unwrap();
        let public = engine.organism(snapshot.id).unwrap();
        assert_eq!(
            public.position,
            [
                reference.single("position_x").unwrap(),
                reference.single("position_y").unwrap()
            ]
        );
        assert_eq!(
            public.velocity,
            [
                reference.single("velocity_x").unwrap(),
                reference.single("velocity_y").unwrap()
            ]
        );
        assert_eq!(public.age, reference.long("age").unwrap() as u64);
        assert_eq!(
            public.birth_cycle,
            reference.long("birth_cycle").unwrap() as u64
        );
        assert_eq!(
            public.energy,
            reference.single("energy").unwrap().round_ties_even() as i32
        );
        assert_eq!(
            public.body,
            reference.single("body").unwrap().round_ties_even() as i32
        );
        let memory = reference.bytes("memory").unwrap();
        for address in 1..=1_000 {
            let offset = address as usize * 2;
            let original = i16::from_le_bytes([memory[offset], memory[offset + 1]]);
            assert_eq!(
                engine.memory_at(public.id, address).unwrap(),
                i32::from(original),
                "public live C{cycle} robot{absolute_id}.memory{address}"
            );
        }
    }
}

#[test]
fn genuine_reproduction_c1_retains_the_request_without_a_newborn_across_checkpoints() {
    let input = NativeRecordFile::decode(REPRODUCTION_START).unwrap();
    let tail = NativeSimulationTail::decode(&input.tail).unwrap();
    assert_eq!(tail.integer("total_cycles").unwrap(), 0);
    assert_eq!(tail.integer("seed").unwrap(), 48032);
    let expected = NativeRecordFile::decode(ORIGINAL_REPRODUCTION_C1).unwrap();
    assert_eq!(expected.robots.len(), 1);
    assert_eq!(expected.robots[0].long("age").unwrap(), 1);
    for checkpoint_before_first_tick in [false, true] {
        let mut engine = Engine::from_native_sim(REPRODUCTION_START).unwrap();
        if checkpoint_before_first_tick {
            engine = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        }
        engine.tick().unwrap();
        assert_original_live_capture(&engine, ORIGINAL_REPRODUCTION_C1, 1);
        assert_eq!(engine.population(), 1);
        let parent = engine.snapshot().organisms[0].id;
        assert_eq!(engine.memory_at(parent, 300).unwrap(), 50);
        let restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        assert_original_live_capture(&restored, ORIGINAL_REPRODUCTION_C1, 1);
    }
}

#[test]
fn genuine_reproduction_c10_and_c100_preserve_birth_at_c2_and_saved_continuation() {
    let expected = NativeRecordFile::decode(ORIGINAL_REPRODUCTION_C10).unwrap();
    assert_eq!(expected.robots.len(), 2);
    let parent = expected
        .robots
        .iter()
        .find(|robot| robot.long("absolute_id").unwrap() == 1)
        .unwrap();
    let child = expected
        .robots
        .iter()
        .find(|robot| robot.long("absolute_id").unwrap() == 2)
        .unwrap();
    assert_eq!(parent.long("age").unwrap(), 10);
    assert_eq!(parent.word("offspring").unwrap(), 1);
    assert_eq!(child.long("age").unwrap(), 8);
    assert_eq!(child.long("parent").unwrap(), 1);
    assert_eq!(child.long("birth_cycle").unwrap(), 2);
    assert_eq!(child.word("generation").unwrap(), 1);
    let mut continuous = Engine::from_native_sim(REPRODUCTION_START).unwrap();
    continuous.tick_many(10).unwrap();
    assert_original_live_capture(&continuous, ORIGINAL_REPRODUCTION_C10, 10);
    let mut resumed = Engine::from_native_sim(REPRODUCTION_START).unwrap();
    resumed = SaveFile::decode(&SaveFile::encode(&resumed).unwrap()).unwrap();
    resumed.tick().unwrap();
    assert_original_live_capture(&resumed, ORIGINAL_REPRODUCTION_C1, 1);
    resumed = SaveFile::decode(&SaveFile::encode(&resumed).unwrap()).unwrap();
    resumed.tick_many(9).unwrap();
    assert_original_live_capture(&resumed, ORIGINAL_REPRODUCTION_C10, 10);
    let restored = SaveFile::decode(&SaveFile::encode(&resumed).unwrap()).unwrap();
    assert_original_live_capture(&restored, ORIGINAL_REPRODUCTION_C10, 10);
    assert_eq!(
        SaveFile::encode(&resumed).unwrap(),
        SaveFile::encode(&continuous).unwrap()
    );
    let mut resumed_at_c10 = SaveFile::decode(&SaveFile::encode(&continuous).unwrap()).unwrap();
    for observed in [&mut continuous, &mut resumed, &mut resumed_at_c10] {
        observed.tick_many(90).unwrap();
        assert_original_live_capture(observed, ORIGINAL_REPRODUCTION_C100, 100);
        let restored = SaveFile::decode(&SaveFile::encode(observed).unwrap()).unwrap();
        assert_original_live_capture(&restored, ORIGINAL_REPRODUCTION_C100, 100);
    }
    assert_eq!(
        SaveFile::encode(&resumed).unwrap(),
        SaveFile::encode(&continuous).unwrap()
    );
    assert_eq!(
        SaveFile::encode(&resumed_at_c10).unwrap(),
        SaveFile::encode(&continuous).unwrap()
    );
}

#[test]
fn genuine_contacts_c10_c11_and_c100_match_live_state_and_saved_continuation() {
    for (reference, cycle) in [
        (ORIGINAL_CONTACTS_C10, 10),
        (ORIGINAL_CONTACTS_C11, 11),
        (ORIGINAL_CONTACTS_C100, 100),
    ] {
        let expected = NativeRecordFile::decode(reference).unwrap();
        let tail = NativeSimulationTail::decode(&expected.tail).unwrap();
        assert_eq!(tail.integer("total_cycles").unwrap(), cycle);
        assert_eq!(tail.integer("seed").unwrap(), 48032);
        assert_eq!(expected.robots.len(), 3);
        assert!(
            expected
                .robots
                .iter()
                .all(|robot| robot.long("age").unwrap() == cycle)
        );
    }
    let mut continuous = Engine::from_native_sim(CONTACTS_START).unwrap();
    continuous.tick_many(10).unwrap();
    assert_original_live_capture(&continuous, ORIGINAL_CONTACTS_C10, 10);
    let mut resumed_at_c10 = SaveFile::decode(&SaveFile::encode(&continuous).unwrap()).unwrap();
    resumed_at_c10.tick().unwrap();
    continuous.tick().unwrap();
    assert_original_live_capture(&continuous, ORIGINAL_CONTACTS_C11, 11);
    let mut resumed_at_c11 = SaveFile::decode(&SaveFile::encode(&continuous).unwrap()).unwrap();
    let mut resumed_at_c1 = Engine::from_native_sim(CONTACTS_START).unwrap();
    resumed_at_c1 = SaveFile::decode(&SaveFile::encode(&resumed_at_c1).unwrap()).unwrap();
    resumed_at_c1.tick().unwrap();
    assert_original_live_capture(&resumed_at_c1, ORIGINAL_CONTACTS_C1, 1);
    resumed_at_c1 = SaveFile::decode(&SaveFile::encode(&resumed_at_c1).unwrap()).unwrap();
    resumed_at_c1.tick_many(9).unwrap();
    assert_original_live_capture(&resumed_at_c1, ORIGINAL_CONTACTS_C10, 10);
    resumed_at_c1.tick().unwrap();
    assert_original_live_capture(&resumed_at_c1, ORIGINAL_CONTACTS_C11, 11);
    for observed in [
        &mut continuous,
        &mut resumed_at_c11,
        &mut resumed_at_c1,
        &mut resumed_at_c10,
    ] {
        observed.tick_many(89).unwrap();
        assert_original_live_capture(observed, ORIGINAL_CONTACTS_C100, 100);
        let restored = SaveFile::decode(&SaveFile::encode(observed).unwrap()).unwrap();
        assert_original_live_capture(&restored, ORIGINAL_CONTACTS_C100, 100);
    }
    assert_eq!(
        SaveFile::encode(&resumed_at_c11).unwrap(),
        SaveFile::encode(&continuous).unwrap()
    );
    assert_eq!(
        SaveFile::encode(&resumed_at_c1).unwrap(),
        SaveFile::encode(&continuous).unwrap()
    );
    assert_eq!(
        SaveFile::encode(&resumed_at_c10).unwrap(),
        SaveFile::encode(&continuous).unwrap()
    );
}

#[test]
fn genuine_ties_later_cycles_preserve_both_endpoint_records_across_checkpoints() {
    let mut continuous = Engine::from_native_sim(TIES_START).unwrap();
    continuous.tick().unwrap();
    assert_original_live_capture(&continuous, ORIGINAL_TIES_C1, 1);
    let mut resumed = SaveFile::decode(&SaveFile::encode(&continuous).unwrap()).unwrap();
    let mut previous = 1;
    for (cycle, reference) in [(10, ORIGINAL_TIES_C10), (100, ORIGINAL_TIES_C100)] {
        for observed in [&mut continuous, &mut resumed] {
            observed.tick_many((cycle - previous) as u32).unwrap();
            assert_original_live_capture(observed, reference, cycle);
            let actual = NativeRecordFile::decode(&observed.export_native_sim().unwrap()).unwrap();
            let expected = NativeRecordFile::decode(reference).unwrap();
            for original in &expected.robots {
                let id = original.long("absolute_id").unwrap();
                let robot = actual
                    .robots
                    .iter()
                    .find(|robot| robot.long("absolute_id").unwrap() == id)
                    .unwrap();
                for name in ["ties_core", "ties_extension"] {
                    assert_eq!(
                        robot.bytes(name).unwrap(),
                        original.bytes(name).unwrap(),
                        "original ties C{cycle} robot{id}.{name}"
                    );
                }
            }
            observed.validate_invariants().unwrap();
        }
        resumed = SaveFile::decode(&SaveFile::encode(&resumed).unwrap()).unwrap();
        assert_original_live_capture(&resumed, reference, cycle);
        assert_eq!(
            SaveFile::encode(&resumed).unwrap(),
            SaveFile::encode(&continuous).unwrap()
        );
        previous = cycle;
    }
}
