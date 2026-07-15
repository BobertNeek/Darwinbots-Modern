use darwinbots_engine::{
    ffi::{
        db_api_version, db_buffer_free, db_engine_backend, db_engine_capabilities_json, db_engine_command_batch,
        db_engine_create, db_engine_destroy, db_engine_import_dna, db_engine_load, db_engine_save,
        db_engine_snapshot_json, db_engine_tick,
        db_last_error_json, DbBuffer, DbStatus, ABI_VERSION,
    },
    BackendKind, BackendPreference, CpuPhysicsBackend, Engine, EngineConfig, GpuPhysicsBackend,
    LegacyDna, PhysicsBackend, PhysicsBatch,
};

#[test]
fn cpu_physics_integrates_and_clamps_world_bounds() {
    let mut batch = PhysicsBatch {
        positions: vec![[4.0, 8.0], [99.0, 1.0]],
        velocities: vec![[2.0, -3.0], [5.0, -5.0]],
        world_size: [100.0, 100.0],
        toroidal: false,
    };

    CpuPhysicsBackend.step(&mut batch).unwrap();

    assert_eq!(batch.positions, vec![[6.0, 5.0], [100.0, 0.0]]);
}

#[test]
fn gpu_physics_matches_cpu_when_an_adapter_is_available() {
    let Ok(mut gpu) = GpuPhysicsBackend::new() else {
        return;
    };
    let source = PhysicsBatch {
        positions: (0..257).map(|index| [index as f32, (index % 31) as f32]).collect(),
        velocities: (0..257).map(|index| [1.25, -(index as f32) * 0.01]).collect(),
        world_size: [256.0, 256.0],
        toroidal: false,
    };
    let mut expected = source.clone();
    let mut actual = source;

    CpuPhysicsBackend.step(&mut expected).unwrap();
    gpu.step(&mut actual).unwrap();

    for (cpu, gpu) in expected.positions.iter().zip(actual.positions.iter()) {
        assert!((cpu[0] - gpu[0]).abs() <= 0.0001);
        assert!((cpu[1] - gpu[1]).abs() <= 0.0001);
    }
}

#[test]
fn gpu_sensing_matches_nearest_neighbor_contract_when_adapter_is_available() {
    let Ok(gpu) = GpuPhysicsBackend::new() else { return };
    let targets = gpu.sense_nearest(
        &[[0.0, 0.0], [10.0, 0.0], [100.0, 0.0]],
        &[true, true, true],
        &[0, 2, 2, 3],
        &[0, 1, 2],
        [3, 1],
        50.0,
        100.0,
    ).unwrap();
    assert_eq!(targets, vec![Some(1), Some(0), Some(1)]);
}

#[test]
fn gpu_builds_spatial_grid_before_neighborhood_queries_when_adapter_is_available() {
    let Ok(gpu) = GpuPhysicsBackend::new() else { return };
    let targets = gpu.sense_nearest_gpu_grid(
        &[[5.0, 5.0], [20.0, 5.0], [190.0, 190.0], [175.0, 190.0]],
        &[true, true, true, true],
        [200.0, 200.0],
        32.0,
        80.0,
    ).unwrap();

    assert_eq!(targets, vec![Some(1), Some(0), Some(3), Some(2)]);
}

#[test]
fn fused_gpu_sensing_and_motion_match_cpu_contract_when_adapter_is_available() {
    let Ok(gpu) = GpuPhysicsBackend::new() else { return };
    let (targets, positions) = gpu.sense_and_integrate(
        &[[0.0, 0.0], [10.0, 0.0], [100.0, 0.0]],
        &[[2.0, 3.0], [-20.0, 1.0], [5.0, -200.0]],
        &[true, true, true],
        &[0, 2, 2, 3],
        &[0, 1, 2],
        [3, 1],
        50.0,
        100.0,
        [100.0, 100.0],
    ).unwrap();
    assert_eq!(targets, vec![Some(1), Some(0), Some(1)]);
    assert_eq!(positions, vec![[2.0, 3.0], [0.0, 1.0], [100.0, 0.0]]);
}

