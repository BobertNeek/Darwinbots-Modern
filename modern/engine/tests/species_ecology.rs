use darwinbots_engine::{
    sysvar_address, Engine, EngineConfig, LegacyDna, SpeciesDefinition,
};

#[test]
fn modern_chloroplast_sysvars_use_the_latest_vb6_addresses() {
    assert_eq!(sysvar_address("chlr"), Some(920));
    assert_eq!(sysvar_address("mkchlr"), Some(921));
    assert_eq!(sysvar_address("rmchlr"), Some(922));
    assert_eq!(sysvar_address("light"), Some(923));
    assert_eq!(sysvar_address("availability"), Some(923));
    assert_eq!(sysvar_address("sharechlr"), Some(924));
}

#[test]
fn species_identity_and_vegetable_energy_are_part_of_the_world_state() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 1,
        vegetable_energy_per_tick: 0,
        ..EngineConfig::testing()
    }).unwrap();
    let vegetable = engine.register_species(SpeciesDefinition {
        name: "Alga Minimalis".to_owned(),
        vegetable: true,
        color: 0xff4c963b,
        ..SpeciesDefinition::default()
    });
    let animal = engine.register_species(SpeciesDefinition {
        name: "Animal Minimalis".to_owned(),
        vegetable: false,
        color: 0xff239ac0,
        ..SpeciesDefinition::default()
    });
    let dna = LegacyDna::parse("start\nstop").unwrap();
    engine.spawn_species_at(dna.clone(), vegetable, [200.0, 200.0]).unwrap();
    engine.spawn_species_at(dna, animal, [800.0, 800.0]).unwrap();

    engine.tick().unwrap();

    let snapshot = engine.snapshot();
    assert_eq!(snapshot.world_size, [1_000.0, 1_000.0]);
    assert_eq!(snapshot.species.len(), 3);
    assert_eq!(snapshot.organisms[0].species, vegetable);
    assert!(snapshot.organisms[0].vegetable);
    assert!(snapshot.organisms[0].energy > 1_000);
    assert!(snapshot.organisms[0].body > 1_000);
    assert_eq!(snapshot.organisms[1].species, animal);
    assert!(!snapshot.organisms[1].vegetable);
    assert_eq!(snapshot.organisms[1].energy, 999);
}

#[test]
fn negative_shots_feed_the_attacker_from_the_target() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let predator = LegacyDna::parse("start\n-1 .shoot store\n100 .shootval store\nstop").unwrap();
    let prey = LegacyDna::parse("start\nstop").unwrap();
    let attacker = engine.spawn_at(predator, [100.0, 100.0]).unwrap();
    engine.spawn_at(prey, [350.0, 100.0]).unwrap();

    engine.tick().unwrap();

    assert!(engine.organism(attacker).unwrap().energy > 1_000);
    assert_eq!(engine.snapshot().stats.energy_harvested, 75);
}

#[test]
fn species_batches_publish_once_with_configured_initial_energy() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let species = engine.register_species(SpeciesDefinition {
        name: "Configured species".to_owned(),
        ..SpeciesDefinition::default()
    });
    let dna = LegacyDna::parse("start\nstop").unwrap();

    let ids = engine.spawn_species_batch(
        dna,
        species,
        [[100.0, 100.0], [200.0, 200.0], [300.0, 300.0]],
        2_500,
    ).unwrap();

    assert_eq!(ids.len(), 3);
    assert_eq!(engine.snapshot().organisms.len(), 3);
    assert!(engine.snapshot().organisms.iter().all(|organism| {
        organism.species == species && organism.energy == 2_500
    }));
}

#[test]
fn configured_species_reseed_after_extinction() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 1,
        vegetable_energy_per_tick: 0,
        ..EngineConfig::testing()
    }).unwrap();
    let species = engine.register_species(SpeciesDefinition {
        name: "Protected lineage".to_owned(),
        minimum_population: 2,
        reseed: true,
        ..SpeciesDefinition::default()
    });
    let dna = LegacyDna::parse("start\nstop").unwrap();
    engine.spawn_species_batch(dna, species, [[500.0, 500.0]], 1).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.population(), 2);
    assert!(engine.snapshot().organisms.iter().all(|organism| organism.species == species));
    assert_eq!(engine.snapshot().stats.deaths, 1);
    assert_eq!(engine.snapshot().stats.reseeds, 2);
}

#[test]
fn species_mutation_rate_applies_to_ordinary_reproduction() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let species = engine.register_species(SpeciesDefinition {
        name: "Rapid evolution".to_owned(),
        mutation_rate: 100.0,
        ..SpeciesDefinition::default()
    });
    let dna = LegacyDna::parse("start\n10 .up store\n50 .repro store\nstop").unwrap();
    let parent = engine.spawn_species_batch(dna, species, [[500.0, 500.0]], 1_000).unwrap()[0];

    engine.tick().unwrap();

    let child = engine.snapshot().organisms.iter().find(|organism| organism.id != parent).unwrap().id;
    assert_ne!(engine.dna(parent).unwrap(), engine.dna(child).unwrap());
    assert_eq!(engine.snapshot().stats.mutations, 1);
}

