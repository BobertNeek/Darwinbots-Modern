mod support;

use darwinbots_engine::{
    DnaCosts, Engine, EngineConfig, LegacyDna, SaveFile, SpeciesDefinition, VegetationSettings,
    sysvar_address,
};
use support::db2_fixtures::{
    MEM_AVAILABILITY, MEM_CHLR, MEM_LIGHT, MEM_MAKE_CHLR, MEM_REMOVE_CHLR, MEM_SHARE_CHLR,
    START_CHLR,
};

const MEM_DAYTIME: i32 = 218;

// Old DB3S checkpoints without native_random keep the explicit legacy profile.
fn legacy_profile(config: EngineConfig) -> Engine {
    let encoded = SaveFile::encode(&Engine::new(config).unwrap()).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    payload["native_random"] = serde_json::Value::Null;
    let payload = serde_json::to_vec(&payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(&encoded[..6]);
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    SaveFile::decode(&save).unwrap()
}

#[test]
fn chloroplast_sysvars_use_db2_memory_addresses() {
    assert_eq!(sysvar_address("chlr"), Some(MEM_CHLR));
    assert_eq!(sysvar_address("mkchlr"), Some(MEM_MAKE_CHLR));
    assert_eq!(sysvar_address("rmchlr"), Some(MEM_REMOVE_CHLR));
    assert_eq!(sysvar_address("light"), Some(MEM_LIGHT));
    assert_eq!(sysvar_address("daytime"), Some(MEM_DAYTIME));
    assert_eq!(sysvar_address("availability"), Some(MEM_AVAILABILITY));
    assert_eq!(sysvar_address("sharechlr"), Some(MEM_SHARE_CHLR));
}

#[test]
fn slime_shell_poison_and_venom_use_db2_caps_costs_and_decay() {
    let mut config = EngineConfig::testing();
    config.metabolism_cost = 0;
    config.gravity = [0.0, 0.0];
    config.drag = 0.0;
    config.physics.surface_gravity = 0.0;
    config.physics.static_friction = 0.0;
    config.physics.kinetic_friction = 0.0;
    config.dna_costs = DnaCosts {
        shell: 0.5,
        slime: 0.5,
        venom: 0.5,
        poison: 0.5,
        ..DnaCosts::default()
    };
    let mut engine = Engine::new(config).unwrap();
    let id = engine
        .spawn_at(
            LegacyDna::parse(
                "start\n1000 .mkshell store\n1000 .mkslime store\n1000 .mkvenom store\n1000 .mkpoison store\nstop",
            )
            .unwrap(),
            [500.0, 500.0],
        )
        .unwrap();
    engine.set_organism_energy(id, 32_000).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory(id, "shell").unwrap(), 100);
    assert_eq!(engine.memory(id, "slime").unwrap(), 200);
    assert_eq!(engine.memory(id, "venom").unwrap(), 100);
    assert_eq!(engine.memory(id, "poison").unwrap(), 100);
    assert_eq!(engine.organism(id).unwrap().energy, 31_595);

    engine
        .replace_dna(id, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.memory(id, "slime").unwrap(), 196);
    assert_eq!(engine.memory(id, "poison").unwrap(), 98);
}

#[test]
fn slime_blocks_ties_by_the_legacy_probability_threshold() {
    let mut config = EngineConfig::testing();
    config.metabolism_cost = 0;
    config.gravity = [0.0, 0.0];
    config.drag = 0.0;
    config.physics.surface_gravity = 0.0;
    config.physics.static_friction = 0.0;
    config.physics.kinetic_friction = 0.0;
    config.organism_capacity = 128;
    let mut engine = Engine::new(config).unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n1 .tie store\nstop").unwrap(),
            [200.0, 500.0],
        )
        .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n1 .mkslime store\nstop").unwrap(),
            [300.0, 500.0],
        )
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.snapshot().stats.ties_created, 1);
}

#[test]
fn every_db2_248_sysvar_resolves_to_its_vb6_address() {
    let source = include_str!("../../../Darwinbots2/DNATokenizing.bas");
    let mut pending_name = None;
    let mut checked = 0;

    for line in source.lines().map(str::trim) {
        if let Some((_, remainder)) = line
            .strip_prefix("sysvar(")
            .and_then(|line| line.split_once(".Name = \""))
        {
            pending_name = Some(remainder.trim_end_matches('"').to_ascii_lowercase());
            continue;
        }

        let Some(name) = pending_name.take() else {
            continue;
        };
        let Some((_, value)) = line
            .strip_prefix("sysvar(")
            .and_then(|line| line.split_once(".value = "))
        else {
            pending_name = Some(name);
            continue;
        };
        let expected: i32 = value.trim().parse().expect("VB6 sysvar address");
        assert_eq!(sysvar_address(&name), Some(expected), "DB2 sysvar .{name}");
        checked += 1;
    }

    assert_eq!(checked, 255, "the DB2 2.48 source sysvar table changed");
}

