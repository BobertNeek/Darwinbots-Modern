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
        vegetable_energy_per_tick: 4,
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
    engine.spawn_species_batch(LegacyDna::parse(ALGA_MINIMALIS).unwrap(), alga, grid(300, 16_000.0, 12_000.0), 1_000).unwrap();
    engine.spawn_species_batch(LegacyDna::parse(ANIMAL_MINIMALIS).unwrap(), animal, grid(100, 16_000.0, 12_000.0), 1_000).unwrap();

    engine.tick_many(10_000).unwrap();

    let snapshot = engine.snapshot();
    assert!(engine.vegetable_population() <= 500);
    assert!(snapshot.stats.intentional_movement_events > 0);
    assert!(snapshot.stats.energy_harvested > 0);
    assert!(snapshot.organisms.iter().any(|organism| organism.species == animal && organism.parents[0].is_some()));
}

fn grid(count: usize, width: f32, height: f32) -> Vec<[f32; 2]> {
    let columns = (count as f64).sqrt().ceil() as usize;
    let rows = count.div_ceil(columns);
    (0..count).map(|index| [
        width * ((index % columns) + 1) as f32 / (columns + 1) as f32,
        height * ((index / columns) + 1) as f32 / (rows + 1) as f32,
    ]).collect()
}
