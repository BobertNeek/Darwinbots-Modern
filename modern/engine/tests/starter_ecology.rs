use darwinbots_engine::{Engine, EngineConfig, LegacyDna, SpeciesDefinition};

const ALGA_MINIMALIS: &str = include_str!("../../../Installer/bots/Alga_Minimalis_Chloroplastus.txt");
const ANIMAL_MINIMALIS: &str = include_str!("../../../Installer/bots/Animal_Minimalis.txt");

#[test]
fn starter_ecology_remains_bounded_and_animals_move_feed_and_reproduce() {
    let mut engine = Engine::new(EngineConfig {
        organism_capacity: 25_000,
        vegetable_population_cap: 500,
        world_width: 16_000.0,
        world_height: 12_000.0,
        metabolism_cost: 1,
        vegetable_energy_per_tick: 0,
        sunlight_energy: 100,
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

    engine.tick_many(10_000).unwrap();

    let snapshot = engine.snapshot();
    assert!(engine.vegetable_population() <= 500);
    assert!(snapshot.stats.intentional_movement_events > 0);
    assert!(snapshot.stats.energy_harvested > 0);
    assert!(snapshot.organisms.iter().any(|organism| organism.species == animal && organism.parents[0].is_some()));
}

fn random_positions(count: usize, width: f32, height: f32, seed: u64) -> Vec<[f32; 2]> {
    let mut state = seed.max(1);
    (0..count).map(|_| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let x = (state as u32) as f32 / u32::MAX as f32;
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let y = (state as u32) as f32 / u32::MAX as f32;
        [x * width, y * height]
    }).collect()
}