#[test]
fn initial_and_reseeded_vegetables_start_with_db2_chloroplasts() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        vegetation: VegetationSettings {
            minimum_chloroplast_equivalents: 1,
            repopulation_amount: 1,
            repopulation_cooldown: 1,
            ..VegetationSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        name: "Fixture vegetables".to_owned(),
        vegetable: true,
        minimum_population: 1,
        reseed: true,
        ..SpeciesDefinition::default()
    });
    let initial = engine
        .spawn_species_batch(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetables,
            [[100.0, 100.0]],
            1_000,
        )
        .unwrap()[0];
    assert_eq!(engine.organism(initial).unwrap().chloroplasts, START_CHLR);

    engine.remove(initial).unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.snapshot().organisms.len(), 1);
    assert_eq!(engine.snapshot().organisms[0].chloroplasts, START_CHLR);
}

#[test]
fn vegetable_reproduction_splits_parent_biology_without_resetting_child() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        name: "Reproducing vegetables".to_owned(),
        vegetable: true,
        ..SpeciesDefinition::default()
    });
    engine
        .spawn_species_batch(
            LegacyDna::parse("start\n50 .repro store\nstop").unwrap(),
            vegetables,
            [[500.0, 500.0]],
            1_000,
        )
        .unwrap();

    engine.tick().unwrap();

    let plants: Vec<_> = engine
        .snapshot()
        .organisms
        .iter()
        .filter(|bot| bot.species == vegetables)
        .collect();
    assert_eq!(plants.len(), 2);
    assert_eq!(
        plants.iter().map(|bot| bot.chloroplasts).sum::<i32>(),
        START_CHLR
    );
    assert!(plants.iter().all(|bot| bot.chloroplasts > 0));
}

#[test]
fn vegetable_reproduction_is_throttled_near_the_population_cap() {
    let mut engine = Engine::new(EngineConfig {
        seed: 1,
        metabolism_cost: 0,
        vegetable_energy_per_tick: 0,
        vegetable_population_cap: 11,
        vegetation: VegetationSettings {
            daytime: false,
            start_chloroplasts: 32_000,
            ..VegetationSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        name: "Near-cap vegetables".to_owned(),
        vegetable: true,
        reseed: true,
        ..SpeciesDefinition::default()
    });
    let parent = engine
        .spawn_species_at(
            LegacyDna::parse("start\n50 .repro store\nstop").unwrap(),
            vegetables,
            [1_000.0, 1_000.0],
        )
        .unwrap();
    engine
        .spawn_species_batch(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetables,
            [
                [3_000.0, 1_000.0],
                [5_000.0, 1_000.0],
                [7_000.0, 1_000.0],
                [9_000.0, 1_000.0],
            ],
            1_000,
        )
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.vegetable_population(), 5);
    assert_eq!(engine.organism(parent).unwrap().offspring, 0);
}

#[test]
fn full_chloroplast_plant_receives_db2_light_and_body_split() {
    let mut engine = Engine::new_native(EngineConfig {
        metabolism_cost: 0,
        vegetable_energy_per_tick: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        vegetable: true,
        ..SpeciesDefinition::default()
    });
    let plant = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetables,
            [500.0, 500.0],
        )
        .unwrap();
    let before = engine.organism(plant).unwrap();

    engine.tick().unwrap();

    let after = engine.organism(plant).unwrap();
    assert!(after.energy > before.energy);
    assert!(after.body > before.body);
    // ManageChlr reads the previous process LightAval before this late feed.
    assert_eq!(engine.memory_at(plant, MEM_LIGHT).unwrap(), 32_000);
    assert_eq!(engine.memory_at(plant, MEM_DAYTIME).unwrap(), 1);
}

