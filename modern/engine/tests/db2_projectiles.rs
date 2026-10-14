mod support;

use darwinbots_engine::{
    DnaCosts, Engine, EngineConfig, LegacyDna, Obstacle, PhysicsSettings, ShapeShotPolicy,
    ShotSettings, SpeciesDefinition,
};
use support::db2_fixtures::SHOT_SPEED;

#[test]
fn db24832_shootval_positive_power_and_negative_range_are_applied_to_spawn() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let power_shooter = engine
        .spawn_at(
            LegacyDna::parse("start\n-1 .shoot store\n5 .shootval store\nstop").unwrap(),
            [250.0, 500.0],
        )
        .unwrap();
    let range_shooter = engine
        .spawn_at(
            LegacyDna::parse("start\n-1 .shoot store\n-8 .shootval store\nstop").unwrap(),
            [100.0, 400.0],
        )
        .unwrap();

    engine.tick().unwrap();

    let power_shot = engine
        .snapshot()
        .shots
        .iter()
        .find(|shot| shot.owner == power_shooter)
        .unwrap();
    let range_shot = engine
        .snapshot()
        .shots
        .iter()
        .find(|shot| shot.owner == range_shooter)
        .unwrap();

    assert_eq!(power_shot.value, 291);
    assert_eq!(power_shot.range, 11);
    assert_eq!(range_shot.value, 220);
    assert_eq!(range_shot.range, 21);
}

#[test]
fn firing_creates_a_moving_projectile_instead_of_an_instant_hit_line() {
    let mut engine = Engine::new(EngineConfig {
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let attacker = engine
        .spawn_at(
            LegacyDna::parse("start\n-1 .shoot store\nstop").unwrap(),
            [250.0, 500.0],
        )
        .unwrap();
    engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [650.0, 500.0])
        .unwrap();

    engine.tick().unwrap();
    engine.tick().unwrap();

    let shot = &engine.snapshot().shots[0];
    assert_eq!(shot.owner, attacker);
    assert!((segment_length(shot.start, shot.end) - SHOT_SPEED).abs() < 0.01);
    assert_ne!(shot.end, [650.0, 500.0]);
}

#[test]
fn energy_feeding_request_extracts_target_resources_and_returns_energy_packet() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        shots: ShotSettings {
            feeding_proportional: false,
            feeding_fixed_power: 100,
            ..Default::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let attacker = engine.spawn_at(idle.clone(), [250.0, 500.0]).unwrap();
    let target = engine.spawn_at(idle, [500.0, 500.0]).unwrap();
    engine.tick().unwrap();
    let before = engine.robot_state(target).unwrap().materials.clone();

    engine
        .replace_dna(
            attacker,
            LegacyDna::parse("start\n-1 .shoot store\n100 .shootval store\nstop").unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(attacker, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();

    let after = engine.robot_state(target).unwrap().materials.clone();
    assert_eq!(after.energy, before.energy - 90.0);
    assert_eq!(after.body, before.body - 1.0);
    assert_eq!(after.waste, before.waste);
    assert!(
        engine
            .snapshot()
            .shots
            .iter()
            .any(|shot| shot.owner == target && shot.kind == -2 && shot.value == 100)
    );
}

#[test]
fn energy_packet_applies_db24832_transfer_without_a_live_packet_owner() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        shots: ShotSettings {
            energy_shots_do_not_decay: true,
            ..ShotSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let source = engine
        .spawn_species_batch(
            LegacyDna::parse("start\n-2 .shoot store\n1000 .shootval store\nstop").unwrap(),
            darwinbots_engine::SpeciesId::default(),
            [[250.0, 500.0]],
            1_000,
        )
        .unwrap()[0];
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
        .unwrap();
    let before = engine.robot_state(target).unwrap().materials.clone();

    engine.tick().unwrap();
    assert!(engine.organism(source).is_err());
    engine.tick().unwrap();

    let after = engine.robot_state(target).unwrap().materials.clone();
    assert_eq!(after.energy, before.energy + 950.0);
    assert_eq!(after.body, before.body + 4.0);
    assert_eq!(after.waste, before.waste + 10.0);
}

#[test]
fn shot_velocity_inherits_the_firers_actual_velocity() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let attacker = engine
        .spawn_at(
            LegacyDna::parse("start\n10 .up store\nstop").unwrap(),
            [250.0, 500.0],
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

    let shot = &engine.snapshot().shots[0];
    assert!(shot.velocity[0] > SHOT_SPEED);
}

#[test]
fn projectile_moves_once_per_tick_and_expires_after_range() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("cond\n*.robage 1 <\nstart\n-1 .shoot store\nstop").unwrap(),
            [250.0, 500.0],
        )
        .unwrap();

    engine.tick().unwrap();
    let first = engine.snapshot().shots[0].clone();
    engine.tick().unwrap();
    let second = engine.snapshot().shots[0].clone();
    assert_eq!(second.start, first.end);

    engine.tick_many(20).unwrap();
    assert!(engine.snapshot().shots.is_empty());
}

#[test]
fn swept_collision_hits_a_bot_between_projectile_endpoints() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        shots: ShotSettings {
            feeding_proportional: false,
            feeding_fixed_power: 100,
            ..Default::default()
        },
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n-1 .shoot store\n100 .shootval store\nstop").unwrap(),
            [250.0, 500.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
        .unwrap();
    let before = engine.organism(target).unwrap().energy;

    engine.tick().unwrap();
    engine.tick().unwrap();

    assert!(engine.organism(target).unwrap().energy < before);
    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
    assert_eq!(engine.snapshot().stats.projectile_effects, 1);
}

#[test]
fn lethal_projectile_hit_cannot_be_undone_by_same_tick_photosynthesis() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        fixed_bot_radii: true,
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        name: "Killable vegetables".to_owned(),
        vegetable: true,
        reseed: false,
        ..SpeciesDefinition::default()
    });
    engine
        .spawn_at(
            LegacyDna::parse("start\n-1 .shoot store\n100 .shootval store\nstop").unwrap(),
            [250.0, 500.0],
        )
        .unwrap();
    let target = engine
        .spawn_species_batch(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetables,
            [[400.0, 500.0]],
            1,
        )
        .unwrap()[0];

    engine.tick().unwrap();
    engine.tick().unwrap();

    assert!(engine.organism(target).is_err());
    assert_eq!(engine.snapshot().stats.deaths, 1);
}

