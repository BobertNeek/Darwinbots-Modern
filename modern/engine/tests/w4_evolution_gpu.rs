use darwinbots_engine::{
    BackendKind, BackendPreference, Engine, EngineConfig, LegacyDna, LegacySimulationFile,
    StandardMutationKind, StandardMutationProfile,
};

fn test_engine() -> Engine {
    Engine::new(EngineConfig::testing()).expect("test engine should initialize")
}

#[test]
fn real_reproduction_propagates_descendants_and_writes_legacy_snapshot_headers() {
    let mut engine = test_engine();
    let reproducing_dna = LegacyDna::parse("start\nstop").unwrap();
    let parent = engine.spawn_at(reproducing_dna, [500.0, 500.0]).unwrap();

    let child = engine
        .manual_reproduce(parent, None, [1_000.0, 500.0])
        .unwrap();
    let grandchild = engine
        .manual_reproduce(child, None, [1_500.0, 500.0])
        .unwrap();

    let parent_snapshot = engine.organism(parent).unwrap();
    let child_snapshot = engine.organism(child).unwrap();
    let grandchild_snapshot = engine.organism(grandchild).unwrap();
    assert_eq!(parent_snapshot.descendants, 2);
    assert_eq!(child_snapshot.descendants, 1);
    assert_eq!(grandchild_snapshot.generation, 2);
    assert!(
        engine
            .lineage(parent, 2)
            .unwrap()
            .iter()
            .any(|entry| entry.id == grandchild && entry.depth == 2)
    );

    let legacy = LegacySimulationFile::from_engine(&engine).unwrap();
    let exported_headers = legacy
        .organisms
        .iter()
        .map(|organism| {
            let dna = LegacyDna::parse(&organism.dna).unwrap();
            (dna.metadata().generation, dna.metadata().mutations)
        })
        .collect::<Vec<_>>();
    assert!(exported_headers.contains(&(1, 0)));
    assert!(exported_headers.contains(&(2, 0)));
}

fn force_reproductive_kind(profile: &mut StandardMutationProfile, kind: StandardMutationKind) {
    let inactive = 1.0e30;
    profile.copy_error.one_in = inactive;
    profile.insertion.one_in = inactive;
    profile.reversal.one_in = inactive;
    profile.minor_deletion.one_in = inactive;
    profile.major_deletion.one_in = inactive;
    match kind {
        StandardMutationKind::CopyError => profile.copy_error.one_in = 1.0,
        StandardMutationKind::Insertion => profile.insertion.one_in = 1.0,
        StandardMutationKind::Reversal => profile.reversal.one_in = 1.0,
        StandardMutationKind::MinorDeletion => profile.minor_deletion.one_in = 1.0,
        StandardMutationKind::MajorDeletion => profile.major_deletion.one_in = 1.0,
        _ => unreachable!("test only selects reproductive structural kinds"),
    }
}

#[test]
fn reproduction_scan_applies_configured_structural_mutation_distribution() {
    for kind in [
        StandardMutationKind::CopyError,
        StandardMutationKind::Insertion,
        StandardMutationKind::Reversal,
        StandardMutationKind::MinorDeletion,
        StandardMutationKind::MajorDeletion,
    ] {
        let mut engine = test_engine();
        let parent = engine
            .spawn_at(
                LegacyDna::parse("start\n1 2 3 4 5 6 7 8 9 10\n50 .mrepro store\nstop").unwrap(),
                [500.0, 500.0],
            )
            .unwrap();
        let mut profile = engine.mutation_profile(parent).unwrap().clone();
        force_reproductive_kind(&mut profile, kind);
        profile.copy_error.mean = 3.0;
        profile.copy_error.std_dev = 0.0;
        profile.insertion.mean = 3.0;
        profile.insertion.std_dev = 0.0;
        profile.reversal.mean = 3.0;
        profile.reversal.std_dev = 0.0;
        profile.minor_deletion.mean = 3.0;
        profile.minor_deletion.std_dev = 0.0;
        profile.major_deletion.mean = 3.0;
        profile.major_deletion.std_dev = 0.0;
        engine.set_mutation_profile(parent, profile).unwrap();
        let parent_dna = engine.dna(parent).unwrap().clone();

        engine.tick().unwrap();

        let child = engine
            .snapshot()
            .organisms
            .iter()
            .find(|organism| organism.id != parent)
            .expect("mrepro should publish one child");
        assert_eq!(child.last_mutation_kind, Some(kind));
        assert!(child.total_mutations > 0);
        assert_ne!(engine.dna(child.id).unwrap(), &parent_dna);
        let parent_length = parent_dna.blocks().len();
        let child_dna = engine.dna(child.id).unwrap();
        match kind {
            StandardMutationKind::CopyError => {
                // A length-three copy pass visits each original coding position,
                // truncating the final two runs at the master end.
                assert_eq!(child.total_mutations, (3 * (parent_length - 1) - 3) as u64);
                assert_eq!(child_dna.blocks().len(), parent_length);
            }
            StandardMutationKind::Insertion => {
                let inserted = 3 * (parent_length - 1);
                assert_eq!(child.total_mutations, (inserted * 2) as u64);
                assert_eq!(child_dna.blocks().len(), parent_length + inserted);
            }
            StandardMutationKind::Reversal => {
                assert_eq!(child.total_mutations, (parent_length - 3) as u64);
                assert_eq!(child_dna.blocks().len(), parent_length);
            }
            StandardMutationKind::MinorDeletion | StandardMutationKind::MajorDeletion => {
                let events = (parent_length - 1).div_ceil(4);
                assert_eq!(child.total_mutations, events as u64);
                assert_eq!(child_dna.blocks().len(), parent_length - events * 3);
            }
            _ => unreachable!(),
        }
    }
}