#[test]
fn gpu_prepares_render_instances_with_integrated_positions() {
    let Ok(gpu) = GpuPhysicsBackend::new() else { return };
    let (_, positions, instances) = gpu.sense_integrate_render(
        &[[0.0, 0.0], [10.0, 0.0]],
        &[[2.0, 3.0], [1.0, 1.0]],
        &[400, 100],
        &[true, true],
        &[0, 2],
        &[0, 1],
        [1, 1],
        100.0,
        100.0,
        [100.0, 100.0],
    ).unwrap();
    assert_eq!(instances.len(), 2);
    assert_eq!(instances[0].slot, 0);
    assert_eq!(instances[0].position, positions[0]);
    assert!((instances[0].radius - 9.0).abs() < 0.001);
    assert_ne!(instances[0].color, instances[1].color);
}

#[test]
fn gpu_sensing_and_rendering_do_not_reintegrate_db2_positions_when_adapter_is_available() {
    let Ok(gpu) = GpuPhysicsBackend::new() else { return };
    let positions = [[20.0, 30.0], [70.0, 30.0]];
    let (targets, instances) = gpu.sense_and_render_gpu_grid(
        &positions,
        &[400, 100],
        &[114.25, 57.5],
        &[true, true],
        [200.0, 200.0],
        64.0,
        100.0,
    ).unwrap();

    assert_eq!(targets, vec![Some(1), Some(0)]);
    assert_eq!(instances.len(), 2);
    assert_eq!(instances[0].position, positions[0]);
    assert_eq!(instances[1].position, positions[1]);
    assert!((instances[0].radius - 114.25).abs() < 0.001);
    assert!((instances[1].radius - 57.5).abs() < 0.001);
}

#[test]
fn gpu_position_integration_matches_cpu_after_db2_forces_when_adapter_is_available() {
    if GpuPhysicsBackend::new().is_err() {
        return;
    }
    let config = EngineConfig {
        gravity: [0.01, 0.02],
        drag: 0.001,
        brownian_motion: 0.0,
        world_width: 10_000.0,
        world_height: 10_000.0,
        ..EngineConfig::testing()
    };
    let mut cpu = Engine::new(EngineConfig {
        backend: BackendPreference::Cpu,
        ..config.clone()
    }).unwrap();
    let mut gpu = Engine::new(EngineConfig {
        backend: BackendPreference::Gpu,
        allow_cpu_fallback: false,
        ..config
    }).unwrap();
    let dna = LegacyDna::parse(
        "cond\n*.robage 0 =\nstart\n10 .up store\nstop",
    ).unwrap();
    cpu.spawn_at(dna.clone(), [5_000.0, 5_000.0]).unwrap();
    gpu.spawn_at(dna, [5_000.0, 5_000.0]).unwrap();

    cpu.tick_many(200).unwrap();
    gpu.tick_many(200).unwrap();

    let left = &cpu.snapshot().organisms[0];
    let right = &gpu.snapshot().organisms[0];
    assert!((left.position[0] - right.position[0]).abs() < 0.05);
    assert!((left.position[1] - right.position[1]).abs() < 0.05);
    assert!((left.velocity[0] - right.velocity[0]).abs() < 0.05);
    assert!((left.velocity[1] - right.velocity[1]).abs() < 0.05);
    assert!(right.velocity[0].abs() > 0.01 || right.velocity[1].abs() > 0.01);
}

#[test]
fn engine_retains_gpu_backend_and_uses_it_for_tick_physics() {
    if GpuPhysicsBackend::new().is_err() {
        return;
    }
    let mut gpu = Engine::new(EngineConfig {
        backend: BackendPreference::Gpu,
        allow_cpu_fallback: false,
        ..EngineConfig::testing()
    }).unwrap();
    let mut cpu = Engine::new(EngineConfig::testing()).unwrap();
    let dna = LegacyDna::parse("start\n10 .up store\nstop").unwrap();
    let gpu_id = gpu.spawn(dna.clone()).unwrap();
    let cpu_id = cpu.spawn(dna).unwrap();

    gpu.tick().unwrap();
    cpu.tick().unwrap();

    assert_eq!(gpu.backend(), BackendKind::Gpu);
    let gpu_position = gpu.organism(gpu_id).unwrap().position;
    let cpu_position = cpu.organism(cpu_id).unwrap().position;
    assert!((gpu_position[0] - cpu_position[0]).abs() < 0.0001);
    assert!((gpu_position[1] - cpu_position[1]).abs() < 0.0001);
}