#[test]
fn newborn_is_immune_to_parent_stream_for_one_tick() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let parent = engine
        .spawn_at(
            LegacyDna::parse("start\n-1 .shoot store\n100 .shootval store\nstop").unwrap(),
            [250.0, 500.0],
        )
        .unwrap();
    let newborn = engine
        .manual_reproduce(parent, None, [500.0, 500.0])
        .unwrap();
    let initial_energy = engine.organism(newborn).unwrap().energy;

    engine.tick().unwrap();

    assert_eq!(engine.organism(newborn).unwrap().energy, initial_energy);
}

#[test]
fn positive_memory_shot_writes_the_target_address() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n500 .shoot store\n37 .shootval store\nstop").unwrap(),
            [250.0, 500.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
        .unwrap();

    engine.tick().unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.memory_at(target, 500).unwrap(), 37);
}

#[test]
fn delgene_information_shot_is_inert_without_defensive_poison_return() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n340 .shoot store\n37 .shootval store\nstop").unwrap(),
            [250.0, 500.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(
            LegacyDna::parse("start\n1000 .strpoison store\nstop").unwrap(),
            [500.0, 500.0],
        )
        .unwrap();

    engine.tick().unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.memory_at(target, 340).unwrap(), 0);
    assert!(
        !engine
            .snapshot()
            .shots
            .iter()
            .any(|shot| shot.kind == -5 && shot.owner == target && !shot.impact_flash)
    );
}

