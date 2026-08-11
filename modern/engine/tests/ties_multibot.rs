use darwinbots_engine::{
    Engine, EngineConfig, LegacyDna, OrganismId, SaveFile, SpeciesDefinition, Teleporter,
};
use serde_json::{Value, json};

fn tie_test_config() -> EngineConfig {
    EngineConfig {
        metabolism_cost: 0,
        world_width: 3_000.0,
        world_height: 2_000.0,
        toroidal_world: false,
        ..EngineConfig::testing()
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.000_01,
        "expected {expected}, got {actual}"
    );
}

fn tie_field_i64(engine: &Engine, field: &str) -> Option<i64> {
    serde_json::to_value(&engine.snapshot().ties[0]).unwrap()[field].as_i64()
}

fn tie_field_f64(engine: &Engine, field: &str) -> Option<f64> {
    serde_json::to_value(&engine.snapshot().ties[0]).unwrap()[field].as_f64()
}

fn assert_tie_float(engine: &Engine, field: &str, expected: f64) {
    let actual = tie_field_f64(engine, field).expect("tie snapshot field is missing");
    assert!(
        (actual - expected).abs() < 0.000_001,
        "expected tie {field}={expected}, got {actual}"
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TieMaterialValues {
    energy: i32,
    body: i32,
    waste: i32,
    permanent_waste: i32,
    venom: i32,
    poison: i32,
}

#[derive(Clone, Copy, Debug)]
struct TieResourceCase {
    name: &'static str,
    tieloc: i32,
    tieval: i32,
    source: TieMaterialValues,
    target: TieMaterialValues,
    expected_source: TieMaterialValues,
    expected_target: TieMaterialValues,
    expected_source_poisoning: (bool, i32, i32, i32),
    expected_target_paralysis: (bool, i32, i32, i32),
}

fn tie_material_values(engine: &Engine, id: OrganismId) -> TieMaterialValues {
    let robot = engine.robot_state(id).unwrap();
    TieMaterialValues {
        energy: robot.materials.energy,
        body: robot.materials.body,
        waste: robot.materials.waste,
        permanent_waste: robot.materials.permanent_waste,
        venom: robot.materials.venom,
        poison: robot.materials.poison,
    }
}

fn set_tie_memory(memory: &mut Value, address: u16, value: i32) {
    let cells = memory["cells"].as_array_mut().unwrap();
    cells.retain(|cell| cell[0].as_u64() != Some(u64::from(address)));
    if value != 0 {
        cells.push(json!([address, value]));
        cells.sort_by_key(|cell| cell[0].as_u64().unwrap());
    }
}

fn set_tie_materials(payload: &mut Value, id: OrganismId, values: TieMaterialValues) {
    let robot = &mut payload["slots"][id.slot() as usize]["organism"];
    let materials = &mut robot["materials"];
    materials["energy"] = json!(values.energy);
    materials["body"] = json!(values.body);
    materials["waste"] = json!(values.waste);
    materials["permanent_waste"] = json!(values.permanent_waste);
    materials["venom"] = json!(values.venom);
    materials["poison"] = json!(values.poison);
    materials["previous_energy"] = json!(values.energy);
    set_tie_memory(&mut robot["program"]["memory"], 310, values.energy);
    set_tie_memory(&mut robot["program"]["memory"], 311, values.body);
    set_tie_memory(&mut robot["program"]["memory"], 828, values.waste);
    set_tie_memory(&mut robot["program"]["memory"], 829, values.permanent_waste);
    set_tie_memory(&mut robot["program"]["memory"], 825, values.venom);
    set_tie_memory(&mut robot["program"]["memory"], 827, values.poison);
}

fn tie_resource_engine(case: TieResourceCase) -> (Engine, OrganismId, OrganismId) {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        world_width: 3_000.0,
        world_height: 2_000.0,
        toroidal_world: false,
        ..EngineConfig::testing()
    })
    .unwrap();
    let source_species = engine.register_species(SpeciesDefinition {
        name: "Tie source".to_owned(),
        ..SpeciesDefinition::default()
    });
    let target_species = engine.register_species(SpeciesDefinition {
        name: "Tie target".to_owned(),
        ..SpeciesDefinition::default()
    });
    let source = engine
        .spawn_species_at(
            LegacyDna::parse("start\n41 .tie store\nstop").unwrap(),
            source_species,
            [400.0, 1_000.0],
        )
        .unwrap();
    let target = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            target_species,
            [700.0, 1_000.0],
        )
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().ties.len(), 1);
    engine
        .replace_dna(
            source,
            LegacyDna::parse(&format!(
                "start\n41 .tienum store\n{} .tieloc store\n{} .tieval store\n0 .fdbody store\n0 .strbody store\nstop",
                case.tieloc, case.tieval
            ))
            .unwrap(),
        )
        .unwrap();

    let encoded = SaveFile::encode(&engine).unwrap();
    let mut payload: Value = serde_json::from_slice(&encoded[10..]).unwrap();
    set_tie_materials(&mut payload, source, case.source);
    set_tie_materials(&mut payload, target, case.target);
    set_tie_memory(
        &mut payload["slots"][source.slot() as usize]["organism"]["program"]["memory"],
        835,
        777,
    );
    set_tie_memory(
        &mut payload["slots"][source.slot() as usize]["organism"]["program"]["memory"],
        836,
        456,
    );
    set_tie_memory(
        &mut payload["slots"][target.slot() as usize]["organism"]["program"]["memory"],
        834,
        888,
    );
    set_tie_memory(
        &mut payload["slots"][target.slot() as usize]["organism"]["program"]["memory"],
        839,
        654,
    );
    let payload = serde_json::to_vec(&payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(&encoded[..6]);
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    (SaveFile::decode(&save).unwrap(), source, target)
}

