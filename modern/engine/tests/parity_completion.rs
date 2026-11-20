use darwinbots_engine::{
    AdvancedMutationSettings, Engine, EngineConfig, LegacyDna, PointMutationSchedule, SaveFile,
    SpeciesDefinition, StandardMutationProfile,
};

#[test]
fn point_waiting_times_are_geometric_and_preserve_long_tails() {
    let mut random = 0x1234_5678_9abc_def0_u64;
    let mut total = 0_u64;
    let mut maximum = 0;
    for _ in 0..20000 {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        let schedule = PointMutationSchedule::sample(0, 2, 1.0, 1.0, random, 0).unwrap();
        total += schedule.cycle;
        maximum = maximum.max(schedule.cycle);
    }
    let mean = total as f64 / 20000.0;
    assert!((950.0..1050.0).contains(&mean), "mean={mean}");
    assert!(
        maximum > 5000,
        "a bounded uniform clock loses the geometric tail"
    );
}

fn quiet_engine() -> Engine {
    Engine::new(EngineConfig {
        brownian_motion: 0.0,
        metabolism_cost: 0,
        advanced_mutation: AdvancedMutationSettings {
            point2_enabled: true,
            ..Default::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap()
}

#[test]
fn genetic_distance_aligns_insertions_and_uses_vb6_nucleotide_encoding() {
    let left = LegacyDna::parse("1 2 3").unwrap();
    let insertion = LegacyDna::parse("1 9 2 3").unwrap();
    // One unmatched base; the two dummy cells and master ends also participate.
    assert!((left.genetic_distance(&insertion) - 1.0 / 11.0).abs() < 1e-6);
    assert!((insertion.genetic_distance(&left) - 1.0 / 11.0).abs() < 1e-6);
    assert_eq!(
        LegacyDna::parse("1000")
            .unwrap()
            .genetic_distance(&LegacyDna::parse("1001").unwrap()),
        0.0
    );
    assert_eq!(left.genetic_distance(&left), 0.0);
}

fn edit_save(engine: &Engine, edit: impl FnOnce(&mut serde_json::Value)) -> Engine {
    let bytes = SaveFile::encode(engine).unwrap();
    let mut value = serde_json::from_slice(&bytes[10..]).unwrap();
    edit(&mut value);
    let payload = serde_json::to_vec(&value).unwrap();
    let mut result = bytes[..6].to_vec();
    result.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    result.extend_from_slice(&payload);
    SaveFile::decode(&result).unwrap()
}

#[test]
fn age_clocks_are_independent_and_survive_full_state_save() {
    let mut engine = quiet_engine();
    let id = engine
        .spawn_at(LegacyDna::parse("1 2 3 4 5 6 7 8").unwrap(), [500.0, 500.0])
        .unwrap();
    let mut profile = StandardMutationProfile::default();
    profile.point.one_in = 1e12;
    profile.point2.one_in = 1e12;
    profile.delta.one_in = 0.0;
    engine.set_mutation_profile(id, profile).unwrap();
    let mut engine = edit_save(&engine, |value| {
        value["tick"] = serde_json::json!(1000);
        let robot = &mut value["slots"][id.slot() as usize]["organism"];
        robot["counters"]["age"] = serde_json::json!(1);
        robot["program"]["point_mutation_schedule"] = serde_json::json!({"location":1,"cycle":2});
        robot["program"]["point2_mutation_schedule"] = serde_json::json!({"location":2,"cycle":3});
    });
    engine.tick().unwrap();
    assert_eq!(
        engine
            .robot_state(id)
            .unwrap()
            .program
            .point_mutation_schedule
            .unwrap()
            .cycle,
        2
    );
    assert_eq!(
        engine
            .robot_state(id)
            .unwrap()
            .program
            .mutation_metadata
            .total_mutations,
        0
    );
    engine.tick().unwrap();
    assert!(
        engine
            .robot_state(id)
            .unwrap()
            .program
            .mutation_metadata
            .total_mutations
            > 0
    );
    assert_eq!(
        engine
            .robot_state(id)
            .unwrap()
            .program
            .point2_mutation_schedule
            .unwrap()
            .cycle,
        3
    );
    let mut restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
    engine.tick().unwrap();
    restored.tick().unwrap();
    // DNA inspector history is deliberately transient; compare all persisted state.
    assert_eq!(
        SaveFile::encode(&engine).unwrap(),
        SaveFile::encode(&restored).unwrap()
    );
}

#[test]
fn fractional_defense_decay_does_not_stall_at_the_integer_rounding_plateau() {
    let mut engine = quiet_engine();
    let id = engine
        .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0, 500.0])
        .unwrap();
    let mut engine = edit_save(&engine, |value| {
        let materials = &mut value["slots"][id.slot() as usize]["organism"]["materials"];
        materials["slime"] = serde_json::json!(100);
        materials["poison"] = serde_json::json!(100);
    });
    for _ in 0..50 {
        engine.tick().unwrap();
    }
    let mut restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
    for _ in 0..150 {
        engine.tick().unwrap();
        restored.tick().unwrap();
    }
    assert!(engine.organism(id).unwrap().slime < 5);
    assert!(engine.organism(id).unwrap().poison < 5);
    assert_eq!(
        engine.robot_state(id).unwrap().materials,
        restored.robot_state(id).unwrap().materials
    );
}

#[test]
fn find_best_excludes_plants_and_scores_the_living_family() {
    let mut engine = quiet_engine();
    let plant = engine.register_species(SpeciesDefinition {
        vegetable: true,
        ..Default::default()
    });
    let dna = LegacyDna::parse("start stop").unwrap();
    let parent = engine.spawn_at(dna.clone(), [100.0, 100.0]).unwrap();
    let child = engine.spawn_at(dna.clone(), [500.0, 500.0]).unwrap();
    let rich = engine.spawn_at(dna.clone(), [800.0, 800.0]).unwrap();
    let vegetable = engine.spawn_species_at(dna, plant, [200.0, 800.0]).unwrap();
    engine.set_organism_energy(rich, 32000).unwrap();
    engine.set_organism_energy(vegetable, 32000).unwrap();
    let engine = edit_save(&engine, |value| {
        value["slots"][child.slot() as usize]["organism"]["identity"]["parents"] =
            serde_json::json!([parent, null]);
    });
    assert_eq!(engine.find_best_robot(false), Some(parent));
    assert_eq!(engine.find_best_robot(true), Some(parent));
}

#[test]
fn find_best_original_weighting_separates_population_energy_and_intermediate_scores() {
    let mut engine = quiet_engine();
    let dna = LegacyDna::parse("start stop").unwrap();
    let parent = engine.spawn_at(dna.clone(), [400.0, 400.0]).unwrap();
    let child = engine.spawn_at(dna.clone(), [600.0, 400.0]).unwrap();
    let rich = engine.spawn_at(dna, [800.0, 800.0]).unwrap();
    engine.set_organism_energy(rich, 32000).unwrap();
    let engine = edit_save(&engine, |value| {
        value["slots"][child.slot() as usize]["organism"]["identity"]["parents"] =
            serde_json::json!([parent, null]);
    });
    assert_eq!(engine.find_best_robot_weighted(0), Some(parent));
    assert_eq!(engine.find_best_robot_weighted(50), Some(parent));
    assert_eq!(engine.find_best_robot_weighted(100), Some(parent));
    assert_eq!(engine.find_best_robot_weighted(150), Some(rich));
    assert_eq!(engine.find_best_robot_weighted(200), Some(rich));
}

#[test]
fn zerobot_find_best_rejects_cancer_families() {
    let mut engine = quiet_engine();
    engine
        .configure_local_mode(darwinbots_engine::LocalModeConfig {
            kind: darwinbots_engine::LocalModeKind::Zerobot,
            ..Default::default()
        })
        .unwrap();
    let dna = LegacyDna::parse("start stop").unwrap();
    let parent = engine.spawn_at(dna.clone(), [400.0, 400.0]).unwrap();
    let child = engine.spawn_at(dna.clone(), [600.0, 400.0]).unwrap();
    let healthy = engine.spawn_at(dna, [800.0, 800.0]).unwrap();
    let engine = edit_save(&engine, |value| {
        value["slots"][child.slot() as usize]["organism"]["identity"]["parents"] =
            serde_json::json!([parent, null]);
        value["slots"][child.slot() as usize]["organism"]["materials"]["body"] =
            serde_json::json!(1);
        value["slots"][child.slot() as usize]["organism"]["materials"]["energy"] =
            serde_json::json!(1);
    });
    assert_eq!(engine.find_best_robot_weighted(0), Some(healthy));
}

#[test]
fn console_dna_execution_and_memory_1000_do_not_advance_physics() {
    let mut engine = quiet_engine();
    let id = engine
        .spawn_at(
            LegacyDna::parse("start 42 1000 store 20 .up store stop").unwrap(),
            [500.0, 500.0],
        )
        .unwrap();
    let before = engine.organism(id).unwrap();
    let position = before.position;
    let age = before.age;
    engine.execute_robots().unwrap();
    assert_eq!(engine.snapshot().tick, 0);
    assert_eq!(engine.organism(id).unwrap().position, position);
    assert_eq!(engine.organism(id).unwrap().age, age);
    assert_eq!(
        engine.robot_state(id).unwrap().program.memory.read(1000),
        42
    );
}

#[test]
fn history_obeys_the_configured_sampling_interval() {
    let mut config = EngineConfig::testing();
    config.recording.sampling_interval = 17;
    let mut engine = Engine::new(config).unwrap();
    engine.tick_many(35).unwrap();
    assert_eq!(
        engine
            .snapshot()
            .history
            .iter()
            .map(|sample| sample.tick)
            .collect::<Vec<_>>(),
        vec![17, 34]
    );
}

#[test]
fn league_profiles_change_the_world_and_restart_original_population() {
    use darwinbots_engine::{LeaguePreset, LocalModeConfig};
    for (preset, energy, size) in [
        (LeaguePreset::F1, 40, [9237.0, 6928.0]),
        (LeaguePreset::F2, 30, [9237.0, 6928.0]),
        (LeaguePreset::F3, 30, [9237.0, 6928.0]),
        (LeaguePreset::Multibot, 30, [9237.0, 6928.0]),
        (LeaguePreset::ShortBot, 40, [16000.0, 12000.0]),
    ] {
        let mut engine = quiet_engine();
        let species = engine.register_species(SpeciesDefinition {
            name: "Entrant".into(),
            ..Default::default()
        });
        engine
            .spawn_species_batch(
                LegacyDna::parse("start stop").unwrap(),
                species,
                [[500.0, 500.0]],
                3000,
            )
            .unwrap();
        engine
            .configure_local_mode(LocalModeConfig {
                preset,
                ..Default::default()
            })
            .unwrap();
        let save = SaveFile::encode(&engine).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&save[10..]).unwrap();
        assert_eq!(value["config"]["vegetation"]["max_energy_per_tick"], energy);
        assert_eq!(value["config"]["physics"]["max_velocity"], 180.0);
        assert_eq!(value["config"]["dna_costs"]["store"], 0.04);
        assert_eq!(engine.snapshot().world_size, size);
        assert!(!engine.snapshot().mutations_enabled);
        engine.restart_local_mode().unwrap();
        assert_eq!(engine.population(), 5);
    }
}