#[test]
fn darkness_prevents_chloroplast_feeding() {
    let mut engine = Engine::new_native(EngineConfig {
        metabolism_cost: 0,
        vegetable_energy_per_tick: 0,
        vegetation: VegetationSettings {
            daytime: false,
            ..VegetationSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        vegetable: true,
        ..SpeciesDefinition::default()
    });
    let plant = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetables,
            [500.0, 500.0],
        )
        .unwrap();
    let before = engine.organism(plant).unwrap().energy;

    engine.tick().unwrap();

    assert!(engine.organism(plant).unwrap().energy <= before);
    // Darkness skips FeedVeg, so availability stays at its fresh process image.
    assert_eq!(engine.memory_at(plant, MEM_LIGHT).unwrap(), 32_000);
    assert_eq!(engine.memory_at(plant, MEM_DAYTIME).unwrap(), 0);
}

#[test]
fn chloroplasts_decay_naturally_without_a_removal_command() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        vegetable_energy_per_tick: 0,
        vegetation: VegetationSettings {
            daytime: false,
            start_chloroplasts: 1,
            ..VegetationSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        vegetable: true,
        ..SpeciesDefinition::default()
    });
    let plant = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetables,
            [500.0, 500.0],
        )
        .unwrap();

    engine.tick_many(3).unwrap();

    assert_eq!(engine.organism(plant).unwrap().chloroplasts, 0);
}

#[test]
fn chloroplast_pressure_uses_equivalents_and_a_strict_population_boundary() {
    // Robots.bas ChangeChlr uses TotalChlr > MaxPopulation, not bot count.
    for (initial, expected) in [(START_CHLR, START_CHLR + 10), (32_000, 32_000)] {
        let mut engine = Engine::new(EngineConfig {
            metabolism_cost: 0,
            vegetable_energy_per_tick: 0,
            vegetable_population_cap: 1,
            vegetation: VegetationSettings {
                daytime: false,
                start_chloroplasts: initial,
                ..VegetationSettings::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        let vegetables = engine.register_species(SpeciesDefinition {
            vegetable: true,
            ..SpeciesDefinition::default()
        });
        let plant = engine
            .spawn_species_at(
                LegacyDna::parse("start\n10 .mkchlr store\nstop").unwrap(),
                vegetables,
                [500.0, 500.0],
            )
            .unwrap();
        engine.tick().unwrap();
        assert_eq!(engine.organism(plant).unwrap().chloroplasts, expected);
        assert_eq!(engine.memory(plant, "mkchlr").unwrap(), 0);
    }
}

#[test]
fn chloroplast_construction_charges_the_configured_cost_once() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        vegetable_energy_per_tick: 0,
        dna_costs: DnaCosts {
            chloroplast_construction: 2.0,
            ..DnaCosts::default()
        },
        vegetation: VegetationSettings {
            daytime: false,
            start_chloroplasts: 0,
            ..VegetationSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        vegetable: true,
        ..SpeciesDefinition::default()
    });
    let plant = engine
        .spawn_species_at(
            LegacyDna::parse("start\n10 .mkchlr store\nstop").unwrap(),
            vegetables,
            [500.0, 500.0],
        )
        .unwrap();
    let before = engine.organism(plant).unwrap();

    engine.tick().unwrap();

    let after = engine.organism(plant).unwrap();
    assert_eq!(after.chloroplasts, 10);
    assert_eq!(after.energy, before.energy - 20);
}

#[test]
fn chloroplasts_metabolize_waste_into_energy_and_body() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        vegetable_energy_per_tick: 0,
        vegetation: VegetationSettings {
            daytime: false,
            feeding_to_body: 0.5,
            ..VegetationSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        vegetable: true,
        ..SpeciesDefinition::default()
    });
    let plant = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetables,
            [500.0, 500.0],
        )
        .unwrap();
    let encoded = SaveFile::encode(&engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    let organism = &mut payload["slots"][plant.slot() as usize]["organism"];
    organism["materials"]["chloroplasts"] = serde_json::json!(32_000);
    organism["materials"]["waste"] = serde_json::json!(100);
    let payload = serde_json::to_vec(&payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(&encoded[..6]);
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    let mut engine = SaveFile::decode(&save).unwrap();
    let before = engine.organism(plant).unwrap();

    engine.tick_many(50).unwrap();

    let after = engine.organism(plant).unwrap();
    assert!(after.waste < before.waste);
    assert!(after.energy > before.energy);
    assert!(after.body > before.body);
}

#[test]
fn repopulation_waits_for_cooldown_and_spawns_configured_batch() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        vegetable_energy_per_tick: 0,
        vegetation: VegetationSettings {
            minimum_chloroplast_equivalents: 1,
            repopulation_amount: 2,
            repopulation_cooldown: 3,
            ..VegetationSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        vegetable: true,
        reseed: true,
        ..SpeciesDefinition::default()
    });
    let initial = engine
        .spawn_species_batch(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetables,
            [[100.0, 100.0]],
            1_000,
        )
        .unwrap()[0];
    engine.remove(initial).unwrap();

    engine.tick_many(2).unwrap();
    assert_eq!(engine.vegetable_population(), 0);
    engine.tick().unwrap();
    assert_eq!(engine.vegetable_population(), 2);
}