fn differential_config(backend: BackendPreference) -> EngineConfig {
    EngineConfig {
        backend,
        allow_cpu_fallback: false,
        metabolism_cost: 0,
        gravity: [0.0, 0.0],
        drag: 0.0,
        brownian_motion: 0.0,
        world_width: 4_000.0,
        world_height: 3_000.0,
        ..EngineConfig::testing()
    }
}

fn seed_differential_engine(engine: &mut Engine) -> darwinbots_engine::OrganismId {
    let parent = engine
        .spawn_at(
            LegacyDna::parse(
                "start\n8 .strvenom store\n1 2 3 4 5 6 7 8 9 10\n50 .mrepro store\nstop",
            )
            .unwrap(),
            [500.0, 500.0],
        )
        .unwrap();
    engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [1_500.0, 1_000.0])
        .unwrap();
    let mut profile = engine.mutation_profile(parent).unwrap().clone();
    force_reproductive_kind(&mut profile, StandardMutationKind::Insertion);
    profile.insertion.mean = 3.0;
    profile.insertion.std_dev = 0.0;
    engine.set_mutation_profile(parent, profile).unwrap();
    parent
}

#[test]
#[ignore = "requires a physical GPU; run with --ignored on the GPU qualification host"]
fn physical_gpu_phase2_ecological_differential_has_adapter_receipt() {
    let mut cpu = Engine::new(differential_config(BackendPreference::Cpu)).unwrap();
    let mut gpu = Engine::new(differential_config(BackendPreference::Gpu)).unwrap();
    assert_eq!(gpu.backend(), BackendKind::Gpu);
    let receipt = gpu
        .gpu_adapter_receipt()
        .expect("GPU engine must expose its physical adapter receipt");
    assert!(!receipt.name.is_empty());
    assert!(!receipt.backend.is_empty());
    println!(
        "physical gpu adapter={} backend={} device_type={}",
        receipt.name, receipt.backend, receipt.device_type
    );

    let cpu_parent = seed_differential_engine(&mut cpu);
    let gpu_parent = seed_differential_engine(&mut gpu);
    cpu.tick().unwrap();
    gpu.tick().unwrap();

    assert_eq!(
        cpu.snapshot().organisms.len(),
        gpu.snapshot().organisms.len()
    );
    let cpu_parent_snapshot = cpu.organism(cpu_parent).unwrap();
    let gpu_parent_snapshot = gpu.organism(gpu_parent).unwrap();
    assert_eq!(cpu_parent_snapshot.venom, gpu_parent_snapshot.venom);
    assert_eq!(cpu_parent_snapshot.offspring, gpu_parent_snapshot.offspring);
    assert_eq!(
        cpu_parent_snapshot.descendants,
        gpu_parent_snapshot.descendants
    );
    assert_eq!(
        cpu_parent_snapshot.last_mutation_kind,
        gpu_parent_snapshot.last_mutation_kind
    );

    let cpu_child = cpu
        .snapshot()
        .organisms
        .iter()
        .find(|organism| organism.id != cpu_parent && organism.parents.contains(&Some(cpu_parent)))
        .expect("CPU reproduction scan should publish a child");
    let gpu_child = gpu
        .snapshot()
        .organisms
        .iter()
        .find(|organism| organism.id != gpu_parent && organism.parents.contains(&Some(gpu_parent)))
        .expect("GPU reproduction scan should publish a child");
    assert_eq!(
        cpu_child.last_mutation_kind,
        Some(StandardMutationKind::Insertion)
    );
    assert_eq!(
        gpu_child.last_mutation_kind,
        Some(StandardMutationKind::Insertion)
    );
    assert_eq!(cpu_child.dna_length, gpu_child.dna_length);
    assert_eq!(cpu_child.total_mutations, gpu_child.total_mutations);
    assert_eq!(cpu_child.generation, gpu_child.generation);
    assert_eq!(cpu_child.parents, gpu_child.parents);
    assert_eq!(cpu_child.venom, gpu_child.venom);
}