#[test]
fn negative_tieloc_resource_exchange_runs_in_engine_tick() {
    let cases = [
        TieResourceCase {
            name: "energy give",
            tieloc: -1,
            tieval: 1_000,
            source: TieMaterialValues {
                energy: 1_500,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            target: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_source: TieMaterialValues {
                energy: 500,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_target: TieMaterialValues {
                energy: 1_700,
                body: 1_029,
                waste: 10,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_source_poisoning: (false, 0, 0, 0),
            expected_target_paralysis: (false, 0, 0, 0),
        },
        TieResourceCase {
            name: "energy give clamps and rounds",
            tieloc: -1,
            tieval: 3_000,
            source: TieMaterialValues {
                energy: 1_500,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            target: TieMaterialValues {
                energy: 1_234,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_source: TieMaterialValues {
                energy: 500,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_target: TieMaterialValues {
                energy: 1_934,
                body: 1_029,
                waste: 10,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_source_poisoning: (false, 0, 0, 0),
            expected_target_paralysis: (false, 0, 0, 0),
        },
        TieResourceCase {
            name: "venom inject",
            tieloc: -3,
            tieval: 100,
            source: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 150,
                poison: 0,
            },
            target: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_source: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 50,
                poison: 0,
            },
            expected_target: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_source_poisoning: (false, 0, 0, 0),
            expected_target_paralysis: (true, 777, 456, 100),
        },
        TieResourceCase {
            name: "venom steal clamps",
            tieloc: -3,
            tieval: -100,
            source: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            target: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 37,
                poison: 0,
            },
            expected_source: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 37,
                poison: 0,
            },
            expected_target: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_source_poisoning: (false, 0, 0, 0),
            expected_target_paralysis: (false, 0, 0, 0),
        },
        TieResourceCase {
            name: "waste give",
            tieloc: -4,
            tieval: 1_000,
            source: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 1_000,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            target: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_source: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 10,
                venom: 0,
                poison: 0,
            },
            expected_target: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 990,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_source_poisoning: (false, 0, 0, 0),
            expected_target_paralysis: (false, 0, 0, 0),
        },
        TieResourceCase {
            name: "waste take",
            tieloc: -4,
            tieval: -1_000,
            source: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            target: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 250,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_source: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 248,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_target: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 3,
                venom: 0,
                poison: 0,
            },
            expected_source_poisoning: (false, 0, 0, 0),
            expected_target_paralysis: (false, 0, 0, 0),
        },
        TieResourceCase {
            name: "body give",
            tieloc: -6,
            tieval: 300,
            source: TieMaterialValues {
                energy: 1_000,
                body: 101,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            target: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_source: TieMaterialValues {
                energy: 1_000,
                body: 1,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_target: TieMaterialValues {
                energy: 1_003,
                body: 1_099,
                waste: 1,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_source_poisoning: (false, 0, 0, 0),
            expected_target_paralysis: (false, 0, 0, 0),
        },
        TieResourceCase {
            name: "body take",
            tieloc: -6,
            tieval: -300,
            source: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            target: TieMaterialValues {
                energy: 1_000,
                body: 1_234,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_source: TieMaterialValues {
                energy: 1_009,
                body: 1_296,
                waste: 3,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_target: TieMaterialValues {
                energy: 1_000,
                body: 934,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_source_poisoning: (false, 0, 0, 0),
            expected_target_paralysis: (false, 0, 0, 0),
        },
        TieResourceCase {
            name: "energy take poison defense",
            tieloc: -1,
            tieval: -100,
            source: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            target: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 100,
            },
            expected_source: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_target: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 75,
            },
            expected_source_poisoning: (true, 888, 654, 25),
            expected_target_paralysis: (false, 0, 0, 0),
        },
        TieResourceCase {
            name: "body take poison defense",
            tieloc: -6,
            tieval: -100,
            source: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            target: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 100,
            },
            expected_source: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 0,
            },
            expected_target: TieMaterialValues {
                energy: 1_000,
                body: 1_000,
                waste: 0,
                permanent_waste: 0,
                venom: 0,
                poison: 75,
            },
            expected_source_poisoning: (true, 888, 654, 25),
            expected_target_paralysis: (false, 0, 0, 0),
        },
    ];

    for case in cases {
        let (mut engine, source, target) = tie_resource_engine(case);
        engine.tick().unwrap();

        assert_eq!(
            tie_material_values(&engine, source),
            case.expected_source,
            "{} source",
            case.name
        );
        assert_eq!(
            tie_material_values(&engine, target),
            case.expected_target,
            "{} target",
            case.name
        );
        let source_memory = &engine.robot_state(source).unwrap().program.memory;
        let target_memory = &engine.robot_state(target).unwrap().program.memory;
        assert_eq!(
            (
                source_memory.read(310),
                source_memory.read(311),
                source_memory.read(828),
                source_memory.read(829),
                target_memory.read(310),
                target_memory.read(311),
                target_memory.read(828),
                target_memory.read(829),
            ),
            (
                case.expected_source.energy,
                case.expected_source.body,
                case.expected_source.waste,
                case.expected_source.permanent_waste,
                case.expected_target.energy,
                case.expected_target.body,
                case.expected_target.waste,
                case.expected_target.permanent_waste,
            ),
            "{} published material stores",
            case.name
        );
        let source_effects = &engine.robot_state(source).unwrap().effects;
        assert_eq!(
            (
                source_effects.poisoning_active,
                source_effects.poisoning_location,
                source_effects.poisoning_value,
                source_effects.poisoning_countdown.round() as i32,
            ),
            case.expected_source_poisoning,
            "{} source poison",
            case.name
        );
        let target_effects = &engine.robot_state(target).unwrap().effects;
        assert_eq!(
            (
                target_effects.paralysis_active,
                target_effects.paralysis_location,
                target_effects.paralysis_value,
                target_effects.paralysis_countdown.round() as i32,
            ),
            case.expected_target_paralysis,
            "{} target paralysis",
            case.name
        );
    }
}

fn spawn_soft_tie() -> (Engine, OrganismId, OrganismId) {
    let mut engine = Engine::new(tie_test_config()).unwrap();
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
    assert_eq!(engine.snapshot().ties.len(), 1);
    (engine, source, target)
}

#[test]
fn review_projection_hardened_removal_clears_state_before_reused_slot_dna() {
    let (mut engine, source, target) = spawn_soft_tie();
    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n777 .trefbody store\n41 .readtie store\nstop").unwrap(),
        )
        .unwrap();
    engine.tick_many(19).unwrap();
    assert_eq!(engine.snapshot().tick, 20);
    assert_eq!(engine.memory(source, "multi").unwrap(), 1);
    assert!(engine.robot_state(source).unwrap().counters.multibot);
    engine
        .replace_dna(
            source,
            LegacyDna::parse(
                "start\n*.numties 50 store\n*.myties 51 store\n*.tiepres 52 store\n*.multi 53 store\nstop",
            )
            .unwrap(),
        )
        .unwrap();

    engine.remove_many(&[target]).unwrap();
    let replacement = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [700.0, 1_000.0])
        .unwrap();

    assert_eq!(replacement.slot(), target.slot());
    assert_ne!(replacement, target);
    assert!(engine.snapshot().ties.is_empty());
    assert_eq!(
        (
            engine.memory(source, "numties").unwrap(),
            engine.memory(source, "myties").unwrap(),
            engine.memory(source, "tiepres").unwrap(),
            engine.memory(source, "multi").unwrap(),
            engine.robot_state(source).unwrap().counters.multibot,
            engine.memory(source, "trefbody").unwrap(),
        ),
        (0, 0, 0, 0, false, 0)
    );

    engine.tick().unwrap();
    assert_eq!(
        (
            engine.memory_at(source, 50).unwrap(),
            engine.memory_at(source, 51).unwrap(),
            engine.memory_at(source, 52).unwrap(),
            engine.memory_at(source, 53).unwrap(),
        ),
        (0, 0, 0, 0)
    );
}

#[test]
fn review_projection_create_save_load_break_tracks_one_canonical_tie() {
    let (engine, source, target) = spawn_soft_tie();
    assert_eq!(
        (
            engine
                .robot_state(source)
                .unwrap()
                .ties
                .iter()
                .flatten()
                .count(),
            engine
                .robot_state(target)
                .unwrap()
                .ties
                .iter()
                .flatten()
                .count(),
        ),
        (1, 1)
    );
    let source_endpoint = engine.robot_state(source).unwrap().ties[0].unwrap();
    let target_endpoint = engine.robot_state(target).unwrap().ties[0].unwrap();
    assert_eq!(source_endpoint.other, Some(target));
    assert_eq!(target_endpoint.other, Some(source));
    assert_eq!((source_endpoint.port, target_endpoint.port), (41, 1));
    assert_eq!(
        (
            source_endpoint.reciprocal_index,
            target_endpoint.reciprocal_index,
        ),
        (1, 1)
    );
    assert_eq!(
        (source_endpoint.countdown, target_endpoint.countdown),
        (-20, -20)
    );
    assert_close(source_endpoint.current_length, 300.0);
    assert_close(target_endpoint.current_length, 300.0);
    assert_close(source_endpoint.requested_length, 300.0);
    assert_close(target_endpoint.requested_length, 300.0);

    let mut restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
    assert_eq!(restored.snapshot().ties, engine.snapshot().ties);
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
        restored.robot_state(source).unwrap().ties[0].unwrap().other,
        Some(target)
    );
    assert_eq!(
        restored.robot_state(target).unwrap().ties[0].unwrap().other,
        Some(source)
    );

    let source_radius = restored
        .snapshot()
        .render_instances
        .iter()
        .find(|instance| instance.slot == source.slot())
        .unwrap()
        .radius;
    let target_radius = restored
        .snapshot()
        .render_instances
        .iter()
        .find(|instance| instance.slot == target.slot())
        .unwrap()
        .radius;
    let mut batch = restored.physics_batch();
    let source_index = batch.ids.iter().position(|id| *id == source).unwrap();
    let target_index = batch.ids.iter().position(|id| *id == target).unwrap();
    batch.batch.positions[source_index] = [400.0, 1_000.0];
    batch.batch.positions[target_index] =
        [400.0 + source_radius + target_radius + 1_000.5, 1_000.0];
    batch.batch.velocities[source_index] = [0.0; 2];
    batch.batch.velocities[target_index] = [0.0; 2];
    restored.apply_physics_batch(batch).unwrap();
    restored.tick().unwrap();

    assert!(restored.snapshot().ties.is_empty());
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
        (0, 0)
    );
}

