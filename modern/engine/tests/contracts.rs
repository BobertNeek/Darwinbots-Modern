use darwinbots_engine::{
    BackendKind, BackendPreference, Engine, EngineConfig, EngineError, LegacyDna, SaveFile,
    SpeciesDefinition, TokenClass,
};

const SIMPLE_BOT: &str = r#"
' Legacy comments remain legal.
def target 50
cond
  *.robage 0 =
start
  10 .up store
stop
"#;

#[test]
fn imports_legacy_dna_and_preserves_user_variables() {
    let program = LegacyDna::parse(SIMPLE_BOT).expect("legacy DNA should parse");

    assert_eq!(program.user_variable("target"), Some(50));
    assert!(program.blocks().len() >= 8);
}

#[test]
fn parser_compiles_symbolic_memory_operands_to_numeric_addresses() {
    let program = LegacyDna::parse("def custom 777\nstart\n*.custom .up store\nstop").unwrap();
    assert!(
        program
            .blocks()
            .iter()
            .any(|block| block.class == TokenClass::Dereference && block.value == 777)
    );
    assert!(
        program
            .blocks()
            .iter()
            .any(|block| block.class == TokenClass::Number && block.value == 1)
    );
}

#[test]
fn unknown_legacy_instruction_becomes_zero_with_location_warning() {
    let program = LegacyDna::parse("start\nnot-a-command\nstop").unwrap();

    assert!(
        program
            .blocks()
            .iter()
            .any(|block| block.class == TokenClass::Number && block.value == 0)
    );
    assert!(program.compatibility_warnings()[0].contains("line 2"));
    assert!(program.compatibility_warnings()[0].contains("not-a-command"));
}

#[test]
fn batch_spawn_publishes_one_complete_population_snapshot() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let dna = LegacyDna::parse("start\nstop").unwrap();
    let ids = engine
        .spawn_batch((0..16).map(|index| (dna.clone(), [index as f32, 5.0])))
        .unwrap();

    assert_eq!(ids.len(), 16);
    assert_eq!(engine.population(), 16);
    assert_eq!(engine.snapshot().organisms.len(), 16);
    assert_eq!(engine.snapshot().render_instances.len(), 16);
    assert_eq!(engine.snapshot().render_instances[7].slot, 7);
    assert_eq!(engine.snapshot().render_instances[7].position, [7.0, 5.0]);
}

#[test]
fn seeded_cpu_ticks_are_reproducible_and_publish_snapshots() {
    let config = EngineConfig {
        seed: 42,
        ..EngineConfig::testing()
    };
    let mut left = Engine::new(config.clone()).unwrap();
    let mut right = Engine::new(config).unwrap();
    let dna = LegacyDna::parse("start\n10 .up store\nstop").unwrap();
    left.spawn(dna.clone()).unwrap();
    right.spawn(dna).unwrap();

    for _ in 0..20 {
        left.tick().unwrap();
        right.tick().unwrap();
    }

    assert_eq!(left.snapshot(), right.snapshot());
    assert_eq!(left.snapshot().tick, 20);
    assert_eq!(left.snapshot().organisms.len(), 1);
}

#[test]
fn tick_many_advances_a_batch_and_publishes_the_final_state() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();

    engine.tick_many(25).unwrap();

    assert_eq!(engine.snapshot().tick, 25);
    assert_eq!(engine.snapshot().organisms[0].age, 25);
}

#[test]
fn invariant_checker_accepts_slot_reuse_and_seeded_ticks() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let first = engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    engine.remove(first).unwrap();
    engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    engine.validate_invariants().unwrap();
}

