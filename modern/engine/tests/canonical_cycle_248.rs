use darwinbots_engine::{
    CycleEvent, CyclePhase, CycleTrace, Engine, EngineConfig, LegacyDna, OrganismId,
    QueueChangeKind, SpeciesId,
};

const CANONICAL_PHASES: &str = include_str!("fixtures/db24832/canonical_cycle_phases.json");

fn phase_index(trace: &CycleTrace, wanted: CyclePhase) -> usize {
    trace
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                CycleEvent::PhaseStarted { phase, .. } if *phase == wanted
            )
        })
        .unwrap_or_else(|| panic!("trace omitted {wanted:?}"))
}

fn visits(trace: &CycleTrace, wanted: CyclePhase) -> Vec<OrganismId> {
    trace
        .events
        .iter()
        .filter_map(|event| match event {
            CycleEvent::RobotVisited { phase, id, .. } if *phase == wanted => Some(*id),
            _ => None,
        })
        .collect()
}

#[test]
fn trace_matches_the_complete_24832_phase_manifest() {
    let expected: Vec<CyclePhase> = serde_json::from_str(CANONICAL_PHASES).unwrap();
    assert_eq!(CyclePhase::ALL, expected);

    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    let trace = engine.tick_with_trace().unwrap();
    let actual: Vec<_> = trace
        .events
        .iter()
        .filter_map(|event| match event {
            CycleEvent::PhaseStarted { phase, .. } => Some(*phase),
            _ => None,
        })
        .collect();

    assert_eq!(actual, expected);
}

#[test]
fn dna_and_observable_robot_visits_use_ascending_slot_order() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let idle = LegacyDna::parse("start\nstop").unwrap();
    let first = engine.spawn(idle.clone()).unwrap();
    let removed = engine.spawn(idle.clone()).unwrap();
    let third = engine.spawn(idle.clone()).unwrap();
    engine.remove(removed).unwrap();
    let reused = engine.spawn(idle).unwrap();

    let trace = engine.tick_with_trace().unwrap();

    assert_eq!(visits(&trace, CyclePhase::Dna), vec![first, reused, third]);
    assert_eq!(
        visits(&trace, CyclePhase::Upkeep),
        vec![first, reused, third]
    );
    assert_eq!(
        visits(&trace, CyclePhase::Senses),
        vec![first, reused, third]
    );
}

#[test]
fn three_cycle_sensor_and_command_visibility_is_canonical() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let observer = engine
        .spawn_at(
            LegacyDna::parse("start\n*.eye5 50 store\n314 .setaim store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [400.0, 100.0])
        .unwrap();

    let first = engine.tick_with_trace().unwrap();
    assert_eq!(engine.memory_at(observer, 50).unwrap(), 0);
    assert!(engine.memory(observer, "eye5").unwrap() > 0);
    assert!(phase_index(&first, CyclePhase::Dna) < phase_index(&first, CyclePhase::Senses));

    engine.move_organism(target, [5_000.0, 5_000.0]).unwrap();
    let second = engine.tick_with_trace().unwrap();
    assert!(engine.memory_at(observer, 50).unwrap() > 0);
    assert_eq!(engine.memory(observer, "eye5").unwrap(), 0);
    assert!(phase_index(&second, CyclePhase::Dna) < phase_index(&second, CyclePhase::Senses));

    engine.tick_with_trace().unwrap();
    assert_eq!(engine.memory_at(observer, 50).unwrap(), 0);
}

#[test]
fn post_dna_tie_write_precedes_shot_and_reproduction_consumption() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let source = engine
        .spawn_at(
            LegacyDna::parse("start\n41 .tie store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [110.0, 100.0])
        .unwrap();
    engine.tick().unwrap();
    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n41 .tienum store\n7 .tieloc store\n-1 .tieval store\nstop")
                .unwrap(),
        )
        .unwrap();

    let trace = engine.tick_with_trace().unwrap();
    let write = trace
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                CycleEvent::MemoryWrite {
                    phase: CyclePhase::TieData,
                    id,
                    address: 7,
                    before: 0,
                    after: -1,
                    ..
                } if *id == target
            )
        })
        .expect("selected tie write was not traced");

    assert!(phase_index(&trace, CyclePhase::Dna) < write);
    assert!(write < phase_index(&trace, CyclePhase::Shot));
    assert!(write < phase_index(&trace, CyclePhase::Reproduction));
}

#[test]
fn a_new_tie_exchanges_resources_only_from_the_following_cycle() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let source = engine
        .spawn_at(
            LegacyDna::parse("start\n41 .tie store\n10 .sharenrg store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [110.0, 100.0])
        .unwrap();

    let first = engine.tick_with_trace().unwrap();
    let source_after_creation = engine.organism(source).unwrap().energy;
    let target_after_creation = engine.organism(target).unwrap().energy;
    let creation = first
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                CycleEvent::Effect {
                    phase: CyclePhase::TieFire,
                    source: Some(id),
                    target: target_id,
                    kind,
                    ..
                } if *id == source && *target_id == target && kind == "tie_created"
            )
        })
        .expect("tie creation was not attributed to the final tie-fire pass");

    assert_eq!(engine.snapshot().ties.len(), 1);
    assert!(creation > phase_index(&first, CyclePhase::Tie));
    assert!(source_after_creation * 10 > target_after_creation * 9);
    assert!(!first.events.iter().any(|event| {
        matches!(
            event,
            CycleEvent::Effect {
                phase: CyclePhase::Tie,
                kind,
                ..
            } if kind == "tie_share_energy"
        )
    }));

    let second = engine.tick_with_trace().unwrap();
    let exchange = second
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                CycleEvent::Effect {
                    phase: CyclePhase::Tie,
                    source: Some(id),
                    target: target_id,
                    kind,
                    ..
                } if *id == source && *target_id == target && kind == "tie_share_energy"
            )
        })
        .expect("existing tie energy exchange was not attributed to the tie pass");

    assert!(exchange < phase_index(&second, CyclePhase::TieFire));
    let target_after_exchange = engine.organism(target).unwrap().energy;
    assert!(
        target_after_exchange > target_after_creation + 50,
        "target energy did not show the exchange: after creation={target_after_creation}, after exchange={target_after_exchange}"
    );
}

