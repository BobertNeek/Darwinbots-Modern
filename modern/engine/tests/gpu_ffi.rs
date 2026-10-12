use darwinbots_engine::{
    BackendKind, BackendPreference, CpuPhysicsBackend, Engine, EngineConfig, GpuPhysicsBackend,
    LegacyDna, PhysicsBackend, PhysicsBatch, SpeciesDefinition,
    ffi::{
        ABI_VERSION, DbBuffer, DbStatus, db_api_version, db_buffer_free, db_engine_backend,
        db_engine_capabilities_json, db_engine_command_batch, db_engine_create, db_engine_destroy,
        db_engine_import_dna, db_engine_load, db_engine_save, db_engine_snapshot_json,
        db_engine_tick, db_last_error_json,
    },
};

const TOXIN_SOURCE_POSITION: [f32; 2] = [250.0, 500.0];
const TOXIN_TARGET_POSITION: [f32; 2] = [500.0, 500.0];
const DEFAULT_ROBOT_RADII_SUM: f32 = 228.56;

#[test]
fn ffi_loads_genuine_native_sim_and_preserves_the_world_after_rejected_load() {
    let config = serde_json::to_vec(&EngineConfig::testing()).unwrap();
    let mut engine = std::ptr::null_mut();
    assert_eq!(
        db_engine_create(config.as_ptr(), config.len(), &mut engine),
        DbStatus::Ok
    );
    let native = include_bytes!("fixtures/db24832-original-user.sim");
    assert_eq!(
        db_engine_load(engine, native.as_ptr(), native.len()),
        DbStatus::Ok
    );
    let loaded = unsafe { &*engine };
    assert_eq!(loaded.snapshot().tick, 281);
    assert_eq!(loaded.snapshot().organisms.len(), 196);
    let before = darwinbots_engine::SaveFile::encode(loaded).unwrap();
    assert_eq!(
        db_engine_load(engine, native.as_ptr(), native.len() - 1),
        DbStatus::EngineError
    );
    assert_eq!(
        darwinbots_engine::SaveFile::encode(unsafe { &*engine }).unwrap(),
        before
    );
    assert_eq!(db_engine_tick(engine), DbStatus::Ok);
    assert_eq!(unsafe { &*engine }.snapshot().tick, 282);
    db_engine_destroy(engine);
}

fn remove_wall_clock_timings(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            for key in [
                "elapsed_ms",
                "duration_ms",
                "wall_clock_ms",
                "phase_timings",
            ] {
                object.remove(key);
            }
            for child in object.values_mut() {
                remove_wall_clock_timings(child);
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                remove_wall_clock_timings(child);
            }
        }
        _ => {}
    }
}

#[test]
fn cpu_physics_integrates_and_clamps_world_bounds() {
    let mut batch = PhysicsBatch {
        positions: vec![[4.0, 8.0], [99.0, 1.0]],
        velocities: vec![[2.0, -3.0], [5.0, -5.0]],
        world_size: [100.0, 100.0],
        wrapped_axes: [false, false],
        apply_world_bounds: true,
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
        positions: (0..257)
            .map(|index| [index as f32, (index % 31) as f32])
            .collect(),
        velocities: (0..257)
            .map(|index| [1.25, -(index as f32) * 0.01])
            .collect(),
        world_size: [256.0, 256.0],
        wrapped_axes: [false, false],
        apply_world_bounds: true,
    };
    for apply_world_bounds in [true, false] {
        let mut expected = source.clone();
        expected.apply_world_bounds = apply_world_bounds;
        let mut actual = expected.clone();
        CpuPhysicsBackend.step(&mut expected).unwrap();
        gpu.step(&mut actual).unwrap();
        for (cpu, gpu) in expected.positions.iter().zip(actual.positions.iter()) {
            assert!((cpu[0] - gpu[0]).abs() <= 0.0001);
            assert!((cpu[1] - gpu[1]).abs() <= 0.0001);
        }
    }
}