#[test]
fn new_tie_hardens_on_db2_cycle() {
    let (mut engine, source, target) = spawn_soft_tie();
    let created = &engine.snapshot().ties[0];
    assert_eq!(engine.snapshot().tick, 1);
    assert_eq!(created.first, source);
    assert_eq!(created.second, target);
    assert_eq!(created.first_port, 41);
    assert_eq!(created.second_port, 1);
    assert_eq!(tie_field_i64(&engine, "last"), Some(-20));
    assert_tie_float(&engine, "b", 0.02);
    assert_tie_float(&engine, "k", 0.01);
    assert_eq!(engine.memory(source, "multi").unwrap(), 0);
    assert_eq!(engine.memory(target, "multi").unwrap(), 0);

    engine.tick().unwrap();
    assert_eq!(engine.snapshot().tick, 2);
    assert_eq!(tie_field_i64(&engine, "last"), Some(-19));
    assert_tie_float(&engine, "b", 0.02);
    assert_tie_float(&engine, "k", 0.01);
    assert_eq!(engine.memory(source, "multi").unwrap(), 0);
    assert_eq!(engine.memory(target, "multi").unwrap(), 0);

    engine.tick_many(17).unwrap();
    assert_eq!(engine.snapshot().tick, 19);
    assert_eq!(tie_field_i64(&engine, "last"), Some(-2));
    assert_tie_float(&engine, "b", 0.02);
    assert_tie_float(&engine, "k", 0.01);
    assert_eq!(engine.memory(source, "multi").unwrap(), 0);
    assert_eq!(engine.memory(target, "multi").unwrap(), 0);

    engine.tick().unwrap();
    assert_eq!(engine.snapshot().tick, 20);
    assert_eq!(tie_field_i64(&engine, "last"), Some(-1));
    assert_tie_float(&engine, "b", 0.1);
    assert_tie_float(&engine, "k", 0.05);
    assert_eq!(engine.memory(source, "multi").unwrap(), 1);
    assert_eq!(engine.memory(target, "multi").unwrap(), 1);

    engine.tick().unwrap();
    assert_eq!(engine.snapshot().tick, 21);
    assert_eq!(tie_field_i64(&engine, "last"), Some(0));
    assert_tie_float(&engine, "b", 0.1);
    assert_tie_float(&engine, "k", 0.05);
    assert_eq!(engine.memory(source, "multi").unwrap(), 1);
    assert_eq!(engine.memory(target, "multi").unwrap(), 1);
}