#[test]
fn contest_detects_a_surviving_species_and_rejects_invalid_winners_atomically() {
    use darwinbots_engine::{LocalModeConfig, LocalModeStatus, SpeciesId};
    let mut engine = quiet_engine();
    let a = engine.register_species(SpeciesDefinition {
        name: "A".into(),
        ..Default::default()
    });
    let b = engine.register_species(SpeciesDefinition {
        name: "B".into(),
        ..Default::default()
    });
    let dna = LegacyDna::parse("start stop").unwrap();
    engine
        .spawn_species_batch(dna.clone(), a, [[500.0, 500.0]], 3000)
        .unwrap();
    engine
        .spawn_species_batch(dna, b, [[900.0, 900.0]], 3000)
        .unwrap();
    engine
        .configure_local_mode(LocalModeConfig {
            minimum_rounds: 1,
            maximum_rounds: 1,
            ..Default::default()
        })
        .unwrap();
    engine.start_local_mode().unwrap();
    let before = SaveFile::encode(&engine).unwrap();
    assert!(
        engine
            .adjudicate_local_round(Some(SpeciesId(999)), "bad winner".into())
            .is_err()
    );
    assert_eq!(before, SaveFile::encode(&engine).unwrap());
    // Starting a league reloads five competitors per species and retires the
    // setup IDs. Eliminate the current opponent roster, not a stale setup ID.
    let opponents: Vec<_> = engine
        .snapshot()
        .organisms
        .iter()
        .filter(|robot| robot.species == b)
        .map(|robot| robot.id)
        .collect();
    engine.remove_many(&opponents).unwrap();
    engine.tick_many(10).unwrap(); // Native contest population is sampled every ten cycles.
    assert_eq!(
        engine.snapshot().local_mode.status,
        LocalModeStatus::Completed
    );
    assert_eq!(engine.snapshot().local_mode.results[0].winner, Some(a));
    assert!(engine.snapshot().local_mode.results[0].automatic);
}