#[test]
fn cpu_and_gpu_match_native_border_overshoot_and_next_cycle_respawn() {
    let Ok(adapter) = GpuPhysicsBackend::new() else {
        return;
    };
    println!("Native border GPU: {:?}", adapter.adapter_receipt());
    let config = EngineConfig {
        fixed_bot_radii: true,
        toroidal_world: true,
        physics: darwinbots_engine::PhysicsSettings {
            max_velocity: 200.0,
            movement_efficiency: 1.0,
            density: 0.0,
            ..Default::default()
        },
        ..EngineConfig::testing()
    };
    let mut cpu = Engine::new(EngineConfig {
        backend: BackendPreference::Cpu,
        ..config.clone()
    })
    .unwrap();
    let mut gpu = Engine::new(EngineConfig {
        backend: BackendPreference::Gpu,
        allow_cpu_fallback: false,
        ..config
    })
    .unwrap();
    let dna = LegacyDna::parse("cond *.robage 1 < start 200 .dx store stop").unwrap();
    let cpu_id = cpu.spawn_at(dna.clone(), [930.0, 500.0]).unwrap();
    let gpu_id = gpu.spawn_at(dna, [930.0, 500.0]).unwrap();
    for expected in [[1_130.0, 500.0], [311.0, 500.0]] {
        cpu.tick().unwrap();
        gpu.tick().unwrap();
        assert_eq!(gpu.backend(), BackendKind::Gpu);
        assert_eq!(cpu.organism(cpu_id).unwrap().position, expected);
        assert_eq!(gpu.organism(gpu_id).unwrap().position, expected);
        assert_eq!(
            gpu.robot_state(gpu_id).unwrap().kinematics.actual_velocity,
            [200.0, 0.0]
        );
    }
}

#[test]
fn gpu_sensing_matches_nearest_neighbor_contract_when_adapter_is_available() {
    let Ok(gpu) = GpuPhysicsBackend::new() else {
        return;
    };
    let targets = gpu
        .sense_nearest(
            &[[0.0, 0.0], [10.0, 0.0], [100.0, 0.0]],
            &[true, true, true],
            &[0, 2, 2, 3],
            &[0, 1, 2],
            [3, 1],
            50.0,
            100.0,
        )
        .unwrap();
    assert_eq!(targets, vec![Some(1), Some(0), Some(1)]);
}

#[test]
fn gpu_builds_spatial_grid_before_neighborhood_queries_when_adapter_is_available() {
    let Ok(gpu) = GpuPhysicsBackend::new() else {
        return;
    };
    let targets = gpu
        .sense_nearest_gpu_grid(
            &[[5.0, 5.0], [20.0, 5.0], [190.0, 190.0], [175.0, 190.0]],
            &[true, true, true, true],
            [200.0, 200.0],
            32.0,
            80.0,
        )
        .unwrap();

    assert_eq!(targets, vec![Some(1), Some(0), Some(3), Some(2)]);
}

#[test]
fn fused_gpu_sensing_and_motion_match_cpu_contract_when_adapter_is_available() {
    let Ok(gpu) = GpuPhysicsBackend::new() else {
        return;
    };
    let (targets, positions) = gpu
        .sense_and_integrate(
            &[[0.0, 0.0], [10.0, 0.0], [100.0, 0.0]],
            &[[2.0, 3.0], [-20.0, 1.0], [5.0, -200.0]],
            &[true, true, true],
            &[0, 2, 2, 3],
            &[0, 1, 2],
            [3, 1],
            50.0,
            100.0,
            [100.0, 100.0],
        )
        .unwrap();
    assert_eq!(targets, vec![Some(1), Some(0), Some(1)]);
    assert_eq!(positions, vec![[2.0, 3.0], [0.0, 1.0], [100.0, 0.0]]);
}

#[test]
fn gpu_prepares_render_instances_with_integrated_positions() {
    let Ok(gpu) = GpuPhysicsBackend::new() else {
        return;
    };
    let (_, positions, instances) = gpu
        .sense_integrate_render(
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
        )
        .unwrap();
    assert_eq!(instances.len(), 2);
    assert_eq!(instances[0].slot, 0);
    assert_eq!(instances[0].position, positions[0]);
    assert!((instances[0].radius - 9.0).abs() < 0.001);
    assert_ne!(instances[0].color, instances[1].color);
}

#[test]
fn gpu_sensing_and_rendering_do_not_reintegrate_db2_positions_when_adapter_is_available() {
    let Ok(gpu) = GpuPhysicsBackend::new() else {
        return;
    };
    let positions = [[20.0, 30.0], [70.0, 30.0]];
    let (targets, instances) = gpu
        .sense_and_render_gpu_grid(
            &positions,
            &[400, 100],
            &[114.25, 57.5],
            &[true, true],
            [200.0, 200.0],
            64.0,
            100.0,
        )
        .unwrap();

    assert_eq!(targets, vec![Some(1), Some(0)]);
    assert_eq!(instances.len(), 2);
    assert_eq!(instances[0].position, positions[0]);
    assert_eq!(instances[1].position, positions[1]);
    assert!((instances[0].radius - 114.25).abs() < 0.001);
    assert!((instances[1].radius - 57.5).abs() < 0.001);
}