#[test]
fn fixlen_command_changes_the_authoritative_tie_spring_response() {
    let (mut engine, source, target) = spawn_soft_tie();
    engine.tick_many(19).unwrap();
    assert!(engine.snapshot().ties[0].hardened);
    let original_length = engine.snapshot().ties[0].rest_length;

    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n41 .tienum store\n20 .fixlen store\nstop").unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();

    let fixed_length = engine.snapshot().ties[0].rest_length;
    assert!(fixed_length < original_length);

    let mut batch = engine.physics_batch();
    let source_index = batch.ids.iter().position(|id| *id == source).unwrap();
    let target_index = batch.ids.iter().position(|id| *id == target).unwrap();
    batch.batch.positions[source_index] = [400.0, 1_000.0];
    batch.batch.positions[target_index] = [700.0, 1_000.0];
    batch.batch.velocities[source_index] = [0.0; 2];
    batch.batch.velocities[target_index] = [0.0; 2];
    engine.apply_physics_batch(batch).unwrap();
    engine.tick().unwrap();

    let source_after = engine.organism(source).unwrap();
    let target_after = engine.organism(target).unwrap();
    assert!(source_after.velocity[0] > 0.0);
    assert!(target_after.velocity[0] < 0.0);
}

#[test]
fn stifftie_command_changes_hardened_tie_force() {
    let (mut engine, source, target) = spawn_soft_tie();
    engine.tick_many(19).unwrap();

    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n41 .tienum store\n80 .stifftie store\nstop").unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();

    assert_tie_float(&engine, "b", 0.4);
    assert_tie_float(&engine, "k", 0.2);

    let rest_length = engine.snapshot().ties[0].rest_length;
    let mut batch = engine.physics_batch();
    let source_index = batch.ids.iter().position(|id| *id == source).unwrap();
    let target_index = batch.ids.iter().position(|id| *id == target).unwrap();
    batch.batch.positions[source_index] = [400.0, 1_000.0];
    batch.batch.positions[target_index] = [400.0 + rest_length + 30.0, 1_000.0];
    batch.batch.velocities[source_index] = [0.0; 2];
    batch.batch.velocities[target_index] = [0.0; 2];
    engine.apply_physics_batch(batch).unwrap();
    engine.tick().unwrap();

    assert!(engine.organism(source).unwrap().velocity[0] > 1.0);
    assert!(engine.organism(target).unwrap().velocity[0] < -1.0);
}

#[test]
fn fixang_command_applies_canonical_tie_torque() {
    let (mut engine, source, target) = spawn_soft_tie();
    engine.tick_many(19).unwrap();

    let mut batch = engine.physics_batch();
    let source_index = batch.ids.iter().position(|id| *id == source).unwrap();
    let target_index = batch.ids.iter().position(|id| *id == target).unwrap();
    batch.batch.positions[source_index] = [400.0, 1_000.0];
    batch.batch.positions[target_index] = [400.0, 1_300.0];
    batch.batch.velocities[source_index] = [0.0; 2];
    batch.batch.velocities[target_index] = [0.0; 2];
    engine.apply_physics_batch(batch).unwrap();
    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n41 .tienum store\n0 .fixang store\nstop").unwrap(),
        )
        .unwrap();

    engine.tick().unwrap();
    engine.tick().unwrap();

    assert_ne!(engine.organism(source).unwrap().aim, 0);
    assert!(
        engine
            .robot_state(source)
            .unwrap()
            .kinematics
            .angular_momentum
            .abs()
            > 0.001
    );
}

#[test]
fn structural_tie_torque_rotates_both_members_in_opposite_directions() {
    let (mut engine, source, target) = spawn_soft_tie();
    engine.tick_many(19).unwrap();

    let mut batch = engine.physics_batch();
    let source_index = batch.ids.iter().position(|id| *id == source).unwrap();
    let target_index = batch.ids.iter().position(|id| *id == target).unwrap();
    batch.batch.positions[source_index] = [400.0, 1_000.0];
    batch.batch.positions[target_index] = [400.0, 1_300.0];
    batch.batch.velocities[source_index] = [0.0; 2];
    batch.batch.velocities[target_index] = [0.0; 2];
    engine.apply_physics_batch(batch).unwrap();

    engine.tick().unwrap();
    engine.tick().unwrap();

    let source_aim = engine.organism(source).unwrap().aim;
    let target_aim = engine.organism(target).unwrap().aim;
    let source_angular = engine
        .robot_state(source)
        .unwrap()
        .kinematics
        .angular_momentum;
    let target_angular = engine
        .robot_state(target)
        .unwrap()
        .kinematics
        .angular_momentum;
    assert_ne!(source_aim, 0);
    assert_ne!(target_aim, 0);
    assert!(source_angular * target_angular < 0.0);
}

