use darwinbots_engine::{
    Engine, EngineConfig, LegacyDna, OrganismId, SaveFile, SpeciesDefinition, WasteSettings,
    sysvar_address,
};

const TOXIN_SOURCE_POSITION: [f32; 2] = [100.0, 100.0];
const TOXIN_TARGET_POSITION: [f32; 2] = [350.0, 100.0];
const FRACTIONAL_TOXIN_TARGET_POSITION: [f32; 2] = [400.0, 100.0];
const DEFAULT_ROBOT_RADII_SUM: f32 = 228.56;

fn assert_positive_toxin_edge_gap(target_position: [f32; 2]) {
    assert_eq!(target_position[1], TOXIN_SOURCE_POSITION[1]);
    let center_gap = target_position[0] - TOXIN_SOURCE_POSITION[0];
    assert!(center_gap == 250.0 || center_gap == 300.0);
    assert!(center_gap - DEFAULT_ROBOT_RADII_SUM > 0.0);
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
    assert_eq!(u16::from_le_bytes([encoded[4], encoded[5]]), 3);
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
fn negative_shots_feed_the_attacker_from_the_target() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let predator = LegacyDna::parse("start\n-1 .shoot store\n100 .shootval store\nstop").unwrap();
    let prey = LegacyDna::parse("start\nstop").unwrap();
    let attacker = engine.spawn_at(predator, [100.0, 100.0]).unwrap();
    engine.spawn_at(prey, [350.0, 100.0]).unwrap();

    engine.tick().unwrap();
    engine.tick().unwrap();
    engine.tick().unwrap();

    assert!(engine.organism(attacker).unwrap().energy > 1_000);
    assert_eq!(engine.snapshot().stats.energy_harvested, 95);
}

