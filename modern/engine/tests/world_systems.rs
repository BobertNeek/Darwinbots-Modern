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
            LegacyDna::parse("start\n314 .setaim store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    engine.spawn_at(idle, [400.0, 100.0]).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory(observer, "refxpos").unwrap(), 400);
    assert_eq!(engine.memory(observer, "refypos").unwrap(), 100);
    assert!(engine.memory(observer, "eye5").unwrap() > 0);
}

#[test]
fn sensing_projects_focus_reference_occurrences_materials_and_addressed_memory() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let observer = engine
        .spawn_at(
            LegacyDna::parse(
                "start\n314 .setaim store\n0 .focuseye store\n950 .memloc store\nstop",
            )
            .unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [400.0, 100.0])
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
            LegacyDna::parse("start\n314 .setaim store\n0 .focuseye store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n0 .tie store\nstop").unwrap(),
            [400.0, 100.0],
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
            LegacyDna::parse("start\n314 .setaim store\n0 .focuseye store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n7 .shoot store\nstop").unwrap(),
            [400.0, 100.0],
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
            LegacyDna::parse("start\n314 .setaim store\n0 .focuseye store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    engine
        .spawn_at(
            LegacyDna::parse(
                "start\n1 .up store\n2 .dn store\n3 .sx store\n4 .dx store\n5 .aimright store\n6 .aimleft store\nstop",
            )
            .unwrap(),
            [400.0, 100.0],
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
    let attacker = LegacyDna::parse("start\n-1 .shoot store\n100 .shootval store\nstop").unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let attacker = engine.spawn_at(attacker, [100.0, 100.0]).unwrap();
    let target = engine.spawn_at(idle, [350.0, 100.0]).unwrap();
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
fn tie_commands_create_multibot_link_and_share_energy() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let connector = LegacyDna::parse("start\n1 .tie store\n10 .sharenrg store\nstop").unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let source = engine.spawn_at(connector, [100.0, 100.0]).unwrap();
    let target = engine.spawn_at(idle, [140.0, 100.0]).unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.snapshot().ties.len(), 1);
    assert_eq!(engine.snapshot().stats.ties_created, 1);

    engine.tick().unwrap();

    assert!(engine.organism(source).unwrap().energy < engine.organism(target).unwrap().energy);
}
