use darwinbots_engine::{
    BackendKind, BackendPreference, Engine, EngineConfig, EngineError, LegacyDna, SaveFile,
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
    assert!(program.instructions().len() >= 8);
}

#[test]
fn parser_compiles_symbolic_memory_operands_to_numeric_addresses() {
    let program = LegacyDna::parse("def custom 777\nstart\n*.custom .up store\nstop").unwrap();
    assert!(program.instructions().iter().any(|instruction| {
        matches!(instruction, darwinbots_engine::Instruction::ReadResolved(777))
    }));
    assert!(program.instructions().iter().any(|instruction| {
        matches!(instruction, darwinbots_engine::Instruction::AddressResolved(1))
    }));
    assert!(!program.instructions().iter().any(|instruction| {
        matches!(
            instruction,
            darwinbots_engine::Instruction::Read(_) | darwinbots_engine::Instruction::Address(_)
        )
    }));
}

#[test]
fn rejects_unknown_legacy_instruction_with_location() {
    let error = LegacyDna::parse("start\nnot-a-command\nstop").unwrap_err();

    assert!(error.to_string().contains("line 2"));
    assert!(error.to_string().contains("not-a-command"));
}

#[test]
fn stable_ids_change_generation_when_slots_are_reused() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let first = engine.spawn(LegacyDna::parse("start\nstop").unwrap()).unwrap();

    engine.remove(first).unwrap();
    let second = engine.spawn(LegacyDna::parse("start\nstop").unwrap()).unwrap();

    assert_eq!(first.slot(), second.slot());
    assert_ne!(first.generation(), second.generation());
    assert!(matches!(engine.organism(first), Err(EngineError::StaleOrganismId)));
}

#[test]
fn batch_spawn_publishes_one_complete_population_snapshot() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let dna = LegacyDna::parse("start\nstop").unwrap();
    let ids = engine.spawn_batch((0..16).map(|index| (dna.clone(), [index as f32, 5.0]))).unwrap();

    assert_eq!(ids.len(), 16);
    assert_eq!(engine.population(), 16);
    assert_eq!(engine.snapshot().organisms.len(), 16);
    assert_eq!(engine.snapshot().render_instances.len(), 16);
    assert_eq!(engine.snapshot().render_instances[7].slot, 7);
    assert_eq!(engine.snapshot().render_instances[7].position, [7.0, 5.0]);
}

#[test]
fn seeded_cpu_ticks_are_reproducible_and_publish_snapshots() {
    let config = EngineConfig { seed: 42, ..EngineConfig::testing() };
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
    engine.spawn(LegacyDna::parse("start\nstop").unwrap()).unwrap();

    engine.tick_many(25).unwrap();

    assert_eq!(engine.snapshot().tick, 25);
    assert_eq!(engine.snapshot().organisms[0].age, 25);
}

#[test]
fn invariant_checker_accepts_slot_reuse_and_seeded_ticks() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let first = engine.spawn(LegacyDna::parse("start\nstop").unwrap()).unwrap();
    engine.tick().unwrap();
    engine.remove(first).unwrap();
    engine.spawn(LegacyDna::parse("start\nstop").unwrap()).unwrap();
    engine.tick().unwrap();
    engine.validate_invariants().unwrap();
}

#[test]
fn engine_uses_vm_flow_semantics_for_movement_intents() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let dna = LegacyDna::parse(
        "cond\n2 1 >\nstart\n10 .up store\nelse\n25 .up store\nstop",
    ).unwrap();
    let id = engine.spawn(dna).unwrap();

    engine.tick().unwrap();

    assert!((engine.organism(id).unwrap().position[1] - 6.6).abs() < 0.01);
}

#[test]
fn save_files_are_versioned_and_round_trip_reference_state() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine.spawn(LegacyDna::parse("start\nstop").unwrap()).unwrap();
    engine.tick().unwrap();

    let bytes = SaveFile::encode(&engine).unwrap();
    let restored = SaveFile::decode(&bytes).unwrap();

    assert_eq!(&bytes[..4], b"DB3S");
    assert_eq!(restored.snapshot(), engine.snapshot());
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

    assert!(matches!(Engine::new(config), Err(EngineError::GpuUnavailable(_))));
}
#[test]
fn db2_defaults_are_exposed_by_engine_config() {
    let config = EngineConfig::default();
    assert_eq!(config.physics.max_velocity, 60.0);
    assert_eq!(config.physics.movement_efficiency, 0.66);
    assert_eq!(config.shots.speed, 40.0);
    assert_eq!(config.vegetation.start_chloroplasts, 16_000);
    assert_eq!(config.vegetation.repopulation_amount, 10);
    assert_eq!(config.vegetation.repopulation_cooldown, 10);
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