#[test]
fn dna_controls_body_defenses_chloroplasts_waste_and_aim() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let dna = LegacyDna::parse("start\n10 .strbody store\n25 .mkshell store\n30 .mkchlr store\n314 .setaim store\nstop").unwrap();
    let id = engine.spawn_at(dna, [500.0, 500.0]).unwrap();

    engine.tick().unwrap();

    let organism = engine.organism(id).unwrap();
    assert_eq!(organism.body, 1_010);
    assert_eq!(organism.shell, 25);
    assert_eq!(organism.chloroplasts, 30);
    assert_eq!(organism.aim, 314);
    assert!(organism.waste > 0);
    assert!(organism.energy < 1_000);
}

#[test]
fn aim_rotates_forward_movement() {
    let mut config = EngineConfig::testing();
    config.physics.density = 0.0;
    let mut engine = Engine::new(config).unwrap();
    let dna = LegacyDna::parse("start\n314 .setaim store\n10 .up store\nstop").unwrap();
    let id = engine.spawn_at(dna, [500.0, 500.0]).unwrap();

    engine.tick().unwrap();

    let position = engine.organism(id).unwrap().position;
    assert!((position[0] - 506.6).abs() < 0.01);
    assert!((position[1] - 500.0).abs() < 1.0);
}

#[test]
fn chloroplasts_generate_environmental_energy_for_any_species() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    }).unwrap();
    let builder = LegacyDna::parse("start\n160 .mkchlr store\nstop").unwrap();
    let id = engine.spawn_at(builder, [500.0, 500.0]).unwrap();
    engine.tick().unwrap();
    engine.replace_dna(id, LegacyDna::parse("start\nstop").unwrap()).unwrap();
    let before = engine.organism(id).unwrap().energy;

    engine.tick_many(4).unwrap();

    assert!(engine.organism(id).unwrap().energy > before);
}

#[test]
fn feeding_shots_without_shootval_use_legacy_body_based_strength() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let predator = LegacyDna::parse("start\n-1 .shoot store\nstop").unwrap();
    let prey = LegacyDna::parse("start\nstop").unwrap();
    let attacker = engine.spawn_at(predator, [100.0, 100.0]).unwrap();
    engine.spawn_at(prey, [350.0, 100.0]).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.snapshot().stats.energy_harvested, 165);
    assert!(engine.organism(attacker).unwrap().energy > 1_000);
}

#[test]
fn vegetable_cap_applies_to_initial_imports_and_reproduction() {
    let mut engine = Engine::new(EngineConfig {
        organism_capacity: 32,
        vegetable_population_cap: 3,
        metabolism_cost: 0,
        ..EngineConfig::testing()
    }).unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        name: "Capped vegetables".to_owned(),
        vegetable: true,
        ..SpeciesDefinition::default()
    });
    let dna = LegacyDna::parse("start\n50 .repro store\nstop").unwrap();

    let imported = engine.spawn_species_batch(
        dna,
        vegetables,
        [[100.0, 100.0], [200.0, 100.0], [300.0, 100.0], [400.0, 100.0]],
        1_000,
    ).unwrap();
    assert_eq!(imported.len(), 3);

    engine.tick().unwrap();

    assert_eq!(engine.vegetable_population(), 3);
    assert_eq!(engine.population(), 3);
}

#[test]
fn animal_minimalis_recognizes_and_moves_toward_non_family_targets() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    }).unwrap();
    let animal = LegacyDna::parse("cond\n*.eye5 0 >\n*.refeye *.myeye !=\nstart\n*.refveldx .dx store\n*.refvelup 30 add .up store\nstop\nend").unwrap();
    let alga = LegacyDna::parse("start\nstop\nend").unwrap();
    let animal_id = engine.spawn_at(animal, [500.0, 500.0]).unwrap();
    engine.spawn_at(alga, [500.0, 800.0]).unwrap();

    engine.tick().unwrap();
    assert_eq!(engine.memory_at(animal_id, 728).unwrap(), 1);
    assert_eq!(engine.memory_at(animal_id, 708).unwrap(), 0);
    engine.tick().unwrap();

    assert!(engine.organism(animal_id).unwrap().position[1] > 500.0);
}

#[test]
fn reproduction_synchronizes_energy_memory_before_the_next_dna_cycle() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        vegetable_energy_per_tick: 0,
        sunlight_energy: 0,
        ..EngineConfig::testing()
    }).unwrap();
    let dna = LegacyDna::parse("cond\n*.nrg 6000 >\nstart\n50 .repro store\nstop\nend").unwrap();
    engine.spawn_species_batch(dna, Default::default(), [[500.0, 500.0]], 7_000).unwrap();

    engine.tick().unwrap();
    assert_eq!(engine.population(), 2);
    engine.tick().unwrap();

    assert_eq!(engine.population(), 2);
}

