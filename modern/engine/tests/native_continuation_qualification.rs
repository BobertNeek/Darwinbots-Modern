use darwinbots_engine::{BackendKind, BackendPreference, DnaCosts, Engine, EngineConfig, SaveFile};

const ORIGINAL_WORLD: &[u8] = include_bytes!("fixtures/db24832-original-user.sim");

fn describe_differences(
    left: &serde_json::Value,
    right: &serde_json::Value,
    path: &str,
    count: &mut usize,
) {
    if left == right || *count >= 8 {
        return;
    }
    match (left, right) {
        (serde_json::Value::Object(left), serde_json::Value::Object(right)) => {
            for key in left
                .keys()
                .chain(right.keys())
                .collect::<std::collections::BTreeSet<_>>()
            {
                describe_differences(
                    left.get(key).unwrap_or(&serde_json::Value::Null),
                    right.get(key).unwrap_or(&serde_json::Value::Null),
                    &format!("{path}.{key}"),
                    count,
                );
            }
        }
        (serde_json::Value::Array(left), serde_json::Value::Array(right))
            if left.len() == right.len() =>
        {
            for (index, (left, right)) in left.iter().zip(right).enumerate() {
                describe_differences(left, right, &format!("{path}[{index}]"), count);
            }
        }
        (serde_json::Value::Array(left), serde_json::Value::Array(right)) => {
            eprintln!(
                "saved-state difference {path}: array lengths {} versus {}",
                left.len(),
                right.len()
            );
            *count += 1;
        }
        _ => {
            eprintln!("saved-state difference {path}: {left} versus {right}");
            *count += 1;
        }
    }
}