#[test]
fn physical_gpu_fuses_grid_sensing_integration_and_render_preparation() {
    let gpu = GpuPhysicsBackend::new().unwrap_or_else(|error| {
        panic!("AUT-02 physical GPU gate unavailable: {error}");
    });
    let receipt = gpu.adapter_receipt();
    assert!(!receipt.name.trim().is_empty());
    assert!(!receipt.backend.trim().is_empty());
    assert!(!receipt.device_type.trim().is_empty());
    println!(
        "AUT-02 physical gpu adapter={} backend={} device_type={}",
        receipt.name, receipt.backend, receipt.device_type
    );

    let (targets, integrated, instances) = gpu
        .sense_integrate_render_gpu_grid(
            &[[5.0, 5.0], [20.0, 5.0], [190.0, 190.0], [175.0, 190.0]],
            &[[2.0, 3.0], [-20.0, 1.0], [15.0, 20.0], [-10.0, -3.0]],
            &[400, 100, 4_000, 0],
            &[true, true, true, true],
            [200.0, 200.0],
            32.0,
            80.0,
        )
        .unwrap();

    assert_eq!(targets, vec![Some(1), Some(0), Some(3), Some(2)]);
    assert_eq!(
        integrated,
        vec![[7.0, 8.0], [0.0, 6.0], [200.0, 200.0], [165.0, 187.0]]
    );
    assert_eq!(instances.len(), 4);
    assert_eq!(instances[0].slot, 0);
    assert_eq!(instances[0].position, [7.0, 8.0]);
    assert!((instances[0].radius - 9.0).abs() < 0.001);
    assert_eq!(instances[1].slot, 1);
    assert_eq!(instances[1].position, [0.0, 6.0]);
    assert!((instances[1].radius - 4.5).abs() < 0.001);
    assert_eq!(instances[2].slot, 2);
    assert_eq!(instances[2].position, [200.0, 200.0]);
    assert!((instances[2].radius - 24.0).abs() < 0.001);
    assert_eq!(instances[3].slot, 3);
    assert_eq!(instances[3].position, [165.0, 187.0]);
    assert!((instances[3].radius - 2.0).abs() < 0.001);
}

#[test]
fn cpu_and_gpu_match_gameplay_state_within_existing_physics_tolerance() {
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
    })
    .unwrap();
    let mut gpu = Engine::new(EngineConfig {
        backend: BackendPreference::Gpu,
        allow_cpu_fallback: false,
        ..config
    })
    .unwrap();
    let dna = LegacyDna::parse("cond\n*.robage 0 =\nstart\n10 .up store\nstop").unwrap();
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
    assert_eq!(left.energy, right.energy);
    assert_eq!(left.age, right.age);
    assert_eq!(left.current_gene, right.current_gene);
    assert_eq!(left.condition_count, right.condition_count);
    assert_eq!(left.body, right.body);
    assert_eq!(left.waste, right.waste);
    assert_eq!(left.shell, right.shell);
    assert_eq!(left.slime, right.slime);
    assert_eq!(left.venom, right.venom);
    assert_eq!(left.poison, right.poison);
    assert_eq!(left.chloroplasts, right.chloroplasts);
}

#[test]
fn cpu_and_gpu_match_tie_lifecycle_spring_and_hardening() {
    if GpuPhysicsBackend::new().is_err() {
        return;
    }
    let config = EngineConfig {
        world_width: 3_000.0,
        world_height: 2_000.0,
        toroidal_world: false,
        ..EngineConfig::testing()
    };
    let mut cpu = Engine::new(EngineConfig {
        backend: BackendPreference::Cpu,
        ..config.clone()
    })
    .unwrap();
    let mut gpu = Engine::new(EngineConfig {
        backend: BackendPreference::Gpu,
        allow_cpu_fallback: false,
        ..config
    })
    .unwrap();

    for engine in [&mut cpu, &mut gpu] {
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
        let mut batch = engine.physics_batch();
        let source_index = batch.ids.iter().position(|id| *id == source).unwrap();
        let target_index = batch.ids.iter().position(|id| *id == target).unwrap();
        batch.batch.positions[source_index] = [400.0, 1_000.0];
        batch.batch.positions[target_index] = [730.0, 1_000.0];
        batch.batch.velocities[source_index] = [0.0; 2];
        batch.batch.velocities[target_index] = [0.0; 2];
        engine.apply_physics_batch(batch).unwrap();
        engine.tick().unwrap();
    }

    for index in 0..2 {
        let left = &cpu.snapshot().organisms[index];
        let right = &gpu.snapshot().organisms[index];
        assert!((left.position[0] - right.position[0]).abs() < 0.05);
        assert!((left.velocity[0] - right.velocity[0]).abs() < 0.05);
    }
    assert_eq!(cpu.snapshot().ties[0].last, -19);
    assert_eq!(gpu.snapshot().ties[0].last, -19);
    assert!(!cpu.snapshot().ties[0].hardened);
    assert!(!gpu.snapshot().ties[0].hardened);

    cpu.tick_many(18).unwrap();
    gpu.tick_many(18).unwrap();

    let left_tie = &cpu.snapshot().ties[0];
    let right_tie = &gpu.snapshot().ties[0];
    assert_eq!(left_tie.last, -1);
    assert_eq!(right_tie.last, -1);
    assert!(left_tie.hardened);
    assert!(right_tie.hardened);
    assert_eq!(left_tie.b, right_tie.b);
    assert_eq!(left_tie.k, right_tie.k);
    assert!((left_tie.rest_length - right_tie.rest_length).abs() < 0.05);
    for index in 0..2 {
        let left = &cpu.snapshot().organisms[index];
        let right = &gpu.snapshot().organisms[index];
        assert!((left.position[0] - right.position[0]).abs() < 0.05);
        assert!((left.velocity[0] - right.velocity[0]).abs() < 0.05);
        assert_eq!(cpu.memory(left.id, "multi").unwrap(), 1);
        assert_eq!(gpu.memory(right.id, "multi").unwrap(), 1);
    }
}