#[test]
fn runtime_gpu_failure_retries_on_cpu_and_records_reason() {
    if GpuPhysicsBackend::new().is_err() { return; }
    let mut engine = Engine::new(EngineConfig {
        backend: BackendPreference::Gpu,
        allow_cpu_fallback: true,
        force_gpu_runtime_failure_for_tests: true,
        ..EngineConfig::testing()
    }).unwrap();
    engine.spawn(LegacyDna::parse("start\nstop").unwrap()).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.backend(), BackendKind::Cpu);
    assert!(!engine.capabilities().gpu_available);
    assert!(engine.capabilities().fallback_reason.as_deref().unwrap().contains("forced runtime failure"));
}

#[test]
fn runtime_gpu_failure_is_fatal_when_fallback_is_disabled() {
    if GpuPhysicsBackend::new().is_err() { return; }
    let mut engine = Engine::new(EngineConfig {
        backend: BackendPreference::Gpu,
        allow_cpu_fallback: false,
        force_gpu_runtime_failure_for_tests: true,
        ..EngineConfig::testing()
    }).unwrap();
    engine.spawn(LegacyDna::parse("start\nstop").unwrap()).unwrap();

    assert!(matches!(engine.tick(), Err(darwinbots_engine::EngineError::Gpu(_))));
    assert_eq!(engine.backend(), BackendKind::Gpu);
}

#[test]
fn engine_can_switch_backends_while_preserving_live_state() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let id = engine.spawn_at(LegacyDna::parse("start\n10 .up store\nstop").unwrap(), [100.0, 100.0]).unwrap();
    engine.tick().unwrap();
    let before = engine.organism(id).unwrap();

    engine.switch_backend(BackendPreference::Cpu).unwrap();

    assert_eq!(engine.backend(), BackendKind::Cpu);
    assert_eq!(engine.organism(id).unwrap(), before);
    if GpuPhysicsBackend::new().is_ok() {
        engine.switch_backend(BackendPreference::Gpu).unwrap();
        assert_eq!(engine.backend(), BackendKind::Gpu);
        assert_eq!(engine.organism(id).unwrap(), before);
    }
}

