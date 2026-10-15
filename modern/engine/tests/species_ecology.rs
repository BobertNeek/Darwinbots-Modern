use darwinbots_engine::{
    Engine, EngineConfig, LegacyDna, OrganismId, SaveFile, SpeciesDefinition, WasteSettings,
    sysvar_address,
};

// Keep 1000-body fixture robots (radius 114.28) clear of native borders.
const TOXIN_SOURCE_POSITION: [f32; 2] = [300.0, 500.0];
const TOXIN_TARGET_POSITION: [f32; 2] = [550.0, 500.0];
const FRACTIONAL_TOXIN_TARGET_POSITION: [f32; 2] = [600.0, 500.0];
const DEFAULT_ROBOT_RADII_SUM: f32 = 228.56;

fn assert_positive_toxin_edge_gap(target_position: [f32; 2]) {
    assert_eq!(target_position[1], TOXIN_SOURCE_POSITION[1]);
    let center_gap = target_position[0] - TOXIN_SOURCE_POSITION[0];
    assert!(center_gap == 250.0 || center_gap == 300.0);
    assert!(center_gap - DEFAULT_ROBOT_RADII_SUM > 0.0);
}

fn restore_with_material(engine: &Engine, id: OrganismId, field: &str, value: f32) -> Engine {
    let encoded = SaveFile::encode(engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    payload["slots"][id.slot() as usize]["organism"]["materials"][field] = serde_json::json!(value);
    let payload = serde_json::to_vec(&payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(&encoded[..6]);
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    SaveFile::decode(&save).unwrap()
}

fn restore_with_toxin_motion(
    engine: &Engine,
    source: OrganismId,
    source_actual_velocity: [f32; 2],
    target: OrganismId,
    target_actual_velocity: [f32; 2],
    incoming_velocity: [f32; 2],
) -> Engine {
    let encoded = SaveFile::encode(engine).unwrap();
    assert_eq!(u16::from_le_bytes([encoded[4], encoded[5]]), 4);
    let mut payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    payload["slots"][source.slot() as usize]["organism"]["kinematics"]["actual_velocity"] =
        serde_json::json!(source_actual_velocity);
    payload["slots"][target.slot() as usize]["organism"]["kinematics"]["actual_velocity"] =
        serde_json::json!(target_actual_velocity);
    payload["projectiles"]["velocities"][0] = serde_json::json!(incoming_velocity);
    let payload = serde_json::to_vec(&payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(&encoded[..6]);
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    SaveFile::decode(&save).unwrap()
}

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
    })
    .unwrap();
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
    engine
        .spawn_species_at(dna.clone(), vegetable, [200.0, 200.0])
        .unwrap();
    engine
        .spawn_species_at(dna, animal, [800.0, 800.0])
        .unwrap();

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
fn body_conversion_uses_db2_energy_units_and_cycle_caps() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let store = engine
        .spawn_at(
            LegacyDna::parse("start\n50 .strbody store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let feed = engine
        .spawn_at(
            LegacyDna::parse("start\n50 .fdbody store\nstop").unwrap(),
            [500.0, 500.0],
        )
        .unwrap();
    let store_before = engine.organism(store).unwrap();
    let feed_before = engine.organism(feed).unwrap();

    engine.tick().unwrap();

    let store_after = engine.organism(store).unwrap();
    let feed_after = engine.organism(feed).unwrap();
    assert_eq!(store_after.energy, store_before.energy - 50);
    assert_eq!(store_after.body, store_before.body + 5);
    assert_eq!(feed_after.energy, feed_before.energy + 50);
    assert_eq!(feed_after.body, feed_before.body - 5);
    assert_eq!(engine.memory(store, "strbody").unwrap(), 0);
    assert_eq!(engine.memory(feed, "fdbody").unwrap(), 0);
}

#[test]
fn negative_shots_feed_the_attacker_from_the_target() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let predator = LegacyDna::parse("start\n-1 .shoot store\n100 .shootval store\nstop").unwrap();
    let prey = LegacyDna::parse("start\nstop").unwrap();
    let attacker = engine.spawn_at(predator, TOXIN_SOURCE_POSITION).unwrap();
    engine.spawn_at(prey, TOXIN_TARGET_POSITION).unwrap();

    engine.tick().unwrap();
    engine
        .replace_dna(attacker, LegacyDna::parse("stop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    engine.tick().unwrap();

    assert!(engine.organism(attacker).unwrap().energy > 1_000);
    assert_eq!(engine.snapshot().stats.energy_harvested, 950);
}

#[test]
fn waste_phase_runs_alzheimer_and_defecation_after_waste_shot() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        waste: WasteSettings {
            alzheimer_threshold: -1,
            defecation_threshold: i32::MAX,
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let source = engine
        .spawn_at(
            LegacyDna::parse("start\n-4 .shoot store\n32000 .shootval store\nstop").unwrap(),
            TOXIN_SOURCE_POSITION,
        )
        .unwrap();
    let target = engine
        .spawn_at(
            LegacyDna::parse("start\nstop").unwrap(),
            TOXIN_TARGET_POSITION,
        )
        .unwrap();
    engine = restore_with_material(&engine, source, "waste", 32_000.0);
    engine.tick().unwrap();
    engine
        .replace_dna(source, LegacyDna::parse("stop").unwrap())
        .unwrap();
    // Enable the target response after the source has emitted a controlled shot.
    let encoded = SaveFile::encode(&engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    payload["config"]["waste"]["alzheimer_threshold"] = serde_json::json!(1);
    payload["config"]["waste"]["defecation_threshold"] = serde_json::json!(10);
    let payload = serde_json::to_vec(&payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(&encoded[..6]);
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    engine = SaveFile::decode(&save).unwrap();
    let before_memory = engine.memory_at(target, 500).unwrap();

    engine.tick().unwrap();

    let after = engine.robot_state(target).unwrap();
    assert!(after.materials.waste < 32_000.0);
    assert!(after.materials.permanent_waste > 0.0);
    assert_ne!(engine.memory_at(target, 500).unwrap(), before_memory);
    assert!(
        engine
            .snapshot()
            .shots
            .iter()
            .any(|shot| shot.owner == target && shot.kind == -4 && shot.value == 200)
    );
    assert!(engine.snapshot().stats.shots_fired > 1);
    assert!(engine.organism(source).unwrap().waste < 320);
}

#[test]
fn negative_alzheimer_threshold_disables_memory_mutation() {
    let mut base = Engine::new(EngineConfig {
        metabolism_cost: 0,
        waste: WasteSettings {
            alzheimer_threshold: -1,
            defecation_threshold: i32::MAX,
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let owner = base
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [100.0, 100.0])
        .unwrap();

    let make_engine = |waste| {
        let encoded = SaveFile::encode(&base).unwrap();
        let mut payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
        payload["slots"][owner.slot() as usize]["organism"]["materials"]["waste"] =
            serde_json::json!(waste);
        let payload = serde_json::to_vec(&payload).unwrap();
        let mut save = Vec::with_capacity(10 + payload.len());
        save.extend_from_slice(&encoded[..6]);
        save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        save.extend_from_slice(&payload);
        SaveFile::decode(&save).unwrap()
    };
    let mut high_waste = make_engine(4_000);
    let mut low_waste = make_engine(0);
    high_waste.tick().unwrap();
    low_waste.tick().unwrap();

    assert_eq!(
        high_waste.memory_at(owner, 500).unwrap(),
        low_waste.memory_at(owner, 500).unwrap()
    );
    let encoded = SaveFile::encode(&high_waste).unwrap();
    let high_payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    let encoded = SaveFile::encode(&low_waste).unwrap();
    let low_payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    assert_eq!(
        high_payload["slots"][owner.slot() as usize]["organism"]["program"]["rng_state"],
        low_payload["slots"][owner.slot() as usize]["organism"]["program"]["rng_state"]
    );
}

#[test]
fn automatic_overflow_defecation_subtracts_the_emitted_waste_packet() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        dna_costs: darwinbots_engine::DnaCosts {
            shell: 1.0,
            ..Default::default()
        },
        waste: WasteSettings {
            alzheimer_threshold: 0,
            defecation_threshold: 32_000,
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let owner = engine
        .spawn_at(
            LegacyDna::parse("start\n100 .mkshell store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();

    let encoded = SaveFile::encode(&engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    let robot = &mut payload["slots"][owner.slot() as usize]["organism"];
    robot["materials"]["waste"] = serde_json::json!(32_000);
    robot["materials"]["permanent_waste"] = serde_json::json!(7);
    let payload = serde_json::to_vec(&payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(&encoded[..6]);
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    let mut engine = SaveFile::decode(&save).unwrap();

    engine.tick().unwrap();

    let after = engine.robot_state(owner).unwrap();
    assert_eq!(after.materials.waste, 31_000.0);
    assert_eq!(after.materials.permanent_waste, 7.5);
    assert_eq!(engine.snapshot().tick, 1);
    let packet = engine
        .snapshot()
        .shots
        .iter()
        .find(|shot| shot.owner == owner && shot.kind == -4 && shot.value == 500)
        .expect("automatic defecation packet missing");
    assert_eq!(packet.age, 0);
}

#[test]
fn species_batches_publish_once_with_configured_initial_energy() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let species = engine.register_species(SpeciesDefinition {
        name: "Configured species".to_owned(),
        ..SpeciesDefinition::default()
    });
    let dna = LegacyDna::parse("start\nstop").unwrap();

    let ids = engine
        .spawn_species_batch(
            dna,
            species,
            [[100.0, 100.0], [200.0, 200.0], [300.0, 300.0]],
            2_500,
        )
        .unwrap();

    assert_eq!(ids.len(), 3);
    assert_eq!(engine.snapshot().organisms.len(), 3);
    assert!(
        engine
            .snapshot()
            .organisms
            .iter()
            .all(|organism| { organism.species == species && organism.energy == 2_500 })
    );
}

#[test]
fn configured_species_reseed_after_extinction() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 1,
        vegetable_energy_per_tick: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let species = engine.register_species(SpeciesDefinition {
        name: "Protected lineage".to_owned(),
        minimum_population: 2,
        reseed: true,
        ..SpeciesDefinition::default()
    });
    let dna = LegacyDna::parse("start\nstop").unwrap();
    engine
        .spawn_species_batch(dna, species, [[500.0, 500.0]], 1)
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.population(), 2);
    assert!(
        engine
            .snapshot()
            .organisms
            .iter()
            .all(|organism| organism.species == species)
    );
    assert_eq!(engine.snapshot().stats.deaths, 1);
    assert_eq!(engine.snapshot().stats.reseeds, 2);
}

#[test]
fn species_native_copy_error_profile_applies_to_ordinary_reproduction() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let mut profile = darwinbots_engine::StandardMutationProfile::default();
    profile.point.one_in = 0.0;
    profile.delta.one_in = 0.0;
    profile.copy_error.one_in = 1.0;
    profile.copy_error.mean = 1.0;
    profile.copy_error.std_dev = 0.0;
    profile.insertion.one_in = 0.0;
    profile.reversal.one_in = 0.0;
    profile.major_deletion.one_in = 0.0;
    profile.minor_deletion.one_in = 0.0;
    let species = engine.register_species(SpeciesDefinition {
        name: "Rapid evolution".to_owned(),
        mutation_profile: Some(profile),
        ..SpeciesDefinition::default()
    });
    let dna = LegacyDna::parse("start\n10 .up store\n50 .repro store\nstop").unwrap();
    let parent = engine
        .spawn_species_batch(dna, species, [[500.0, 500.0]], 1_000)
        .unwrap()[0];

    engine.tick().unwrap();

    let child = engine
        .snapshot()
        .organisms
        .iter()
        .find(|organism| organism.id != parent)
        .unwrap()
        .id;
    assert_ne!(engine.dna(parent).unwrap(), engine.dna(child).unwrap());
    assert!(engine.snapshot().stats.mutations > 0);
    assert_eq!(
        engine
            .organism(child)
            .unwrap()
            .mutation_class_counts
            .values()
            .sum::<u64>(),
        engine.snapshot().stats.mutations
    );
}

#[test]
fn dna_controls_body_defenses_chloroplasts_waste_and_aim() {
    let mut engine = Engine::new(EngineConfig {
        dna_costs: darwinbots_engine::DnaCosts {
            shell: 0.1,
            ..Default::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let dna = LegacyDna::parse(
        "start\n10 .strbody store\n25 .mkshell store\n30 .mkchlr store\n314 .setaim store\nstop",
    )
    .unwrap();
    let id = engine.spawn_at(dna, [500.0, 500.0]).unwrap();

    engine.tick().unwrap();

    let organism = engine.organism(id).unwrap();
    assert_eq!(organism.body, 1_001);
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
    let aim = LegacyDna::parse("start\n314 .setaim store\nstop").unwrap();
    let id = engine.spawn_at(aim, [500.0, 500.0]).unwrap();

    engine.tick().unwrap();
    engine
        .replace_dna(id, LegacyDna::parse("start\n10 .up store\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();

    let position = engine.organism(id).unwrap().position;
    assert!((position[0] - 500.0).abs() < 0.01);
    assert!((position[1] - 493.4).abs() < 0.01);
}

#[test]
fn chloroplasts_generate_environmental_energy_for_any_species() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let builder = LegacyDna::parse("start\n160 .mkchlr store\nstop").unwrap();
    let id = engine.spawn_at(builder, [500.0, 500.0]).unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(id, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    let before = engine.organism(id).unwrap().energy;

    engine.tick_many(4).unwrap();

    assert!(engine.organism(id).unwrap().energy > before);
}

#[test]
fn feeding_shots_without_shootval_use_legacy_body_based_strength() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let predator = LegacyDna::parse("start\n-1 .shoot store\nstop").unwrap();
    let prey = LegacyDna::parse("start\nstop").unwrap();
    let attacker = engine.spawn_at(predator, TOXIN_SOURCE_POSITION).unwrap();
    engine.spawn_at(prey, TOXIN_TARGET_POSITION).unwrap();

    engine.tick().unwrap();
    assert_eq!(engine.snapshot().shots[0].value, 220, "20 + body / 5");
    engine
        .replace_dna(attacker, LegacyDna::parse("stop").unwrap())
        .unwrap();
    for _ in 0..16 {
        engine.tick().unwrap();
        if engine.snapshot().stats.feeding_events > 0 {
            break;
        }
    }

    assert!(engine.snapshot().stats.energy_harvested > 0);
    assert!(engine.organism(attacker).unwrap().energy > 1_000);
}

#[test]
fn vegetable_cap_applies_to_initial_imports_and_reproduction() {
    let mut engine = Engine::new(EngineConfig {
        organism_capacity: 32,
        vegetable_population_cap: 3,
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        name: "Capped vegetables".to_owned(),
        vegetable: true,
        ..SpeciesDefinition::default()
    });
    let dna = LegacyDna::parse("start\n50 .repro store\nstop").unwrap();

    let imported = engine
        .spawn_species_batch(
            dna,
            vegetables,
            [
                [100.0, 100.0],
                [200.0, 100.0],
                [300.0, 100.0],
                [400.0, 100.0],
            ],
            1_000,
        )
        .unwrap();
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
    })
    .unwrap();
    let animal = LegacyDna::parse("cond\n*.eye5 0 >\n*.refeye *.myeye !=\nstart\n*.refveldx .dx store\n*.refvelup 30 add .up store\nstop\nend").unwrap();
    let alga = LegacyDna::parse("start\nstop\nend").unwrap();
    let animal_id = engine.spawn_at(animal, [500.0, 500.0]).unwrap();
    engine.spawn_at(alga, [800.0, 500.0]).unwrap();

    engine.tick().unwrap();
    assert!(
        engine.memory_at(animal_id, 505).unwrap() > 0,
        "actor {:?}, eye {:?}, flags {:?}",
        engine.organism(animal_id).unwrap().position,
        engine.memory_at(animal_id, 505),
        engine.robot_state(animal_id).unwrap().flags
    );
    assert_eq!(engine.memory_at(animal_id, 728).unwrap(), 1);
    assert_eq!(engine.memory_at(animal_id, 708).unwrap(), 0);
    engine.tick().unwrap();

    assert!(engine.organism(animal_id).unwrap().position[0] > 500.0);
}

#[test]
fn reproduction_synchronizes_energy_memory_before_the_next_dna_cycle() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        vegetable_energy_per_tick: 0,
        sunlight_energy: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let dna = LegacyDna::parse("cond\n*.nrg 6000 >\nstart\n50 .repro store\nstop\nend").unwrap();
    engine
        .spawn_species_batch(dna, Default::default(), [[500.0, 500.0]], 7_000)
        .unwrap();

    engine.tick().unwrap();
    assert_eq!(engine.population(), 2);
    engine.tick().unwrap();

    assert_eq!(engine.population(), 2);
}

#[test]
fn reproduction_wave_stops_cleanly_at_population_capacity() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        world_width: 5_000.0,
        world_height: 4_000.0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let dna = LegacyDna::parse("start\n50 .repro store\nstop").unwrap();
    for index in 0..20 {
        engine
            .spawn_at(
                dna.clone(),
                [
                    500.0 + (index % 5) as f32 * 800.0,
                    500.0 + (index / 5) as f32 * 800.0,
                ],
            )
            .unwrap();
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
    let source = engine.spawn_at(feeder, TOXIN_SOURCE_POSITION).unwrap();
    let target = engine.spawn_at(zerobot, TOXIN_TARGET_POSITION).unwrap();

    engine.tick().unwrap();
    engine
        .replace_dna(source, LegacyDna::parse("stop").unwrap())
        .unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.organism(source).unwrap().energy, 950);
    assert!(engine.organism(target).unwrap().energy > 1_000);
    assert_eq!(engine.snapshot().stats.energy_donated, 47);
}

#[test]
fn shot_300_forces_target_reproduction_for_zerobot_feeding() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let feeder = LegacyDna::parse("start\n300 .shoot store\n50 .shootval store\nstop").unwrap();
    let zerobot = LegacyDna::parse("0 0 0 0 0").unwrap();
    engine.spawn_at(feeder, [300.0, 500.0]).unwrap();
    engine.spawn_at(zerobot, [550.0, 500.0]).unwrap();

    engine.tick().unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.population(), 3);
    assert_eq!(engine.snapshot().stats.births, 1);
}

#[test]
fn feeding_energy_shots_ignore_shell() {
    let mut engine = Engine::new(EngineConfig {
        shots: darwinbots_engine::ShotSettings {
            feeding_proportional: false,
            feeding_fixed_power: 50,
            ..Default::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let attacker = engine
        .spawn_at(idle.clone(), TOXIN_SOURCE_POSITION)
        .unwrap();
    let target = engine
        .spawn_at(
            LegacyDna::parse("start\n100 .mkshell store\nstop").unwrap(),
            TOXIN_TARGET_POSITION,
        )
        .unwrap();
    engine.tick().unwrap();
    engine.replace_dna(target, idle.clone()).unwrap();
    engine
        .replace_dna(
            attacker,
            LegacyDna::parse("start\n-1 .shoot store\n50 .shootval store\nstop").unwrap(),
        )
        .unwrap();
    let attacker_before = engine.organism(attacker).unwrap();
    let target_before = engine.organism(target).unwrap();

    engine.tick().unwrap();
    engine.replace_dna(attacker, idle).unwrap();
    for _ in 0..16 {
        engine.tick().unwrap();
        if engine.snapshot().stats.feeding_events > 0 {
            break;
        }
    }

    let attacker_after = engine.organism(attacker).unwrap();
    let target_after = engine.organism(target).unwrap();
    assert_eq!(target_after.energy, target_before.energy - 45);
    assert_eq!(target_after.shell, target_before.shell);
    assert_eq!(attacker_after.energy, attacker_before.energy + 48);
    assert_eq!(engine.snapshot().stats.shots_fired, 1);
    assert_eq!(engine.snapshot().stats.projectile_impacts, 2);
    assert_eq!(engine.snapshot().stats.projectile_effects, 2);
    assert_eq!(engine.snapshot().stats.energy_harvested, 47);
    assert_eq!(engine.snapshot().stats.feeding_events, 1);
}

#[test]
fn body_feeding_shots_consume_shell_before_body() {
    let mut engine = Engine::new(EngineConfig {
        shots: darwinbots_engine::ShotSettings {
            feeding_proportional: false,
            feeding_fixed_power: 40,
            ..Default::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let attacker = engine
        .spawn_at(idle.clone(), TOXIN_SOURCE_POSITION)
        .unwrap();
    let target = engine
        .spawn_at(
            LegacyDna::parse("start\n100 .mkshell store\nstop").unwrap(),
            TOXIN_TARGET_POSITION,
        )
        .unwrap();
    engine.tick().unwrap();
    engine.replace_dna(target, idle.clone()).unwrap();
    engine
        .replace_dna(
            attacker,
            LegacyDna::parse("start\n-6 .shoot store\n40 .shootval store\nstop").unwrap(),
        )
        .unwrap();
    let attacker_before = engine.organism(attacker).unwrap();
    let target_before = engine.organism(target).unwrap();

    engine.tick().unwrap();
    engine.replace_dna(attacker, idle).unwrap();
    engine.tick().unwrap();

    let attacker_after = engine.organism(attacker).unwrap();
    let target_after = engine.organism(target).unwrap();
    assert_eq!(target_after.energy, target_before.energy);
    assert_eq!(target_after.body, target_before.body);
    assert_eq!(target_after.shell, target_before.shell - 2);
    assert_eq!(attacker_after.energy, attacker_before.energy);
    assert_eq!(engine.snapshot().stats.shots_fired, 1);
    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
    assert_eq!(engine.snapshot().stats.projectile_effects, 1);
    assert_eq!(engine.snapshot().stats.energy_harvested, 0);
    assert_eq!(engine.snapshot().stats.feeding_events, 0);
}

#[test]
fn body_feeding_shots_use_body_coefficients_and_return_energy_from_target() {
    let mut engine = Engine::new(EngineConfig {
        shots: darwinbots_engine::ShotSettings {
            feeding_proportional: false,
            feeding_fixed_power: 100,
            ..Default::default()
        },
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let attacker = engine
        .spawn_at(idle.clone(), TOXIN_SOURCE_POSITION)
        .unwrap();
    let target = engine
        .spawn_at(idle.clone(), TOXIN_TARGET_POSITION)
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(
            attacker,
            LegacyDna::parse("start\n-6 .shoot store\n100 .shootval store\nstop").unwrap(),
        )
        .unwrap();
    let attacker_before = engine.organism(attacker).unwrap();
    let target_before = engine.organism(target).unwrap();

    engine.tick().unwrap();
    engine.replace_dna(attacker, idle).unwrap();
    engine.tick().unwrap();

    let attacker_after_impact = engine.organism(attacker).unwrap();
    let target_after = engine.organism(target).unwrap();
    assert_eq!(target_after.energy, target_before.energy - 20);
    assert_eq!(target_after.body, target_before.body - 8);
    assert_eq!(attacker_after_impact.energy, attacker_before.energy);
    assert!(
        engine
            .snapshot()
            .shots
            .iter()
            .any(|shot| { shot.kind == -2 && shot.owner == target && shot.value == 100 })
    );

    engine.tick_many(8).unwrap();
    assert!(engine.organism(attacker).unwrap().energy > attacker_before.energy);
}

#[test]
fn direct_dna_poison_request_does_not_fire_a_projectile() {
    for requested_kind in [-5, -13] {
        let mut engine = Engine::new(EngineConfig::testing()).unwrap();
        engine
            .spawn_at(
                LegacyDna::parse(&format!(
                    "start\n100 .strpoison store\n{requested_kind} .shoot store\n20 .shootval store\nstop"
                ))
                .unwrap(),
                [100.0, 100.0],
            )
            .unwrap();

        engine.tick().unwrap();

        assert_eq!(
            engine.snapshot().stats.shots_fired,
            0,
            "DNA request {requested_kind} normalized into a poison shot"
        );
        assert!(engine.snapshot().shots.is_empty());
    }
}

#[test]
fn heterospecific_venom_forces_memory_without_energy_drain_or_movement_suppression() {
    assert_positive_toxin_edge_gap(TOXIN_TARGET_POSITION);
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let source_species = engine.register_species(SpeciesDefinition {
        name: "Venom source".to_owned(),
        ..SpeciesDefinition::default()
    });
    let target_species = engine.register_species(SpeciesDefinition {
        name: "Venom target".to_owned(),
        ..SpeciesDefinition::default()
    });
    let source = engine
        .spawn_species_at(
            LegacyDna::parse("start\n8 .strvenom store\nstop").unwrap(),
            source_species,
            TOXIN_SOURCE_POSITION,
        )
        .unwrap();
    let target = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            target_species,
            TOXIN_TARGET_POSITION,
        )
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(
            source,
            LegacyDna::parse(
                "start\n-3 .shoot store\n8 .shootval store\n500 .vloc store\n77 .venval store\nstop",
            )
            .unwrap(),
        )
        .unwrap();
    let energy_before = engine.organism(target).unwrap().energy;
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
    engine
        .replace_dna(source, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();

    let snapshot = engine.snapshot();
    assert_eq!(
        snapshot.stats.projectile_impacts, 1,
        "live shots: {:?}",
        snapshot.shots
    );
    let after = engine.organism(target).unwrap();
    assert_eq!(after.energy, energy_before);
    // ShotUpdate delivers power 8 at age zero; Poisons decrements once.
    assert_eq!(after.paralyzed, 7);
    assert_eq!(engine.memory_at(target, 500).unwrap(), 77);
    assert_eq!(engine.organism(source).unwrap().venom, 0);

    let position_before_move = after.position;
    engine
        .replace_dna(
            target,
            LegacyDna::parse("start\n10 .dx store\nstop").unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();

    let after_move = engine.organism(target).unwrap();
    assert_eq!(after_move.energy, energy_before);
    assert_eq!(after_move.paralyzed, 6);
    assert_ne!(after_move.position, position_before_move);
}

#[test]
fn information_defense_applies_vb_cint_to_reversed_velocity() {
    assert_positive_toxin_edge_gap(TOXIN_TARGET_POSITION);
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        gravity: [0.0, 0.0],
        drag: 0.0,
        brownian_motion: 0.0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let attacker = engine
        .spawn_at(idle.clone(), TOXIN_SOURCE_POSITION)
        .unwrap();
    let defender = engine
        .spawn_at(
            LegacyDna::parse("start\n220 .strpoison store\nstop").unwrap(),
            TOXIN_TARGET_POSITION,
        )
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(
            attacker,
            LegacyDna::parse("start\n500 .shoot store\n77 .shootval store\nstop").unwrap(),
        )
        .unwrap();
    engine.replace_dna(defender, idle.clone()).unwrap();

    engine.tick().unwrap();
    engine = restore_with_toxin_motion(
        &engine,
        attacker,
        [0.5, -2.5],
        defender,
        [1.25, -0.75],
        [40.5, -2.5],
    );
    assert_eq!(
        engine
            .robot_state(attacker)
            .unwrap()
            .kinematics
            .actual_velocity,
        [0.5, -2.5]
    );
    assert_eq!(
        engine
            .robot_state(defender)
            .unwrap()
            .kinematics
            .actual_velocity,
        [1.25, -0.75]
    );
    let incoming = engine
        .snapshot()
        .shots
        .iter()
        .find(|shot| shot.kind == 500)
        .unwrap()
        .clone();
    assert_eq!(incoming.range, 11);
    engine.replace_dna(attacker, idle).unwrap();
    engine = restore_with_material(&engine, defender, "poison", 300.0);
    assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
    engine.tick().unwrap();

    let returned = engine
        .snapshot()
        .shots
        .iter()
        .find(|shot| shot.kind == -5 && !shot.impact_flash)
        .unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
    assert_eq!(returned.owner, defender);
    assert_eq!(incoming.velocity, [40.5, -2.5]);
    assert_eq!(returned.velocity, [-40.0, 2.0]);
    assert_eq!(returned.range, 12);
    assert_eq!(returned.energy, 481.0);
}

#[test]
fn release_energy_defense_applies_vb_cint_after_target_velocity_formula() {
    assert_positive_toxin_edge_gap(TOXIN_TARGET_POSITION);
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        gravity: [0.0, 0.0],
        drag: 0.0,
        brownian_motion: 0.0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let attacker = engine
        .spawn_at(idle.clone(), TOXIN_SOURCE_POSITION)
        .unwrap();
    let defender = engine
        .spawn_at(
            LegacyDna::parse("start\n221 .strpoison store\n-10 .dx store\nstop").unwrap(),
            TOXIN_TARGET_POSITION,
        )
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(
            attacker,
            LegacyDna::parse("start\n-1 .shoot store\nstop").unwrap(),
        )
        .unwrap();

    engine.tick().unwrap();
    engine = restore_with_toxin_motion(
        &engine,
        attacker,
        [0.25, -2.75],
        defender,
        [3.5, -1.5],
        [40.25, -2.75],
    );
    assert_eq!(
        engine
            .robot_state(attacker)
            .unwrap()
            .kinematics
            .actual_velocity,
        [0.25, -2.75]
    );
    let incoming = engine
        .snapshot()
        .shots
        .iter()
        .find(|shot| shot.kind == -1)
        .unwrap()
        .clone();
    assert_eq!(incoming.range, 11);
    engine.replace_dna(attacker, idle).unwrap();
    engine = restore_with_material(&engine, defender, "poison", 500.0);
    assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
    let target_actual_velocity = engine
        .robot_state(defender)
        .unwrap()
        .kinematics
        .actual_velocity;
    assert_eq!(target_actual_velocity, [3.5, -1.5]);
    engine.tick().unwrap();

    let returned = engine
        .snapshot()
        .shots
        .iter()
        .find(|shot| shot.kind == -5 && !shot.impact_flash)
        .unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
    assert_eq!(returned.owner, defender);
    assert_eq!(incoming.velocity, [40.25, -2.75]);
    assert_eq!(returned.velocity, [-35.0, 0.0]);
    assert_eq!(returned.range, 23);
    assert_eq!(returned.energy, 921.0);
}

#[test]
fn defensive_poison_uses_db2_information_and_feeding_equality_rules() {
    assert_positive_toxin_edge_gap(TOXIN_TARGET_POSITION);
    let idle = LegacyDna::parse("start\nstop").unwrap();

    let mut information_engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let information_attacker = information_engine
        .spawn_at(idle.clone(), TOXIN_SOURCE_POSITION)
        .unwrap();
    let information_defender = information_engine
        .spawn_at(
            LegacyDna::parse("start\n220 .strpoison store\nstop").unwrap(),
            TOXIN_TARGET_POSITION,
        )
        .unwrap();
    information_engine.tick().unwrap();
    information_engine
        .replace_dna(
            information_attacker,
            LegacyDna::parse("start\n500 .shoot store\n77 .shootval store\nstop").unwrap(),
        )
        .unwrap();
    information_engine
        .replace_dna(information_defender, idle.clone())
        .unwrap();
    let waste_before = information_engine
        .organism(information_defender)
        .unwrap()
        .waste;
    information_engine.tick().unwrap();
    assert_eq!(information_engine.snapshot().stats.projectile_impacts, 0);
    information_engine
        .replace_dna(information_attacker, idle.clone())
        .unwrap();
    // Seed equality immediately before impact; MakePoison caps requests at 100.
    information_engine =
        restore_with_material(&information_engine, information_defender, "poison", 220.0);
    information_engine.tick().unwrap();
    assert_eq!(information_engine.snapshot().stats.projectile_impacts, 1);

    let information_after = information_engine.organism(information_defender).unwrap();
    assert_eq!(information_after.poison, 22);
    assert_eq!(information_after.waste, waste_before + 22);
    assert!(
        information_engine
            .snapshot()
            .shots
            .iter()
            .any(|shot| shot.kind == -5 && !shot.impact_flash)
    );
    assert_ne!(
        information_engine
            .memory_at(information_defender, 500)
            .unwrap(),
        77
    );

    let mut feeding_engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let feeding_attacker = feeding_engine
        .spawn_at(idle.clone(), TOXIN_SOURCE_POSITION)
        .unwrap();
    let feeding_defender = feeding_engine
        .spawn_at(
            LegacyDna::parse("start\n220 .strpoison store\nstop").unwrap(),
            TOXIN_TARGET_POSITION,
        )
        .unwrap();
    feeding_engine.tick().unwrap();
    feeding_engine
        .replace_dna(
            feeding_attacker,
            LegacyDna::parse("start\n-1 .shoot store\nstop").unwrap(),
        )
        .unwrap();
    feeding_engine
        .replace_dna(feeding_defender, idle.clone())
        .unwrap();
    let feeding_energy_before = feeding_engine.organism(feeding_defender).unwrap().energy;
    feeding_engine.tick().unwrap();
    assert_eq!(feeding_engine.snapshot().stats.projectile_impacts, 0);
    feeding_engine.replace_dna(feeding_attacker, idle).unwrap();
    feeding_engine = restore_with_material(&feeding_engine, feeding_defender, "poison", 220.0);
    feeding_engine.tick().unwrap();
    assert_eq!(feeding_engine.snapshot().stats.projectile_impacts, 1);

    let feeding_after = feeding_engine.organism(feeding_defender).unwrap();
    assert_eq!(feeding_after.poison, 216);
    assert!(feeding_after.energy < feeding_energy_before);
    assert!(
        !feeding_engine
            .snapshot()
            .shots
            .iter()
            .any(|shot| shot.kind == -5 && !shot.impact_flash)
    );

    let mut poisoned_feeding_engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let poisoned_feeding_attacker = poisoned_feeding_engine
        .spawn_at(
            LegacyDna::parse("start\nstop").unwrap(),
            TOXIN_SOURCE_POSITION,
        )
        .unwrap();
    let poisoned_feeding_defender = poisoned_feeding_engine
        .spawn_at(
            LegacyDna::parse("start\n221 .strpoison store\nstop").unwrap(),
            TOXIN_TARGET_POSITION,
        )
        .unwrap();
    poisoned_feeding_engine.tick().unwrap();
    poisoned_feeding_engine
        .replace_dna(
            poisoned_feeding_attacker,
            LegacyDna::parse("start\n-1 .shoot store\nstop").unwrap(),
        )
        .unwrap();
    poisoned_feeding_engine
        .replace_dna(
            poisoned_feeding_defender,
            LegacyDna::parse("start\nstop").unwrap(),
        )
        .unwrap();
    let poisoned_feeding_energy_before = poisoned_feeding_engine
        .organism(poisoned_feeding_defender)
        .unwrap()
        .energy;
    let poisoned_feeding_waste_before = poisoned_feeding_engine
        .organism(poisoned_feeding_defender)
        .unwrap()
        .waste;
    poisoned_feeding_engine.tick().unwrap();
    assert_eq!(
        poisoned_feeding_engine.snapshot().stats.projectile_impacts,
        0
    );
    poisoned_feeding_engine
        .replace_dna(
            poisoned_feeding_attacker,
            LegacyDna::parse("start\nstop").unwrap(),
        )
        .unwrap();
    poisoned_feeding_engine = restore_with_material(
        &poisoned_feeding_engine,
        poisoned_feeding_defender,
        "poison",
        221.0,
    );
    poisoned_feeding_engine.tick().unwrap();
    assert_eq!(
        poisoned_feeding_engine.snapshot().stats.projectile_impacts,
        1
    );

    let poisoned_feeding_after = poisoned_feeding_engine
        .organism(poisoned_feeding_defender)
        .unwrap();
    assert_eq!(poisoned_feeding_after.poison, 23);
    assert_eq!(poisoned_feeding_after.waste, poisoned_feeding_waste_before);
    assert_eq!(
        poisoned_feeding_after.energy,
        poisoned_feeding_energy_before
    );
    assert!(
        poisoned_feeding_engine
            .snapshot()
            .shots
            .iter()
            .any(|shot| shot.kind == -5 && !shot.impact_flash)
    );
}

#[test]
fn conspecific_venom_adds_to_store_at_cap_without_shell_loss_or_effect() {
    assert_positive_toxin_edge_gap(TOXIN_TARGET_POSITION);
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let species = engine.register_species(SpeciesDefinition {
        name: "Exact shared species name".to_owned(),
        ..SpeciesDefinition::default()
    });
    let source = engine
        .spawn_species_at(
            LegacyDna::parse("start\n20 .strvenom store\nstop").unwrap(),
            species,
            TOXIN_SOURCE_POSITION,
        )
        .unwrap();
    let target = engine
        .spawn_species_batch(
            LegacyDna::parse("start\n31990 .strvenom store\n5 .mkshell store\nstop").unwrap(),
            species,
            [TOXIN_TARGET_POSITION],
            32_000,
        )
        .unwrap()[0];
    engine.tick().unwrap();
    // MakeVenom caps a DNA request at 100 per cycle; seed this cap fixture explicitly.
    engine = restore_with_material(&engine, target, "venom", 31_990.0);
    engine
        .replace_dna(
            source,
            LegacyDna::parse(
                "start\n-3 .shoot store\n20 .shootval store\n500 .vloc store\n77 .venval store\nstop",
            )
            .unwrap(),
        )
        .unwrap();
    engine
        .replace_dna(target, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
    engine
        .replace_dna(source, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);

    let target = engine.organism(target).unwrap();
    assert_eq!(target.venom, 32_000);
    assert_eq!(target.shell, 5);
    assert_eq!(target.paralyzed, 0);
}

#[test]
fn conspecific_defensive_poison_adds_to_store_without_forced_effect() {
    assert_positive_toxin_edge_gap(TOXIN_TARGET_POSITION);
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        shots: darwinbots_engine::ShotSettings {
            feeding_proportional: false,
            feeding_fixed_power: 20,
            ..Default::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let species = engine.register_species(SpeciesDefinition {
        name: "Poison siblings".to_owned(),
        ..SpeciesDefinition::default()
    });
    let attacker = engine
        .spawn_species_at(
            LegacyDna::parse("start\n10 .strpoison store\nstop").unwrap(),
            species,
            TOXIN_SOURCE_POSITION,
        )
        .unwrap();
    let defender = engine
        .spawn_species_at(
            LegacyDna::parse("start\n21 .strpoison store\n500 .ploc store\n77 .pval store\nstop")
                .unwrap(),
            species,
            TOXIN_TARGET_POSITION,
        )
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(
            attacker,
            LegacyDna::parse("start\n-1 .shoot store\n20 .shootval store\nstop").unwrap(),
        )
        .unwrap();
    engine
        .replace_dna(defender, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
    engine
        .replace_dna(attacker, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 2);

    let attacker = engine.organism(attacker).unwrap();
    assert!(
        attacker.poison > 10,
        "the returned conspecific poison is absorbed"
    );
    assert_eq!(attacker.poisoned, 0);
    assert_ne!(engine.memory_at(attacker.id, 500).unwrap(), 77);
}

#[test]
fn heterospecific_poison_ignores_shell_and_forces_payload_for_power_over_one_point_five() {
    assert_positive_toxin_edge_gap(TOXIN_TARGET_POSITION);
    let mut engine = Engine::new(EngineConfig {
        shots: darwinbots_engine::ShotSettings {
            feeding_proportional: false,
            feeding_fixed_power: 15,
            ..Default::default()
        },
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let attacker_species = engine.register_species(SpeciesDefinition {
        name: "Poison target".to_owned(),
        ..SpeciesDefinition::default()
    });
    let defender_species = engine.register_species(SpeciesDefinition {
        name: "Poison source".to_owned(),
        ..SpeciesDefinition::default()
    });
    let attacker = engine
        .spawn_species_at(
            LegacyDna::parse("start\n5 .mkshell store\nstop").unwrap(),
            attacker_species,
            TOXIN_SOURCE_POSITION,
        )
        .unwrap();
    let defender = engine
        .spawn_species_at(
            LegacyDna::parse("start\n16 .strpoison store\n500 .ploc store\n77 .pval store\nstop")
                .unwrap(),
            defender_species,
            TOXIN_TARGET_POSITION,
        )
        .unwrap();
    engine.tick().unwrap();
    let energy_before = engine.organism(attacker).unwrap().energy;
    engine
        .replace_dna(
            attacker,
            LegacyDna::parse("start\n-1 .shoot store\n15 .shootval store\nstop").unwrap(),
        )
        .unwrap();
    engine
        .replace_dna(defender, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
    engine
        .replace_dna(attacker, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 2);

    let attacker = engine.organism(attacker).unwrap();
    assert_eq!(attacker.energy, energy_before);
    assert_eq!(attacker.shell, 5);
    assert_eq!(attacker.poisoned, 9);
    assert_eq!(engine.memory_at(attacker.id, 500).unwrap(), 77);
}

#[test]
fn heterospecific_venom_consumes_shell_before_starting_an_effect() {
    fn run_case(power: i32) -> (i32, i32) {
        assert_positive_toxin_edge_gap(TOXIN_TARGET_POSITION);
        let mut engine = Engine::new(EngineConfig {
            metabolism_cost: 0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let source_species = engine.register_species(SpeciesDefinition {
            name: format!("Venom source {power}"),
            ..SpeciesDefinition::default()
        });
        let target_species = engine.register_species(SpeciesDefinition {
            name: format!("Venom target {power}"),
            ..SpeciesDefinition::default()
        });
        let source = engine
            .spawn_species_at(
                LegacyDna::parse(&format!("start\n{power} .strvenom store\nstop")).unwrap(),
                source_species,
                TOXIN_SOURCE_POSITION,
            )
            .unwrap();
        let target = engine
            .spawn_species_at(
                LegacyDna::parse("start\n5 .mkshell store\nstop").unwrap(),
                target_species,
                TOXIN_TARGET_POSITION,
            )
            .unwrap();
        engine.tick().unwrap();
        engine
            .replace_dna(
                source,
                LegacyDna::parse(&format!(
                    "start\n-3 .shoot store\n{power} .shootval store\n500 .vloc store\n77 .venval store\nstop"
                ))
                .unwrap(),
            )
            .unwrap();
        engine
            .replace_dna(target, LegacyDna::parse("start\nstop").unwrap())
            .unwrap();
        engine.tick().unwrap();
        assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
        engine
            .replace_dna(source, LegacyDna::parse("start\nstop").unwrap())
            .unwrap();
        engine.tick().unwrap();
        assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
        let target = engine.organism(target).unwrap();
        (target.shell, target.paralyzed)
    }

    assert_eq!(run_case(8), (0, 3));
    assert_eq!(run_case(4), (0, 0));
}

#[test]
fn attenuated_venom_below_one_has_no_effect() {
    assert_positive_toxin_edge_gap(FRACTIONAL_TOXIN_TARGET_POSITION);
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let source_species = engine.register_species(SpeciesDefinition {
        name: "Weak venom source".to_owned(),
        ..SpeciesDefinition::default()
    });
    let target_species = engine.register_species(SpeciesDefinition {
        name: "Weak venom target".to_owned(),
        ..SpeciesDefinition::default()
    });
    let source = engine
        .spawn_species_at(
            LegacyDna::parse("start\n1 .strvenom store\nstop").unwrap(),
            source_species,
            TOXIN_SOURCE_POSITION,
        )
        .unwrap();
    let target = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            target_species,
            FRACTIONAL_TOXIN_TARGET_POSITION,
        )
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(
            source,
            LegacyDna::parse(
                "start\n-3 .shoot store\n1 .shootval store\n500 .vloc store\n77 .venval store\nstop",
            )
            .unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
    engine
        .replace_dna(source, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    // The first travel cycle must miss so the one-power shot actually ages.
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
    engine.tick().unwrap();

    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
    assert_eq!(engine.organism(target).unwrap().paralyzed, 0);
    assert_ne!(engine.memory_at(target, 500).unwrap(), 77);
}

#[test]
fn non_integral_venom_power_keeps_fractional_countdown_through_expiry() {
    assert_positive_toxin_edge_gap(FRACTIONAL_TOXIN_TARGET_POSITION);
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        gravity: [0.0, 0.0],
        drag: 0.0,
        brownian_motion: 0.0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let source_species = engine.register_species(SpeciesDefinition {
        name: "Fractional venom source".to_owned(),
        ..SpeciesDefinition::default()
    });
    let target_species = engine.register_species(SpeciesDefinition {
        name: "Fractional venom target".to_owned(),
        ..SpeciesDefinition::default()
    });
    let source = engine
        .spawn_species_at(
            LegacyDna::parse("start\n8 .strvenom store\nstop").unwrap(),
            source_species,
            TOXIN_SOURCE_POSITION,
        )
        .unwrap();
    let target = engine
        .spawn_species_at(
            LegacyDna::parse("start\n55 500 store\nstop").unwrap(),
            target_species,
            FRACTIONAL_TOXIN_TARGET_POSITION,
        )
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(
            source,
            LegacyDna::parse(
                "start\n-3 .shoot store\n8 .shootval store\n500 .vloc store\n77 .venval store\nstop",
            )
            .unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(source, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();

    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
    assert_eq!(
        engine
            .robot_state(target)
            .unwrap()
            .effects
            .paralysis_countdown,
        6.9870706
    );
    assert_eq!(engine.organism(target).unwrap().paralyzed, 6);
    assert_eq!(engine.memory_at(target, 500).unwrap(), 77);

    engine.tick().unwrap();
    assert_eq!(
        engine
            .robot_state(target)
            .unwrap()
            .effects
            .paralysis_countdown,
        5.9870706
    );
    assert_eq!(engine.organism(target).unwrap().paralyzed, 5);
    engine.tick().unwrap();
    assert_eq!(
        engine
            .robot_state(target)
            .unwrap()
            .effects
            .paralysis_countdown,
        4.9870706
    );
    assert_eq!(engine.organism(target).unwrap().paralyzed, 4);
    engine.tick().unwrap();
    assert_eq!(
        engine
            .robot_state(target)
            .unwrap()
            .effects
            .paralysis_countdown,
        3.9870706
    );
    assert_eq!(engine.organism(target).unwrap().paralyzed, 3);
    engine.tick().unwrap();
    assert_eq!(
        engine
            .robot_state(target)
            .unwrap()
            .effects
            .paralysis_countdown,
        2.9870706
    );
    assert_eq!(engine.organism(target).unwrap().paralyzed, 2);
    engine.tick().unwrap();
    assert_eq!(
        engine
            .robot_state(target)
            .unwrap()
            .effects
            .paralysis_countdown,
        1.9870706
    );
    assert_eq!(engine.organism(target).unwrap().paralyzed, 1);
    engine.tick().unwrap();
    let expired = &engine.robot_state(target).unwrap().effects;
    assert!(!expired.paralysis_active);
    assert_eq!(expired.paralysis_countdown, 0.98707056);
    assert_eq!(expired.paralysis_location, 0);
    assert_eq!(expired.paralysis_value, 0);
    assert_eq!(engine.organism(target).unwrap().paralyzed, 0);
    assert_eq!(engine.memory_at(target, 500).unwrap(), 77);

    engine.tick().unwrap();
    assert_eq!(engine.memory_at(target, 500).unwrap(), 55);
}

#[test]
fn toxin_location_340_is_a_no_op_and_random_location_is_safe() {
    fn effect_location(raw_location: i32) -> i32 {
        assert_positive_toxin_edge_gap(TOXIN_TARGET_POSITION);
        let mut engine = Engine::new(EngineConfig {
            metabolism_cost: 0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let source_species = engine.register_species(SpeciesDefinition {
            name: format!("Location source {raw_location}"),
            ..SpeciesDefinition::default()
        });
        let target_species = engine.register_species(SpeciesDefinition {
            name: format!("Location target {raw_location}"),
            ..SpeciesDefinition::default()
        });
        let source = engine
            .spawn_species_at(
                LegacyDna::parse("start\n3 .strvenom store\nstop").unwrap(),
                source_species,
                TOXIN_SOURCE_POSITION,
            )
            .unwrap();
        let target = engine
            .spawn_species_at(
                LegacyDna::parse("start\nstop").unwrap(),
                target_species,
                TOXIN_TARGET_POSITION,
            )
            .unwrap();
        engine.tick().unwrap();
        engine
            .replace_dna(
                source,
                LegacyDna::parse(&format!(
                    "start\n-3 .shoot store\n3 .shootval store\n{raw_location} .vloc store\n77 .venval store\nstop"
                ))
                .unwrap(),
            )
            .unwrap();
        engine.tick().unwrap();
        assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
        engine
            .replace_dna(source, LegacyDna::parse("start\nstop").unwrap())
            .unwrap();
        engine.tick().unwrap();
        assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
        assert_eq!(engine.organism(target).unwrap().paralyzed, 2);
        let save = SaveFile::encode(&engine).unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&save[10..]).unwrap();
        payload["slots"][target.slot() as usize]["organism"]["effects"]["paralysis_location"]
            .as_i64()
            .unwrap() as i32
    }

    assert_eq!(effect_location(340), 0);
    let random = effect_location(0);
    assert!((1..=1_000).contains(&random));
    assert_ne!(random, 340);
}

#[test]
fn later_venom_hit_adds_duration_and_replaces_payload() {
    assert_positive_toxin_edge_gap(TOXIN_TARGET_POSITION);
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let source_species = engine.register_species(SpeciesDefinition {
        name: "Replacement source".to_owned(),
        ..SpeciesDefinition::default()
    });
    let target_species = engine.register_species(SpeciesDefinition {
        name: "Replacement target".to_owned(),
        ..SpeciesDefinition::default()
    });
    let source = engine
        .spawn_species_at(
            LegacyDna::parse("start\n8 .strvenom store\nstop").unwrap(),
            source_species,
            TOXIN_SOURCE_POSITION,
        )
        .unwrap();
    let target = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            target_species,
            TOXIN_TARGET_POSITION,
        )
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(
            source,
            LegacyDna::parse(
                "start\n-3 .shoot store\n4 .shootval store\n500 .vloc store\n77 .venval store\nstop",
            )
            .unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
    engine
        .replace_dna(source, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
    assert_eq!(engine.organism(target).unwrap().paralyzed, 3);
    assert_eq!(engine.memory_at(target, 500).unwrap(), 77);

    engine
        .replace_dna(
            source,
            LegacyDna::parse(
                "start\n-3 .shoot store\n4 .shootval store\n600 .vloc store\n88 .venval store\nstop",
            )
            .unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
    engine
        .replace_dna(source, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.snapshot().stats.projectile_impacts, 2);
    assert_eq!(engine.organism(target).unwrap().paralyzed, 5);
    assert_eq!(engine.memory_at(target, 600).unwrap(), 88);
}

#[test]
fn toxin_final_cycle_writes_then_cleans_effect_without_clearing_memory() {
    assert_positive_toxin_edge_gap(TOXIN_TARGET_POSITION);
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let source_species = engine.register_species(SpeciesDefinition {
        name: "Final write source".to_owned(),
        ..SpeciesDefinition::default()
    });
    let target_species = engine.register_species(SpeciesDefinition {
        name: "Final write target".to_owned(),
        ..SpeciesDefinition::default()
    });
    let source = engine
        .spawn_species_at(
            LegacyDna::parse("start\n3 .strvenom store\nstop").unwrap(),
            source_species,
            TOXIN_SOURCE_POSITION,
        )
        .unwrap();
    let target = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            target_species,
            TOXIN_TARGET_POSITION,
        )
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(
            source,
            LegacyDna::parse(
                "start\n-3 .shoot store\n3 .shootval store\n500 .vloc store\n77 .venval store\nstop",
            )
            .unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
    engine
        .replace_dna(source, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
    assert_eq!(engine.organism(target).unwrap().paralyzed, 2);

    engine
        .replace_dna(
            target,
            LegacyDna::parse("start\n55 500 store\nstop").unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.organism(target).unwrap().paralyzed, 1);
    assert_eq!(engine.memory_at(target, 500).unwrap(), 77);

    engine.tick().unwrap();
    assert_eq!(engine.organism(target).unwrap().paralyzed, 0);
    assert_eq!(engine.memory_at(target, 500).unwrap(), 77);

    engine.tick().unwrap();
    assert_eq!(engine.memory_at(target, 500).unwrap(), 55);
}

#[test]
fn in_flight_venom_keeps_payload_after_owner_removal() {
    assert_positive_toxin_edge_gap(TOXIN_TARGET_POSITION);
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let source_species = engine.register_species(SpeciesDefinition {
        name: "Removed source".to_owned(),
        ..SpeciesDefinition::default()
    });
    let target_species = engine.register_species(SpeciesDefinition {
        name: "Surviving target".to_owned(),
        ..SpeciesDefinition::default()
    });
    let source = engine
        .spawn_species_at(
            LegacyDna::parse("start\n8 .strvenom store\nstop").unwrap(),
            source_species,
            TOXIN_SOURCE_POSITION,
        )
        .unwrap();
    let target = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            target_species,
            TOXIN_TARGET_POSITION,
        )
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(
            source,
            LegacyDna::parse(
                "start\n-3 .shoot store\n8 .shootval store\n500 .vloc store\n77 .venval store\nstop",
            )
            .unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
    engine.remove(source).unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
    assert_eq!(engine.organism(target).unwrap().paralyzed, 7);
    assert_eq!(engine.memory_at(target, 500).unwrap(), 77);
}

#[test]
fn corpses_ignore_venom_impacts() {
    assert_positive_toxin_edge_gap(TOXIN_TARGET_POSITION);
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 1,
        corpse_decay: darwinbots_engine::CorpseDecaySettings {
            enabled: true,
            packet_size: 0,
            ..Default::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let source_species = engine.register_species(SpeciesDefinition {
        name: "Corpse venom source".to_owned(),
        ..SpeciesDefinition::default()
    });
    let corpse_species = engine.register_species(SpeciesDefinition {
        name: "Corpse target".to_owned(),
        ..SpeciesDefinition::default()
    });
    let source = engine
        .spawn_species_at(
            LegacyDna::parse("start\n8 .strvenom store\nstop").unwrap(),
            source_species,
            TOXIN_SOURCE_POSITION,
        )
        .unwrap();
    engine
        .spawn_species_batch(
            LegacyDna::parse("start\nstop").unwrap(),
            corpse_species,
            [TOXIN_TARGET_POSITION],
            1,
        )
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
    let corpse_before = engine.snapshot().corpses[0].clone();
    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n-3 .shoot store\n8 .shootval store\nstop").unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
    engine
        .replace_dna(source, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    let corpse_after = &engine.snapshot().corpses[0];

    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
    assert_eq!(corpse_after.energy, corpse_before.energy);
    assert_eq!(corpse_after.body, corpse_before.body);
}

#[test]
fn shot07_venom_resolution_contract_covers_store_effect_and_edges() {
    #[derive(Debug, PartialEq)]
    struct Expected {
        source_venom: i32,
        impacts: u64,
        effects: u64,
        target: Option<[i32; 6]>, // venom, shell, paralyzed, location, value, memory[500]
        countdown: Option<f32>,
        corpse_unchanged: bool,
    }

    struct Case {
        name: &'static str,
        shared_species: bool,
        source_venom: i32,
        shoot_value: i32,
        target_venom: i32,
        target_shell: i32,
        toxin_location: i32,
        toxin_value: i32,
        corpse: bool,
        followup_paralyzed: Option<i32>,
        expected: Expected,
    }

    let cases = [
        Case {
            name: "zero shootval absorbs one twentieth",
            shared_species: true,
            source_venom: 100,
            shoot_value: 0,
            target_venom: 10,
            target_shell: 5,
            toxin_location: 500,
            toxin_value: 77,
            corpse: false,
            followup_paralyzed: None,
            expected: Expected {
                source_venom: 95,
                impacts: 1,
                effects: 1,
                target: Some([15, 5, 0, 0, 0, 55]),
                countdown: None,
                corpse_unchanged: false,
            },
        },
        Case {
            name: "heterospecific payload counts down",
            shared_species: false,
            source_venom: 8,
            shoot_value: 8,
            target_venom: 0,
            target_shell: 0,
            toxin_location: 500,
            toxin_value: 77,
            corpse: false,
            followup_paralyzed: Some(5),
            expected: Expected {
                source_venom: 0,
                impacts: 1,
                effects: 1,
                target: Some([0, 0, 6, 500, 77, 77]),
                countdown: Some(6.9870706),
                corpse_unchanged: false,
            },
        },
        Case {
            name: "invalid delgene address is inert",
            shared_species: false,
            source_venom: 3,
            shoot_value: 3,
            target_venom: 0,
            target_shell: 0,
            toxin_location: 340,
            toxin_value: 77,
            corpse: false,
            followup_paralyzed: None,
            expected: Expected {
                source_venom: 0,
                impacts: 1,
                effects: 1,
                target: Some([0, 0, 2, 0, 77, 55]),
                countdown: None,
                corpse_unchanged: false,
            },
        },
        Case {
            name: "attenuated one-power venom is inert",
            shared_species: false,
            source_venom: 1,
            shoot_value: 1,
            target_venom: 0,
            target_shell: 0,
            toxin_location: 500,
            toxin_value: 77,
            corpse: false,
            followup_paralyzed: None,
            expected: Expected {
                source_venom: 0,
                impacts: 1,
                effects: 0,
                target: Some([0, 0, 0, 0, 0, 55]),
                countdown: None,
                corpse_unchanged: false,
            },
        },
    ];

    for case in cases {
        let mut engine = Engine::new(EngineConfig {
            metabolism_cost: if case.corpse { 1 } else { 0 },
            gravity: [0.0, 0.0],
            drag: 0.0,
            brownian_motion: 0.0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let species_name = format!("SHOT-07 {}", case.name);
        let source_species = engine.register_species(SpeciesDefinition {
            name: species_name.clone(),
            ..SpeciesDefinition::default()
        });
        let target_species = if case.shared_species {
            source_species
        } else {
            engine.register_species(SpeciesDefinition {
                name: format!("{species_name} target"),
                ..SpeciesDefinition::default()
            })
        };
        let source = engine
            .spawn_species_at(
                LegacyDna::parse(&format!(
                    "start\n{} .strvenom store\nstop",
                    case.source_venom
                ))
                .unwrap(),
                source_species,
                TOXIN_SOURCE_POSITION,
            )
            .unwrap();
        let target_dna = if case.corpse {
            LegacyDna::parse("start\nstop").unwrap()
        } else {
            LegacyDna::parse(&format!(
                "start\n{} .strvenom store\n{} .mkshell store\n55 500 store\nstop",
                case.target_venom, case.target_shell
            ))
            .unwrap()
        };
        let target_position = if case.followup_paralyzed.is_some() || case.source_venom == 1 {
            FRACTIONAL_TOXIN_TARGET_POSITION
        } else {
            TOXIN_TARGET_POSITION
        };
        let target = if case.corpse {
            engine
                .spawn_species_batch(target_dna, target_species, [target_position], 1)
                .unwrap()[0]
        } else {
            engine
                .spawn_species_at(target_dna, target_species, target_position)
                .unwrap()
        };
        engine.tick().unwrap();
        let corpse_before = case.corpse.then(|| engine.snapshot().corpses[0].clone());
        engine
            .replace_dna(
                source,
                LegacyDna::parse(&format!(
                    "start\n-3 .shoot store\n{} .shootval store\n{} .vloc store\n{} .venval store\nstop",
                    case.shoot_value, case.toxin_location, case.toxin_value
                ))
                .unwrap(),
            )
            .unwrap();
        if !case.corpse {
            engine
                .replace_dna(target, LegacyDna::parse("start\nstop").unwrap())
                .unwrap();
        }
        engine.tick().unwrap();
        engine
            .replace_dna(source, LegacyDna::parse("start\nstop").unwrap())
            .unwrap();
        for _ in 0..20 {
            engine.tick().unwrap();
            if engine.snapshot().stats.projectile_impacts == 1 {
                break;
            }
        }

        let observed = Expected {
            source_venom: engine.organism(source).unwrap().venom,
            impacts: engine.snapshot().stats.projectile_impacts,
            effects: engine.snapshot().stats.projectile_effects,
            target: (!case.corpse).then(|| {
                let target_after = engine.organism(target).unwrap();
                let effects = &engine.robot_state(target).unwrap().effects;
                [
                    target_after.venom,
                    target_after.shell,
                    target_after.paralyzed,
                    effects.paralysis_location,
                    effects.paralysis_value,
                    engine.memory_at(target, 500).unwrap(),
                ]
            }),
            countdown: (!case.corpse && case.followup_paralyzed.is_some()).then(|| {
                engine
                    .robot_state(target)
                    .unwrap()
                    .effects
                    .paralysis_countdown
            }),
            corpse_unchanged: case.corpse
                && corpse_before.as_ref().is_some_and(|before| {
                    let after = &engine.snapshot().corpses[0];
                    before.energy == after.energy && before.body == after.body
                }),
        };
        assert_eq!(observed, case.expected, "{}", case.name);
        if let Some(expected_paralyzed) = case.followup_paralyzed {
            engine.tick().unwrap();
            assert_eq!(
                engine.organism(target).unwrap().paralyzed,
                expected_paralyzed
            );
            assert_eq!(engine.memory_at(target, 500).unwrap(), case.toxin_value);
        }
    }
}

#[test]
fn eye_strength_uses_db2_edge_to_edge_distance() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let observer = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
        .unwrap();
    engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [850.0, 500.0])
        .unwrap();

    engine.tick().unwrap();

    let eye5 = engine.memory_at(observer, 505).unwrap();
    assert!(
        (120..=122).contains(&eye5),
        "expected DB2 eye strength near 121, got {eye5}"
    );
}