#[test]
fn engine_uses_vm_flow_semantics_for_movement_intents() {
    let mut engine = Engine::new(EngineConfig {
        physics: darwinbots_engine::PhysicsSettings {
            density: 0.0,
            surface_gravity: 0.0,
            ..darwinbots_engine::PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let dna =
        LegacyDna::parse("cond\n2 1 >\nstart\n10 .up store\nelse\n25 .up store\nstop").unwrap();
    let id = engine.spawn(dna).unwrap();

    engine.tick().unwrap();

    assert!((engine.organism(id).unwrap().position[1] - 6.6).abs() < 0.01);
}

#[test]
fn save_files_are_versioned_and_round_trip_reference_state() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();

    let bytes = SaveFile::encode(&engine).unwrap();
    let restored = SaveFile::decode(&bytes).unwrap();

    assert_eq!(&bytes[..4], b"DB3S");
    assert_eq!(restored.snapshot(), engine.snapshot());
}

#[test]
fn snapshot_v2_exposes_phase1_telemetry() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine
        .spawn(LegacyDna::parse("cond\n1 0 >\nstart\nstop").unwrap())
        .unwrap();

    engine.tick().unwrap();

    let snapshot = engine.snapshot();
    let organism = &snapshot.organisms[0];
    assert_eq!(snapshot.schema_version, 2);
    assert_eq!(organism.current_gene, 1);
    assert_eq!(organism.condition_count, 1);
    assert_eq!(organism.generation, 0);
    assert_eq!(organism.birth_cycle, 0);
}

#[test]
fn save_round_trip_preserves_pre_refactor_robot_state_fields() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let mut species = SpeciesDefinition::default();
    species.name = "save-contract".to_owned();
    species.vegetable = true;
    let species = engine.register_species(species);
    let id = engine
        .spawn_species_at(
            LegacyDna::parse("start\n10 .up store\nstop").unwrap(),
            species,
            [125.0, 250.0],
        )
        .unwrap();
    engine.tick().unwrap();
    let before = engine.robot_state(id).unwrap().clone();

    let restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
    let after = restored.robot_state(id).unwrap();

    assert_eq!(after.identity.species, before.identity.species);
    assert_eq!(after.identity.parents, before.identity.parents);
    assert_eq!(after.flags, before.flags);
    assert_eq!(after.kinematics, before.kinematics);
    assert_eq!(after.materials, before.materials);
    assert_eq!(after.counters.age, before.counters.age);
    assert_eq!(after.program.dna, before.program.dna);
    assert_eq!(after.program.memory, before.program.memory);
    assert_eq!(after.program.rng_state, before.program.rng_state);
    assert_eq!(
        after.effects.forced_reproduction,
        before.effects.forced_reproduction
    );
    assert_eq!(after.phenotype, before.phenotype);
}

#[test]
fn db3s_v2_migrates_every_existing_robot_field() {
    let fixture = include_bytes!("fixtures/db3s_v2_phase1.bin");
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let mut species = SpeciesDefinition::default();
    species.name = "db3s-v2-fixture".to_owned();
    species.vegetable = true;
    let species = engine.register_species(species);
    let id = engine
        .spawn_species_at(
            LegacyDna::parse("start\n17 .up store\nstop").unwrap(),
            species,
            [321.0, 654.0],
        )
        .unwrap();
    let before = engine.robot_state(id).unwrap().clone();

    let restored = SaveFile::decode(fixture).unwrap();
    let after = restored.robot_state(id).unwrap();
    assert_eq!(after.identity.species, before.identity.species);
    assert_eq!(after.identity.parents, before.identity.parents);
    assert_eq!(after.flags, before.flags);
    assert_eq!(after.kinematics, before.kinematics);
    assert_eq!(after.materials, before.materials);
    assert_eq!(after.counters.age, before.counters.age);
    assert_eq!(after.program.dna, before.program.dna);
    assert_eq!(after.program.memory, before.program.memory);
    assert_eq!(after.program.rng_state, before.program.rng_state);
    assert_eq!(
        after.effects.forced_reproduction,
        before.effects.forced_reproduction
    );
    assert_eq!(after.phenotype, before.phenotype);
}

#[test]
fn db3s_v3_round_trip_preserves_complete_phase1_state() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let mut species = SpeciesDefinition::default();
    species.name = "db3s-v3-phase1".to_owned();
    species.vegetable = true;
    let species = engine.register_species(species);
    let id = engine
        .spawn_species_at(
            LegacyDna::parse("cond\n1 0 >\nstart\n17 .up store\nstop").unwrap(),
            species,
            [321.0, 654.0],
        )
        .unwrap();
    engine.tick().unwrap();
    let before = engine.robot_state(id).unwrap().clone();

    let bytes = SaveFile::encode(&engine).unwrap();
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 3);
    assert!(!String::from_utf8_lossy(&bytes).contains("activation_bitmap"));
    assert!(!String::from_utf8_lossy(&bytes).contains("debug_text"));

    let restored = SaveFile::decode(&bytes).unwrap();
    let after = restored.robot_state(id).unwrap();
    assert_eq!(after.identity, before.identity);
    assert_eq!(after.flags, before.flags);
    assert_eq!(after.kinematics, before.kinematics);
    assert_eq!(after.materials, before.materials);
    assert_eq!(after.counters, before.counters);
    assert_eq!(after.program.dna, before.program.dna);
    assert_eq!(after.program.memory, before.program.memory);
    assert_eq!(
        after.program.private_variables,
        before.program.private_variables
    );
    assert_eq!(after.program.rng_state, before.program.rng_state);
    assert_eq!(
        after.program.mutation_metadata,
        before.program.mutation_metadata
    );
    assert_eq!(
        after.program.fractional_cost_remainder,
        before.program.fractional_cost_remainder
    );
    assert_eq!(after.program.current_gene, before.program.current_gene);
    assert_eq!(
        after.program.condition_count,
        before.program.condition_count
    );
    assert!(after.program.activation_bitmap.is_empty());
    assert!(after.program.debug_text.is_empty());
    assert_eq!(after.senses, before.senses);
    assert_eq!(after.effects, before.effects);
    assert_eq!(after.heredity, before.heredity);
    assert_eq!(after.ties, before.ties);
    assert_eq!(after.phenotype, before.phenotype);
}