#[test]
fn ffi_exposes_versioned_engine_and_owned_snapshot_buffers() {
    assert_eq!(db_api_version(), ABI_VERSION);
    let mut engine = std::ptr::null_mut();
    let config = br#"{"seed":77,"organism_capacity":16,"world_width":1000.0,"world_height":1000.0,"backend":"Cpu","allow_cpu_fallback":true}"#;

    assert_eq!(
        db_engine_create(config.as_ptr(), config.len(), &mut engine),
        DbStatus::Ok
    );
    assert!(!engine.is_null());
    assert_eq!(db_engine_backend(engine), 0);
    let dna = b"start\nstop";
    assert_eq!(db_engine_import_dna(engine, dna.as_ptr(), dna.len()), DbStatus::Ok);
    assert_eq!(db_engine_tick(engine), DbStatus::Ok);

    let mut buffer = DbBuffer::default();
    assert_eq!(db_engine_snapshot_json(engine, &mut buffer), DbStatus::Ok);
    assert!(!buffer.data.is_null());
    assert!(buffer.len > 0);
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.len) };
    let snapshot: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    assert_eq!(snapshot["tick"], 1);
    assert_eq!(snapshot["organisms"].as_array().unwrap().len(), 1);
    assert!(snapshot["render_instances"][0]["aim"].is_number());
    assert_eq!(
        snapshot["render_instances"][0]["skin"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    assert!(snapshot["render_instances"][0]["lineage_id"].is_number());
    assert_eq!(
        snapshot["organisms"][0]["vision"]["eyes"]
            .as_array()
            .unwrap()
            .len(),
        9
    );

    assert_eq!(db_buffer_free(&mut buffer), DbStatus::Ok);
    assert!(buffer.data.is_null());
    assert_eq!(db_engine_destroy(engine), DbStatus::Ok);
}

#[test]
fn ffi_executes_versioned_command_batches_and_reports_capabilities() {
    let mut engine = std::ptr::null_mut();
    assert_eq!(db_engine_create(std::ptr::null(), 0, &mut engine), DbStatus::Ok);
    let batch = br#"{"version":1,"commands":[{"type":"import_dna","dna":"start\nstop","position":[12.0,34.0]},{"type":"tick","count":2}]}"#;
    let mut result = DbBuffer::default();
    assert_eq!(db_engine_command_batch(engine, batch.as_ptr(), batch.len(), &mut result), DbStatus::Ok);
    let json: serde_json::Value = serde_json::from_slice(unsafe { std::slice::from_raw_parts(result.data, result.len) }).unwrap();
    assert_eq!(json["version"], 1);
    assert_eq!(json["tick"], 2);
    assert_eq!(json["population"], 1);
    assert_eq!(db_buffer_free(&mut result), DbStatus::Ok);

    let mut capabilities = DbBuffer::default();
    assert_eq!(db_engine_capabilities_json(engine, &mut capabilities), DbStatus::Ok);
    let json: serde_json::Value = serde_json::from_slice(unsafe { std::slice::from_raw_parts(capabilities.data, capabilities.len) }).unwrap();
    assert_eq!(json["version"], 1);
    assert!(json["active"].is_string());
    assert_eq!(db_buffer_free(&mut capabilities), DbStatus::Ok);
    assert_eq!(db_engine_destroy(engine), DbStatus::Ok);
}

#[test]
fn ffi_imports_a_configured_species_as_one_command() {
    let mut engine = std::ptr::null_mut();
    assert_eq!(db_engine_create(std::ptr::null(), 0, &mut engine), DbStatus::Ok);
    let batch = br#"{"version":1,"commands":[{"type":"import_species","dna":"start\nstop","name":"Alga","vegetable":true,"color":4283213371,"minimum_population":10,"reseed":true,"initial_energy":2500,"positions":[[100.0,100.0],[200.0,200.0]]}]}"#;
    let mut result = DbBuffer::default();

    assert_eq!(db_engine_command_batch(engine, batch.as_ptr(), batch.len(), &mut result), DbStatus::Ok);
    assert_eq!(db_buffer_free(&mut result), DbStatus::Ok);
    let mut snapshot = DbBuffer::default();
    assert_eq!(db_engine_snapshot_json(engine, &mut snapshot), DbStatus::Ok);
    let json: serde_json::Value = serde_json::from_slice(unsafe {
        std::slice::from_raw_parts(snapshot.data, snapshot.len)
    }).unwrap();
    assert_eq!(json["species"][1]["name"], "Alga");
    assert_eq!(json["species"][1]["vegetable"], true);
    assert_eq!(json["organisms"].as_array().unwrap().len(), 2);
    assert_eq!(json["organisms"][0]["energy"], 2500);

    assert_eq!(db_buffer_free(&mut snapshot), DbStatus::Ok);
    assert_eq!(db_engine_destroy(engine), DbStatus::Ok);
}

#[test]
fn ffi_import_reports_accepted_but_inert_dna_features() {
    let mut engine = std::ptr::null_mut();
    assert_eq!(db_engine_create(std::ptr::null(), 0, &mut engine), DbStatus::Ok);
    let batch = br#"{"version":1,"commands":[{"type":"import_dna","dna":"start\n10 .mkvirus store\nstop","position":[10.0,20.0]}]}"#;
    let mut result = DbBuffer::default();

    assert_eq!(db_engine_command_batch(engine, batch.as_ptr(), batch.len(), &mut result), DbStatus::Ok);
    let json: serde_json::Value = serde_json::from_slice(unsafe {
        std::slice::from_raw_parts(result.data, result.len)
    }).unwrap();
    assert!(json["results"][0]["compatibility_warnings"][0].as_str().unwrap().contains("mkvirus"));
    assert_eq!(db_buffer_free(&mut result), DbStatus::Ok);
    assert_eq!(db_engine_destroy(engine), DbStatus::Ok);
}

#[test]
fn ffi_adds_and_removes_live_environment_features() {
    let mut engine = std::ptr::null_mut();
    assert_eq!(db_engine_create(std::ptr::null(), 0, &mut engine), DbStatus::Ok);
    let add = br#"{"version":1,"commands":[{"type":"add_obstacle","id":7,"minimum":[10.0,20.0],"maximum":[30.0,40.0]},{"type":"add_teleporter","id":8,"center":[50.0,60.0],"radius":15.0,"destination":[500.0,600.0]}]}"#;
    let mut result = DbBuffer::default();
    assert_eq!(db_engine_command_batch(engine, add.as_ptr(), add.len(), &mut result), DbStatus::Ok);
    assert_eq!(db_buffer_free(&mut result), DbStatus::Ok);
    let mut snapshot = DbBuffer::default();
    assert_eq!(db_engine_snapshot_json(engine, &mut snapshot), DbStatus::Ok);
    let json: serde_json::Value = serde_json::from_slice(unsafe { std::slice::from_raw_parts(snapshot.data, snapshot.len) }).unwrap();
    assert_eq!(json["obstacles"][0]["id"], 7);
    assert_eq!(json["teleporters"][0]["id"], 8);
    assert_eq!(db_buffer_free(&mut snapshot), DbStatus::Ok);

    let remove = br#"{"version":1,"commands":[{"type":"remove_obstacle","id":7},{"type":"remove_teleporter","id":8}]}"#;
    assert_eq!(db_engine_command_batch(engine, remove.as_ptr(), remove.len(), &mut result), DbStatus::Ok);
    assert_eq!(db_buffer_free(&mut result), DbStatus::Ok);
    assert_eq!(db_engine_snapshot_json(engine, &mut snapshot), DbStatus::Ok);
    let json: serde_json::Value = serde_json::from_slice(unsafe { std::slice::from_raw_parts(snapshot.data, snapshot.len) }).unwrap();
    assert!(json["obstacles"].as_array().unwrap().is_empty());
    assert!(json["teleporters"].as_array().unwrap().is_empty());
    assert_eq!(db_buffer_free(&mut snapshot), DbStatus::Ok);
    assert_eq!(db_engine_destroy(engine), DbStatus::Ok);
}

#[test]
fn ffi_manual_tools_return_ids_and_exported_dna() {
    let mut engine = std::ptr::null_mut();
    assert_eq!(db_engine_create(std::ptr::null(), 0, &mut engine), DbStatus::Ok);
    let batch = br#"{"version":1,"commands":[
      {"type":"import_dna","dna":"start\n10 .up store\nstop","position":[10.0,10.0]},
      {"type":"move_organism","slot":0,"generation":0,"position":[40.0,50.0]},
      {"type":"clone_organism","slot":0,"generation":0,"position":[60.0,50.0]},
      {"type":"replace_dna","slot":0,"generation":0,"dna":"start\n20 .dx store\nstop"},
      {"type":"export_dna","slot":0,"generation":0},
      {"type":"manual_reproduce","first_slot":0,"first_generation":0,"second_slot":1,"second_generation":0,"position":[50.0,50.0]}
    ]}"#;
    let mut result = DbBuffer::default();

    assert_eq!(db_engine_command_batch(engine, batch.as_ptr(), batch.len(), &mut result), DbStatus::Ok);
    let json: serde_json::Value = serde_json::from_slice(unsafe {
        std::slice::from_raw_parts(result.data, result.len)
    }).unwrap();
    assert_eq!(json["results"][2]["slot"], 1);
    assert!(json["results"][4]["dna"].as_str().unwrap().contains(".dx"));
    assert_eq!(json["results"][5]["slot"], 2);
    assert_eq!(json["population"], 3);

    assert_eq!(db_buffer_free(&mut result), DbStatus::Ok);
    assert_eq!(db_engine_destroy(engine), DbStatus::Ok);
}

