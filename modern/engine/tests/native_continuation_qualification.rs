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
    let before = SaveFile::encode(&engine).unwrap();
    let decoded = SaveFile::decode(&before).unwrap();
    assert_eq!(before, SaveFile::encode(&decoded).unwrap());
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

#[test]
#[ignore = "requires an idle physical GPU; run with --ignored on the qualification host"]
fn physical_gpu_original_world_long_continuation_matches_cpu_and_repeated_save_resume() {
    let mut cpu = Engine::from_native_sim(ORIGINAL_WORLD).unwrap();
    let mut gpu = Engine::from_native_sim(ORIGINAL_WORLD).unwrap();
    gpu.switch_backend(BackendPreference::Gpu).unwrap();
    assert_eq!(gpu.backend(), BackendKind::Gpu);
    assert!(gpu.capabilities().fallback_reason.is_none());
    let receipt = gpu
        .gpu_adapter_receipt()
        .expect("physical adapter required");
    assert!(matches!(
        receipt.device_type.as_str(),
        "DiscreteGpu" | "IntegratedGpu"
    ));
    println!("native long GPU adapter={receipt:?}");
    let mut resumed = SaveFile::decode(&SaveFile::encode(&gpu).unwrap()).unwrap();
    for checkpoint in 1..=10 {
        cpu.tick_many(1_000).unwrap();
        gpu.tick_many(1_000).unwrap();
        resumed.tick_many(1_000).unwrap();
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
    }
}
