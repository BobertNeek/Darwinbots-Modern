use darwinbots_engine::{Engine, EngineConfig, LegacyDna, SaveFile, SpatialIndex};

#[test]
fn uniform_spatial_index_returns_sorted_nearby_slots() {
    let index = SpatialIndex::build(&[[10.0, 10.0], [22.0, 10.0], [200.0, 200.0]], 32.0);

    assert_eq!(index.neighbors([10.0, 10.0], 40.0), vec![0, 1]);
    assert_eq!(index.nearest([10.0, 10.0], Some(0), 40.0), Some(1));
    assert_eq!(
        index.segment_candidates([0.0, 10.0], [40.0, 10.0], 4.0),
        vec![0, 1]
    );
}

#[test]
fn sensing_writes_nearest_organism_reference_sysvars() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let observer = engine
        .spawn_at(
            LegacyDna::parse("start\n0 .setaim store\nstop").unwrap(),
            [300.0, 500.0],
        )
        .unwrap();
    engine.spawn_at(idle, [600.0, 500.0]).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory(observer, "refxpos").unwrap(), 600);
    assert_eq!(engine.memory(observer, "refypos").unwrap(), 500);
    assert!(engine.memory(observer, "eye5").unwrap() > 0);
}

#[test]
fn sensing_projects_focus_reference_occurrences_materials_and_addressed_memory() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let observer = engine
        .spawn_at(
            LegacyDna::parse("start\n0 .setaim store\n0 .focuseye store\n950 .memloc store\nstop")
                .unwrap(),
            [300.0, 500.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [600.0, 500.0])
        .unwrap();

    let encoded = SaveFile::encode(&engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    let target_organism = &mut payload["slots"][target.slot() as usize]["organism"];
    target_organism["counters"]["kills"] = serde_json::json!(7);
    target_organism["materials"]["shell"] = serde_json::json!(40);
    target_organism["materials"]["poison"] = serde_json::json!(20);
    target_organism["materials"]["venom"] = serde_json::json!(30);
    target_organism["materials"]["aim"] = serde_json::json!(77);
    let payload = serde_json::to_vec(&payload).unwrap();
    let mut save = Vec::with_capacity(10 + payload.len());
    save.extend_from_slice(&encoded[..6]);
    save.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    save.extend_from_slice(&payload);
    let mut engine = SaveFile::decode(&save).unwrap();
    engine.set_organism_memory(target, 800, 321).unwrap();
    engine.set_organism_memory(target, 950, 456).unwrap();
    engine.set_organism_memory(target, 19, 77).unwrap();
    assert_eq!(engine.memory(target, "out1").unwrap(), 321);

    engine.tick().unwrap();

    assert_eq!(engine.memory(target, "out1").unwrap(), 321);
    assert_eq!(engine.memory(observer, "refshell").unwrap(), 40);
    assert_eq!(engine.memory(observer, "refpoison").unwrap(), 20);
    assert_eq!(engine.memory(observer, "refvenom").unwrap(), 30);
    assert_eq!(engine.memory(observer, "refbody").unwrap(), 1_000);
    assert_eq!(engine.memory(observer, "refkills").unwrap(), 7);
    assert_eq!(engine.memory(observer, "refaim").unwrap(), 77);
    assert_eq!(engine.memory(observer, "in1").unwrap(), 321);
    assert_eq!(engine.memory(observer, "memloc").unwrap(), 950);
    assert_eq!(engine.memory_at(target, 950).unwrap(), 456);
    assert_eq!(engine.memory_at(observer, 473).unwrap(), 456);
    assert_eq!(engine.memory(observer, "memval").unwrap(), 456);
    assert_eq!(engine.memory(observer, "refmulti").unwrap(), 0);
    assert_eq!(engine.memory(observer, "reffixed").unwrap(), 0);
}

#[test]
fn sensing_projects_tie_occurrence_reference() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let observer = engine
        .spawn_at(
            LegacyDna::parse("start\n0 .setaim store\n0 .focuseye store\nstop").unwrap(),
            [300.0, 500.0],
        )
        .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n0 .tie store\nstop").unwrap(),
            [600.0, 500.0],
        )
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory(observer, "reftie").unwrap(), 1);
}

#[test]
fn sensing_projects_shoot_occurrence_reference() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let observer = engine
        .spawn_at(
            LegacyDna::parse("start\n0 .setaim store\n0 .focuseye store\nstop").unwrap(),
            [300.0, 500.0],
        )
        .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n7 .shoot store\nstop").unwrap(),
            [600.0, 500.0],
        )
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory(observer, "refshoot").unwrap(), 1);
}