#[test]
fn ffi_rejects_unknown_batch_versions_with_owned_structured_error() {
    let mut engine = std::ptr::null_mut();
    assert_eq!(db_engine_create(std::ptr::null(), 0, &mut engine), DbStatus::Ok);
    let batch = br#"{"version":99,"commands":[]}"#;
    let mut result = DbBuffer::default();
    assert_eq!(db_engine_command_batch(engine, batch.as_ptr(), batch.len(), &mut result), DbStatus::InvalidCommand);
    let mut error = DbBuffer::default();
    assert_eq!(db_last_error_json(&mut error), DbStatus::Ok);
    let json: serde_json::Value = serde_json::from_slice(unsafe { std::slice::from_raw_parts(error.data, error.len) }).unwrap();
    assert!(json["message"].as_str().unwrap().contains("version 99"));
    assert_eq!(db_buffer_free(&mut error), DbStatus::Ok);
    assert_eq!(db_engine_destroy(engine), DbStatus::Ok);
}

#[test]
fn ffi_binary_save_load_round_trip_replaces_engine_atomically() {
    let mut engine = std::ptr::null_mut();
    assert_eq!(db_engine_create(std::ptr::null(), 0, &mut engine), DbStatus::Ok);
    let dna = b"start\nstop";
    assert_eq!(db_engine_import_dna(engine, dna.as_ptr(), dna.len()), DbStatus::Ok);
    assert_eq!(db_engine_tick(engine), DbStatus::Ok);
    let mut save = DbBuffer::default();
    assert_eq!(db_engine_save(engine, &mut save), DbStatus::Ok);
    let saved = unsafe { std::slice::from_raw_parts(save.data, save.len) }.to_vec();
    assert_eq!(&saved[..4], b"DB3S");
    assert_eq!(db_engine_tick(engine), DbStatus::Ok);
    assert_eq!(db_engine_load(engine, saved.as_ptr(), saved.len()), DbStatus::Ok);
    let mut snapshot = DbBuffer::default();
    assert_eq!(db_engine_snapshot_json(engine, &mut snapshot), DbStatus::Ok);
    let json: serde_json::Value = serde_json::from_slice(unsafe { std::slice::from_raw_parts(snapshot.data, snapshot.len) }).unwrap();
    assert_eq!(json["tick"], 1);
    assert_eq!(json["organisms"].as_array().unwrap().len(), 1);
    assert_eq!(db_buffer_free(&mut snapshot), DbStatus::Ok);

    let corrupt = b"not-a-save";
    assert_eq!(db_engine_load(engine, corrupt.as_ptr(), corrupt.len()), DbStatus::EngineError);
    let mut unchanged = DbBuffer::default();
    assert_eq!(db_engine_snapshot_json(engine, &mut unchanged), DbStatus::Ok);
    let json: serde_json::Value = serde_json::from_slice(unsafe { std::slice::from_raw_parts(unchanged.data, unchanged.len) }).unwrap();
    assert_eq!(json["tick"], 1);
    assert_eq!(db_buffer_free(&mut unchanged), DbStatus::Ok);
    assert_eq!(db_buffer_free(&mut save), DbStatus::Ok);
    assert_eq!(db_engine_destroy(engine), DbStatus::Ok);
}
#[test]
fn ffi_environment_updates_preserve_optional_db2_settings() {
    let mut engine = std::ptr::null_mut();
    assert_eq!(db_engine_create(std::ptr::null(), 0, &mut engine), DbStatus::Ok);

    let update = br#"{"version":1,"commands":[{"type":"update_environment","metabolism_cost":2,"vegetable_energy_per_tick":3,"sunlight_energy":4,"gravity":[1.0,2.0],"drag":0.1,"brownian_motion":0.2,"physics":{"max_velocity":42.0,"movement_efficiency":0.5,"surface_gravity":0.3,"static_friction":0.4,"kinetic_friction":0.2,"density":0.000001,"viscosity":0.00002,"elasticity":0.8},"shots":{"speed":35.0,"range_multiplier":1.5,"decay":20.0,"energy_shots_do_not_decay":true,"waste_shots_do_not_decay":true},"vegetation":{"start_chloroplasts":12000,"max_energy_per_tick":80,"minimum_chloroplast_equivalents":40,"repopulation_amount":7,"repopulation_cooldown":12,"feeding_to_body":0.6,"daytime":false,"day_night_enabled":true,"cycle_length":5000}},{"type":"update_environment","metabolism_cost":5,"vegetable_energy_per_tick":6,"sunlight_energy":7,"gravity":[0.0,0.0],"drag":0.0,"brownian_motion":0.0}]}"#;
    let mut result = DbBuffer::default();
    assert_eq!(
        db_engine_command_batch(engine, update.as_ptr(), update.len(), &mut result),
        DbStatus::Ok
    );
    assert_eq!(db_buffer_free(&mut result), DbStatus::Ok);

    let mut save = DbBuffer::default();
    assert_eq!(db_engine_save(engine, &mut save), DbStatus::Ok);
    let bytes = unsafe { std::slice::from_raw_parts(save.data, save.len) };
    let payload: serde_json::Value = serde_json::from_slice(&bytes[10..]).unwrap();
    assert_eq!(payload["config"]["physics"]["max_velocity"], 42.0);
    assert_eq!(payload["config"]["shots"]["speed"], 35.0);
    assert_eq!(payload["config"]["vegetation"]["start_chloroplasts"], 12_000);

    assert_eq!(db_buffer_free(&mut save), DbStatus::Ok);
    assert_eq!(db_engine_destroy(engine), DbStatus::Ok);
}
