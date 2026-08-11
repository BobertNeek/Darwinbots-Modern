use darwinbots_engine::{Engine, EngineConfig, LegacyDna, Obstacle, PhysicsSettings, SpeciesId};

const HIT: i32 = 201;
const HIT_UP: i32 = 205;
const HIT_DOWN: i32 = 206;
const HIT_RIGHT: i32 = 207;
const HIT_LEFT: i32 = 208;
const SHOT_ANGLE: i32 = 209;
const SHOT_UP: i32 = 210;
const SHOT_DOWN: i32 = 211;
const SHOT_RIGHT: i32 = 212;
const SHOT_LEFT: i32 = 213;
const SHOT_FLAVOR: i32 = 202;

fn inert_dna() -> LegacyDna {
    LegacyDna::parse("start\nstop").unwrap()
}

fn contact_cells(engine: &Engine, id: darwinbots_engine::OrganismId) -> [i32; 5] {
    [HIT, HIT_UP, HIT_DOWN, HIT_RIGHT, HIT_LEFT]
        .map(|address| engine.memory_at(id, address).unwrap())
}

fn taste_cells(engine: &Engine, id: darwinbots_engine::OrganismId) -> [i32; 6] {
    [
        SHOT_FLAVOR,
        SHOT_ANGLE,
        SHOT_UP,
        SHOT_DOWN,
        SHOT_RIGHT,
        SHOT_LEFT,
    ]
    .map(|address| engine.memory_at(id, address).unwrap())
}

fn projectile_engine() -> Engine {
    Engine::new(EngineConfig {
        metabolism_cost: 0,
        drag: 0.0,
        physics: PhysicsSettings {
            density: 0.0,
            surface_gravity: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap()
}

fn impact_from_known_previous_point(
    requested_kind: i32,
    value: i32,
) -> (Engine, darwinbots_engine::OrganismId) {
    let mut engine = projectile_engine();
    let fire_once = LegacyDna::parse(&format!(
        "cond\n*.robage 1 <\nstart\n{requested_kind} .shoot store\n{value} .shootval store\nstop"
    ))
    .unwrap();
    engine.spawn_at(fire_once, [100.0, 100.0]).unwrap();
    let target = engine.spawn_at(inert_dna(), [900.0, 800.0]).unwrap();
    engine.set_organism_aim(target, 0).unwrap();

    engine.tick().unwrap();
    let previous = engine.snapshot().shots[0].end;
    engine
        .move_organism(target, [previous[0] + 100.0, previous[1] + 81.0])
        .unwrap();
    engine.tick().unwrap();

    assert!(engine.snapshot().shots[0].impact_flash);
    assert_eq!(engine.snapshot().shots[0].start, previous);
    (engine, target)
}

#[test]
fn actual_overlap_touches_both_participants_but_not_a_nearby_non_overlap() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let ids = engine
        .spawn_batch([
            (inert_dna(), [500.0, 500.0]),
            (inert_dna(), [600.0, 500.0]),
            (inert_dna(), [850.0, 500.0]),
        ])
        .unwrap();
    engine.set_organism_aim(ids[0], 0).unwrap();
    engine.set_organism_aim(ids[1], 0).unwrap();

    engine.tick().unwrap();

    assert_eq!(contact_cells(&engine, ids[0]), [1, 1, 0, 0, 0]);
    assert_eq!(contact_cells(&engine, ids[1]), [1, 0, 1, 0, 0]);
    assert_eq!(contact_cells(&engine, ids[2]), [0, 0, 0, 0, 0]);
}

#[test]
fn touch_bankers_rounds_the_other_robot_center_before_direction_selection() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let ids = engine
        .spawn_batch([
            (inert_dna(), [500.0, 500.0]),
            (inert_dna(), [550.49, 549.5]),
        ])
        .unwrap();
    engine.set_organism_aim(ids[0], 0).unwrap();

    engine.tick().unwrap();

    assert_eq!(contact_cells(&engine, ids[0]), [1, 0, 0, 1, 0]);
}

#[test]
fn vertical_overlap_writes_right_and_left_runtime_touch_cells() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let ids = engine
        .spawn_batch([(inert_dna(), [500.0, 500.0]), (inert_dna(), [500.0, 600.0])])
        .unwrap();
    engine.set_organism_aim(ids[0], 0).unwrap();
    engine.set_organism_aim(ids[1], 0).unwrap();

    engine.tick().unwrap();

    assert_eq!(contact_cells(&engine, ids[0]), [1, 0, 0, 1, 0]);
    assert_eq!(contact_cells(&engine, ids[1]), [1, 0, 0, 0, 1]);
}