#[test]
fn modern_save_preserves_double_precision_cost_values_without_a_tick() {
    let engine = Engine::new(EngineConfig {
        dna_costs: DnaCosts {
            global_multiplier: 1077.3647073988097,
            ..DnaCosts::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    assert_eq!(engine.snapshot_publication_count(), 0);
    let before = SaveFile::encode(&engine).unwrap();
    let decoded = SaveFile::decode(&before).unwrap();
    for state in [&engine, &decoded] {
        assert_eq!(state.snapshot().tick, 0);
        assert_eq!(state.snapshot().stats.native_energy_displayed, Some(0));
        assert_eq!(state.snapshot().stats.native_energy_delta, Some(0));
    }
    let after = SaveFile::encode(&decoded).unwrap();
    for bytes in [&before, &after] {
        let payload: serde_json::Value = serde_json::from_slice(&bytes[10..]).unwrap();
        assert_eq!(payload["stats"]["native_energy_displayed"], 0);
        assert_eq!(payload["stats"]["native_energy_delta"], 0);
        assert_eq!(
            payload["config"]["dna_costs"]["global_multiplier"]
                .as_f64()
                .unwrap()
                .to_bits(),
            1077.3647073988097_f64.to_bits()
        );
    }
    assert_eq!(before, after);
    let repeated = SaveFile::decode(&after).unwrap();
    assert_eq!(before, SaveFile::encode(&repeated).unwrap());
}

#[test]
fn original_world_cpu_continuation_preserves_complete_state_across_repeated_saves() {
    let mut continuous = Engine::from_native_sim(ORIGINAL_WORLD).unwrap();
    let mut resumed = SaveFile::decode(&SaveFile::encode(&continuous).unwrap()).unwrap();
    assert_eq!(continuous.backend(), BackendKind::Cpu);
    for checkpoint in 1..=4 {
        continuous.tick_many(500).unwrap();
        resumed.tick_many(500).unwrap();
        continuous.validate_invariants().unwrap();
        resumed.validate_invariants().unwrap();
        let uninterrupted = SaveFile::encode(&continuous).unwrap();
        let restarted = SaveFile::encode(&resumed).unwrap();
        if uninterrupted != restarted {
            let left: serde_json::Value = serde_json::from_slice(&uninterrupted[10..]).unwrap();
            let right: serde_json::Value = serde_json::from_slice(&restarted[10..]).unwrap();
            describe_differences(&left, &right, "$", &mut 0);
            let offset = uninterrupted
                .iter()
                .zip(&restarted)
                .position(|(left, right)| left != right)
                .unwrap();
            let start = offset.saturating_sub(60);
            eprintln!(
                "first byte difference at {offset}: {} versus {}",
                String::from_utf8_lossy(
                    &uninterrupted[start..(offset + 90).min(uninterrupted.len())]
                ),
                String::from_utf8_lossy(&restarted[start..(offset + 90).min(restarted.len())])
            );
        }
        assert!(
            uninterrupted == restarted,
            "complete saved state differs at checkpoint {checkpoint}, cycle {}",
            continuous.snapshot().tick
        );
        assert!(!continuous.snapshot().organisms.is_empty());
        println!(
            "native CPU cycle={} robots={} shots={} births={} mutations={} save_bytes={}",
            continuous.snapshot().tick,
            continuous.snapshot().organisms.len(),
            continuous.snapshot().shots.len(),
            continuous.snapshot().stats.births,
            continuous.snapshot().stats.mutations,
            uninterrupted.len()
        );
        resumed = SaveFile::decode(&restarted).unwrap();
    }
}

fn assert_physical_gpu(engine: &Engine, role: &str) {
    assert_eq!(
        engine.backend(),
        BackendKind::Gpu,
        "{role} must remain on GPU at cycle {}",
        engine.snapshot().tick
    );
    let capabilities = engine.capabilities();
    assert!(
        capabilities.gpu_available,
        "{role} lost GPU availability at cycle {}: {capabilities:?}",
        engine.snapshot().tick
    );
    assert!(
        capabilities.fallback_reason.is_none(),
        "{role} fell back at cycle {}: {capabilities:?}",
        engine.snapshot().tick
    );
    let receipt = engine
        .gpu_adapter_receipt()
        .unwrap_or_else(|| panic!("{role} must expose its active physical adapter"));
    assert!(
        !receipt.name.trim().is_empty(),
        "{role} adapter name is empty"
    );
    assert!(
        !receipt.backend.trim().is_empty(),
        "{role} adapter backend is empty"
    );
    assert!(
        matches!(
            receipt.device_type.as_str(),
            "DiscreteGpu" | "IntegratedGpu"
        ),
        "{role} must use a physical adapter: {receipt:?}"
    );
    println!(
        "native long GPU role={role} cycle={} adapter={receipt:?}",
        engine.snapshot().tick
    );
}

#[test]
#[ignore = "requires an idle physical GPU; run with --ignored on the qualification host"]
fn physical_gpu_original_world_long_continuation_matches_cpu_and_repeated_save_resume() {
    let mut cpu = Engine::from_native_sim(ORIGINAL_WORLD).unwrap();
    let mut gpu = Engine::from_native_sim(ORIGINAL_WORLD).unwrap();
    gpu.switch_backend(BackendPreference::Gpu).unwrap();
    assert_physical_gpu(&gpu, "continuous initialization");
    let mut resumed = SaveFile::decode(&SaveFile::encode(&gpu).unwrap()).unwrap();
    assert_physical_gpu(&resumed, "resumed initialization");
    for checkpoint in 1..=10 {
        cpu.tick_many(1_000).unwrap();
        gpu.tick_many(1_000).unwrap();
        assert_physical_gpu(&gpu, "continuous checkpoint");
        resumed.tick_many(1_000).unwrap();
        assert_physical_gpu(&resumed, "resumed checkpoint");
        for engine in [&cpu, &gpu, &resumed] {
            engine.validate_invariants().unwrap();
        }
        assert_eq!(resumed.backend(), BackendKind::Gpu);
        assert!(
            cpu.snapshot() == gpu.snapshot(),
            "native CPU/GPU gameplay differs at checkpoint {checkpoint}, cycle {}",
            cpu.snapshot().tick
        );
        let continuous = SaveFile::encode(&gpu).unwrap();
        let restarted = SaveFile::encode(&resumed).unwrap();
        assert!(
            continuous == restarted,
            "complete GPU saved state differs at checkpoint {checkpoint}"
        );
        println!(
            "native GPU cycle={} robots={} shots={} births={} mutations={} save_bytes={}",
            gpu.snapshot().tick,
            gpu.snapshot().organisms.len(),
            gpu.snapshot().shots.len(),
            gpu.snapshot().stats.births,
            gpu.snapshot().stats.mutations,
            continuous.len()
        );
        resumed = SaveFile::decode(&restarted).unwrap();
        assert_physical_gpu(&resumed, "resumed reload");
    }
}