#[test]
fn cpu_and_gpu_emit_identical_nontiming_cycle_traces() {
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
    })
    .unwrap();
    let mut gpu = Engine::new(EngineConfig {
        backend: BackendPreference::Gpu,
        allow_cpu_fallback: false,
        ..config
    })
    .unwrap();
    let dna = LegacyDna::parse("cond\n*.robage 0 =\nstart\n10 .up store\nstop").unwrap();
    for position in [[5_000.0, 5_000.0], [5_200.0, 5_100.0]] {
        cpu.spawn_at(dna.clone(), position).unwrap();
        gpu.spawn_at(dna.clone(), position).unwrap();
    }

    let mut cpu_trace = serde_json::to_value(cpu.tick_with_trace().unwrap()).unwrap();
    let mut gpu_trace = serde_json::to_value(gpu.tick_with_trace().unwrap()).unwrap();
    remove_wall_clock_timings(&mut cpu_trace);
    remove_wall_clock_timings(&mut gpu_trace);

    assert_eq!(gpu_trace, cpu_trace);
}

#[test]
fn cpu_and_gpu_match_toxin_state_memory_and_position() {
    if GpuPhysicsBackend::new().is_err() {
        return;
    }
    let config = EngineConfig {
        metabolism_cost: 0,
        gravity: [0.0, 0.0],
        drag: 0.0,
        brownian_motion: 0.0,
        ..EngineConfig::testing()
    };
    let mut cpu = Engine::new(EngineConfig {
        backend: BackendPreference::Cpu,
        ..config.clone()
    })
    .unwrap();
    let mut gpu = Engine::new(EngineConfig {
        backend: BackendPreference::Gpu,
        allow_cpu_fallback: false,
        ..config
    })
    .unwrap();

    fn seed(engine: &mut Engine) -> (darwinbots_engine::OrganismId, darwinbots_engine::OrganismId) {
        let center_gap = TOXIN_TARGET_POSITION[0] - TOXIN_SOURCE_POSITION[0];
        assert_eq!(center_gap, 250.0);
        assert!(center_gap - DEFAULT_ROBOT_RADII_SUM > 0.0);
        let source_species = engine.register_species(SpeciesDefinition {
            name: "GPU toxin source".to_owned(),
            ..SpeciesDefinition::default()
        });
        let target_species = engine.register_species(SpeciesDefinition {
            name: "GPU toxin target".to_owned(),
            ..SpeciesDefinition::default()
        });
        let source = engine
            .spawn_species_at(
                LegacyDna::parse("start\n8 .strvenom store\nstop").unwrap(),
                source_species,
                TOXIN_SOURCE_POSITION,
            )
            .unwrap();
        let target = engine
            .spawn_species_at(
                LegacyDna::parse("start\nstop").unwrap(),
                target_species,
                TOXIN_TARGET_POSITION,
            )
            .unwrap();
        (source, target)
    }

    let (cpu_source, cpu_target) = seed(&mut cpu);
    let (gpu_source, gpu_target) = seed(&mut gpu);
    cpu.tick().unwrap();
    gpu.tick().unwrap();
    assert_eq!(cpu.snapshot().stats.projectile_impacts, 0);
    assert_eq!(gpu.snapshot().stats.projectile_impacts, 0);
    let shot = LegacyDna::parse(
        "start\n-3 .shoot store\n8 .shootval store\n500 .vloc store\n77 .venval store\nstop",
    )
    .unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    cpu.replace_dna(cpu_source, shot.clone()).unwrap();
    gpu.replace_dna(gpu_source, shot).unwrap();
    cpu.tick().unwrap();
    gpu.tick().unwrap();
    assert_eq!(cpu.snapshot().stats.projectile_impacts, 0);
    assert_eq!(gpu.snapshot().stats.projectile_impacts, 0);
    cpu.replace_dna(cpu_source, idle.clone()).unwrap();
    gpu.replace_dna(gpu_source, idle).unwrap();
    cpu.tick().unwrap();
    gpu.tick().unwrap();
    assert_eq!(cpu.snapshot().stats.projectile_impacts, 1);
    assert_eq!(gpu.snapshot().stats.projectile_impacts, 1);
    let mover = LegacyDna::parse("start\n10 .dx store\nstop").unwrap();
    cpu.replace_dna(cpu_target, mover.clone()).unwrap();
    gpu.replace_dna(gpu_target, mover).unwrap();
    cpu.tick().unwrap();
    gpu.tick().unwrap();

    let left = cpu.organism(cpu_target).unwrap();
    let right = gpu.organism(gpu_target).unwrap();
    assert_eq!(left.paralyzed, 6);
    assert_eq!(right.paralyzed, left.paralyzed);
    assert_eq!(gpu.memory_at(gpu_target, 500).unwrap(), 77);
    assert_eq!(
        gpu.memory_at(gpu_target, 500).unwrap(),
        cpu.memory_at(cpu_target, 500).unwrap()
    );
    assert_eq!(
        gpu.robot_state(gpu_target).unwrap().effects,
        cpu.robot_state(cpu_target).unwrap().effects
    );
    assert!((right.position[0] - left.position[0]).abs() < 0.0001);
    assert!((right.position[1] - left.position[1]).abs() < 0.0001);
    assert!((right.velocity[0] - left.velocity[0]).abs() < 0.0001);
    assert!((right.velocity[1] - left.velocity[1]).abs() < 0.0001);
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
    })
    .unwrap();
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
fn engine_can_switch_backends_while_preserving_live_state() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let id = engine
        .spawn_at(
            LegacyDna::parse("start\n10 .up store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
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
fn ffi_v1_consumers_accept_the_additive_snapshot() {
    #[derive(serde::Deserialize)]
    struct FfiV1Snapshot {
        tick: u64,
        organisms: Vec<FfiV1Organism>,
    }

    #[derive(serde::Deserialize)]
    struct FfiV1Organism {
        energy: i32,
    }

    assert_eq!(ABI_VERSION, 1);
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
    assert_eq!(
        db_engine_import_dna(engine, dna.as_ptr(), dna.len()),
        DbStatus::Ok
    );
    assert_eq!(db_engine_tick(engine), DbStatus::Ok);

    let mut buffer = DbBuffer::default();
    assert_eq!(db_engine_snapshot_json(engine, &mut buffer), DbStatus::Ok);
    assert!(!buffer.data.is_null());
    assert!(buffer.len > 0);
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.len) };
    let v1_snapshot: FfiV1Snapshot = serde_json::from_slice(bytes).unwrap();
    assert_eq!(v1_snapshot.tick, 1);
    assert_eq!(v1_snapshot.organisms[0].energy, 1_000);
    let snapshot: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    assert_eq!(snapshot["schema_version"], 2);
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
    assert_eq!(
        db_engine_create(std::ptr::null(), 0, &mut engine),
        DbStatus::Ok
    );
    let batch = br#"{"version":1,"commands":[{"type":"import_dna","dna":"start\nstop","position":[12.0,34.0]},{"type":"tick","count":2}]}"#;
    let mut result = DbBuffer::default();
    assert_eq!(
        db_engine_command_batch(engine, batch.as_ptr(), batch.len(), &mut result),
        DbStatus::Ok
    );
    let json: serde_json::Value =
        serde_json::from_slice(unsafe { std::slice::from_raw_parts(result.data, result.len) })
            .unwrap();
    assert_eq!(json["version"], 1);
    assert_eq!(json["tick"], 2);
    assert_eq!(json["population"], 1);
    assert_eq!(db_buffer_free(&mut result), DbStatus::Ok);

    let mut capabilities = DbBuffer::default();
    assert_eq!(
        db_engine_capabilities_json(engine, &mut capabilities),
        DbStatus::Ok
    );
    let json: serde_json::Value = serde_json::from_slice(unsafe {
        std::slice::from_raw_parts(capabilities.data, capabilities.len)
    })
    .unwrap();
    assert_eq!(json["version"], 1);
    assert!(json["active"].is_string());
    assert_eq!(db_buffer_free(&mut capabilities), DbStatus::Ok);
    assert_eq!(db_engine_destroy(engine), DbStatus::Ok);
}

