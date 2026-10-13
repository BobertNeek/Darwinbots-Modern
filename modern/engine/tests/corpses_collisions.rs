use darwinbots_engine::{
    CorpseDecayMode, CorpseDecaySettings, Engine, EngineConfig, LegacyDna, SaveFile, ShotSettings,
    SpeciesId, VegetationSettings, WasteSettings,
};

fn corpse_settings() -> CorpseDecaySettings {
    CorpseDecaySettings {
        enabled: true,
        death_energy_threshold: 15,
        ..Default::default()
    }
}

fn body_amount(engine: &Engine) -> f32 {
    let save = SaveFile::encode(engine).unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&save[10..]).unwrap();
    let corpse = payload["slots"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|slot| slot["organism"].as_object())
        .find(|robot| robot["flags"]["corpse"] == true)
        .unwrap();
    corpse["materials"]["body"].as_f64().unwrap() as f32
}

#[test]
fn energy_death_leaves_a_decayable_corpse_in_world_state() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 1,
        waste: WasteSettings {
            alzheimer_threshold: -1,
            defecation_threshold: i32::MAX,
        },
        corpse_decay: CorpseDecaySettings {
            mode: CorpseDecayMode::EnergyShot,
            packet_size: 100,
            period: 50,
            ..corpse_settings()
        },
        vegetation: VegetationSettings {
            minimum_chloroplast_equivalents: 0,
            ..Default::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let dna = LegacyDna::parse("start\nstop").unwrap();
    engine.spawn_batch([(dna, [400.0, 500.0])]).unwrap();
    engine.update_mutation_settings(false, 0.0, &[]).unwrap();

    engine.tick_many(1_001).unwrap();

    assert_eq!(engine.population(), 0);
    assert_eq!(engine.snapshot().corpses.len(), 1);
    let corpse = &engine.snapshot().corpses[0];
    assert_eq!(corpse.position, [400.0, 500.0]);
    assert_eq!(corpse.energy, 0);
    assert!(corpse.body > 0);
    assert!(corpse.age > 0);

    engine.tick_many(20_000).unwrap();
    assert!(engine.snapshot().corpses.is_empty());
}

#[test]
fn configured_corpse_decay_changes_authoritative_lifecycle_state_after_tick() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 1,
        corpse_decay: CorpseDecaySettings {
            mode: CorpseDecayMode::EnergyShot,
            packet_size: 3,
            period: 1,
            ..corpse_settings()
        },
        vegetation: VegetationSettings {
            minimum_chloroplast_equivalents: 0,
            ..Default::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    engine
        .spawn_species_batch(
            LegacyDna::parse("start\nstop").unwrap(),
            SpeciesId::default(),
            [[400.0, 500.0]],
            1,
        )
        .unwrap();
    engine.tick().unwrap();
    let corpse_body_before = body_amount(&engine);

    engine.tick().unwrap();

    let corpse = &engine.snapshot().corpses[0];
    assert_eq!(corpse.energy, 0);
    assert!((body_amount(&engine) - (corpse_body_before - 0.3)).abs() < 0.0001);
    assert!(
        engine
            .snapshot()
            .shots
            .iter()
            .any(|shot| shot.kind == -2 && shot.value == 3)
    );
}

