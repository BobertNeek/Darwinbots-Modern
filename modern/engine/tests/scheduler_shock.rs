use darwinbots_engine::{
    CycleEvent, CyclePhase, CycleTrace, Engine, EngineConfig, LegacyDna, QueueChangeKind, SaveFile,
};

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

#[test]
fn shock_overload_queues_death_after_the_final_robot_pass() {
    let mut engine = Engine::new(EngineConfig {
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    let id = engine
        .spawn(LegacyDna::parse("start\n-1 .shoot store\n100 .shootval store\nstop").unwrap())
        .unwrap();

    let encoded = SaveFile::encode(&engine).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
    let robot = &mut payload["slots"][id.slot() as usize]["organism"];
    robot["materials"]["energy"] = serde_json::json!(10_000);
    robot["materials"]["previous_energy"] = serde_json::json!(10_000);
    robot["materials"]["body"] = serde_json::json!(100);
    robot["materials"]["previous_body"] = serde_json::json!(100);
    payload["config"]["dna_costs"]["shot_formation"] = serde_json::json!(150.0);
    let bytes = serde_json::to_vec(&payload).unwrap();
    let mut save = Vec::with_capacity(10 + bytes.len());
    save.extend_from_slice(&encoded[..6]);
    save.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    save.extend_from_slice(&bytes);
    let mut engine = SaveFile::decode(&save).unwrap();

    let trace = engine.tick_with_trace().unwrap();
    let shock = phase_index(&trace, CyclePhase::Shock);
    let senses = phase_index(&trace, CyclePhase::Senses);
    let death_queued = trace
        .events
        .iter()
        .enumerate()
        .find_map(|(index, event)| match event {
            CycleEvent::QueueChange {
                phase: CyclePhase::CorpseRemoval,
                id: queued,
                kind: QueueChangeKind::DeathQueued,
                ..
            } if *queued == id => Some(index),
            _ => None,
        })
        .expect("shock overload did not queue a death");

    assert!(shock < senses, "shock must run before senses");
    assert!(death_queued > senses, "shock death must queue after senses");
    assert!(trace.events.iter().any(|event| {
        matches!(
            event,
            CycleEvent::QueueChange {
                phase: CyclePhase::CommitBirthsDeaths,
                id: committed,
                kind: QueueChangeKind::DeathCommitted,
                ..
            } if *committed == id
        )
    }));
    assert!(engine.organism(id).is_err());
}