#[test]
fn sensing_projects_direction_and_aim_occurrence_references() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let observer = engine
        .spawn_at(
            LegacyDna::parse("start\n0 .setaim store\n0 .focuseye store\nstop").unwrap(),
            [300.0, 500.0],
        )
        .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse(
                "start\n1 .up store\n2 .dn store\n3 .sx store\n4 .dx store\n5 .aimright store\n6 .aimleft store\nstop",
            )
            .unwrap(),
            [600.0, 500.0],
        )
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory(observer, "refup").unwrap(), 1);
    assert_eq!(engine.memory(observer, "refdn").unwrap(), 1);
    assert_eq!(engine.memory(observer, "refsx").unwrap(), 1);
    assert_eq!(engine.memory(observer, "refdx").unwrap(), 1);
    assert_eq!(engine.memory(observer, "refaimdx").unwrap(), 1);
    assert_eq!(engine.memory(observer, "refaimsx").unwrap(), 1);
}

#[test]
fn negative_shot_damages_nearest_target_and_records_event() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let attacker = LegacyDna::parse("start\n-1 .shoot store\n0 .shootval store\nstop").unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let attacker = engine.spawn_at(attacker, [300.0, 500.0]).unwrap();
    let target = engine.spawn_at(idle, [550.0, 500.0]).unwrap();
    let before = engine.organism(target).unwrap().energy;

    engine.tick().unwrap();
    engine
        .replace_dna(attacker, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.tick().unwrap();

    assert!(engine.organism(target).unwrap().energy < before);
    assert_eq!(engine.snapshot().stats.shots_fired, 1);
    assert_eq!(engine.snapshot().stats.projectile_impacts, 1);
}

#[test]
fn reproduction_creates_child_with_stable_id_and_birth_statistics() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let reproducer = LegacyDna::parse("start\n50 .repro store\nstop").unwrap();
    engine.spawn_at(reproducer, [500.0, 500.0]).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.population(), 2);
    assert_eq!(engine.snapshot().stats.births, 1);
    assert_ne!(
        engine.snapshot().organisms[0].id,
        engine.snapshot().organisms[1].id
    );
}

#[test]
fn reproduction_cluster_defers_mode_blocked_birth_then_rolls_back_greedy_descendant() {
    let mut config = EngineConfig::testing();
    config.metabolism_cost = 0;
    config.gravity = [0.0, 0.0];
    config.drag = 0.0;
    config.greedy_reproduction = true;
    let mut engine = Engine::new(config).unwrap();
    engine.update_legacy_restrictions(true, false);

    let parent = engine
        .spawn_at(
            LegacyDna::parse("start\n25 .repro store\nstop").unwrap(),
            [500.0, 500.0],
        )
        .unwrap();
    engine.set_organism_memory(parent, 500, 88).unwrap();
    let before = engine.robot_state(parent).unwrap().clone();

    engine.tick().unwrap();

    assert_eq!(engine.population(), 1);
    assert_eq!(engine.snapshot().stats.births, 0);
    assert_eq!(engine.snapshot().stats.deaths, 0);
    assert_eq!(engine.memory(parent, "repro").unwrap(), 25);
    assert_eq!(engine.memory_at(parent, 500).unwrap(), 88);
    assert_eq!(
        engine.robot_state(parent).unwrap().identity,
        before.identity
    );
    assert_eq!(engine.robot_state(parent).unwrap().counters.offspring, 0);

    engine.update_legacy_restrictions(true, true);
    engine.tick().unwrap();

    let child = engine
        .snapshot()
        .organisms
        .iter()
        .find(|organism| organism.id != parent)
        .cloned()
        .expect("deferred birth did not publish a child")
        .id;
    let parent_snapshot = engine.organism(parent).unwrap();
    let child_snapshot = engine.organism(child).unwrap();
    let parent_state = engine.robot_state(parent).unwrap();
    let child_state = engine.robot_state(child).unwrap();
    let parent_identity = parent_state.identity.clone();
    let child_identity = child_state.identity.clone();

    assert_eq!(engine.snapshot().stats.births, 1);
    assert_eq!(engine.snapshot().stats.self_reproductions, 1);
    assert_eq!(parent_state.counters.offspring, 1);
    assert_eq!(parent_snapshot.descendants, 1);
    assert_eq!(child_snapshot.descendants, 0);
    assert_eq!(child_state.identity.parents, [Some(parent), None]);
    assert_ne!(
        child_state.identity.absolute_id,
        parent_state.identity.absolute_id
    );
    assert_eq!(
        child_state.counters.generation,
        parent_state.counters.generation + 1
    );
    assert_eq!(engine.memory(parent, "repro").unwrap(), 0);
    assert_eq!(engine.memory_at(parent, 500).unwrap(), 88);
    assert_eq!(engine.memory(child, "repro").unwrap(), 0);
    assert_eq!(engine.memory_at(child, 500).unwrap(), 0);
    assert!(child_snapshot.position[0] > parent_snapshot.position[0]);
    assert_eq!(child_snapshot.position[1], parent_snapshot.position[1]);

    engine
        .replace_dna(
            child,
            LegacyDna::parse("start\n2 .repro store\nstop").unwrap(),
        )
        .unwrap();
    engine.set_organism_memory(child, 500, 144).unwrap();
    engine
        .replace_dna(parent, LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    engine.update_legacy_restrictions(true, false);
    engine.tick().unwrap();

    assert_eq!(engine.population(), 2);
    assert_eq!(engine.snapshot().stats.births, 1);
    assert_eq!(engine.memory(child, "repro").unwrap(), 2);
    assert_eq!(engine.memory_at(child, 500).unwrap(), 144);

    engine.update_legacy_restrictions(true, true);
    engine.tick().unwrap();

    // Reproduce marks a greedy parent Dead after ManageDeath already queued
    // this cycle's kills. The actual kill occurs at the end of the next cycle.
    assert_eq!(engine.population(), 2);
    assert!(engine.robot_state(child).unwrap().flags.dead);
    assert_eq!(engine.snapshot().stats.deaths, 0);
    engine.tick().unwrap();

    assert_eq!(engine.population(), 1);
    assert_eq!(engine.snapshot().stats.births, 1);
    assert_eq!(engine.snapshot().stats.deaths, 1);
    assert_eq!(engine.snapshot().stats.self_reproductions, 1);
    assert!(engine.robot_state(child).is_err());
    let parent_after = engine.robot_state(parent).unwrap();
    assert_eq!(parent_after.identity, parent_identity);
    assert_eq!(parent_after.counters.offspring, 1);
    assert_ne!(child_identity.absolute_id, parent_identity.absolute_id);
}

#[test]
fn tie_commands_create_multibot_link_and_share_energy() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let connector =
        LegacyDna::parse("cond *.robage 1 < start 1 .tie store stop start 10 .sharenrg store stop")
            .unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let source = engine.spawn_at(connector, [300.0, 500.0]).unwrap();
    let target = engine.spawn_at(idle, [600.0, 500.0]).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.snapshot().ties.len(), 1);
    assert_eq!(engine.snapshot().stats.ties_created, 1);

    assert!(!engine.snapshot().ties[0].hardened);
    assert_eq!(
        engine.organism(source).unwrap().energy,
        engine.organism(target).unwrap().energy
    );
    engine.tick_many(20).unwrap();
    assert!(engine.snapshot().ties[0].hardened);
    assert!(engine.organism(source).unwrap().energy < engine.organism(target).unwrap().energy);
}