#[test]
fn legacy_profile_sunlight_intensity_scales_non_pond_photosynthesis() {
    fn energy_after_one_tick(sunlight_energy: i32) -> i32 {
        let mut engine = legacy_profile(EngineConfig {
            metabolism_cost: 0,
            sunlight_energy,
            vegetation: VegetationSettings {
                max_energy_per_tick: 40,
                feeding_to_body: 0.0,
                ..VegetationSettings::default()
            },
            ..EngineConfig::testing()
        });
        let vegetables = engine.register_species(SpeciesDefinition {
            vegetable: true,
            ..SpeciesDefinition::default()
        });
        let plant = engine
            .spawn_species_at(
                LegacyDna::parse("start\nstop").unwrap(),
                vegetables,
                [500.0, 500.0],
            )
            .unwrap();
        engine.tick().unwrap();
        engine.organism(plant).unwrap().energy
    }

    let darkness = energy_after_one_tick(0);
    let full_light = energy_after_one_tick(100);

    assert_eq!(darkness, 1_000);
    assert!(full_light > darkness);
}

#[test]
fn native_non_pond_photosynthesis_uses_max_energy_without_a_sunlight_scalar() {
    fn energy_after_one_tick(max_energy_per_tick: i32, sunlight_energy: i32) -> i32 {
        let mut engine = Engine::new_native(EngineConfig {
            metabolism_cost: 0,
            sunlight_energy,
            vegetation: VegetationSettings {
                max_energy_per_tick,
                feeding_to_body: 0.0,
                ..VegetationSettings::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        let vegetables = engine.register_species(SpeciesDefinition {
            vegetable: true,
            ..SpeciesDefinition::default()
        });
        let plant = engine
            .spawn_species_at(
                LegacyDna::parse("start\nstop").unwrap(),
                vegetables,
                [500.0, 500.0],
            )
            .unwrap();
        engine.tick().unwrap();
        engine.organism(plant).unwrap().energy
    }

    // Vegs.feedvegs takes the MaxEnergy Long directly when Pondmode is false.
    let no_legacy_scalar = energy_after_one_tick(40, 0);
    assert_eq!(no_legacy_scalar, energy_after_one_tick(40, 100));
    assert_eq!(no_legacy_scalar, energy_after_one_tick(40, 200));
    assert_eq!(energy_after_one_tick(0, 100), 1_000);
    assert!(no_legacy_scalar > 1_000);
}

#[test]
fn day_night_cycle_changes_after_the_configured_number_of_completed_cycles() {
    let mut engine = Engine::new_native(EngineConfig {
        fixed_bot_radii: true,
        metabolism_cost: 0,
        vegetation: VegetationSettings {
            day_night_enabled: true,
            cycle_length: 2,
            ..VegetationSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        vegetable: true,
        ..SpeciesDefinition::default()
    });
    let plant = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetables,
            [500.0, 500.0],
        )
        .unwrap();

    engine.tick().unwrap();
    assert_eq!(engine.memory_at(plant, MEM_DAYTIME).unwrap(), 1);
    assert_eq!(engine.memory_at(plant, MEM_LIGHT).unwrap(), 32_000);
    engine.tick().unwrap();
    assert_eq!(engine.memory_at(plant, MEM_DAYTIME).unwrap(), 1);
    // Radius 60 in a 1000 by 1000 world: round(32000 - pi*60^2/1e6*32000).
    assert_eq!(engine.memory_at(plant, MEM_LIGHT).unwrap(), 31_638);
    engine.tick().unwrap();
    assert_eq!(engine.memory_at(plant, MEM_DAYTIME).unwrap(), 0);
    assert_eq!(engine.memory_at(plant, MEM_LIGHT).unwrap(), 31_638);
}

#[test]
fn tides_reduce_photosynthetic_yield_at_high_tide() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        sunlight_energy: 100,
        vegetation: VegetationSettings {
            max_energy_per_tick: 40,
            feeding_to_body: 0.0,
            tides: 4,
            ..VegetationSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        vegetable: true,
        ..SpeciesDefinition::default()
    });
    let plant = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetables,
            [500.0, 500.0],
        )
        .unwrap();
    let initial_energy = engine.organism(plant).unwrap().energy;

    engine.tick().unwrap();
    let high_tide_energy = engine.organism(plant).unwrap().energy;
    engine.tick().unwrap();
    let low_tide_energy = engine.organism(plant).unwrap().energy;

    assert_eq!(high_tide_energy, initial_energy);
    assert!(low_tide_energy > high_tide_energy);
}

