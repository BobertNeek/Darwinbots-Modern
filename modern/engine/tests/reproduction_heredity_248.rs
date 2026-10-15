use darwinbots_engine::{
    Engine, EngineConfig, LegacyDna, Obstacle, OrganismId, RobotState, SaveFile,
    StandardMutationProfile,
};
use serde_json::{Value, json};

const PARENT_POSITION: [f32; 2] = [1_000.0, 1_000.0];
const FORWARD_BIRTH_POSITION: [f32; 2] = [1_120.293_6, 1_000.0];
const IMMEDIATE_MEMORY: [i32; 5] = [1_971, 1_972, 1_973, 1_974, 1_975];
const STAGED_MEMORY: [i32; 15] = [
    2_976, 2_977, 2_978, 2_979, 2_980, 2_981, 2_982, 2_983, 2_984, 2_985, 2_986, 2_987, 2_988,
    2_989, 2_990,
];

fn testing_engine() -> Engine {
    Engine::new(EngineConfig {
        organism_capacity: 64,
        world_width: 4_000.0,
        world_height: 3_000.0,
        toroidal_world: false,
        metabolism_cost: 0,
        vegetable_energy_per_tick: 0,
        ..EngineConfig::testing()
    })
    .unwrap()
}

fn frame_v3(payload: &Value) -> Vec<u8> {
    let payload = serde_json::to_vec(payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(b"DB3S");
    save.extend_from_slice(&3_u16.to_le_bytes());
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    save
}

fn edit_engine(engine: &Engine, edit: impl FnOnce(&mut Value)) -> Engine {
    let encoded = SaveFile::encode(engine).unwrap();
    let mut payload: Value = serde_json::from_slice(&encoded[10..]).unwrap();
    edit(&mut payload);
    SaveFile::decode(&frame_v3(&payload)).unwrap()
}

fn engine_payload(engine: &Engine) -> Value {
    let encoded = SaveFile::encode(engine).unwrap();
    serde_json::from_slice(&encoded[10..]).unwrap()
}

fn robot_value_mut(payload: &mut Value, id: OrganismId) -> &mut Value {
    &mut payload["slots"][id.slot() as usize]["organism"]
}

fn set_memory(memory: &mut Value, address: u16, value: i32) {
    let cells = memory["cells"].as_array_mut().unwrap();
    cells.retain(|cell| cell[0].as_u64() != Some(u64::from(address)));
    if value != 0 {
        cells.push(json!([address, value]));
        cells.sort_by_key(|cell| cell[0].as_u64().unwrap());
    }
}

fn set_transfer_materials(robot: &mut Value) {
    let materials = &mut robot["materials"];
    materials["energy"] = json!(4_000);
    materials["body"] = json!(400);
    // Source FindRadius(400 body, 200 chloroplasts) before the first physics pass.
    materials["radius"] = json!(82.392_265);
    materials["waste"] = json!(40);
    materials["permanent_waste"] = json!(20);
    materials["shell"] = json!(70);
    materials["slime"] = json!(80);
    materials["venom"] = json!(90);
    materials["poison"] = json!(100);
    materials["chloroplasts"] = json!(200);
    materials["aim"] = json!(0);
    materials["previous_energy"] = json!(4_000);
    materials["plant_energy_carry_micros"] = json!(0);
    materials["plant_body_carry_micros"] = json!(0);
}

fn set_hereditary_memory(robot: &mut Value) {
    let memory = &mut robot["program"]["memory"];
    set_memory(memory, 500, 1_234);
    for (offset, value) in IMMEDIATE_MEMORY.into_iter().enumerate() {
        set_memory(memory, 971 + offset as u16, value);
    }
    for (offset, value) in STAGED_MEMORY.into_iter().enumerate() {
        set_memory(memory, 976 + offset as u16, value);
    }
}

fn nondefault_profile() -> StandardMutationProfile {
    let mut profile = StandardMutationProfile::default();
    profile.point.one_in = 1_250.0;
    profile.delta.mean = 750.0;
    profile.reversal.std_dev = 2.0;
    profile.point_value_change_percent = 79;
    profile.copy_error_value_change_percent = 65;
    profile
}

fn configured_parent(
    dna: LegacyDna,
    reproduction: i32,
    generation: u32,
) -> (Engine, OrganismId, StandardMutationProfile) {
    let mut engine = testing_engine();
    let parent = engine.spawn_at(dna, PARENT_POSITION).unwrap();
    let profile = nondefault_profile();
    engine
        .set_mutation_profile(parent, profile.clone())
        .unwrap();
    let engine = edit_engine(&engine, |payload| {
        payload["mutations_enabled"] = json!(false);
        payload["global_mutation_multiplier"] = json!(0.0);
        payload["species"][0]["mutation_rate"] = json!(0.0);
        let robot = robot_value_mut(payload, parent);
        set_transfer_materials(robot);
        // Keep this birth fixture independent of the end-of-cycle feeding pass.
        robot["flags"]["can_use_chloroplasts"] = json!(false);
        set_hereditary_memory(robot);
        set_memory(&mut robot["program"]["memory"], 300, reproduction);
        robot["counters"]["generation"] = json!(generation);
        robot["identity"]["species_file"] = json!("alga_minimalis.txt");
        robot["identity"]["owner"] = json!("source-owner");
        robot["identity"]["tag"] = json!("source-tag");
        robot["identity"]["simulation_id"] = json!(7);
        robot["identity"]["absolute_id"] = json!(41);
    });
    (engine, parent, profile)
}

fn child_of(engine: &Engine, parent: OrganismId) -> OrganismId {
    engine
        .snapshot()
        .organisms
        .iter()
        .find(|organism| organism.parents == [Some(parent), None])
        .unwrap_or_else(|| panic!("no child of {parent:?}"))
        .id
}

fn assert_transfer(parent: &RobotState, child: &RobotState, parent_energy: i32) {
    assert_eq!(parent.materials.energy, parent_energy as f32);
    assert_eq!(parent.materials.body, 300.0);
    assert_eq!(parent.materials.waste, 30.0);
    assert_eq!(parent.materials.permanent_waste, 15.0);
    assert_eq!(parent.materials.chloroplasts, 149.645_98);
    assert_eq!(child.materials.energy, 999.0);
    assert_eq!(child.materials.body, 100.0);
    assert_eq!(child.materials.waste, 10.0);
    assert_eq!(child.materials.permanent_waste, 5.0);
    assert_eq!(child.materials.chloroplasts, 49.881_992);
}

fn control_without_birth_request(engine: &Engine, parent: OrganismId) -> RobotState {
    let mut control = edit_engine(engine, |payload| {
        for address in [300, 301, 302] {
            set_memory(
                &mut robot_value_mut(payload, parent)["program"]["memory"],
                address,
                0,
            );
        }
    });
    control.tick().unwrap();
    control.robot_state(parent).unwrap().clone()
}

fn assert_reproduction_owned_materials_unchanged(control: &RobotState, after: &RobotState) {
    assert_eq!(after.materials, control.materials);
}

fn make_blocker_small(engine: &Engine, blocker: OrganismId) -> Engine {
    edit_engine(engine, |payload| {
        let robot = robot_value_mut(payload, blocker);
        let materials = &mut robot["materials"];
        materials["energy"] = json!(1_000);
        materials["body"] = json!(1);
        materials["radius"] = json!(1);
        materials["waste"] = json!(0);
        materials["permanent_waste"] = json!(0);
        materials["shell"] = json!(0);
        materials["slime"] = json!(0);
        materials["venom"] = json!(0);
        materials["poison"] = json!(0);
        materials["chloroplasts"] = json!(0);
        materials["previous_energy"] = json!(1_000);
        robot["identity"]["absolute_id"] = json!(99);
    })
}

#[test]
fn repro_125_transfers_twenty_five_percent_of_db2_resources() {
    let (mut engine, parent, _) =
        configured_parent(LegacyDna::parse("start\nstop").unwrap(), 125, 7);

    engine.tick().unwrap();

    let child = child_of(&engine, parent);
    assert_transfer(
        engine.robot_state(parent).unwrap(),
        engine.robot_state(child).unwrap(),
        2_999,
    );
    let parent_state = engine.robot_state(parent).unwrap();
    let child_state = engine.robot_state(child).unwrap();
    assert_eq!(parent_state.materials.shell, 70.0);
    assert_eq!(parent_state.materials.slime, 78.4);
    assert_eq!(parent_state.materials.venom, 90.0);
    assert_eq!(parent_state.materials.poison, 98.0);
    assert_eq!(child_state.materials.shell, 0.0);
    assert_eq!(child_state.materials.slime, 0.0);
    assert_eq!(child_state.materials.venom, 0.0);
    assert_eq!(child_state.materials.poison, 0.0);
    assert_eq!(engine.memory(parent, "repro").unwrap(), 0);
}

#[test]
fn repro_with_zero_effective_percentage_does_not_birth_or_consume_request() {
    let (mut engine, parent, _) =
        configured_parent(LegacyDna::parse("start\nstop").unwrap(), 100, 7);

    let control = control_without_birth_request(&engine, parent);

    engine.tick().unwrap();

    assert_eq!(engine.population(), 1);
    let after = engine.robot_state(parent).unwrap();
    assert_reproduction_owned_materials_unchanged(&control, after);
    assert_eq!(engine.memory(parent, "repro").unwrap(), 100);
}

#[test]
fn child_inherits_parent_program_profile_species_and_birth_identity() {
    let dna = LegacyDna::parse("start\nstop").unwrap();
    let (engine, parent, profile) = configured_parent(dna.clone(), 25, 7);
    let mut engine = edit_engine(&engine, |payload| {
        let robot = robot_value_mut(payload, parent);
        robot["kinematics"]["velocity"] = json!([2.0, -3.0]);
        robot["flags"]["fixed"] = json!(true);
        robot["flags"]["can_see"] = json!(false);
        robot["flags"]["can_move"] = json!(false);
        robot["flags"]["can_mutate"] = json!(false);
        robot["flags"]["can_use_chloroplasts"] = json!(false);
        robot["flags"]["can_receive_viruses"] = json!(false);
    });

    engine.tick().unwrap();

    let child = child_of(&engine, parent);
    let parent_state = engine.robot_state(parent).unwrap();
    let child_state = engine.robot_state(child).unwrap();
    assert_eq!(child_state.program.dna, dna);
    assert_eq!(child_state.program.mutation_profile, profile);
    assert_eq!(child_state.identity.species, parent_state.identity.species);
    assert_eq!(child_state.phenotype, parent_state.phenotype);
    assert_eq!(
        child_state.kinematics.velocity,
        parent_state.kinematics.velocity
    );
    assert_eq!(child_state.materials.aim, 628);
    assert_eq!(child_state.identity.species_file, "alga_minimalis.txt");
    assert_eq!(child_state.identity.owner, "source-owner");
    assert_eq!(child_state.identity.tag, "source-tag");
    assert_eq!(child_state.identity.parents, [Some(parent), None]);
    assert_eq!(child_state.identity.absolute_id, 42);
    assert_ne!(
        child_state.identity.absolute_id,
        parent_state.identity.absolute_id
    );
    assert_eq!(child_state.counters.birth_cycle, 1);
    assert_eq!(child_state.counters.generation, 8);
    assert_eq!(parent_state.counters.offspring, 1);
    assert_eq!(child_state.flags.fixed, parent_state.flags.fixed);
    assert_eq!(
        engine.memory_at(child, 216).unwrap(),
        i32::from(parent_state.flags.fixed)
    );
    assert_eq!(child_state.flags.can_see, parent_state.flags.can_see);
    assert_eq!(child_state.flags.can_move, parent_state.flags.can_move);
    assert_eq!(child_state.flags.can_mutate, parent_state.flags.can_mutate);
    assert_eq!(
        child_state.flags.can_use_chloroplasts,
        parent_state.flags.can_use_chloroplasts
    );
    assert_eq!(
        child_state.flags.can_receive_viruses,
        parent_state.flags.can_receive_viruses
    );
}

#[test]
fn child_generation_saturates_at_32000() {
    for (parent_generation, expected_child_generation) in [(31_999, 32_000), (32_000, 32_000)] {
        let (mut engine, parent, _) = configured_parent(
            LegacyDna::parse("start\nstop").unwrap(),
            25,
            parent_generation,
        );

        engine.tick().unwrap();

        let child = child_of(&engine, parent);
        assert_eq!(
            engine.robot_state(child).unwrap().counters.generation,
            expected_child_generation
        );
    }
}

#[test]
fn newborn_clears_ordinary_memory_and_stages_only_delayed_heredity() {
    let (mut engine, parent, _) =
        configured_parent(LegacyDna::parse("start\nstop").unwrap(), 25, 7);

    engine.tick().unwrap();

    let child = child_of(&engine, parent);
    assert_eq!(engine.memory_at(child, 500).unwrap(), 0);
    for (offset, expected) in IMMEDIATE_MEMORY.into_iter().enumerate() {
        assert_eq!(
            engine.memory_at(child, 971 + offset as i32).unwrap(),
            expected
        );
    }
    for address in 976..=990 {
        assert_eq!(engine.memory_at(child, address).unwrap(), 0);
    }
    assert_eq!(
        engine
            .robot_state(child)
            .unwrap()
            .heredity
            .epigenetic_staging,
        STAGED_MEMORY
    );
    assert_eq!(
        engine
            .robot_state(child)
            .unwrap()
            .ties
            .iter()
            .flatten()
            .count(),
        1
    );
}

#[test]
fn birth_tie_transfers_one_staged_cell_per_early_age_without_overwrite() {
    let (mut engine, parent, _) =
        configured_parent(LegacyDna::parse("start\nstop").unwrap(), 25, 7);
    engine.tick().unwrap();
    let child = child_of(&engine, parent);
    engine
        .replace_dna(
            child,
            LegacyDna::parse("start\n777 977 store\nend").unwrap(),
        )
        .unwrap();

    engine.tick().unwrap();
    assert_eq!(engine.memory_at(child, 976).unwrap(), 2_976);
    assert_eq!(engine.memory_at(child, 977).unwrap(), 777);
    assert_eq!(engine.memory_at(child, 978).unwrap(), 0);
    assert_eq!(engine.robot_state(child).unwrap().counters.age, 1);

    engine.tick().unwrap();
    assert_eq!(engine.memory_at(child, 976).unwrap(), 2_976);
    assert_eq!(engine.memory_at(child, 977).unwrap(), 777);
    assert_eq!(engine.memory_at(child, 978).unwrap(), 0);
    assert_eq!(engine.robot_state(child).unwrap().counters.age, 2);
}

#[test]
fn disabling_ties_preserves_the_existing_birth_tie_and_delayed_heredity() {
    let (mut engine, parent, _) =
        configured_parent(LegacyDna::parse("start\nstop").unwrap(), 25, 7);
    engine.tick().unwrap();
    let child = child_of(&engine, parent);
    engine.set_ties_enabled(false);
    assert_eq!(engine.snapshot().ties.len(), 1);
    let saved = SaveFile::encode(&engine).unwrap();
    engine = SaveFile::decode(&saved).unwrap();
    assert!(!engine.snapshot().ties_enabled);
    assert_eq!(engine.snapshot().ties.len(), 1);

    engine.tick().unwrap();

    assert_eq!(engine.memory_at(child, 976).unwrap(), 2_976);
    assert_eq!(
        engine
            .robot_state(child)
            .unwrap()
            .heredity
            .epigenetic_staging,
        STAGED_MEMORY
    );
}

#[test]
fn birth_tie_uses_source_asymmetric_ports_and_lifetime_100() {
    let (mut engine, parent, _) =
        configured_parent(LegacyDna::parse("start\nstop").unwrap(), 25, 7);
    engine.tick().unwrap();
    let child = child_of(&engine, parent);

    let tie = &engine.snapshot().ties[0];
    assert_eq!(engine.snapshot().ties.len(), 1);
    assert_eq!((tie.first, tie.second), (parent, child));
    assert_eq!((tie.first_port, tie.second_port), (0, 1));
    assert_eq!(tie.last, 100);
    assert!(!tie.hardened);
    assert!((tie.b - 0.02).abs() < 0.000_001);
    assert!((tie.k - 0.01).abs() < 0.000_001);
    let parent_endpoint = engine
        .robot_state(parent)
        .unwrap()
        .ties
        .iter()
        .flatten()
        .next()
        .unwrap();
    let child_endpoint = engine
        .robot_state(child)
        .unwrap()
        .ties
        .iter()
        .flatten()
        .next()
        .unwrap();
    assert_eq!(
        (parent_endpoint.port, parent_endpoint.reciprocal_index),
        (0, 1)
    );
    assert_eq!(
        (child_endpoint.port, child_endpoint.reciprocal_index),
        (1, 1)
    );
    assert_eq!(
        (parent_endpoint.countdown, child_endpoint.countdown),
        (100, 100)
    );

    engine.tick().unwrap();
    assert_eq!(engine.snapshot().ties[0].last, 99);
}

#[test]
fn later_normal_tie_uses_first_free_child_record_after_birth_tie() {
    let (mut engine, parent, _) =
        configured_parent(LegacyDna::parse("start\nstop").unwrap(), 25, 7);
    engine.tick().unwrap();
    let child = child_of(&engine, parent);
    let child_position = engine.organism(child).unwrap().position;
    let third = engine
        .spawn_at(
            LegacyDna::parse("start\n41 .tie store\nend").unwrap(),
            [child_position[0] + 10.0, child_position[1]],
        )
        .unwrap();

    engine.tick().unwrap();

    let normal = engine
        .snapshot()
        .ties
        .iter()
        .find(|tie| tie.first == third && tie.second == child)
        .expect("third robot did not tie to the newborn");
    assert_eq!((normal.first_port, normal.second_port), (41, 2));
    let child_ports: Vec<_> = engine
        .robot_state(child)
        .unwrap()
        .ties
        .iter()
        .flatten()
        .map(|endpoint| endpoint.port)
        .collect();
    assert_eq!(child_ports, vec![1, 2]);
    assert_eq!(
        engine
            .robot_state(third)
            .unwrap()
            .ties
            .iter()
            .flatten()
            .next()
            .unwrap()
            .port,
        41
    );
}

#[test]
fn occupied_forward_placement_is_atomic_and_keeps_repro_pending() {
    let (mut engine, parent, _) =
        configured_parent(LegacyDna::parse("start\nstop").unwrap(), 25, 7);
    let blocker = engine
        .spawn_at(
            LegacyDna::parse("start\nstop").unwrap(),
            FORWARD_BIRTH_POSITION,
        )
        .unwrap();
    let mut engine = make_blocker_small(&engine, blocker);
    let before = engine.robot_state(parent).unwrap().clone();

    let control = control_without_birth_request(&engine, parent);

    engine.tick().unwrap();

    let after = engine.robot_state(parent).unwrap();
    assert_eq!(engine.population(), 2);
    assert!(
        engine
            .snapshot()
            .organisms
            .iter()
            .all(|organism| organism.parents != [Some(parent), None])
    );
    assert_reproduction_owned_materials_unchanged(&control, after);
    assert_eq!(
        after.program.mutation_profile,
        before.program.mutation_profile
    );
    assert_eq!(after.heredity, before.heredity);
    assert_eq!(after.identity, before.identity);
    assert_eq!(after.ties, before.ties);
    assert_eq!(before.program.memory.read(300), 25);
    assert_eq!(after.program.memory.read(300), 25);
}

#[test]
fn competing_births_commit_in_parent_slot_order_and_leave_loser_unchanged() {
    let mut engine = testing_engine();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let first = engine.spawn_at(idle.clone(), PARENT_POSITION).unwrap();
    // 179.7064 > 164.7845 lets slot 0 birth first; then 59.4128 is within
    // the second-parent-plus-child threshold, so slot 1 is blocked by that child.
    let second = engine.spawn_at(idle, [1_300.0, 1_000.0]).unwrap();
    let first_profile = nondefault_profile();
    let mut second_profile = nondefault_profile();
    second_profile.point.one_in = 2_500.0;
    engine.set_mutation_profile(first, first_profile).unwrap();
    engine.set_mutation_profile(second, second_profile).unwrap();
    let mut engine = edit_engine(&engine, |payload| {
        payload["mutations_enabled"] = json!(false);
        for (id, aim, absolute_id) in [(first, 0, 41), (second, 628, 42)] {
            let robot = robot_value_mut(payload, id);
            set_transfer_materials(robot);
            robot["materials"]["aim"] = json!(aim);
            set_memory(&mut robot["program"]["memory"], 19, aim);
            set_hereditary_memory(robot);
            set_memory(&mut robot["program"]["memory"], 300, 25);
            robot["identity"]["absolute_id"] = json!(absolute_id);
        }
    });
    let loser_before = engine.robot_state(second).unwrap().clone();

    let control = control_without_birth_request(&engine, second);

    engine.tick().unwrap();

    assert_eq!(engine.population(), 3);
    let winner_child = child_of(&engine, first);
    assert_eq!(
        engine.robot_state(winner_child).unwrap().identity.parents,
        [Some(first), None]
    );
    assert!(
        engine
            .snapshot()
            .organisms
            .iter()
            .all(|organism| organism.parents != [Some(second), None])
    );
    assert_eq!(engine.memory(first, "repro").unwrap(), 0);
    let loser_after = engine.robot_state(second).unwrap();
    assert_reproduction_owned_materials_unchanged(&control, loser_after);
    assert_eq!(
        loser_after.program.mutation_profile,
        loser_before.program.mutation_profile
    );
    assert_eq!(loser_after.heredity, loser_before.heredity);
    assert_eq!(loser_after.identity, loser_before.identity);
    assert_eq!(loser_after.ties, loser_before.ties);
    assert_eq!(loser_before.program.memory.read(300), 25);
    assert_eq!(loser_after.program.memory.read(300), 25);
}

#[test]
fn successful_birth_charges_parent_only_for_five_block_dna_copy() {
    let dna = LegacyDna::parse("start\n1 50 store\nend").unwrap();
    assert_eq!(dna.blocks().len(), 5);
    let (engine, parent, _) = configured_parent(dna, 25, 7);
    let mut engine = edit_engine(&engine, |payload| {
        payload["config"]["dna_costs"]["dna_copy"] = json!(2.0);
        payload["config"]["dna_costs"]["global_multiplier"] = json!(3.0);
    });

    engine.tick().unwrap();

    let child = child_of(&engine, parent);
    assert_transfer(
        engine.robot_state(parent).unwrap(),
        engine.robot_state(child).unwrap(),
        2_969,
    );
}

#[test]
fn blocked_birth_charges_no_five_block_dna_copy_cost() {
    let dna = LegacyDna::parse("start\n1 50 store\nend").unwrap();
    assert_eq!(dna.blocks().len(), 5);
    let (mut engine, parent, _) = configured_parent(dna, 25, 7);
    let blocker = engine
        .spawn_at(
            LegacyDna::parse("start\nstop").unwrap(),
            FORWARD_BIRTH_POSITION,
        )
        .unwrap();
    let engine = make_blocker_small(&engine, blocker);
    let mut engine = edit_engine(&engine, |payload| {
        payload["config"]["dna_costs"]["dna_copy"] = json!(2.0);
        payload["config"]["dna_costs"]["global_multiplier"] = json!(3.0);
    });
    let before = engine.robot_state(parent).unwrap().clone();

    let control = control_without_birth_request(&engine, parent);

    engine.tick().unwrap();

    let after = engine.robot_state(parent).unwrap();
    assert_reproduction_owned_materials_unchanged(&control, after);
    assert_eq!(
        after.program.mutation_profile,
        before.program.mutation_profile
    );
    assert_eq!(after.heredity, before.heredity);
    assert_eq!(after.identity, before.identity);
    assert_eq!(after.ties, before.ties);
    assert_eq!(after.program.memory.read(300), 25);
    assert_eq!(engine.population(), 2);
}

#[test]
fn successful_birth_clears_parent_pending_heredity_after_staging_child_memory() {
    let (engine, parent, _) = configured_parent(LegacyDna::parse("start\nstop").unwrap(), 25, 7);
    let mut engine = edit_engine(&engine, |payload| {
        robot_value_mut(payload, parent)["heredity"]["epigenetic_staging"] = json!([
            91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105
        ]);
    });

    engine.tick().unwrap();

    let child = child_of(&engine, parent);
    assert_eq!(
        engine
            .robot_state(parent)
            .unwrap()
            .heredity
            .epigenetic_staging,
        [0; 15]
    );
    assert_eq!(
        engine
            .robot_state(child)
            .unwrap()
            .heredity
            .epigenetic_staging,
        STAGED_MEMORY
    );
}

#[test]
fn blocked_birth_preserves_parent_pending_heredity() {
    let (mut engine, parent, _) =
        configured_parent(LegacyDna::parse("start\nstop").unwrap(), 25, 7);
    let blocker = engine
        .spawn_at(
            LegacyDna::parse("start\nstop").unwrap(),
            FORWARD_BIRTH_POSITION,
        )
        .unwrap();
    let engine = make_blocker_small(&engine, blocker);
    let expected = [
        91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105,
    ];
    let mut engine = edit_engine(&engine, |payload| {
        robot_value_mut(payload, parent)["heredity"]["epigenetic_staging"] = json!(expected);
    });

    engine.tick().unwrap();

    assert_eq!(
        engine
            .robot_state(parent)
            .unwrap()
            .heredity
            .epigenetic_staging,
        expected
    );
    assert_eq!(engine.population(), 2);
    assert_eq!(engine.memory(parent, "repro").unwrap(), 25);
}

#[test]
fn absolute_identity_allocator_survives_child_deletion_and_save_round_trip() {
    let (mut engine, parent, _) =
        configured_parent(LegacyDna::parse("start\nstop").unwrap(), 25, 7);
    engine.tick().unwrap();
    let first_child = child_of(&engine, parent);
    assert_eq!(
        engine
            .robot_state(first_child)
            .unwrap()
            .identity
            .absolute_id,
        42
    );
    engine.remove(first_child).unwrap();
    let bytes = SaveFile::encode(&engine).unwrap();
    let engine = SaveFile::decode(&bytes).unwrap();
    let mut engine = edit_engine(&engine, |payload| {
        set_memory(
            &mut robot_value_mut(payload, parent)["program"]["memory"],
            300,
            25,
        );
    });

    engine.tick().unwrap();

    let second_child = child_of(&engine, parent);
    assert_eq!(
        engine
            .robot_state(second_child)
            .unwrap()
            .identity
            .absolute_id,
        43
    );
}

#[test]
fn positive_energy_below_201_remains_eligible_for_reproduction() {
    let (engine, parent, _) = configured_parent(LegacyDna::parse("start\nstop").unwrap(), 25, 7);
    let mut engine = edit_engine(&engine, |payload| {
        let robot = robot_value_mut(payload, parent);
        robot["materials"]["energy"] = json!(1);
        robot["materials"]["previous_energy"] = json!(1);
        robot["materials"]["body"] = json!(5);
        robot["materials"]["chloroplasts"] = json!(0);
    });

    engine.tick().unwrap();

    assert_eq!(engine.population(), 2);
    assert_eq!(engine.memory(parent, "repro").unwrap(), 0);
    assert_eq!(
        engine
            .robot_state(child_of(&engine, parent))
            .unwrap()
            .materials
            .energy,
        0.24975
    );
}

#[test]
fn non_toroidal_birth_bounds_include_parent_radius_and_smudge_50() {
    let (engine, parent, _) = configured_parent(LegacyDna::parse("start\nstop").unwrap(), 25, 7);
    let mut engine = edit_engine(&engine, |payload| {
        let robot = robot_value_mut(payload, parent);
        robot["kinematics"]["position"] = json!([3_760.0, 1_000.0]);
        robot["kinematics"]["previous_position"] = json!([3_760.0, 1_000.0]);
        set_memory(&mut robot["program"]["memory"], 219, 3_760);
    });

    engine.tick().unwrap();

    assert_eq!(engine.population(), 1);
    assert_eq!(engine.memory(parent, "repro").unwrap(), 25);
}

#[test]
fn collision_uses_radius_stored_before_same_cycle_body_feed() {
    let (mut engine, parent, _) =
        configured_parent(LegacyDna::parse("start\nstop").unwrap(), 25, 7);
    let blocker = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [1_097.0, 1_000.0])
        .unwrap();
    let engine = make_blocker_small(&engine, blocker);
    let mut engine = edit_engine(&engine, |payload| {
        set_memory(
            &mut robot_value_mut(payload, parent)["program"]["memory"],
            312,
            395,
        );
    });

    engine.tick().unwrap();

    assert_eq!(engine.population(), 2);
    assert_eq!(engine.memory(parent, "repro").unwrap(), 25);
}

