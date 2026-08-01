use darwinbots_engine::{Engine, EngineConfig, LegacyDna, OrganismId, SaveFile};

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