#[test]
fn total_energy_threshold_modes_apply_vb6_sun_overrides() {
    fn run(daytime: bool, mode: i32, sun_up: bool, sun_down: bool) -> (i32, bool) {
        let mut engine = Engine::new_native(EngineConfig {
            fixed_bot_radii: true,
            metabolism_cost: 0,
            vegetation: VegetationSettings {
                daytime,
                day_night_enabled: true,
                cycle_length: 10,
                sun_up_enabled: sun_up,
                sun_up_threshold: 20_000,
                sun_down_enabled: sun_down,
                sun_down_threshold: 0,
                sun_threshold_mode: mode,
                ..VegetationSettings::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        let vegetables = engine.register_species(SpeciesDefinition {
            vegetable: true,
            ..SpeciesDefinition::default()
        });
        let plant = engine
            .spawn_species_at(
                LegacyDna::parse("start\nstop").unwrap(),
                vegetables,
                [500.0, 500.0],
            )
            .unwrap();
        let before = engine.organism(plant).unwrap().energy;
        engine.tick().unwrap();
        // Master reads the previous ring slot. Fresh displayed 0 equals SunDown 0,
        // so the strict '>' check cannot suppress this first feed.
        assert_eq!(engine.snapshot().stats.native_energy_displayed, Some(0));
        assert_eq!(engine.memory_at(plant, MEM_LIGHT).unwrap(), 32_000);
        assert_eq!(engine.memory_at(plant, MEM_DAYTIME).unwrap(), 1);
        assert!(engine.organism(plant).unwrap().energy > before);
        engine.tick().unwrap();
        assert!(engine.snapshot().stats.native_energy_displayed.unwrap() > 0);
        assert_eq!(engine.memory_at(plant, MEM_LIGHT).unwrap(), 31_638);
        let payload: serde_json::Value =
            serde_json::from_slice(&SaveFile::encode(&engine).unwrap()[10..]).unwrap();
        (
            engine.memory_at(plant, MEM_DAYTIME).unwrap(),
            payload["config"]["vegetation"]["daytime"]
                .as_bool()
                .unwrap(),
        )
    }

    assert_eq!(run(true, 0, false, true), (0, true));
    assert_eq!(run(true, 1, false, true), (0, false));
    assert_eq!(run(true, 2, false, true), (0, false));
    assert_eq!(run(false, 0, true, false), (1, false));
}

#[test]
fn weather_sun_band_controls_daytime_independently_of_cached_light() {
    let mut engine = Engine::new_native(EngineConfig {
        fixed_bot_radii: true,
        metabolism_cost: 0,
        vegetation: VegetationSettings {
            weather_enabled: true,
            ..VegetationSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        vegetable: true,
        ..SpeciesDefinition::default()
    });
    let inside = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetables,
            [100.0, 500.0],
        )
        .unwrap();
    let outside = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetables,
            [500.0, 500.0],
        )
        .unwrap();
    let encoded = SaveFile::encode(&engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    payload["vegetation_runtime"]["sun_position"] = serde_json::json!(0.1);
    payload["vegetation_runtime"]["sun_range"] = serde_json::json!(0.0);
    payload["vegetation_runtime"]["sun_change"] = serde_json::json!(11);
    let payload = serde_json::to_vec(&payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(&encoded[..6]);
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    let mut engine = SaveFile::decode(&save).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory_at(inside, MEM_DAYTIME).unwrap(), 1);
    assert_eq!(engine.memory_at(outside, MEM_DAYTIME).unwrap(), 0);
    assert_eq!(engine.memory_at(inside, MEM_LIGHT).unwrap(), 32_000);
    assert_eq!(engine.memory_at(outside, MEM_LIGHT).unwrap(), 32_000);

    engine.tick().unwrap();

    // Both actors sense the same prior occupied area, regardless of sun band.
    assert_eq!(engine.memory_at(inside, MEM_LIGHT).unwrap(), 31_276);
    assert_eq!(engine.memory_at(outside, MEM_LIGHT).unwrap(), 31_276);
    assert_eq!(engine.memory_at(inside, MEM_DAYTIME).unwrap(), 1);
    assert_eq!(engine.memory_at(outside, MEM_DAYTIME).unwrap(), 0);
}