#[test]
fn ffi_imports_a_configured_species_as_one_command() {
    let mut engine = std::ptr::null_mut();
    assert_eq!(
        db_engine_create(std::ptr::null(), 0, &mut engine),
        DbStatus::Ok
    );
    let batch = br#"{"version":1,"commands":[{"type":"import_species","dna":"start\nstop","name":"Alga","vegetable":true,"color":4283213371,"minimum_population":10,"reseed":true,"initial_energy":2500,"positions":[[100.0,100.0],[200.0,200.0]]}]}"#;
    let mut result = DbBuffer::default();

    assert_eq!(
        db_engine_command_batch(engine, batch.as_ptr(), batch.len(), &mut result),
        DbStatus::Ok
    );
    assert_eq!(db_buffer_free(&mut result), DbStatus::Ok);
    let mut snapshot = DbBuffer::default();
    assert_eq!(db_engine_snapshot_json(engine, &mut snapshot), DbStatus::Ok);
    let json: serde_json::Value =
        serde_json::from_slice(unsafe { std::slice::from_raw_parts(snapshot.data, snapshot.len) })
            .unwrap();
    assert_eq!(json["species"][1]["name"], "Alga");
    assert_eq!(json["species"][1]["vegetable"], true);
    assert_eq!(json["organisms"].as_array().unwrap().len(), 2);
    assert_eq!(json["organisms"][0]["energy"], 2500);

    assert_eq!(db_buffer_free(&mut snapshot), DbStatus::Ok);
    assert_eq!(db_engine_destroy(engine), DbStatus::Ok);
}