#[test]
fn multibot_amplifies_feeding_shot_and_virtual_body_range() {
    let mut baseline = Engine::new(tie_test_config()).unwrap();
    baseline
        .spawn_at(
            LegacyDna::parse("start\n-1 .shoot store\nstop").unwrap(),
            [400.0, 1_000.0],
        )
        .unwrap();
    baseline.tick().unwrap();
    let baseline_shot = baseline.snapshot().shots[0].clone();

    let (mut tied, source, _target) = spawn_soft_tie();
    tied.tick_many(19).unwrap();
    assert!(tied.snapshot().ties[0].hardened);
    tied.replace_dna(
        source,
        LegacyDna::parse("start\n-1 .shoot store\nstop").unwrap(),
    )
    .unwrap();
    tied.tick().unwrap();
    let tied_shot = tied.snapshot().shots[0].clone();

    assert_eq!(tied_shot.value, baseline_shot.value * 2);
    assert!(tied_shot.range > baseline_shot.range);
}

#[test]
fn hardened_tie_group_follows_teleporter_relocation() {
    let (mut engine, source, target) = spawn_soft_tie();
    engine.tick_many(19).unwrap();
    assert!(engine.snapshot().ties[0].hardened);
    let source_before = engine.organism(source).unwrap().position;
    let target_before = engine.organism(target).unwrap().position;
    let bystander = engine
        .spawn_at(
            LegacyDna::parse("start\nstop").unwrap(),
            [source_before[0], source_before[1] + 40.0],
        )
        .unwrap();
    engine
        .add_teleporter(Teleporter {
            id: 1,
            center: source_before,
            radius: 50.0,
            destination: [source_before[0] + 800.0, source_before[1]],
            capacity: 1,
            ..Teleporter::new(1, source_before, 50.0, [source_before[0] + 800.0, source_before[1]])
        })
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(
        engine.organism(target).unwrap().position,
        [target_before[0] + 800.0, target_before[1]]
    );
    assert!(
        engine.organism(bystander).unwrap().position[0] < source_before[0] + 400.0,
        "teleporter capacity must count the hardened group as one transfer"
    );
}

#[test]
fn tie_cluster_spans_addressed_readbacks_torque_hardening_traversal_and_breakage() {
    let (mut engine, source, target) = spawn_soft_tie();

    assert_eq!(engine.snapshot().tick, 1);
    assert_eq!(engine.snapshot().ties[0].last, -20);
    assert!(!engine.snapshot().ties[0].hardened);
    assert_eq!(engine.memory(source, "numties").unwrap(), 1);
    assert_eq!(engine.memory(source, "myties").unwrap(), 1);
    assert_eq!(engine.memory(source, "multi").unwrap(), 0);

    engine.tick_many(18).unwrap();
    assert_eq!(engine.snapshot().tick, 19);
    assert_eq!(engine.snapshot().ties[0].last, -2);
    assert!(!engine.snapshot().ties[0].hardened);

    engine.tick().unwrap();
    assert_eq!(engine.snapshot().tick, 20);
    assert_eq!(engine.snapshot().ties[0].last, -1);
    assert!(engine.snapshot().ties[0].hardened);
    assert_tie_float(&engine, "b", 0.1);
    assert_tie_float(&engine, "k", 0.05);
    assert_eq!(engine.memory(source, "multi").unwrap(), 1);

    engine
        .replace_dna(
            source,
            LegacyDna::parse(
                "start\n41 .tienum store\n50 .tieloc store\n1234 .tieval store\n0 .fixang store\nstop",
            )
            .unwrap(),
        )
        .unwrap();
    let mut batch = engine.physics_batch();
    let source_index = batch.ids.iter().position(|id| *id == source).unwrap();
    let target_index = batch.ids.iter().position(|id| *id == target).unwrap();
    batch.batch.positions[source_index] = [400.0, 1_000.0];
    batch.batch.positions[target_index] = [400.0, 1_300.0];
    batch.batch.velocities[source_index] = [0.0; 2];
    batch.batch.velocities[target_index] = [0.0; 2];
    engine.apply_physics_batch(batch).unwrap();

    engine.tick().unwrap();
    assert_eq!(engine.memory_at(target, 50).unwrap(), 1_234);
    assert_eq!(engine.memory(source, "tiepres").unwrap(), 41);
    assert_eq!(engine.memory(source, "tielen").unwrap(), 72);
    assert_eq!(
        engine.memory(source, "trefxpos").unwrap(),
        engine.organism(target).unwrap().position[0].round() as i32
    );
    assert_eq!(
        engine.memory(source, "trefypos").unwrap(),
        engine.organism(target).unwrap().position[1].round() as i32
    );

    engine.tick().unwrap();
    let source_angular = engine
        .robot_state(source)
        .unwrap()
        .kinematics
        .angular_momentum;
    let target_angular = engine
        .robot_state(target)
        .unwrap()
        .kinematics
        .angular_momentum;
    assert_ne!(engine.organism(source).unwrap().aim, 0);
    assert_ne!(engine.organism(target).unwrap().aim, 0);
    assert!(source_angular * target_angular < 0.0);

    let source_endpoint = engine.robot_state(source).unwrap().ties[0].unwrap();
    let target_endpoint = engine.robot_state(target).unwrap().ties[0].unwrap();
    assert_eq!(source_endpoint.other, Some(target));
    assert_eq!(target_endpoint.other, Some(source));
    assert_eq!((source_endpoint.port, target_endpoint.port), (41, 1));
    assert_eq!(
        (source_endpoint.reciprocal_index, target_endpoint.reciprocal_index),
        (1, 1)
    );

    let mut baseline = Engine::new(tie_test_config()).unwrap();
    baseline
        .spawn_at(
            LegacyDna::parse("start\n-1 .shoot store\nstop").unwrap(),
            [400.0, 1_000.0],
        )
        .unwrap();
    baseline.tick().unwrap();
    let baseline_shot = baseline.snapshot().shots[0].clone();
    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n-1 .shoot store\nstop").unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    let tied_shot = engine.snapshot().shots.last().unwrap().clone();
    assert_eq!(tied_shot.value, baseline_shot.value * 2);
    assert!(tied_shot.range > baseline_shot.range);

    let source_radius = engine
        .snapshot()
        .render_instances
        .iter()
        .find(|instance| instance.slot == source.slot())
        .unwrap()
        .radius;
    let target_radius = engine
        .snapshot()
        .render_instances
        .iter()
        .find(|instance| instance.slot == target.slot())
        .unwrap()
        .radius;
    let mut break_batch = engine.physics_batch();
    let source_index = break_batch.ids.iter().position(|id| *id == source).unwrap();
    let target_index = break_batch.ids.iter().position(|id| *id == target).unwrap();
    break_batch.batch.positions[source_index] = [400.0, 1_000.0];
    break_batch.batch.positions[target_index] =
        [400.0 + source_radius + target_radius + 1_000.5, 1_000.0];
    break_batch.batch.velocities[source_index] = [0.0; 2];
    break_batch.batch.velocities[target_index] = [0.0; 2];
    engine.apply_physics_batch(break_batch).unwrap();
    engine.tick().unwrap();

    assert!(engine.snapshot().ties.is_empty());
    assert_eq!(
        (
            engine
                .robot_state(source)
                .unwrap()
                .ties
                .iter()
                .flatten()
                .count(),
            engine
                .robot_state(target)
                .unwrap()
                .ties
                .iter()
                .flatten()
                .count(),
        ),
        (0, 0)
    );
    assert_eq!(engine.memory(source, "numties").unwrap(), 0);
    assert_eq!(engine.memory(source, "myties").unwrap(), 0);
    assert_eq!(engine.memory(source, "multi").unwrap(), 0);
    assert_eq!(engine.memory(source, "tiepres").unwrap(), 0);
    assert_eq!(engine.memory(source, "trefxpos").unwrap(), 0);
    assert_eq!(engine.memory(source, "trefypos").unwrap(), 0);
}