#[test]
fn moving_robot_obstacle_edge_preserves_collision_shot_and_drift_invariants() {
    let config = darwinbots_engine::EngineConfig {
        backend: darwinbots_engine::BackendPreference::Cpu,
        world_width: 512.0,
        world_height: 256.0,
        fixed_bot_radii: true,
        ..Default::default()
    };

    let mut engine = darwinbots_engine::Engine::new(config).unwrap();

    let mut obstacle = darwinbots_engine::Obstacle::new(95, [200.0, 96.0], [240.0, 128.0]);
    obstacle.drift = [1.0, 0.0];
    engine.add_obstacle(obstacle).unwrap();

    let mover_and_shooter = darwinbots_engine::LegacyDna::parse(
        "cond\n*.robage 0 =\nstart\n30 .veldx store\n100 .shoot store\nstop",
    )
    .unwrap();
    let mover = engine.spawn_at(mover_and_shooter, [180.0, 112.0]).unwrap();

    engine.tick().unwrap();
    engine.move_organism(mover, [180.0, 112.0]).unwrap();
    engine.tick().unwrap();
    let collision_snapshot = engine.snapshot();
    let collision_mover = collision_snapshot
        .organisms
        .iter()
        .find(|organism| organism.id == mover)
        .expect("moving robot must remain present after obstacle contact");
    let collision_obstacle = collision_snapshot
        .obstacles
        .iter()
        .find(|obstacle| obstacle.id == 95)
        .expect("real obstacle must remain in the published snapshot");
    // DB2 resolves obstacle collision before NetForces, movement, and position publication.
    let expanded_minimum = [
        collision_obstacle.minimum[0] - collision_mover.radius,
        collision_obstacle.minimum[1] - collision_mover.radius,
    ];
    let expanded_maximum = [
        collision_obstacle.maximum[0] + collision_mover.radius,
        collision_obstacle.maximum[1] + collision_mover.radius,
    ];
    let inside_expanded_obstacle = collision_mover.position[0] >= expanded_minimum[0]
        && collision_mover.position[0] <= expanded_maximum[0]
        && collision_mover.position[1] >= expanded_minimum[1]
        && collision_mover.position[1] <= expanded_maximum[1];
    let penetration = if inside_expanded_obstacle {
        let left = (collision_mover.position[0] - expanded_minimum[0]).max(0.0);
        let right = (expanded_maximum[0] - collision_mover.position[0]).max(0.0);
        let above = (collision_mover.position[1] - expanded_minimum[1]).max(0.0);
        let below = (expanded_maximum[1] - collision_mover.position[1]).max(0.0);
        left.min(right).min(above).min(below)
    } else {
        0.0
    };
    let allowed_reentry = collision_obstacle.drift[0]
        .abs()
        .max(collision_obstacle.drift[1].abs())
        + 0.05;
    assert!(
        penetration <= allowed_reentry,
        "radius-aware obstacle re-entry must be bounded: penetration={penetration}, allowed={allowed_reentry}, position={:?}, velocity={:?}",
        collision_mover.position,
        collision_mover.velocity
    );

    engine.tick_many(8).unwrap();
    let snapshot = engine.snapshot();

    let final_obstacle = snapshot
        .obstacles
        .iter()
        .find(|obstacle| obstacle.id == 95)
        .expect("obstacle must still be published");
    assert_eq!(final_obstacle.drift, [1.0, 0.0]);
    assert!((final_obstacle.maximum[0] - final_obstacle.minimum[0] - 40.0).abs() < 0.01);
    assert!((final_obstacle.maximum[1] - final_obstacle.minimum[1] - 32.0).abs() < 0.01);
    assert!(final_obstacle.minimum[0] >= 0.0);
    assert!(final_obstacle.maximum[0] <= snapshot.world_size[0]);

    assert!(
        snapshot.stats.projectile_impacts > 0
            || snapshot.shots.iter().any(|shot| shot.impact_flash),
        "the moving projectile must be intercepted by the obstacle before reaching the far edge"
    );
}

