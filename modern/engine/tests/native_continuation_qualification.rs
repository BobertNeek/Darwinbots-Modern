use darwinbots_engine::{BackendKind, BackendPreference, Engine, SaveFile};

const ORIGINAL_WORLD: &[u8] = include_bytes!("fixtures/db24832-original-user.sim");

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
    let receipt = gpu.gpu_adapter_receipt().expect("physical adapter required");
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