#[test]
fn mutation_free_save_disables_saved_mutations_without_changing_live_engine() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine
        .update_mutation_settings(true, 2.5, &[(darwinbots_engine::SpeciesId(0), 3.5)])
        .unwrap();

    let bytes = SaveFile::encode_without_mutations(&engine).unwrap();
    let restored = SaveFile::decode(&bytes).unwrap();

    assert!(!restored.snapshot().mutations_enabled);
    assert_eq!(restored.snapshot().global_mutation_multiplier, 0.0);
    assert!(
        restored
            .snapshot()
            .species
            .iter()
            .all(|species| species.mutation_rate == 0.0)
    );
    assert!(engine.snapshot().mutations_enabled);
    assert_eq!(engine.snapshot().global_mutation_multiplier, 2.5);
    assert_eq!(engine.snapshot().species[0].mutation_rate, 3.5);
}

#[test]
fn unavailable_gpu_falls_back_to_cpu_when_allowed() {
    let config = EngineConfig {
        backend: BackendPreference::Gpu,
        allow_cpu_fallback: true,
        force_gpu_unavailable_for_tests: true,
        ..EngineConfig::testing()
    };

    let engine = Engine::new(config).unwrap();

    assert_eq!(engine.backend(), BackendKind::Cpu);
    assert!(engine.capabilities().fallback_reason.is_some());
}

#[test]
fn unavailable_gpu_is_an_error_when_fallback_is_disabled() {
    let config = EngineConfig {
        backend: BackendPreference::Gpu,
        allow_cpu_fallback: false,
        force_gpu_unavailable_for_tests: true,
        ..EngineConfig::testing()
    };

    assert!(matches!(
        Engine::new(config),
        Err(EngineError::GpuUnavailable(_))
    ));
}
#[test]
fn db2_defaults_are_exposed_by_engine_config() {
    let config = EngineConfig::default();
    assert_eq!(config.world_width, 16_000.0);
    assert_eq!(config.world_height, 12_000.0);
    assert_eq!(config.brownian_motion, 0.0);
    assert_eq!(config.physics.max_velocity, 180.0);
    assert_eq!(config.physics.movement_efficiency, 0.66);
    assert_eq!(config.physics.surface_gravity, 2.0);
    assert_eq!(config.physics.static_friction, 0.6);
    assert_eq!(config.physics.kinetic_friction, 0.4);
    assert_eq!(config.physics.density, 0.0);
    assert_eq!(config.physics.viscosity, 0.0);
    assert_eq!(config.physics.elasticity, 0.0);
    assert_eq!(config.shots.speed, 40.0);
    assert_eq!(config.shots.range_multiplier, 1.0);
    assert_eq!(config.shots.decay, 40.0);
    assert!(!config.shots.energy_shots_do_not_decay);
    assert!(!config.shots.waste_shots_do_not_decay);
    assert_eq!(config.vegetation.start_chloroplasts, 16_000);
    assert_eq!(config.vegetation.max_energy_per_tick, 40);
    assert_eq!(config.vegetation.minimum_chloroplast_equivalents, 10);
    assert_eq!(config.vegetation.repopulation_amount, 10);
    assert_eq!(config.vegetation.repopulation_cooldown, 25);
    assert_eq!(config.vegetation.feeding_to_body, 0.5);
    assert!(config.vegetation.daytime);
    assert!(!config.vegetation.day_night_enabled);
}

#[test]
fn save_version_one_is_rejected_after_db2_state_upgrade() {
    let engine = Engine::new(EngineConfig::testing()).unwrap();
    let mut bytes = SaveFile::encode(&engine).unwrap();
    bytes[4..6].copy_from_slice(&1u16.to_le_bytes());

    let error = match SaveFile::decode(&bytes) {
        Ok(_) => panic!("save version 1 was accepted"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("unsupported version 1"));
}