#[test]
fn energy_below_configured_corpse_threshold_becomes_corpse_after_age_zero() {
    let mut engine = Engine::new(EngineConfig {
        corpse_decay: CorpseDecaySettings {
            death_energy_threshold: 15,
            ..corpse_settings()
        },
        vegetation: VegetationSettings {
            minimum_chloroplast_equivalents: 0,
            ..Default::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let dna = LegacyDna::parse("start\nstop").unwrap();
    engine
        .spawn_species_batch(dna, SpeciesId::default(), [[400.0, 500.0]], 14)
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.population(), 0);
    assert_eq!(engine.snapshot().corpses.len(), 1);
}

#[test]
fn overlapping_organisms_are_separated_without_non_finite_motion() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let dna = LegacyDna::parse("start\nstop").unwrap();
    engine
        .spawn_batch([(dna.clone(), [500.0, 500.0]), (dna, [500.0, 500.0])])
        .unwrap();

    engine.tick().unwrap();

    let organisms = &engine.snapshot().organisms;
    let dx = organisms[1].position[0] - organisms[0].position[0];
    let dy = organisms[1].position[1] - organisms[0].position[1];
    let distance = (dx * dx + dy * dy).sqrt();
    assert!(distance >= 4.0, "overlap remained at distance {distance}");
    assert!(
        organisms
            .iter()
            .flat_map(|organism| organism.position)
            .all(f32::is_finite)
    );
    engine.validate_invariants().unwrap();
}

#[test]
fn energy_feeding_cannot_extract_energy_from_ordinary_corpses() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        corpse_decay: corpse_settings(),
        vegetation: VegetationSettings {
            minimum_chloroplast_equivalents: 0,
            ..Default::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let predator = engine.spawn_at(idle.clone(), [250.0, 500.0]).unwrap();
    engine
        .spawn_species_batch(idle, SpeciesId::default(), [[500.0, 500.0]], 1)
        .unwrap();
    engine.tick().unwrap();
    let corpse_energy = engine.snapshot().corpses[0].energy;
    let predator_energy = engine.organism(predator).unwrap().energy;
    engine
        .replace_dna(
            predator,
            LegacyDna::parse("start\n-1 .shoot store\n40 .shootval store\nstop").unwrap(),
        )
        .unwrap();

    engine.tick().unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.snapshot().corpses[0].energy, corpse_energy);
    assert_eq!(engine.organism(predator).unwrap().energy, predator_energy);
    assert_eq!(engine.snapshot().stats.energy_harvested, 0);
}

#[test]
fn body_feeding_shots_multiply_corpse_damage_and_return_a_packet() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        corpse_decay: corpse_settings(),
        shots: ShotSettings {
            feeding_proportional: false,
            feeding_fixed_power: 100,
            ..Default::default()
        },
        vegetation: VegetationSettings {
            minimum_chloroplast_equivalents: 0,
            ..Default::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let attacker = engine.spawn_at(idle.clone(), [250.0, 500.0]).unwrap();
    engine
        .spawn_species_batch(idle, SpeciesId::default(), [[500.0, 500.0]], 1)
        .unwrap();
    engine.tick().unwrap();
    let corpse_id = engine
        .snapshot()
        .organisms
        .iter()
        .find(|robot| robot.flags.corpse)
        .unwrap()
        .id;
    engine
        .replace_dna(
            attacker,
            LegacyDna::parse("start\n-6 .shoot store\n100 .shootval store\nstop").unwrap(),
        )
        .unwrap();

    engine.tick().unwrap();
    engine
        .replace_dna(attacker, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();

    // releasebod explicitly sets Dead when nrg <= 0.5, including corpses.
    assert!(engine.snapshot().corpses.is_empty());
    assert!(engine.organism(corpse_id).is_err());
    assert_eq!(engine.organism(attacker).unwrap().kills, 1);
    assert!(
        engine
            .snapshot()
            .shots
            .iter()
            .any(|shot| { shot.kind == -2 && shot.owner == corpse_id && shot.value == 400 })
    );
}

#[test]
fn energy_donation_does_not_revive_or_feed_a_corpse() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        corpse_decay: CorpseDecaySettings {
            packet_size: 0,
            ..corpse_settings()
        },
        vegetation: VegetationSettings {
            minimum_chloroplast_equivalents: 0,
            ..Default::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let source = engine.spawn_at(idle.clone(), [250.0, 500.0]).unwrap();
    engine
        .spawn_species_batch(idle, SpeciesId::default(), [[500.0, 500.0]], 1)
        .unwrap();
    engine.tick().unwrap();
    let body = body_amount(&engine);
    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n-2 .shoot store\n100 .shootval store\nstop").unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(source, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().corpses[0].energy, 0);
    assert_eq!(body_amount(&engine), body);
    assert_eq!(engine.snapshot().stats.energy_donated, 0);
    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
}
