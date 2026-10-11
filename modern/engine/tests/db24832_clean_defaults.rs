use darwinbots_engine::{CorpseDecayMode, EngineConfig, LocalModeConfig};

#[test]
fn clean_start_defaults_match_db24832() {
    let config = EngineConfig::default();

    assert_eq!(
        [config.world_width, config.world_height],
        [16_000.0, 12_000.0]
    );
    assert_eq!(config.physics.max_velocity, 60.0);
    assert_eq!(config.physics.movement_efficiency, 0.66);
    assert_eq!(config.physics.surface_gravity, 0.0);
    assert_eq!(config.physics.static_friction, 0.0);
    assert_eq!(config.physics.kinetic_friction, 0.0);
    assert_eq!(config.physics.density, 0.0);
    assert_eq!(config.physics.viscosity, 0.0);
    assert_eq!(config.physics.elasticity, 0.0);
    assert_eq!(config.brownian_motion, 0.5);

    assert!(!config.corpse_decay.enabled);
    assert_eq!(config.corpse_decay.death_energy_threshold, 0);
    assert_eq!(config.corpse_decay.mode, CorpseDecayMode::None);
    assert_eq!(config.corpse_decay.packet_size, 0);
    assert_eq!(config.corpse_decay.period, 0);
    assert_eq!(config.recording.sampling_interval, 200);

    assert_eq!(config.vegetation.start_chloroplasts, 16_000);
    assert_eq!(config.vegetation.max_energy_per_tick, 100);
    assert_eq!(config.vegetation.minimum_chloroplast_equivalents, 50);
    assert_eq!(config.vegetation.repopulation_amount, 10);
    assert_eq!(config.vegetation.repopulation_cooldown, 10);
    assert_eq!(config.vegetation.feeding_to_body, 0.75);
    assert_eq!(config.vegetation.gradient, 1.02);
    assert!(config.vegetation.daytime);
    assert!(!config.vegetation.day_night_enabled);
}

#[test]
fn local_contest_defaults_match_db24832() {
    let config = LocalModeConfig::default();

    assert_eq!(config.minimum_rounds, 5);
    assert_eq!(config.maximum_rounds, 1);
    assert_eq!(config.maximum_cycles, 15_000);
    assert_eq!(config.maximum_population, 700);
    assert!(config.validate().is_ok());
}