#[test]
fn toroidal_birth_uses_unwrapped_candidate_and_obstacle_segment() {
    let (engine, parent, _) = configured_parent(LegacyDna::parse("start\nstop").unwrap(), 25, 7);
    let mut engine = edit_engine(&engine, |payload| {
        payload["config"]["toroidal_world"] = json!(true);
        let robot = robot_value_mut(payload, parent);
        robot["kinematics"]["position"] = json!([3_900.0, 1_000.0]);
        robot["kinematics"]["previous_position"] = json!([3_900.0, 1_000.0]);
    });
    engine
        .add_obstacle(Obstacle::new(77, [2_000.0, 900.0], [2_010.0, 1_100.0]))
        .unwrap();

    engine.tick().unwrap();

    let child = child_of(&engine, parent);
    assert!(engine.organism(child).unwrap().position[0] > 4_000.0);
}

#[test]
fn buoyancy_command_readback_and_birth_inheritance_round_trip() {
    let dna = LegacyDna::parse("start\n16000 .setboy store\nend").unwrap();
    let (mut engine, parent, _) = configured_parent(dna, 25, 7);

    engine.tick().unwrap();

    let child = child_of(&engine, parent);
    assert_eq!(engine.memory_at(parent, 314).unwrap(), 0);
    assert_eq!(engine.memory_at(parent, 315).unwrap(), 16_000);
    assert_eq!(engine.memory_at(child, 315).unwrap(), 16_000);
    let restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
    let payload = engine_payload(&restored);
    assert_eq!(
        payload["slots"][parent.slot() as usize]["organism"]["buoyancy"],
        json!(0.5)
    );
    assert_eq!(
        payload["slots"][child.slot() as usize]["organism"]["buoyancy"],
        json!(0.5)
    );
}

