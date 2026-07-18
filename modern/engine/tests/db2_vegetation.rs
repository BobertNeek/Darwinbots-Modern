mod support;

use darwinbots_engine::{
    Engine, EngineConfig, LegacyDna, SpeciesDefinition, VegetationSettings, sysvar_address,
};
use support::db2_fixtures::{
    MEM_AVAILABILITY, MEM_CHLR, MEM_LIGHT, MEM_MAKE_CHLR, MEM_REMOVE_CHLR, MEM_SHARE_CHLR,
    START_CHLR,
};

#[test]
fn chloroplast_sysvars_use_db2_memory_addresses() {
    assert_eq!(sysvar_address("chlr"), Some(MEM_CHLR));
    assert_eq!(sysvar_address("mkchlr"), Some(MEM_MAKE_CHLR));
    assert_eq!(sysvar_address("rmchlr"), Some(MEM_REMOVE_CHLR));
    assert_eq!(sysvar_address("light"), Some(MEM_LIGHT));
    assert_eq!(sysvar_address("availability"), Some(MEM_AVAILABILITY));
    assert_eq!(sysvar_address("sharechlr"), Some(MEM_SHARE_CHLR));
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

        let Some(name) = pending_name.take() else { continue };
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
    assert_eq!(plants.iter().map(|bot| bot.chloroplasts).sum::<i32>(), START_CHLR);
    assert!(plants.iter().all(|bot| bot.chloroplasts > 0));
}

#[test]
fn full_chloroplast_plant_receives_db2_light_and_body_split() {
    let mut engine = Engine::new(EngineConfig {
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
    assert_eq!(engine.memory_at(plant, MEM_LIGHT).unwrap(), 1);
}

#[test]
fn darkness_prevents_chloroplast_feeding() {
    let mut engine = Engine::new(EngineConfig {
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
    assert_eq!(engine.memory_at(plant, MEM_LIGHT).unwrap(), 0);
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
fn sunlight_intensity_scales_non_pond_photosynthesis() {
    fn energy_after_one_tick(sunlight_energy: i32) -> i32 {
        let mut engine = Engine::new(EngineConfig {
            metabolism_cost: 0,
            sunlight_energy,
            vegetation: VegetationSettings {
                max_energy_per_tick: 40,
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

    let darkness = energy_after_one_tick(0);
    let full_light = energy_after_one_tick(100);

    assert_eq!(darkness, 1_000);
    assert!(full_light > darkness);
}