#[test]
fn ffi_import_accepts_known_sysvars_without_inert_warnings() {
    let mut engine = std::ptr::null_mut();
    assert_eq!(
        db_engine_create(std::ptr::null(), 0, &mut engine),
        DbStatus::Ok
    );
    let batch = br#"{"version":1,"commands":[{"type":"import_dna","dna":"start\n10 .mkvirus store\nstop","position":[10.0,20.0]}]}"#;
    let mut result = DbBuffer::default();

    assert_eq!(
        db_engine_command_batch(engine, batch.as_ptr(), batch.len(), &mut result),
        DbStatus::Ok
    );
    let json: serde_json::Value =
        serde_json::from_slice(unsafe { std::slice::from_raw_parts(result.data, result.len) })
            .unwrap();
    assert_eq!(
        json["results"][0]["compatibility_warnings"],
        serde_json::json!([])
    );
    assert_eq!(db_buffer_free(&mut result), DbStatus::Ok);
    assert_eq!(db_engine_destroy(engine), DbStatus::Ok);
}

#[test]
fn ffi_adds_and_removes_live_environment_features() {
    let mut engine = std::ptr::null_mut();
    assert_eq!(
        db_engine_create(std::ptr::null(), 0, &mut engine),
        DbStatus::Ok
    );
    let add = br#"{"version":1,"commands":[{"type":"add_obstacle","id":7,"minimum":[10.0,20.0],"maximum":[30.0,40.0]},{"type":"update_obstacle","obstacle":{"id":7,"minimum":[10.0,20.0],"maximum":[30.0,40.0],"color":4279312947,"opacity":0.35,"drift":[2.0,-3.0],"vision_policy":"see_through","shot_policy":"pass_through"}},{"type":"add_teleporter","id":8,"center":[50.0,60.0],"radius":15.0,"destination":[500.0,600.0]},{"type":"add_file_teleporter","teleporter":{"id":9,"center":[70.0,80.0],"radius":20.0,"inbound_path":"in","outbound_path":"out","poll_interval":3,"poll_countdown":1,"capacity":4,"filters":{"teleport_vegetables":false,"teleport_corpses":true,"teleport_heterotrophs":true},"sent":8,"received":5}},{"type":"update_file_teleporter","teleporter":{"id":9,"center":[70.0,80.0],"radius":20.0,"inbound_path":"in2","outbound_path":"out2","poll_interval":4,"poll_countdown":2,"capacity":5,"filters":{"teleport_vegetables":true,"teleport_corpses":false,"teleport_heterotrophs":false},"sent":9,"received":6}}]}"#;
    let mut result = DbBuffer::default();
    assert_eq!(
        db_engine_command_batch(engine, add.as_ptr(), add.len(), &mut result),
        DbStatus::Ok
    );
    assert_eq!(db_buffer_free(&mut result), DbStatus::Ok);
    let mut snapshot = DbBuffer::default();
    assert_eq!(db_engine_snapshot_json(engine, &mut snapshot), DbStatus::Ok);
    let json: serde_json::Value =
        serde_json::from_slice(unsafe { std::slice::from_raw_parts(snapshot.data, snapshot.len) })
            .unwrap();
    assert_eq!(json["obstacles"][0]["id"], 7);
    assert_eq!(json["obstacles"][0]["vision_policy"], "see_through");
    assert_eq!(json["obstacles"][0]["shot_policy"], "pass_through");
    assert_eq!(json["teleporters"][0]["id"], 8);
    assert_eq!(json["file_teleporters"][0]["id"], 9);
    assert_eq!(json["file_teleporters"][0]["inbound_path"], "in2");
    assert_eq!(json["file_teleporters"][0]["sent"], 9);
    assert_eq!(json["file_teleporters"][0]["received"], 6);
    assert_eq!(db_buffer_free(&mut snapshot), DbStatus::Ok);

    let remove = br#"{"version":1,"commands":[{"type":"remove_obstacle","id":7},{"type":"remove_teleporter","id":8},{"type":"remove_file_teleporter","id":9}]}"#;
    assert_eq!(
        db_engine_command_batch(engine, remove.as_ptr(), remove.len(), &mut result),
        DbStatus::Ok
    );
    assert_eq!(db_buffer_free(&mut result), DbStatus::Ok);
    assert_eq!(db_engine_snapshot_json(engine, &mut snapshot), DbStatus::Ok);
    let json: serde_json::Value =
        serde_json::from_slice(unsafe { std::slice::from_raw_parts(snapshot.data, snapshot.len) })
            .unwrap();
    assert!(json["obstacles"].as_array().unwrap().is_empty());
    assert!(json["teleporters"].as_array().unwrap().is_empty());
    assert!(json["file_teleporters"].as_array().unwrap().is_empty());
    assert_eq!(db_buffer_free(&mut snapshot), DbStatus::Ok);
    assert_eq!(db_engine_destroy(engine), DbStatus::Ok);
}

