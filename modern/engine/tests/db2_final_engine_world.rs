use darwinbots_engine::{
    Engine, EngineConfig, LegacyDna, Obstacle, PhysicsSettings, RestrictionColumn, SaveFile,
    ShapeVisionPolicy, SpeciesDefinition, VegetationSettings,
};

fn quiet_config() -> EngineConfig {
    EngineConfig {
        metabolism_cost: 0,
        gravity: [0.0, 0.0],
        drag: 0.0,
        brownian_motion: 0.0,
        physics: PhysicsSettings {
            density: 0.0,
            surface_gravity: 0.0,
            static_friction: 0.0,
            kinetic_friction: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    }
}

fn set_vegetation_runtime(
    engine: &Engine,
    sun_position: f32,
    sun_range: f32,
    sun_change: u8,
) -> Engine {
    let encoded = SaveFile::encode(engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    payload["vegetation_runtime"]["sun_position"] = serde_json::json!(sun_position);
    payload["vegetation_runtime"]["sun_range"] = serde_json::json!(sun_range);
    payload["vegetation_runtime"]["sun_change"] = serde_json::json!(sun_change);
    let payload = serde_json::to_vec(&payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(&encoded[..6]);
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    SaveFile::decode(&save).unwrap()
}

#[test]
fn eco_03_pond_depth_moving_sun_band_and_tide_reduce_authoritative_feed() {
    let mut config = quiet_config();
    config.fixed_bot_radii = true;
    config.vegetable_energy_per_tick = 0;
    config.vegetation = VegetationSettings {
        pond_mode: true,
        sun_on_random: true,
        light_intensity: 100.0,
        gradient: 1.0,
        max_energy_per_tick: 40,
        feeding_to_body: 0.0,
        tides: 4,
        ..VegetationSettings::default()
    };
    let mut engine = Engine::new(config).unwrap();
    let vegetables = engine.register_species(SpeciesDefinition {
        vegetable: true,
        ..SpeciesDefinition::default()
    });
    let shallow = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetables,
            [100.0, 100.0],
        )
        .unwrap();
    let deep = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetables,
            [100.0, 900.0],
        )
        .unwrap();
    let dark = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            vegetables,
            [500.0, 100.0],
        )
        .unwrap();
    let mut engine = set_vegetation_runtime(&engine, 0.1, 0.0, 10);

    engine.tick().unwrap();
    let initial = engine.organism(shallow).unwrap().energy;
    engine.tick().unwrap();

    let shallow_energy = engine.organism(shallow).unwrap().energy;
    let deep_energy = engine.organism(deep).unwrap().energy;
    let dark_energy = engine.organism(dark).unwrap().energy;
    let encoded = SaveFile::encode(&engine).unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    let sun_position = payload["vegetation_runtime"]["sun_position"]
        .as_f64()
        .unwrap() as f32;

    assert!(sun_position < 0.1, "sun band did not move: {sun_position}");
    assert!(shallow_energy > deep_energy);
    assert!(deep_energy > initial);
    assert_eq!(dark_energy, initial);
}

