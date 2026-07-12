use darwinbots_engine::{Engine, EngineConfig, LegacyDna, SpeciesId};

#[test]
fn energy_death_leaves_a_decayable_corpse_in_world_state() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 1,
        ..EngineConfig::testing()
    }).unwrap();
    let dna = LegacyDna::parse("start\nstop").unwrap();
    engine.spawn_batch([(dna, [400.0, 500.0])]).unwrap();

    engine.tick_many(1_001).unwrap();

    assert_eq!(engine.population(), 0);
    assert_eq!(engine.snapshot().corpses.len(), 1);
    let corpse = &engine.snapshot().corpses[0];
    assert_eq!(corpse.position, [400.0, 500.0]);
    assert!(corpse.energy > 0);
    assert!(corpse.age > 0);

    engine.tick_many(20_000).unwrap();
    assert!(engine.snapshot().corpses.is_empty());
}

#[test]
fn overlapping_organisms_are_separated_without_non_finite_motion() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let dna = LegacyDna::parse("start\nstop").unwrap();
    engine.spawn_batch([
        (dna.clone(), [500.0, 500.0]),
        (dna, [500.0, 500.0]),
    ]).unwrap();

    engine.tick().unwrap();

    let organisms = &engine.snapshot().organisms;
    let dx = organisms[1].position[0] - organisms[0].position[0];
    let dy = organisms[1].position[1] - organisms[0].position[1];
    let distance = (dx * dx + dy * dy).sqrt();
    assert!(distance >= 4.0, "overlap remained at distance {distance}");
    assert!(organisms.iter().flat_map(|organism| organism.position).all(f32::is_finite));
    engine.validate_invariants().unwrap();
}

#[test]
fn feeding_shots_harvest_energy_from_nearby_corpses() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 1,
        ..EngineConfig::testing()
    }).unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let predator = engine.spawn_at(idle.clone(), [100.0, 100.0]).unwrap();
    engine.spawn_species_batch(idle, SpeciesId::default(), [[120.0, 100.0]], 1).unwrap();
    engine.tick().unwrap();
    let corpse_energy = engine.snapshot().corpses[0].energy;
    let predator_energy = engine.organism(predator).unwrap().energy;
    engine.replace_dna(predator, LegacyDna::parse("start\n-1 .shoot store\n40 .shootval store\nstop").unwrap()).unwrap();

    engine.tick().unwrap();

    assert!(engine.snapshot().corpses[0].energy < corpse_energy);
    assert!(engine.organism(predator).unwrap().energy > predator_energy);
    assert!(engine.snapshot().stats.energy_harvested > 0);
}