#[test]
fn tie_data_can_change_a_later_tie_fire_command_in_the_same_cycle() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let source = engine
        .spawn_at(
            LegacyDna::parse("start\n41 .tie store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [400.0, 100.0])
        .unwrap();
    let third = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [400.0, 350.0])
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().ties.len(), 1);
    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n41 .tienum store\n330 .tieloc store\n77 .tieval store\nstop")
                .unwrap(),
        )
        .unwrap();

    let trace = engine.tick_with_trace().unwrap();
    let write = trace
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                CycleEvent::MemoryWrite {
                    phase: CyclePhase::TieData,
                    id,
                    address: 330,
                    before: 0,
                    after: 77,
                    ..
                } if *id == target
            )
        })
        .expect("the selected tie did not write the target's tie command");
    let creation = trace
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                CycleEvent::Effect {
                    phase: CyclePhase::TieFire,
                    source: Some(id),
                    target: target_id,
                    kind,
                    ..
                } if *id == target && *target_id == third && kind == "tie_created"
            )
        })
        .expect("the post-DNA tie command was not consumed at tie fire");

    assert!(write < creation);
    assert_eq!(engine.snapshot().ties.len(), 2);
}

#[test]
fn shot_effect_occurs_after_dna_and_before_robot_passes() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine
        .spawn_at(
            LegacyDna::parse("start\n-1 .shoot store\n100 .shootval store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let target = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [350.0, 100.0])
        .unwrap();
    let before = engine.organism(target).unwrap().energy;

    let first = engine.tick_with_trace().unwrap();
    assert!(
        !first.events.iter().any(|event| {
            matches!(event, CycleEvent::Effect { target: id, .. } if *id == target)
        })
    );
    let second = engine.tick_with_trace().unwrap();
    let effect = second
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                CycleEvent::Effect {
                    phase: CyclePhase::ShotUpdate,
                    target: id,
                    ..
                } if *id == target
            )
        })
        .expect("shot effect was not traced in the following cycle");

    assert!(phase_index(&second, CyclePhase::Dna) < effect);
    assert!(effect < phase_index(&second, CyclePhase::RobotPipeline));
    assert!(engine.organism(target).unwrap().energy < before);
}

#[test]
fn births_and_deaths_commit_only_after_the_final_robot_pass() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 1,
        ..EngineConfig::testing()
    })
    .unwrap();
    let parent = engine
        .spawn_at(
            LegacyDna::parse("start\n50 .repro store\nstop").unwrap(),
            [500.0, 500.0],
        )
        .unwrap();
    let doomed = engine
        .spawn_species_batch(
            LegacyDna::parse("start\nstop").unwrap(),
            SpeciesId::default(),
            [[700.0, 500.0]],
            1,
        )
        .unwrap()[0];

    let trace = engine.tick_with_trace().unwrap();
    let commit_phase = phase_index(&trace, CyclePhase::CommitBirthsDeaths);
    let last_final_visit = trace
        .events
        .iter()
        .rposition(|event| {
            matches!(
                event,
                CycleEvent::RobotVisited {
                    phase: CyclePhase::TotalEnergy,
                    ..
                }
            )
        })
        .expect("final pass did not visit robots");
    let birth_queued = trace
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                CycleEvent::QueueChange {
                    phase: CyclePhase::Reproduction,
                    id,
                    kind: QueueChangeKind::BirthQueued,
                    ..
                } if *id == parent
            )
        })
        .expect("birth was not queued");
    let death_queued = trace
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                CycleEvent::QueueChange {
                    phase: CyclePhase::CorpseRemoval,
                    id,
                    kind: QueueChangeKind::DeathQueued,
                    ..
                } if *id == doomed
            )
        })
        .expect("death was not queued");
    let commits: Vec<_> = trace
        .events
        .iter()
        .enumerate()
        .filter(|(_, event)| {
            matches!(
                event,
                CycleEvent::QueueChange {
                    phase: CyclePhase::CommitBirthsDeaths,
                    kind: QueueChangeKind::BirthCommitted | QueueChangeKind::DeathCommitted,
                    ..
                }
            )
        })
        .map(|(index, _)| index)
        .collect();

    assert!(birth_queued < commit_phase);
    assert!(death_queued < commit_phase);
    assert!(last_final_visit < commit_phase);
    assert_eq!(commits.len(), 2);
    assert!(commits.into_iter().all(|index| index > commit_phase));
    assert_eq!(engine.population(), 2);
}

#[test]
fn one_tick_publishes_one_completed_cycle() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    engine
        .spawn(LegacyDna::parse("start\nstop").unwrap())
        .unwrap();
    let before = engine.snapshot_publication_count();

    let trace = engine.tick_with_trace().unwrap();

    assert_eq!(trace.tick, 1);
    assert_eq!(engine.snapshot().tick, 1);
    assert_eq!(engine.snapshot_publication_count(), before + 1);
    assert_eq!(
        trace
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    CycleEvent::PhaseStarted {
                        phase: CyclePhase::Publication,
                        ..
                    }
                )
            })
            .count(),
        1
    );
    assert!(matches!(
        trace.events.last(),
        Some(CycleEvent::PhaseStarted {
            tick: 1,
            phase: CyclePhase::Publication,
        })
    ));
}