#[test]
fn late_internal_senses_publish_full_velocity_body_change_daytime_fixed_and_max_velocity() {
    let mut config = EngineConfig::testing();
    config.metabolism_cost = 0;
    config.gravity = [0.0, 0.0];
    config.drag = 0.0;
    config.physics.max_velocity = 42.0;
    config.physics.movement_efficiency = 1.0;
    config.physics.surface_gravity = 0.0;
    config.physics.static_friction = 0.0;
    config.physics.kinetic_friction = 0.0;
    config.vegetation.daytime = false;
    let mut engine = Engine::new(config).unwrap();
    let moving = engine
        .spawn_at(
            LegacyDna::parse("start\n5 .up store\n100 .strbody store\nstop").unwrap(),
            [500.0, 500.0],
        )
        .unwrap();
    let fixed = engine
        .spawn_at(
            LegacyDna::parse("start\n1 .fixpos store\n5 .up store\nstop").unwrap(),
            [1_000.0, 500.0],
        )
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory(moving, "velscalar").unwrap(), 5);
    assert_eq!(engine.memory(moving, "velsx").unwrap(), 0);
    assert_eq!(engine.memory(moving, "veldx").unwrap(), 0);
    assert_eq!(engine.memory(moving, "veldn").unwrap(), -5);
    assert_eq!(engine.memory(moving, "velup").unwrap(), 5);
    assert_eq!(engine.memory(moving, "bodloss").unwrap(), -10);
    assert_eq!(engine.memory(moving, "bodgain").unwrap(), 10);
    assert_eq!(engine.memory(moving, "daytime").unwrap(), 0);
    assert_eq!(engine.memory(moving, "maxvel").unwrap(), 42);
    assert_eq!(engine.memory(moving, "fixed").unwrap(), 0);
    assert_eq!(engine.memory(fixed, "fixed").unwrap(), 1);
}

#[test]
fn tie_readback_publishes_the_selected_partner_reference_state() {
    let mut config = EngineConfig::testing();
    config.gravity = [0.0, 0.0];
    config.drag = 0.0;
    config.physics.surface_gravity = 0.0;
    config.physics.static_friction = 0.0;
    config.physics.kinetic_friction = 0.0;
    let mut engine = Engine::new(config).unwrap();
    let source = engine
        .spawn_at(
            LegacyDna::parse("start\n1 .tie store\nstop").unwrap(),
            [300.0, 500.0],
        )
        .unwrap();
    engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [700.0, 500.0])
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory(source, "trefbody").unwrap(), 1_000);
    assert_eq!(engine.memory(source, "trefnrg").unwrap(), 1_000);
    assert_eq!(engine.memory(source, "trefxpos").unwrap(), 700);
    assert_eq!(engine.memory(source, "trefypos").unwrap(), 500);
    assert_eq!(engine.memory(source, "trefshell").unwrap(), 0);
    assert_eq!(engine.memory(source, "trefaim").unwrap(), 0);
    assert_eq!(engine.memory(source, "treffixed").unwrap(), 0);
}