fn soft_tie_response(stretch: f32) -> ([f32; 2], [f32; 2], [f32; 2], [f32; 2]) {
    let (mut engine, source, target) = spawn_soft_tie();
    let rest_length = engine.snapshot().ties[0].rest_length;
    let mut batch = engine.physics_batch();
    let source_index = batch.ids.iter().position(|id| *id == source).unwrap();
    let target_index = batch.ids.iter().position(|id| *id == target).unwrap();
    batch.batch.positions[source_index] = [400.0, 1_000.0];
    batch.batch.positions[target_index] = [400.0 + rest_length + stretch, 1_000.0];
    batch.batch.velocities[source_index] = [0.0; 2];
    batch.batch.velocities[target_index] = [0.0; 2];
    engine.apply_physics_batch(batch).unwrap();
    engine.tick().unwrap();
    let source = engine.organism(source).unwrap();
    let target = engine.organism(target).unwrap();
    (
        source.position,
        source.velocity,
        target.position,
        target.velocity,
    )
}

#[test]
fn soft_tie_uses_twenty_unit_deadband() {
    let (source_position, source_velocity, target_position, target_velocity) =
        soft_tie_response(20.0);
    assert_eq!(source_velocity, [0.0; 2]);
    assert_eq!(target_velocity, [0.0; 2]);
    assert_eq!(source_position, [400.0, 1_000.0]);
    assert_eq!(target_position, [720.0, 1_000.0]);

    let (source_position, source_velocity, target_position, target_velocity) =
        soft_tie_response(30.0);
    assert_close(source_velocity[0], 0.1);
    assert_close(target_velocity[0], -0.1);
    assert_close(source_position[0], 400.1);
    assert_close(target_position[0], 729.9);
    assert_close(source_position[1], 1_000.0);
    assert_close(target_position[1], 1_000.0);
}