#[test]
fn shot_taste_uses_previous_position_normalized_kind_and_vb6_angle_rounding() {
    let (engine, target) = impact_from_known_previous_point(-14, 50);

    assert_eq!(taste_cells(&engine, target), [-6, 764, 0, -6, 0, 0]);
}

#[test]
fn positive_memory_shot_taste_uses_canonical_address_kind() {
    let (engine, target) = impact_from_known_previous_point(1_001, 50);

    assert_eq!(taste_cells(&engine, target), [1, 764, 0, 1, 0, 0]);
}

#[test]
fn confirmed_organism_hit_tastes_even_when_effect_returns_false() {
    let (engine, target) = impact_from_known_previous_point(-3, 1);

    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
    assert_eq!(engine.snapshot().stats.projectile_effects, 0);
    assert_eq!(taste_cells(&engine, target), [-3, 764, 0, -3, 0, 0]);
}

#[test]
fn corpse_and_obstacle_impacts_do_not_taste_a_live_robot() {
    let mut corpse_engine = Engine::new(EngineConfig {
        metabolism_cost: 1,
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    corpse_engine
        .spawn_species_batch(inert_dna(), SpeciesId::default(), [[350.0, 100.0]], 1)
        .unwrap();
    corpse_engine.tick().unwrap();
    assert_eq!(corpse_engine.snapshot().corpses.len(), 1);
    corpse_engine
        .spawn_at(
            LegacyDna::parse(
                "cond\n*.robage 1 <\nstart\n-1 .shoot store\n40 .shootval store\nstop",
            )
            .unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let corpse_bystander = corpse_engine.spawn_at(inert_dna(), [600.0, 100.0]).unwrap();
    corpse_engine.tick().unwrap();
    corpse_engine.tick().unwrap();
    assert_eq!(corpse_engine.snapshot().stats.projectile_impacts, 1);
    assert_eq!(
        taste_cells(&corpse_engine, corpse_bystander),
        [0, 0, 0, 0, 0, 0]
    );

    let mut obstacle_engine = projectile_engine();
    obstacle_engine
        .add_obstacle(Obstacle::new(1, [220.0, 100.0], [230.0, 120.0]))
        .unwrap();
    obstacle_engine
        .spawn_at(
            LegacyDna::parse(
                "cond\n*.robage 1 <\nstart\n-1 .shoot store\n40 .shootval store\nstop",
            )
            .unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let obstacle_bystander = obstacle_engine
        .spawn_at(inert_dna(), [350.0, 100.0])
        .unwrap();
    obstacle_engine.tick().unwrap();
    obstacle_engine.tick().unwrap();
    assert_eq!(obstacle_engine.snapshot().stats.projectile_impacts, 1);
    let obstacle_impact = obstacle_engine.snapshot().shots[0].end;
    assert!(
        (220.0..=230.0).contains(&obstacle_impact[0])
            && (100.0..=120.0).contains(&obstacle_impact[1]),
        "projectile did not impact the obstacle AABB: {obstacle_impact:?}"
    );
    assert_eq!(
        taste_cells(&obstacle_engine, obstacle_bystander),
        [0, 0, 0, 0, 0, 0]
    );
}

#[test]
fn late_touch_is_read_by_next_dna_then_clears_without_stale_directions() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let remember_touch = LegacyDna::parse(
        "start\n*.hit 50 store\n*.hitup 51 store\n*.hitdn 52 store\n*.hitdx 53 store\n*.hitsx 54 store\nstop",
    )
    .unwrap();
    let ids = engine
        .spawn_batch([
            (remember_touch, [500.0, 500.0]),
            (inert_dna(), [600.0, 500.0]),
        ])
        .unwrap();

    engine.tick().unwrap();
    assert_eq!(contact_cells(&engine, ids[0]), [1, 1, 0, 0, 0]);
    engine.move_organism(ids[1], [900.0, 500.0]).unwrap();

    engine.tick().unwrap();

    assert_eq!(
        [50, 51, 52, 53, 54].map(|address| engine.memory_at(ids[0], address).unwrap()),
        [1, 1, 0, 0, 0]
    );
    assert_eq!(contact_cells(&engine, ids[0]), [0, 0, 0, 0, 0]);
}