#[test]
fn phy_08_resolves_every_cluster_pair_and_substitutes_fixed_body_mass() {
    let mut config = quiet_config();
    config.physics.elasticity = 0.5;
    let mut cluster = Engine::new(config.clone()).unwrap();
    cluster
        .spawn_batch([
            (LegacyDna::parse("start\nstop").unwrap(), [400.0, 500.0]),
            (LegacyDna::parse("start\nstop").unwrap(), [550.0, 500.0]),
            (LegacyDna::parse("start\nstop").unwrap(), [700.0, 500.0]),
        ])
        .unwrap();
    cluster.tick().unwrap();
    let cluster_snapshot = cluster.snapshot();
    let radius = cluster_snapshot.render_instances[0].radius;
    let middle_to_right =
        cluster_snapshot.organisms[2].position[0] - cluster_snapshot.organisms[1].position[0];
    assert!(
        middle_to_right >= radius * 2.0 - 0.01,
        "cluster pair remained overlapped: {middle_to_right} < {}",
        radius * 2.0
    );

    // Unit elasticity isolates the fixed-body mass substitution.
    config.physics.elasticity = 1.0;
    let mut fixed = Engine::new(config).unwrap();
    let fixed_species = fixed.register_species(SpeciesDefinition {
        restrictions: Some(RestrictionColumn {
            fixed_in_place: true,
            ..RestrictionColumn::default()
        }),
        ..SpeciesDefinition::default()
    });
    let fixed_id = fixed
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            fixed_species,
            [500.0, 500.0],
        )
        .unwrap();
    let moving_id = fixed
        .spawn_at(
            LegacyDna::parse("start\n-10 .up store\nstop").unwrap(),
            [580.0, 500.0],
        )
        .unwrap();
    fixed.update_legacy_restrictions(true, true);
    assert_eq!(fixed.memory_at(fixed_id, 216).unwrap(), 1);
    fixed.tick().unwrap();

    // Repel3 shifts fixed positions too, and damps/clamps its velocity projections.
    let fixed_position = fixed.organism(fixed_id).unwrap().position;
    assert!((fixed_position[0] - 429.95862).abs() < 0.01);
    assert_eq!(fixed_position[1], 500.0);
    let moving_velocity = fixed.organism(moving_id).unwrap().velocity[0];
    assert!(
        (moving_velocity - 0.9339687).abs() < 0.01,
        "fixed-body elasticity used finite mass: {moving_velocity}"
    );
}

#[test]
fn sen_01_opaque_shapes_occlude_and_disabled_vision_clears_nine_eyes() {
    let mut config = quiet_config();
    config.vegetation.day_night_enabled = false;
    let mut engine = Engine::new(config).unwrap();
    let observer = engine
        .spawn_at(
            LegacyDna::parse("start\n0 .eye5dir store\nstop").unwrap(),
            [500.0, 500.0],
        )
        .unwrap();
    engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [850.0, 500.0])
        .unwrap();
    let mut obstacle = Obstacle::new(1, [620.0, 400.0], [720.0, 600.0]);
    obstacle.vision_policy = ShapeVisionPolicy::Opaque;
    engine.add_obstacle(obstacle).unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.memory_at(observer, 505).unwrap(), 0);

    let encoded = SaveFile::encode(&engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    payload["slots"][observer.slot() as usize]["organism"]["flags"]["can_see"] = false.into();
    let payload = serde_json::to_vec(&payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(&encoded[..6]);
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    let mut disabled = SaveFile::decode(&save).unwrap();
    disabled.tick().unwrap();

    for address in 501..=509 {
        assert_eq!(disabled.memory_at(observer, address).unwrap(), 0);
    }
}

#[test]
fn sen_01_projects_each_of_nine_eyes_with_native_bearings_under_both_border_modes() {
    let projections =
        (0..9).map(|index| (index, (4.0 - index as f32) * std::f32::consts::PI / 18.0));

    for toroidal_world in [false, true] {
        for (eye_index, expected_angle) in projections.clone() {
            let mut config = quiet_config();
            config.world_width = 2_000.0;
            config.world_height = 2_000.0;
            config.toroidal_world = toroidal_world;
            config.axis_topology = Some([toroidal_world; 2]);
            let mut engine = Engine::new(config).unwrap();
            let observer_position = [1_000.0, 1_000.0];
            let observer = engine
                .spawn_at(LegacyDna::parse("start\nstop").unwrap(), observer_position)
                .unwrap();
            let target_position = [
                observer_position[0] + 300.0 * expected_angle.cos(),
                observer_position[1] - 300.0 * expected_angle.sin(),
            ];
            engine
                .spawn_at(LegacyDna::parse("start\nstop").unwrap(), target_position)
                .unwrap();

            engine.tick().unwrap();

            let vision = &engine.organism(observer).unwrap().vision;
            let expected_center = expected_angle.rem_euclid(std::f32::consts::TAU);
            assert!((vision.eyes[eye_index].center_radians - expected_center).abs() < 0.001);
            assert!(
                engine.memory_at(observer, 501 + eye_index as i32).unwrap() > 0,
                "eye {eye_index} did not project its centered target in toroidal={toroidal_world}"
            );
        }
    }
}
