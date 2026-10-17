use darwinbots_engine::{
    CorpseDecaySettings, Engine, EngineConfig, LegacyDna, Obstacle, PhysicsSettings, SaveFile,
    SpeciesId,
};

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

// This one shape control retains the modern swept-absorption contract. The
// source-native taste and point-obstacle controls below keep native RNG enabled.
fn legacy_projectile_engine() -> Engine {
    let encoded = SaveFile::encode(&projectile_engine()).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    payload["native_random"] = serde_json::Value::Null;
    let payload = serde_json::to_vec(&payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(&encoded[..6]);
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    SaveFile::decode(&save).unwrap()
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
    let firer = engine.spawn_at(fire_once, [300.0, 500.0]).unwrap();
    let target = engine.spawn_at(inert_dna(), [900.0, 800.0]).unwrap();
    engine.set_organism_aim(target, 0).unwrap();

    engine.tick().unwrap();
    // Updateshots precedes robot firing: the new shot has not been visited.
    assert_eq!(engine.snapshot().stats.native_shots_visited, Some(0));
    let previous = engine.snapshot().shots[0].start;
    let current = engine.snapshot().shots[0].end;
    assert_ne!(previous, current);
    // Shots383 passes saved opos to taste before388 replaces it with impactpos.
    // Scale the exact100/81 ratio to50/40.5: preserve angle764 while placing
    // current safely inside the target and keeping the saved-opos angle distinct.
    engine
        .move_organism(target, [previous[0] + 50.0, previous[1] + 40.5])
        .unwrap();
    // Isolate taste from robot overlap and same-cycle returned-packet re-hits.
    engine.move_organism(firer, [100.0, 100.0]).unwrap();
    let robot = engine.organism(target).unwrap();
    assert!((current[0] - robot.position[0]).hypot(current[1] - robot.position[1]) < robot.radius);
    engine.tick().unwrap();

    assert!(engine.snapshot().shots[0].impact_flash);
    // NewShotCollision starts inside the target, then opos becomes current.
    assert_eq!(engine.snapshot().shots[0].start, current);
    assert_ne!(engine.snapshot().shots[0].start, previous);
    (engine, target)
}

#[test]
fn actual_overlap_touches_both_participants_but_not_a_nearby_non_overlap() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let ids = engine
        .spawn_batch([
            (inert_dna(), [500.0, 500.0]),
            (inert_dna(), [600.0, 500.0]),
            // The original sequential separation can move the middle robot
            // into a formerly clear neighbor. Keep this control out of that reach.
            (inert_dna(), [950.0, 500.0]),
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
fn touch_direction_uses_positions_after_overlap_separation() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let ids = engine
        .spawn_batch([
            (inert_dna(), [500.0, 500.0]),
            (inert_dna(), [550.49, 549.5]),
        ])
        .unwrap();
    engine.set_organism_aim(ids[0], 0).unwrap();

    engine.tick().unwrap();

    // Physics.Repel3 calls touch after separating both centers. Using the
    // old centers would round to a diagonal and incorrectly select hitright.
    let observer = engine.organism(ids[0]).unwrap().position;
    let target = engine.organism(ids[1]).unwrap().position;
    assert_ne!(observer, [500.0, 500.0]);
    assert_ne!(target, [550.49, 549.5]);
    let event = target.map(f32::round_ties_even);
    assert!(((event[1] - observer[1]) / (event[0] - observer[0])).atan() < 0.78);
    assert_eq!(contact_cells(&engine, ids[0]), [1, 1, 0, 0, 0]);
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
fn shot_taste_uses_previous_position_and_vb6_angle_rounding() {
    let (engine, target) = impact_from_known_previous_point(-6, 50);

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
        corpse_decay: CorpseDecaySettings {
            enabled: true,
            packet_size: 0,
            ..Default::default()
        },
        physics: PhysicsSettings {
            density: 0.0,
            ..PhysicsSettings::default()
        },
        ..EngineConfig::testing()
    })
    .unwrap();
    corpse_engine
        .spawn_species_batch(inert_dna(), SpeciesId::default(), [[550.0, 500.0]], 1)
        .unwrap();
    corpse_engine.tick().unwrap();
    assert_eq!(corpse_engine.snapshot().corpses.len(), 1);
    // A memory shot isolates the corpse hit from feeding-return packets, which
    // the original serial loop can visit and collide again in this same cycle.
    corpse_engine
        .spawn_at(
            LegacyDna::parse(
                "cond\n*.robage 1 <\nstart\n400 .shoot store\n40 .shootval store\nstop",
            )
            .unwrap(),
            [300.0, 500.0],
        )
        .unwrap();
    let corpse_bystander = corpse_engine.spawn_at(inert_dna(), [800.0, 500.0]).unwrap();
    corpse_engine.tick().unwrap();
    corpse_engine.tick().unwrap();
    assert_eq!(corpse_engine.snapshot().stats.projectile_impacts, 1);
    assert_eq!(
        taste_cells(&corpse_engine, corpse_bystander),
        [0, 0, 0, 0, 0, 0]
    );

    let mut obstacle_engine = legacy_projectile_engine();
    obstacle_engine
        .add_obstacle(Obstacle::new(1, [420.0, 490.0], [430.0, 520.0]))
        .unwrap();
    obstacle_engine
        .spawn_at(
            LegacyDna::parse(
                "cond\n*.robage 1 <\nstart\n-1 .shoot store\n40 .shootval store\nstop",
            )
            .unwrap(),
            [300.0, 500.0],
        )
        .unwrap();
    let obstacle_bystander = obstacle_engine
        .spawn_at(inert_dna(), [550.0, 500.0])
        .unwrap();
    obstacle_engine.tick().unwrap();
    obstacle_engine.tick().unwrap();
    assert_eq!(obstacle_engine.snapshot().stats.projectile_impacts, 1);
    let obstacle_impact = obstacle_engine.snapshot().shots[0].end;
    assert!(
        (420.0..=430.0).contains(&obstacle_impact[0])
            && (490.0..=520.0).contains(&obstacle_impact[1]),
        "projectile did not impact the obstacle AABB: {obstacle_impact:?}"
    );
    assert_eq!(
        taste_cells(&obstacle_engine, obstacle_bystander),
        [0, 0, 0, 0, 0, 0]
    );

    // Native Obstacles411-430 checks the current point after robot collisions;
    // absorption removes the shot without inventing a robot taste/impact event.
    let mut native_obstacle_engine = projectile_engine();
    native_obstacle_engine
        .spawn_at(
            LegacyDna::parse(
                "cond\n*.robage 1 <\nstart\n-1 .shoot store\n40 .shootval store\nstop",
            )
            .unwrap(),
            [300.0, 500.0],
        )
        .unwrap();
    let native_bystander = native_obstacle_engine
        .spawn_at(inert_dna(), [800.0, 800.0])
        .unwrap();
    native_obstacle_engine.tick().unwrap();
    assert_eq!(
        native_obstacle_engine.snapshot().stats.native_shots_visited,
        Some(0)
    );
    let point = native_obstacle_engine.snapshot().shots[0].end;
    native_obstacle_engine
        .add_obstacle(Obstacle::new(
            2,
            [point[0] - 1.0, point[1] - 1.0],
            [point[0] + 1.0, point[1] + 1.0],
        ))
        .unwrap();
    native_obstacle_engine.tick().unwrap();
    assert!(native_obstacle_engine.snapshot().shots.is_empty());
    assert_eq!(
        native_obstacle_engine.snapshot().stats.native_shots_visited,
        Some(1)
    );
    assert_eq!(
        native_obstacle_engine.snapshot().stats.projectile_impacts,
        0
    );
    assert_eq!(
        native_obstacle_engine.snapshot().stats.projectile_effects,
        0
    );
    assert_eq!(
        taste_cells(&native_obstacle_engine, native_bystander),
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
