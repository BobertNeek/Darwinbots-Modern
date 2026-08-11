use darwinbots_engine::{
    CycleEvent, CyclePhase, CycleTrace, Engine, EngineConfig, LegacyDna, SpeciesDefinition,
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
fn extinct_species_phase_removes_empty_non_reseeding_species_after_commit() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let extinct = engine.register_species(SpeciesDefinition {
        name: "Extinct lineage".to_owned(),
        ..SpeciesDefinition::default()
    });
    let id = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            extinct,
            [500.0, 500.0],
        )
        .unwrap();
    engine.remove(id).unwrap();

    let trace = engine.tick_with_trace().unwrap();

    assert!(
        phase_index(&trace, CyclePhase::CommitBirthsDeaths)
            < phase_index(&trace, CyclePhase::ExtinctSpecies)
    );
    assert!(engine
        .snapshot()
        .species
        .iter()
        .all(|species| species.name != "Extinct lineage"));
    assert_eq!(engine.snapshot().species.len(), 1);
}