#[test]
fn sharing_requires_hardening() {
    let mut engine = Engine::new(tie_test_config()).unwrap();
    let source = engine
        .spawn_at(
            LegacyDna::parse("start\n100 .mkshell store\n41 .tie store\nstop").unwrap(),
            [400.0, 1_000.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [700.0, 1_000.0])
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().ties.len(), 1);
    assert_eq!(engine.organism(source).unwrap().shell, 100);
    assert_eq!(engine.organism(target).unwrap().shell, 0);
    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n50 .shareshell store\nstop").unwrap(),
        )
        .unwrap();

    engine.tick().unwrap();
    assert_eq!(engine.snapshot().tick, 2);
    assert_eq!(engine.organism(source).unwrap().shell, 100);
    assert_eq!(engine.organism(target).unwrap().shell, 0);

    engine.tick_many(17).unwrap();
    assert_eq!(engine.snapshot().tick, 19);
    assert_eq!(engine.organism(source).unwrap().shell, 100);
    assert_eq!(engine.organism(target).unwrap().shell, 0);

    engine.tick().unwrap();
    assert_eq!(engine.snapshot().tick, 20);
    assert_eq!(engine.organism(source).unwrap().shell, 50);
    assert_eq!(engine.organism(target).unwrap().shell, 50);

    let share_chloroplasts =
        LegacyDna::parse("start\n50 .sharechlr store\nstop").unwrap();
    engine.replace_dna(source, share_chloroplasts).unwrap();
    let encoded = SaveFile::encode(&engine).unwrap();
    let mut payload: Value = serde_json::from_slice(&encoded[10..]).unwrap();
    let source_robot = &mut payload["slots"][source.slot() as usize]["organism"];
    source_robot["materials"]["chloroplasts"] = json!(1_000);
    source_robot["flags"]["can_use_chloroplasts"] = json!(false);
    source_robot["counters"]["chloroplast_share_delay"] = json!(0);
    set_tie_memory(&mut source_robot["program"]["memory"], 920, 1_000);
    let target_robot = &mut payload["slots"][target.slot() as usize]["organism"];
    target_robot["materials"]["chloroplasts"] = json!(3_000);
    target_robot["flags"]["can_use_chloroplasts"] = json!(false);
    set_tie_memory(&mut target_robot["program"]["memory"], 920, 3_000);
    let payload = serde_json::to_vec(&payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(&encoded[..6]);
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    let mut engine = SaveFile::decode(&save).unwrap();

    engine.tick().unwrap();
    assert_eq!(engine.robot_state(source).unwrap().materials.chloroplasts, 1_000);
    assert_eq!(engine.robot_state(target).unwrap().materials.chloroplasts, 3_000);

    let encoded = SaveFile::encode(&engine).unwrap();
    let mut payload: Value = serde_json::from_slice(&encoded[10..]).unwrap();
    payload["slots"][source.slot() as usize]["organism"]["flags"]
        ["can_use_chloroplasts"] = json!(true);
    let payload = serde_json::to_vec(&payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(&encoded[..6]);
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    let mut engine = SaveFile::decode(&save).unwrap();

    engine.tick().unwrap();
    assert_eq!(engine.robot_state(source).unwrap().materials.chloroplasts, 1_000);
    assert_eq!(engine.robot_state(target).unwrap().materials.chloroplasts, 3_000);
    assert_eq!(
        engine
            .robot_state(source)
            .unwrap()
            .counters
            .chloroplast_share_delay,
        7
    );
}

#[test]
fn hardened_shell_share_marks_only_the_forward_tie_endpoint() {
    let mut engine = Engine::new(tie_test_config()).unwrap();
    let source = engine
        .spawn_at(
            LegacyDna::parse("start\n100 .mkshell store\n41 .tie store\nstop").unwrap(),
            [400.0, 1_000.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [700.0, 1_000.0])
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n50 .shareshell store\nstop").unwrap(),
        )
        .unwrap();

    engine.tick_many(18).unwrap();
    assert_eq!(engine.snapshot().tick, 19);
    assert_eq!(engine.organism(source).unwrap().shell, 100);
    assert_eq!(engine.organism(target).unwrap().shell, 0);

    engine.tick().unwrap();
    assert_eq!(engine.snapshot().tick, 20);
    assert!(engine.snapshot().ties[0].hardened);
    assert_eq!(engine.organism(source).unwrap().shell, 50);
    assert_eq!(engine.organism(target).unwrap().shell, 50);
    assert_eq!(engine.memory(source, "shareshell").unwrap(), 0);
    assert_eq!(
        engine.robot_state(source).unwrap().ties[0].unwrap().sharing_marker,
        1
    );
    assert_eq!(
        engine.robot_state(target).unwrap().ties[0].unwrap().sharing_marker,
        0
    );

    engine
        .replace_dna(source, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine
        .replace_dna(
            target,
            LegacyDna::parse("start\n50 .shareshell store\nstop").unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.organism(source).unwrap().shell, 75);
    assert_eq!(engine.organism(target).unwrap().shell, 25);
    assert_eq!(engine.memory(target, "shareshell").unwrap(), 0);
    assert_eq!(
        engine.robot_state(source).unwrap().ties[0].unwrap().sharing_marker,
        0
    );
    assert_eq!(
        engine.robot_state(target).unwrap().ties[0].unwrap().sharing_marker,
        1
    );
}

#[test]
fn tie_breaks_when_surface_gap_exceeds_one_thousand() {
    let (mut engine, source, target) = spawn_soft_tie();
    let source_radius = engine
        .snapshot()
        .render_instances
        .iter()
        .find(|instance| instance.slot == source.slot())
        .unwrap()
        .radius;
    let target_radius = engine
        .snapshot()
        .render_instances
        .iter()
        .find(|instance| instance.slot == target.slot())
        .unwrap()
        .radius;
    let mut batch = engine.physics_batch();
    let source_index = batch.ids.iter().position(|id| *id == source).unwrap();
    let target_index = batch.ids.iter().position(|id| *id == target).unwrap();
    batch.batch.positions[source_index] = [400.0, 1_000.0];
    batch.batch.positions[target_index] =
        [400.0 + source_radius + target_radius + 1_000.5, 1_000.0];
    batch.batch.velocities[source_index] = [0.0; 2];
    batch.batch.velocities[target_index] = [0.0; 2];
    engine.apply_physics_batch(batch).unwrap();

    engine.tick().unwrap();

    assert!(engine.snapshot().ties.is_empty());
    assert_eq!(engine.memory(source, "numties").unwrap(), 0);
    assert_eq!(engine.memory(target, "numties").unwrap(), 0);
    assert_eq!(engine.memory(source, "multi").unwrap(), 0);
    assert_eq!(engine.memory(target, "multi").unwrap(), 0);
}

#[test]
fn eleventh_endpoint_tie_is_rejected_at_ten_tie_capacity() {
    let mut engine = Engine::new(tie_test_config()).unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [1_000.0, 1_000.0])
        .unwrap();
    let mut leaves = Vec::new();

    for ordinal in 0..11 {
        let leaf = engine
            .spawn_at(
                LegacyDna::parse("start\n41 .tie store\nstop").unwrap(),
                [700.0, 1_000.0],
            )
            .unwrap();
        let mut batch = engine.physics_batch();
        for (index, id) in batch.ids.iter().copied().enumerate() {
            batch.batch.velocities[index] = [0.0; 2];
            batch.batch.positions[index] = if id == target {
                [1_000.0, 1_000.0]
            } else if id == leaf {
                [700.0, 1_000.0]
            } else {
                let leaf_index = leaves.iter().position(|existing| *existing == id).unwrap();
                [1_800.0, 900.0 + leaf_index as f32 * 20.0]
            };
        }
        engine.apply_physics_batch(batch).unwrap();
        engine.tick().unwrap();
        engine
            .replace_dna(leaf, LegacyDna::parse("start\nstop").unwrap())
            .unwrap();
        leaves.push(leaf);
        assert_eq!(engine.snapshot().ties.len(), (ordinal + 1).min(10));
    }

    let rejected = leaves[10];
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().ties.len(), 10);
    assert!(
        engine
            .snapshot()
            .ties
            .iter()
            .all(|tie| tie.first != rejected && tie.second != rejected)
    );
    assert_eq!(engine.memory(target, "numties").unwrap(), 10);
    assert_eq!(engine.memory(rejected, "numties").unwrap(), 0);
    assert_eq!(engine.memory(rejected, "tiepres").unwrap(), 0);
    let reciprocal_ports: Vec<_> = engine
        .snapshot()
        .ties
        .iter()
        .map(|tie| tie.second_port)
        .collect();
    assert_eq!(reciprocal_ports, (1..=10).collect::<Vec<_>>());
}

