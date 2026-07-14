use darwinbots_engine::{Engine, EngineConfig, LegacyDna, SpeciesDefinition};

const ALGA_MINIMALIS: &str = include_str!("../../../Installer/bots/Alga_Minimalis_Chloroplastus.txt");
const ANIMAL_MINIMALIS: &str = include_str!("../../../Installer/bots/Animal_Minimalis.txt");

#[test]
fn starter_world_sustains_moving_predators_finite_shots_and_bounded_plants() {
    let mut engine = Engine::new(EngineConfig {
        organism_capacity: 25_000,
        vegetable_population_cap: 500,
        world_width: 16_000.0,
        world_height: 12_000.0,
        metabolism_cost: 1,
        vegetable_energy_per_tick: 0,
        ..EngineConfig::testing()
    }).unwrap();
    let alga = engine.register_species(SpeciesDefinition {
        name: "Alga Minimalis".to_owned(),
        vegetable: true,
        minimum_population: 60,
        reseed: true,
        ..SpeciesDefinition::default()
    });
    let animal = engine.register_species(SpeciesDefinition {
        name: "Animal Minimalis".to_owned(),
        minimum_population: 20,
        reseed: true,
        ..SpeciesDefinition::default()
    });
    engine.spawn_species_batch(
        LegacyDna::parse(ALGA_MINIMALIS).unwrap(),
        alga,
        random_positions(300, 16_000.0, 12_000.0, 7),
        1_000,
    ).unwrap();
    engine.spawn_species_batch(
        LegacyDna::parse(ANIMAL_MINIMALIS).unwrap(),
        animal,
        random_positions(100, 16_000.0, 12_000.0, 13),
        1_000,
    ).unwrap();

    engine.tick_many(20_000).unwrap();

    let snapshot = engine.snapshot();
    let animals: Vec<_> = snapshot.organisms.iter()
        .filter(|organism| organism.species == animal)
        .collect();
    let moving = animals.iter()
        .filter(|organism| organism.velocity[0].hypot(organism.velocity[1]) > 0.1)
        .count();
    assert!(!animals.is_empty());
    assert!(moving > animals.len() / 20);
    assert!(animals.iter().any(|organism| organism.parents[0].is_some()));
    assert!(snapshot.shots.iter().all(|shot| segment_length(shot.start, shot.end) <= 200.0));
    assert!(snapshot.shots.len() < snapshot.stats.shots_fired as usize);
    assert!(engine.vegetable_population() <= 500);
    assert!(snapshot.stats.projectile_impacts > 0);
    assert!(snapshot.stats.energy_harvested > 0);
    assert!(snapshot.stats.plant_energy_generated > 0);
    assert!(snapshot.stats.births > 0 && snapshot.stats.deaths > 0);
    assert!(snapshot.phase_timings.projectiles >= 0.0);
    assert!(snapshot.phase_timings.vegetation >= 0.0);
}

fn random_positions(count: usize, width: f32, height: f32, seed: u64) -> Vec<[f32; 2]> {
    let mut state = seed.max(1);
    (0..count).map(|_| {
        state = advance_random(state);
        let x = state as u32 as f32 / u32::MAX as f32;
        state = advance_random(state);
        let y = state as u32 as f32 / u32::MAX as f32;
        [x * width, y * height]
    }).collect()
}

fn advance_random(mut value: u64) -> u64 {
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    value.max(1)
}

fn segment_length(start: [f32; 2], end: [f32; 2]) -> f32 {
    (end[0] - start[0]).hypot(end[1] - start[1])
}