#[test]
fn reproduction_wave_stops_cleanly_at_population_capacity() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    }).unwrap();
    let dna = LegacyDna::parse("start\n50 .repro store\nstop").unwrap();
    for index in 0..20 {
        engine.spawn_at(dna.clone(), [100.0 + index as f32 * 10.0, 500.0]).unwrap();
    }

    engine.tick().unwrap();

    assert_eq!(engine.population(), 32);
    assert_eq!(engine.snapshot().stats.births, 12);
}

#[test]
fn minus_two_shots_donate_energy_to_zerobots() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let feeder = LegacyDna::parse("start\n-2 .shoot store\n50 .shootval store\nstop").unwrap();
    let zerobot = LegacyDna::parse("0 0 0 0 0").unwrap();
    let source = engine.spawn_at(feeder, [100.0, 100.0]).unwrap();
    let target = engine.spawn_at(zerobot, [350.0, 100.0]).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.organism(source).unwrap().energy, 950);
    assert!(engine.organism(target).unwrap().energy > 1_000);
    assert_eq!(engine.snapshot().stats.energy_donated, 50);
}

#[test]
fn shot_302_forces_target_reproduction_for_zerobot_feeding() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let feeder = LegacyDna::parse("start\n302 .shoot store\n50 .shootval store\nstop").unwrap();
    let zerobot = LegacyDna::parse("0 0 0 0 0").unwrap();
    engine.spawn_at(feeder, [100.0, 100.0]).unwrap();
    engine.spawn_at(zerobot, [350.0, 100.0]).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.population(), 3);
    assert_eq!(engine.snapshot().stats.births, 1);
}

#[test]
fn shell_absorbs_feeding_damage_before_energy_is_lost() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let attacker = engine.spawn_at(idle.clone(), [100.0, 100.0]).unwrap();
    let target = engine.spawn_at(LegacyDna::parse("start\n100 .mkshell store\nstop").unwrap(), [120.0, 100.0]).unwrap();
    engine.tick().unwrap();
    engine.replace_dna(target, idle).unwrap();
    engine.replace_dna(attacker, LegacyDna::parse("start\n-1 .shoot store\n50 .shootval store\nstop").unwrap()).unwrap();
    let before = engine.organism(target).unwrap();

    engine.tick().unwrap();

    let after = engine.organism(target).unwrap();
    assert_eq!(after.energy, before.energy);
    assert_eq!(after.shell, before.shell - 50);
}

#[test]
fn poison_drains_energy_and_venom_suppresses_movement() {
    let mut poison_engine = Engine::new(EngineConfig::testing()).unwrap();
    let poisoner = poison_engine.spawn_at(LegacyDna::parse("start\n100 .strpoison store\nstop").unwrap(), [100.0, 100.0]).unwrap();
    let victim = poison_engine.spawn_at(LegacyDna::parse("start\nstop").unwrap(), [120.0, 100.0]).unwrap();
    poison_engine.tick().unwrap();
    poison_engine.replace_dna(poisoner, LegacyDna::parse("start\n-5 .shoot store\n20 .shootval store\nstop").unwrap()).unwrap();
    poison_engine.tick().unwrap();
    let poisoned_energy = poison_engine.organism(victim).unwrap().energy;
    poison_engine.tick().unwrap();
    assert!(poison_engine.organism(victim).unwrap().energy < poisoned_energy - 1);

    let mut venom_engine = Engine::new(EngineConfig::testing()).unwrap();
    let venomous = venom_engine.spawn_at(LegacyDna::parse("start\n100 .strvenom store\nstop").unwrap(), [100.0, 100.0]).unwrap();
    let mover = venom_engine.spawn_at(LegacyDna::parse("start\n10 .dx store\nstop").unwrap(), [120.0, 100.0]).unwrap();
    venom_engine.tick().unwrap();
    venom_engine.replace_dna(venomous, LegacyDna::parse("start\n-3 .shoot store\n20 .shootval store\nstop").unwrap()).unwrap();
    venom_engine.tick().unwrap();
    let after_impact = venom_engine.organism(mover).unwrap();
    assert!(after_impact.paralyzed > 0);
    venom_engine.tick().unwrap();
    assert_eq!(venom_engine.organism(mover).unwrap().position, after_impact.position);
}

#[test]
fn eye_strength_uses_db2_edge_to_edge_distance() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    }).unwrap();
    let observer = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
        .unwrap();
    engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 850.0])
        .unwrap();

    engine.tick().unwrap();

    let eye5 = engine.memory_at(observer, 505).unwrap();
    assert!((120..=122).contains(&eye5), "expected DB2 eye strength near 121, got {eye5}");
}