#[test]
fn ffi_manual_tools_return_ids_and_exported_dna() {
    let mut engine = std::ptr::null_mut();
    assert_eq!(
        db_engine_create(std::ptr::null(), 0, &mut engine),
        DbStatus::Ok
    );
    let batch = br#"{"version":1,"commands":[
      {"type":"import_dna","dna":"start\n10 .up store\nstop","position":[10.0,10.0]},
      {"type":"move_organism","slot":0,"generation":0,"position":[40.0,50.0]},
      {"type":"clone_organism","slot":0,"generation":0,"position":[60.0,50.0]},
      {"type":"replace_dna","slot":0,"generation":0,"dna":"start\n20 .dx store\nstop"},
      {"type":"export_dna","slot":0,"generation":0},
      {"type":"manual_reproduce","first_slot":0,"first_generation":0,"second_slot":1,"second_generation":0,"position":[50.0,50.0]}
    ]}"#;
    let mut result = DbBuffer::default();

    assert_eq!(
        db_engine_command_batch(engine, batch.as_ptr(), batch.len(), &mut result),
        DbStatus::Ok
    );
    let json: serde_json::Value =
        serde_json::from_slice(unsafe { std::slice::from_raw_parts(result.data, result.len) })
            .unwrap();
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
    assert_eq!(
        db_engine_create(std::ptr::null(), 0, &mut engine),
        DbStatus::Ok
    );
    let batch = br#"{"version":99,"commands":[]}"#;
    let mut result = DbBuffer::default();
    assert_eq!(
        db_engine_command_batch(engine, batch.as_ptr(), batch.len(), &mut result),
        DbStatus::InvalidCommand
    );
    let mut error = DbBuffer::default();
    assert_eq!(db_last_error_json(&mut error), DbStatus::Ok);
    let json: serde_json::Value =
        serde_json::from_slice(unsafe { std::slice::from_raw_parts(error.data, error.len) })
            .unwrap();
    assert!(json["message"].as_str().unwrap().contains("version 99"));
    assert_eq!(db_buffer_free(&mut error), DbStatus::Ok);
    assert_eq!(db_engine_destroy(engine), DbStatus::Ok);
}