#[test]
fn body_split_uses_vb6_cint_round_to_even_without_minimums() {
    for (percentage, expected_parent, expected_child) in [(1, 5, 0), (99, 0, 5), (30, 3, 2)] {
        let (engine, parent, _) =
            configured_parent(LegacyDna::parse("start\nstop").unwrap(), percentage, 7);
        let mut engine = edit_engine(&engine, |payload| {
            robot_value_mut(payload, parent)["materials"]["body"] = json!(5);
        });

        engine.tick().unwrap();

        let child = child_of(&engine, parent);
        assert_eq!(
            engine.robot_state(parent).unwrap().materials.body,
            expected_parent as f32
        );
        assert_eq!(
            engine.robot_state(child).unwrap().materials.body,
            expected_child as f32
        );
    }
}

#[test]
fn post_birth_mass_and_previous_energy_match_source_bookkeeping() {
    let dna = LegacyDna::parse("start\n1 50 store\nend").unwrap();
    let (engine, parent, _) = configured_parent(dna, 25, 7);
    let mut engine = edit_engine(&engine, |payload| {
        payload["config"]["dna_costs"]["dna_copy"] = json!(2.0);
        payload["config"]["dna_costs"]["global_multiplier"] = json!(3.0);
    });

    engine.tick().unwrap();

    let child = child_of(&engine, parent);
    assert!((engine.robot_state(parent).unwrap().materials.mass - 198.75).abs() < 0.000_001);
    let parent_radius = engine.robot_state(parent).unwrap().materials.radius;
    assert!(
        (parent_radius - 73.910_965).abs() < 0.000_1,
        "parent radius was {parent_radius}, expected approximately 73.910965"
    );
    assert!((engine.robot_state(child).unwrap().materials.mass - 0.1).abs() < 0.000_001);
    let payload = engine_payload(&engine);
    assert_eq!(
        payload["slots"][parent.slot() as usize]["organism"]["materials"]["previous_energy"],
        json!(2_999.0)
    );
}