#[test]
fn obstacle_intercepts_projectile_before_target() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    engine
        .add_obstacle(Obstacle::new(1, [400.0, 450.0], [430.0, 550.0]))
        .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n-1 .shoot store\n100 .shootval store\nstop").unwrap(),
            [250.0, 500.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [700.0, 500.0])
        .unwrap();
    let before = engine.organism(target).unwrap().energy;

    engine.tick().unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.organism(target).unwrap().energy, before);
    assert!(engine.snapshot().shots[0].impact_flash);
}

#[test]
fn pass_through_shape_allows_a_projectile_to_reach_the_target() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let mut obstacle = Obstacle::new(1, [400.0, 450.0], [430.0, 550.0]);
    obstacle.shot_policy = ShapeShotPolicy::PassThrough;
    engine.add_obstacle(obstacle).unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n500 .shoot store\n37 .shootval store\nstop").unwrap(),
            [250.0, 500.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [700.0, 500.0])
        .unwrap();

    engine.tick_many(7).unwrap();

    assert_eq!(engine.memory_at(target, 500).unwrap(), 37);
}

#[test]
fn direct_poff_command_does_not_create_a_projectile() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n-100 .shoot store\n100 .shootval store\nstop").unwrap(),
            [250.0, 500.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
        .unwrap();
    let before = engine.organism(target).unwrap().energy;

    engine.tick().unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.organism(target).unwrap().energy, before);
    assert_eq!(engine.snapshot().stats.projectile_impacts, 0);
    assert_eq!(engine.snapshot().stats.shots_fired, 0);
}

#[test]
fn direct_minus_seven_command_does_not_emit_a_virus() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    let virus = LegacyDna::parse("start\n-7 .shoot store\nstop").unwrap();
    let owner = engine.spawn_at(virus.clone(), [250.0, 500.0]).unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
        .unwrap();

    engine.tick().unwrap();
    engine.tick().unwrap();

    assert_eq!(
        engine.dna(target).unwrap(),
        &LegacyDna::parse("start\nstop").unwrap()
    );
    assert_eq!(engine.dna(owner).unwrap(), &virus);
    assert_eq!(engine.snapshot().stats.shots_fired, 0);
}

#[test]
fn sperm_shot_fertilizes_without_forcing_reproduction() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n-8 .shoot store\nstop").unwrap(),
            [250.0, 500.0],
        )
        .unwrap();
    let mother = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [500.0, 500.0])
        .unwrap();

    engine.tick().unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.population(), 2);
    assert_eq!(engine.snapshot().stats.births, 0);
    assert_eq!(
        engine
            .robot_state(mother)
            .unwrap()
            .effects
            .fertilization_timer,
        10
    );
    assert!(
        engine
            .robot_state(mother)
            .unwrap()
            .effects
            .stored_sperm
            .is_some()
    );
}