#[test]
fn waste_phase_runs_alzheimer_and_defecation_after_waste_shot() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        waste: WasteSettings {
            alzheimer_threshold: 0,
            defecation_threshold: 10,
            ..WasteSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let source = engine
        .spawn_at(
            LegacyDna::parse("start\n-4 .shoot store\n32000 .shootval store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [350.0, 100.0])
        .unwrap();
    engine.tick().unwrap();
    let before_memory = engine.memory_at(target, 500).unwrap();

    engine.tick().unwrap();

    let after = engine.robot_state(target).unwrap();
    assert!(after.materials.waste < 32_000);
    assert!(after.materials.permanent_waste > 0);
    assert_ne!(engine.memory_at(target, 500).unwrap(), before_memory);
    assert!(
        engine
            .snapshot()
            .shots
            .iter()
            .any(|shot| shot.owner == target && shot.kind == -4 && shot.value == 200)
    );
    assert!(engine.snapshot().stats.shots_fired > 1);
    assert_eq!(engine.organism(source).unwrap().waste, 0);
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
fn species_mutation_rate_applies_to_ordinary_reproduction() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let species = engine.register_species(SpeciesDefinition {
        name: "Rapid evolution".to_owned(),
        mutation_rate: 100.0,
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
    assert_eq!(engine.snapshot().stats.mutations, 1);
}

#[test]
fn dna_controls_body_defenses_chloroplasts_waste_and_aim() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let dna = LegacyDna::parse(
        "start\n10 .strbody store\n25 .mkshell store\n30 .mkchlr store\n314 .setaim store\nstop",
    )
    .unwrap();
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
    let aim = LegacyDna::parse("start\n314 .setaim store\nstop").unwrap();
    let id = engine.spawn_at(aim, [500.0, 500.0]).unwrap();

    engine.tick().unwrap();
    engine
        .replace_dna(id, LegacyDna::parse("start\n10 .up store\nstop").unwrap())
        .unwrap();
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
    let attacker = engine.spawn_at(predator, [100.0, 100.0]).unwrap();
    engine.spawn_at(prey, [350.0, 100.0]).unwrap();

    engine.tick().unwrap();
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
        ..EngineConfig::testing()
    })
    .unwrap();
    let dna = LegacyDna::parse("start\n50 .repro store\nstop").unwrap();
    for index in 0..20 {
        engine
            .spawn_at(dna.clone(), [100.0 + index as f32 * 10.0, 500.0])
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
    let source = engine.spawn_at(feeder, [100.0, 100.0]).unwrap();
    let target = engine.spawn_at(zerobot, [350.0, 100.0]).unwrap();

    engine.tick().unwrap();
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
    engine.tick().unwrap();

    assert_eq!(engine.population(), 3);
    assert_eq!(engine.snapshot().stats.births, 1);
}

#[test]
fn feeding_energy_shots_ignore_shell() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let attacker = engine.spawn_at(idle.clone(), [100.0, 100.0]).unwrap();
    let target = engine
        .spawn_at(
            LegacyDna::parse("start\n100 .mkshell store\nstop").unwrap(),
            [350.0, 100.0],
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
    engine.tick().unwrap();

    let attacker_after = engine.organism(attacker).unwrap();
    let target_after = engine.organism(target).unwrap();
    assert_eq!(target_after.energy, target_before.energy - 50);
    assert_eq!(target_after.shell, target_before.shell);
    assert_eq!(attacker_after.energy, attacker_before.energy + 37);
    assert_eq!(engine.snapshot().stats.shots_fired, 1);
    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
    assert_eq!(engine.snapshot().stats.projectile_effects, 1);
    assert_eq!(engine.snapshot().stats.energy_harvested, 37);
    assert_eq!(engine.snapshot().stats.feeding_events, 1);
}

#[test]
fn body_feeding_shots_consume_shell_before_body() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let attacker = engine.spawn_at(idle.clone(), [100.0, 100.0]).unwrap();
    let target = engine
        .spawn_at(
            LegacyDna::parse("start\n100 .mkshell store\nstop").unwrap(),
            [350.0, 100.0],
        )
        .unwrap();
    engine.tick().unwrap();
    engine.replace_dna(target, idle.clone()).unwrap();
    engine
        .replace_dna(
            attacker,
            LegacyDna::parse("start\n-6 .shoot store\n50 .shootval store\nstop").unwrap(),
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
    assert_eq!(target_after.shell, target_before.shell - 50);
    assert_eq!(attacker_after.energy, attacker_before.energy);
    assert_eq!(engine.snapshot().stats.shots_fired, 1);
    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
    assert_eq!(engine.snapshot().stats.projectile_effects, 1);
    assert_eq!(engine.snapshot().stats.energy_harvested, 0);
    assert_eq!(engine.snapshot().stats.feeding_events, 0);
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
    assert_eq!(after.paralyzed, 6);
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
    assert_eq!(after_move.paralyzed, 5);
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
    feeding_engine.tick().unwrap();
    assert_eq!(feeding_engine.snapshot().stats.projectile_impacts, 1);

    let feeding_after = feeding_engine.organism(feeding_defender).unwrap();
    assert_eq!(feeding_after.poison, 220);
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
    assert_eq!(attacker.poison, 30);
    assert_eq!(attacker.poisoned, 0);
    assert_ne!(engine.memory_at(attacker.id, 500).unwrap(), 77);
}

#[test]
fn heterospecific_poison_ignores_shell_and_forces_payload_for_power_over_one_point_five() {
    assert_positive_toxin_edge_gap(TOXIN_TARGET_POSITION);
    let mut engine = Engine::new(EngineConfig {
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

    assert_eq!(run_case(8), (0, 2));
    assert_eq!(run_case(4), (0, 0));
}

#[test]
fn attenuated_venom_below_one_has_no_effect() {
    assert_positive_toxin_edge_gap(TOXIN_TARGET_POSITION);
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
            TOXIN_TARGET_POSITION,
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
        6.9712706
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
        5.9712706
    );
    assert_eq!(engine.organism(target).unwrap().paralyzed, 5);
    engine.tick().unwrap();
    assert_eq!(
        engine
            .robot_state(target)
            .unwrap()
            .effects
            .paralysis_countdown,
        4.9712706
    );
    assert_eq!(engine.organism(target).unwrap().paralyzed, 4);
    engine.tick().unwrap();
    assert_eq!(
        engine
            .robot_state(target)
            .unwrap()
            .effects
            .paralysis_countdown,
        3.9712706
    );
    assert_eq!(engine.organism(target).unwrap().paralyzed, 3);
    engine.tick().unwrap();
    assert_eq!(
        engine
            .robot_state(target)
            .unwrap()
            .effects
            .paralysis_countdown,
        2.9712706
    );
    assert_eq!(engine.organism(target).unwrap().paralyzed, 2);
    engine.tick().unwrap();
    assert_eq!(
        engine
            .robot_state(target)
            .unwrap()
            .effects
            .paralysis_countdown,
        1.9712706
    );
    assert_eq!(engine.organism(target).unwrap().paralyzed, 1);
    engine.tick().unwrap();
    let expired = &engine.robot_state(target).unwrap().effects;
    assert!(!expired.paralysis_active);
    assert_eq!(expired.paralysis_countdown, 0.97127056);
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
        assert_eq!(engine.organism(target).unwrap().paralyzed, 1);
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
    assert_eq!(engine.organism(target).unwrap().paralyzed, 2);
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
    assert_eq!(engine.organism(target).unwrap().paralyzed, 4);
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
    assert_eq!(engine.organism(target).unwrap().paralyzed, 1);

    engine
        .replace_dna(
            target,
            LegacyDna::parse("start\n55 500 store\nstop").unwrap(),
        )
        .unwrap();
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
    assert_eq!(engine.organism(target).unwrap().paralyzed, 6);
    assert_eq!(engine.memory_at(target, 500).unwrap(), 77);
}

#[test]
fn corpses_ignore_venom_impacts() {
    assert_positive_toxin_edge_gap(TOXIN_TARGET_POSITION);
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 1,
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
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 850.0])
        .unwrap();

    engine.tick().unwrap();

    let eye5 = engine.memory_at(observer, 505).unwrap();
    assert!(
        (120..=122).contains(&eye5),
        "expected DB2 eye strength near 121, got {eye5}"
    );
}