#[test]
fn ties_disabled_still_allows_birth_ties_and_staged_transfer() {
    let (mut engine, parent, _) =
        configured_parent(LegacyDna::parse("start\nstop").unwrap(), 25, 7);
    engine.set_ties_enabled(false);

    engine.tick().unwrap();

    let child = child_of(&engine, parent);
    assert_eq!(engine.snapshot().ties.len(), 1);
    assert_eq!(engine.snapshot().ties[0].last, 100);
    engine.tick().unwrap();
    assert_eq!(engine.memory_at(child, 976).unwrap(), 2_976);
}

#[test]
fn full_parent_tie_capacity_allows_birth_without_birth_tie_or_staged_transfer() {
    let (mut engine, parent, _) =
        configured_parent(LegacyDna::parse("start\nstop").unwrap(), 25, 7);
    let mut others = Vec::new();
    for index in 0..10 {
        others.push(
            engine
                .spawn_at(
                    LegacyDna::parse("start\nstop").unwrap(),
                    [500.0, 500.0 + index as f32 * 100.0],
                )
                .unwrap(),
        );
    }
    let mut engine = edit_engine(&engine, |payload| {
        let ties: Vec<_> = others
            .iter()
            .enumerate()
            .map(|(index, other)| {
                let dy = 500.0 + index as f32 * 100.0 - 1_000.0;
                json!({
                    "first": {"slot": parent.slot(), "generation": 0},
                    "second": {"slot": other.slot(), "generation": 0},
                    "rest_length": (500.0_f32 * 500.0 + dy * dy).sqrt(),
                    "first_port": index as i32 + 1,
                    "second_port": 1,
                    "last": 0,
                    "b": 0.1,
                    "k": 0.05,
                    "hardened": true
                })
            })
            .collect();
        payload["ties"] = json!(ties);
        for other in &others {
            let robot = robot_value_mut(payload, *other);
            robot["materials"]["body"] = json!(1);
            robot["materials"]["chloroplasts"] = json!(0);
        }
    });

    engine.tick().unwrap();

    let child = child_of(&engine, parent);
    assert_eq!(engine.snapshot().ties.len(), 10);
    assert_eq!(
        engine
            .robot_state(child)
            .unwrap()
            .ties
            .iter()
            .flatten()
            .count(),
        0
    );
    engine.tick().unwrap();
    assert_eq!(engine.memory_at(child, 976).unwrap(), 0);
}