#[test]
fn reproductive_projectiles_use_staged_virus_and_sperm_gates() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();

    let virus_owner = engine
        .spawn_at(
            LegacyDna::parse(
                "start\n1 .mkvirus store\n-7 .shoot store\n77 .shootval store\nstop\nend",
            )
            .unwrap(),
            [250.0, 300.0],
        )
        .unwrap();
    let virus_target = engine
        .spawn_at(
            LegacyDna::parse("start\n99 .shootval store\nstop\nend").unwrap(),
            [500.0, 300.0],
        )
        .unwrap();
    let sperm_owner = engine
        .spawn_at(
            LegacyDna::parse("start\n-8 .shoot store\nstop\nend").unwrap(),
            [250.0, 700.0],
        )
        .unwrap();
    let mother = engine
        .spawn_at(
            LegacyDna::parse("start\n0 .sexrepro store\nstop\nend").unwrap(),
            [500.0, 700.0],
        )
        .unwrap();

    engine.tick().unwrap();

    let virus_state = engine.robot_state(virus_owner).unwrap();
    assert!(virus_state.effects.stored_virus.is_some());
    assert!(virus_state.effects.virus_build_timer > 0);

    engine.tick().unwrap();

    let infected = engine.dna(virus_target).unwrap();
    assert!(infected.blocks().iter().any(|block| block.value == 99));
    assert!(!infected.blocks().iter().any(|block| block.value == 77));
    assert!(
        !engine
            .snapshot()
            .shots
            .iter()
            .any(|shot| shot.kind == -7 && !shot.stored)
    );
    assert_ne!(infected, engine.dna(virus_owner).unwrap());

    let sperm_state = engine.robot_state(mother).unwrap();
    assert_eq!(sperm_state.effects.fertilization_timer, 10);
    assert_eq!(
        sperm_state
            .effects
            .stored_sperm
            .as_ref()
            .map(|stored| stored.source),
        Some(Some(sperm_owner))
    );
    assert_eq!(engine.population(), 4);

    engine
        .replace_dna(mother, engine.dna(sperm_owner).unwrap().clone())
        .unwrap();
    engine.set_organism_memory(mother, 302, 50).unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.population(), 5);
    assert_eq!(engine.snapshot().stats.births, 1);
}