#[test]
fn ffi_binary_save_load_round_trip_replaces_engine_atomically() {
    let mut engine = std::ptr::null_mut();
    assert_eq!(
        db_engine_create(std::ptr::null(), 0, &mut engine),
        DbStatus::Ok
    );
    let dna = b"start\nstop";
    assert_eq!(
        db_engine_import_dna(engine, dna.as_ptr(), dna.len()),
        DbStatus::Ok
    );
    assert_eq!(db_engine_tick(engine), DbStatus::Ok);
    let mut save = DbBuffer::default();
    assert_eq!(db_engine_save(engine, &mut save), DbStatus::Ok);
    let saved = unsafe { std::slice::from_raw_parts(save.data, save.len) }.to_vec();
    assert_eq!(&saved[..4], b"DB3S");
    assert_eq!(db_engine_tick(engine), DbStatus::Ok);
    assert_eq!(
        db_engine_load(engine, saved.as_ptr(), saved.len()),
        DbStatus::Ok
    );
    let mut snapshot = DbBuffer::default();
    assert_eq!(db_engine_snapshot_json(engine, &mut snapshot), DbStatus::Ok);
    let json: serde_json::Value =
        serde_json::from_slice(unsafe { std::slice::from_raw_parts(snapshot.data, snapshot.len) })
            .unwrap();
    assert_eq!(json["tick"], 1);
    assert_eq!(json["organisms"].as_array().unwrap().len(), 1);
    assert_eq!(db_buffer_free(&mut snapshot), DbStatus::Ok);

    let corrupt = b"not-a-save";
    assert_eq!(
        db_engine_load(engine, corrupt.as_ptr(), corrupt.len()),
        DbStatus::EngineError
    );
    let mut unchanged = DbBuffer::default();
    assert_eq!(
        db_engine_snapshot_json(engine, &mut unchanged),
        DbStatus::Ok
    );
    let json: serde_json::Value = serde_json::from_slice(unsafe {
        std::slice::from_raw_parts(unchanged.data, unchanged.len)
    })
    .unwrap();
    assert_eq!(json["tick"], 1);
    assert_eq!(db_buffer_free(&mut unchanged), DbStatus::Ok);
    assert_eq!(db_buffer_free(&mut save), DbStatus::Ok);
    assert_eq!(db_engine_destroy(engine), DbStatus::Ok);
}
#[test]
fn ffi_environment_updates_preserve_optional_db2_settings() {
    let mut engine = std::ptr::null_mut();
    assert_eq!(
        db_engine_create(std::ptr::null(), 0, &mut engine),
        DbStatus::Ok
    );

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
    assert_eq!(
        payload["config"]["vegetation"]["start_chloroplasts"],
        12_000
    );

    assert_eq!(db_buffer_free(&mut save), DbStatus::Ok);
    assert_eq!(db_engine_destroy(engine), DbStatus::Ok);
}
#[test]
fn ffi_versioned_public_abi_failures_are_atomic_and_valid_batch_commits_once() {
    fn zeroed_buffer() -> DbBuffer {
        unsafe { std::mem::zeroed() }
    }

    let read_buffer = |buffer: &mut DbBuffer| {
        let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.len) }.to_vec();
        db_buffer_free(buffer);
        bytes
    };

    let snapshot = |engine| {
        let mut buffer = zeroed_buffer();
        assert_eq!(db_engine_snapshot_json(engine, &mut buffer), DbStatus::Ok);
        read_buffer(&mut buffer)
    };

    let last_error = || {
        let mut buffer = zeroed_buffer();
        assert_eq!(db_last_error_json(&mut buffer), DbStatus::Ok);
        let value: serde_json::Value = serde_json::from_slice(&read_buffer(&mut buffer)).unwrap();
        let object = value.as_object().expect("last error must be a JSON object");
        assert!(!object.is_empty(), "last error must retain typed fields");
        assert!(
            object.values().any(|field| match field.as_str() {
                Some(text) => !text.is_empty(),
                None => false,
            }),
            "last error must contain a non-empty typed value"
        );
        value
    };

    let mut engine = std::ptr::null_mut();
    assert_eq!(
        db_engine_create(std::ptr::null(), 0, &mut engine),
        DbStatus::Ok
    );

    let failures: [(&str, &[u8], DbStatus); 4] = [
        ("future version", br#"{"version":99,"commands":[]}"#, DbStatus::InvalidCommand),
        ("malformed envelope", br#"{not-json"#, DbStatus::InvalidCommand),
        (
            "unknown command",
            br#"{"version":1,"commands":[{"type":"unknown_command"}]}"#,
            DbStatus::InvalidCommand,
        ),
        (
            "late invalid operation",
            br#"{"version":1,"commands":[{"type":"import_dna","dna":"start\nstop","position":[10.0,10.0]},{"type":"move_organism","slot":999,"generation":0,"position":[40.0,50.0]}]}"#,
            DbStatus::EngineError,
        ),
    ];

    for (name, batch, expected_status) in failures {
        let before = snapshot(engine);
        let mut output = zeroed_buffer();
        assert_eq!(
            db_engine_command_batch(engine, batch.as_ptr(), batch.len(), &mut output),
            expected_status,
            "{name} must be rejected by the typed command boundary"
        );
        db_buffer_free(&mut output);
        assert_eq!(before, snapshot(engine), "{name} must be byte-atomic");
        let _ = last_error();
    }

    let valid = br#"{"version":1,"commands":[{"type":"import_dna","dna":"start\nstop","position":[10.0,10.0]},{"type":"move_organism","slot":0,"generation":0,"position":[40.0,50.0]}]}"#;
    let before = snapshot(engine);
    let mut output = zeroed_buffer();
    assert_eq!(
        db_engine_command_batch(engine, valid.as_ptr(), valid.len(), &mut output),
        DbStatus::Ok
    );
    let result: serde_json::Value = serde_json::from_slice(&read_buffer(&mut output)).unwrap();
    assert_eq!(
        result["results"].as_array().map(|results| results.len()),
        Some(2)
    );
    assert_eq!(result["population"].as_u64(), Some(1));
    let committed = snapshot(engine);
    assert_ne!(before, committed);
    assert_eq!(committed, snapshot(engine), "the valid batch commits once");

    db_engine_destroy(engine);
}