#[test]
fn tie_formation_skips_existing_pair_for_next_eligible_target() {
    let mut engine = Engine::new(tie_test_config()).unwrap();
    let nearest_target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [900.0, 1_000.0])
        .unwrap();
    let eligible_target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [1_500.0, 1_000.0])
        .unwrap();
    let attacker = engine
        .spawn_at(
            LegacyDna::parse("start\n41 .tie store\nstop").unwrap(),
            [700.0, 1_000.0],
        )
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(attacker, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    assert_eq!(engine.snapshot().ties.len(), 1);

    engine
        .replace_dna(
            attacker,
            LegacyDna::parse("start\n42 .tie store\nstop").unwrap(),
        )
        .unwrap();
    let mut batch = engine.physics_batch();
    for (index, id) in batch.ids.iter().copied().enumerate() {
        batch.batch.velocities[index] = [0.0; 2];
        batch.batch.positions[index] = if id == nearest_target {
            [900.0, 1_000.0]
        } else if id == eligible_target {
            [1_500.0, 1_000.0]
        } else {
            assert_eq!(id, attacker);
            [600.0, 1_000.0]
        };
    }
    engine.apply_physics_batch(batch).unwrap();
    engine.tick().unwrap();

    let snapshot = engine.snapshot();
    assert_eq!(snapshot.ties.len(), 2);
    let created = snapshot
        .ties
        .iter()
        .find(|tie| tie.first == attacker && tie.second == eligible_target)
        .expect("the second attempt should use the next eligible target");
    assert_eq!(created.first_port, 42);
    assert_eq!(created.second_port, 1);
}

#[test]
fn slime_is_consumed_to_resist_new_ties() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let source = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [100.0, 100.0])
        .unwrap();
    let target = engine
        .spawn_at(
            LegacyDna::parse("start\n50 .mkslime store\nstop").unwrap(),
            [120.0, 100.0],
        )
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n1 .tie store\nstop").unwrap(),
        )
        .unwrap();
    engine
        .replace_dna(target, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();

    engine.tick().unwrap();

    assert!(engine.snapshot().ties.is_empty());
    assert!(engine.organism(target).unwrap().slime < 50);
}

#[test]
fn successful_tie_attack_consumes_twenty_slime() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let source = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [100.0, 100.0])
        .unwrap();
    let target = engine
        .spawn_at(
            LegacyDna::parse("start\n10 .mkslime store\nstop").unwrap(),
            [120.0, 100.0],
        )
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n1 .tie store\nstop").unwrap(),
        )
        .unwrap();
    engine
        .replace_dna(target, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.snapshot().ties.len(), 1);
    assert_eq!(engine.organism(target).unwrap().slime, 0);
}

#[test]
fn dragging_one_member_moves_the_tied_organism_and_clears_momentum() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let source = engine
        .spawn_at(
            LegacyDna::parse("start\n1 .tie store\n25 .up store\nstop").unwrap(),
            [250.0, 300.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(
            LegacyDna::parse("start\n25 .up store\nstop").unwrap(),
            [550.0, 300.0],
        )
        .unwrap();
    engine.tick().unwrap();
    let snapshot_before = engine.snapshot();
    assert_eq!(snapshot_before.ties.len(), 1);
    assert_eq!(snapshot_before.ties[0].first, source);
    assert_eq!(snapshot_before.ties[0].second, target);
    let source_before = engine.organism(source).unwrap();
    let target_before = engine.organism(target).unwrap();
    let destination = [400.0, 450.0];
    let source_radius = snapshot_before
        .render_instances
        .iter()
        .find(|instance| instance.slot == source.slot())
        .unwrap()
        .radius;
    let target_radius = snapshot_before
        .render_instances
        .iter()
        .find(|instance| instance.slot == target.slot())
        .unwrap()
        .radius;
    let offset_before = [
        target_before.position[0] - source_before.position[0],
        target_before.position[1] - source_before.position[1],
    ];
    let separation_squared =
        offset_before[0] * offset_before[0] + offset_before[1] * offset_before[1];
    let minimum_separation = source_radius + target_radius;
    assert!(separation_squared > minimum_separation * minimum_separation);
    assert_ne!(source_before.velocity, [0.0, 0.0]);
    assert_ne!(target_before.velocity, [0.0, 0.0]);

    engine.move_organism(source, destination).unwrap();

    let source_after = engine.organism(source).unwrap();
    let target_after = engine.organism(target).unwrap();
    let source_translation = [
        source_after.position[0] - source_before.position[0],
        source_after.position[1] - source_before.position[1],
    ];
    let target_translation = [
        target_after.position[0] - target_before.position[0],
        target_after.position[1] - target_before.position[1],
    ];
    let offset_after = [
        target_after.position[0] - source_after.position[0],
        target_after.position[1] - source_after.position[1],
    ];

    // Mutation guard: removing connected-graph translation or either momentum reset
    // from Engine::move_organism must fail this test.
    assert_eq!(source_after.position, destination);
    assert_eq!(target_translation, source_translation);
    assert_eq!(offset_after, offset_before);
    assert_eq!(source_after.velocity, [0.0, 0.0]);
    assert_eq!(target_after.velocity, [0.0, 0.0]);
}