#[test]
fn shot_01_projectile_corpus_covers_moving_target_edges_and_range_decay_boundaries() {
    struct MovingTargetCase {
        name: &'static str,
        position: [f32; 2],
        expected_impacts: u64,
    }

    for case in [
        MovingTargetCase {
            name: "crossing hit",
            position: [600.0, 500.0],
            expected_impacts: 1,
        },
        MovingTargetCase {
            name: "near miss",
            position: [600.0, 800.0],
            expected_impacts: 0,
        },
    ] {
        let mut engine = Engine::new(EngineConfig {
            metabolism_cost: 0,
            physics: PhysicsSettings {
                density: 0.0,
                ..PhysicsSettings::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        engine
            .spawn_at(
                LegacyDna::parse("start\n-1 .shoot store\n100 .shootval store\nstop").unwrap(),
                [250.0, 500.0],
            )
            .unwrap();
        let target = engine
            .spawn_at(LegacyDna::parse("start\nstop").unwrap(), case.position)
            .unwrap();
        let mut batch = engine.physics_batch();
        let target_index = batch
            .ids
            .iter()
            .position(|id| *id == target)
            .expect("moving target must be in the physics batch");
        batch.batch.velocities[target_index] = [-80.0, 0.0];
        engine.apply_physics_batch(batch).unwrap();

        // VB6 Shots.bas NewShotCollision solves relative shot/bot motion.
        engine.tick_many(2).unwrap();
        assert_eq!(
            engine.snapshot().stats.projectile_impacts,
            case.expected_impacts,
            "moving-target {}",
            case.name
        );
    }

    for (shootval, expected_range) in [(4, 11), (5, 11), (-4, 11), (-8, 21)] {
        let mut engine = Engine::new(EngineConfig {
            metabolism_cost: 0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let shooter = engine
            .spawn_at(
                LegacyDna::parse(&format!(
                    "start\n-1 .shoot store\n{shootval} .shootval store\nstop"
                ))
                .unwrap(),
                [250.0, 500.0],
            )
            .unwrap();

        engine.tick().unwrap();
        let shot = engine
            .snapshot()
            .shots
            .iter()
            .find(|shot| shot.owner == shooter)
            .cloned()
            .expect("range calibration shot must spawn");
        assert_eq!(shot.range, expected_range, "shootval={shootval}");
        assert_eq!(shot.age, 0, "shootval={shootval}");
        assert_eq!(
            shot.energy,
            expected_range as f32 * 40.0,
            "shootval={shootval}"
        );
        engine
            .replace_dna(shooter, LegacyDna::parse("start\nstop").unwrap())
            .unwrap();

        engine.tick_many(expected_range).unwrap();
        assert!(
            engine
                .snapshot()
                .shots
                .iter()
                .any(|shot| shot.owner == shooter && shot.age == expected_range),
            "shot must survive at age == range for shootval={shootval}"
        );
        engine.tick().unwrap();
        assert!(
            engine
                .snapshot()
                .shots
                .iter()
                .all(|shot| shot.owner != shooter),
            "shot must expire after age > range for shootval={shootval}"
        );
    }
}

#[test]
fn shot_formation_spends_the_original_flat_boost_and_ammunition_costs() {
    for (kind, requested, expected_energy, expected_value) in [
        (500, -37, 998, -37),
        (500, 0, 998, 0),
        (-1, 5, 990, 291),
        (-1, -8, 984, 220),
        (-2, 100, 898, 100),
        (-2, 0, 988, 10),
        (-3, 0, 998, 0),
        (-4, 0, 998, 0),
        (-8, 37, 998, 37),
    ] {
        let mut engine = Engine::new(EngineConfig {
            metabolism_cost: 0,
            dna_costs: DnaCosts {
                shot_formation: 2.0,
                ..Default::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = engine
            .spawn_at(
                LegacyDna::parse(&format!(
                    "start {kind} .shoot store {requested} .shootval store stop"
                ))
                .unwrap(),
                [250.0, 500.0],
            )
            .unwrap();
        engine.tick().unwrap();
        assert_eq!(
            engine.organism(id).unwrap().energy,
            expected_energy,
            "kind {kind}, request {requested}"
        );
        assert_eq!(
            engine.snapshot().shots[0].value,
            expected_value,
            "kind {kind}, request {requested}"
        );
        if kind == -2 {
            assert_eq!(engine.snapshot().shots[0].energy, expected_value as f32);
        }
    }
}

#[test]
fn energy_ammunition_is_capped_to_the_senders_available_energy() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let source = engine
        .spawn_at(
            LegacyDna::parse("start -2 .shoot store 1000 .shootval store stop").unwrap(),
            [250.0, 500.0],
        )
        .unwrap();
    engine.set_organism_energy(source, 50).unwrap();
    engine.tick().unwrap();
    assert!(engine.organism(source).is_err());
    assert_eq!(engine.snapshot().shots[0].value, 50);
    assert_eq!(engine.snapshot().shots[0].energy, 50.0);
}

#[test]
fn negative_and_zero_information_values_reach_the_requested_memory_address() {
    for value in [-37, 0] {
        let mut engine = Engine::new(EngineConfig::testing()).unwrap();
        engine
            .spawn_at(
                LegacyDna::parse(&format!(
                    "start 500 .shoot store {value} .shootval store stop"
                ))
                .unwrap(),
                [250.0, 500.0],
            )
            .unwrap();
        let target = engine
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0, 500.0])
            .unwrap();
        engine.set_organism_memory(target, 500, 123).unwrap();
        engine.tick_many(2).unwrap();
        assert_eq!(engine.memory_at(target, 500).unwrap(), value);
    }
}

#[test]
fn information_shots_cannot_bypass_sexual_reproduction_requirements() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start 302 .shoot store 50 .shootval store stop").unwrap(),
            [250.0, 500.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0, 500.0])
        .unwrap();
    engine.tick_many(2).unwrap();
    assert_eq!(engine.population(), 2);
    assert_eq!(engine.snapshot().stats.births, 0);
    assert!(
        engine
            .robot_state(target)
            .unwrap()
            .effects
            .stored_sperm
            .is_none()
    );
    assert!(
        engine
            .robot_state(target)
            .unwrap()
            .effects
            .forced_reproduction
    );
    assert_eq!(
        engine
            .robot_state(target)
            .unwrap()
            .effects
            .reproduction_assistance,
        4
    );
    assert_eq!(engine.memory_at(target, 302).unwrap(), 50);
    assert_eq!(engine.snapshot().stats.self_reproductions, 0);
}

fn segment_length(start: [f32; 2], end: [f32; 2]) -> f32 {
    (end[0] - start[0]).hypot(end[1] - start[1])
}
