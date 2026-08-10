use darwinbots_engine::{
    BackendKind, BackendPreference, Engine, EngineConfig, LegacyDna, SpeciesDefinition,
    VegetationSettings,
};

const ALGA_MINIMALIS: &str =
    include_str!("../../../Installer/bots/Alga_Minimalis_Chloroplastus.txt");
const ANIMAL_MINIMALIS: &str = include_str!("../../../Installer/bots/Animal_Minimalis.txt");

fn assert_untouched_starter_animal_moves(backend: BackendPreference) {
    let config = EngineConfig {
        backend,
        allow_cpu_fallback: backend != BackendPreference::Gpu,
        metabolism_cost: 0,
        vegetable_energy_per_tick: 0,
        sunlight_energy: 0,
        brownian_motion: 0.0,
        vegetation: VegetationSettings {
            minimum_chloroplast_equivalents: 0,
            ..VegetationSettings::default()
        },
        ..EngineConfig::default()
    };
    let mut engine = Engine::new(config).expect("starter engine should initialize");

    if backend == BackendPreference::Gpu {
        assert_eq!(engine.backend(), BackendKind::Gpu);
        let receipt = engine
            .gpu_adapter_receipt()
            .expect("GPU engine must expose its physical adapter receipt");
        assert!(!receipt.name.is_empty());
        assert!(!receipt.backend.is_empty());
        println!(
            "physical gpu adapter={} backend={} device_type={}",
            receipt.name, receipt.backend, receipt.device_type
        );
    }

    let alga = engine.register_species(SpeciesDefinition {
        name: "Alga Minimalis".to_owned(),
        vegetable: true,
        ..SpeciesDefinition::default()
    });
    let animal = engine.register_species(SpeciesDefinition {
        name: "Animal Minimalis".to_owned(),
        ..SpeciesDefinition::default()
    });
    engine
        .spawn_species_at(
            LegacyDna::parse(ALGA_MINIMALIS).expect("bundled alga DNA should parse"),
            alga,
            [500.0, 1_400.0],
        )
        .expect("alga should spawn");
    let animal_id = engine
        .spawn_species_at(
            LegacyDna::parse(ANIMAL_MINIMALIS).expect("bundled animal DNA should parse"),
            animal,
            [500.0, 500.0],
        )
        .expect("animal should spawn");

    let before = engine
        .organism(animal_id)
        .expect("animal should be present")
        .position;
    engine.tick().expect("first starter tick should succeed");
    engine.tick().expect("second starter tick should succeed");
    engine.tick().expect("third starter tick should succeed");
    let snapshot = engine.snapshot();
    let after = snapshot
        .organisms
        .iter()
        .find(|organism| organism.id == animal_id)
        .expect("animal should remain present")
        .position;
    let displacement = (after[0] - before[0]).hypot(after[1] - before[1]);

    assert!(
        displacement > 0.0,
        "Animal Minimalis stayed at {before:?} without any drag input"
    );
    assert!(snapshot.stats.intentional_movement_events > 0);
}

#[test]
fn untouched_starter_animal_moves_on_cpu() {
    assert_untouched_starter_animal_moves(BackendPreference::Cpu);
}

#[test]
fn untouched_starter_animal_moves_on_physical_gpu() {
    assert_untouched_starter_animal_moves(BackendPreference::Gpu);
}
