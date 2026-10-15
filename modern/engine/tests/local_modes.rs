use darwinbots_engine::{
    Engine, EngineConfig, LeaguePreset, LegacyDna, LocalModeConfig, LocalModeKind,
    LocalModeRestrictions, LocalModeStatus, SpeciesDefinition, SpeciesId,
};

#[derive(Clone, Copy)]
struct ModeCase {
    kind: LocalModeKind,
    adjudicate: bool,
}

#[test]
fn local_modes_transition_through_adjudication_and_survival_restart_authority() {
    let cases = [
        ModeCase {
            kind: LocalModeKind::League,
            adjudicate: true,
        },
        ModeCase {
            kind: LocalModeKind::Contest,
            adjudicate: true,
        },
        ModeCase {
            kind: LocalModeKind::SimpleSurvival,
            adjudicate: false,
        },
        ModeCase {
            kind: LocalModeKind::EcoSurvival,
            adjudicate: false,
        },
    ];

    for case in cases {
        let mut engine = Engine::new(EngineConfig::testing()).unwrap();
        let species = engine.register_species(SpeciesDefinition {
            name: "Local entrant".to_owned(),
            minimum_population: 1,
            ..SpeciesDefinition::default()
        });
        let ids = engine
            .spawn_species_batch(
                LegacyDna::parse("start\nstop").unwrap(),
                species,
                [[100.0, 100.0]],
                1_000,
            )
            .unwrap();

        engine
            .configure_local_mode(LocalModeConfig {
                kind: case.kind,
                preset: LeaguePreset::F1,
                minimum_rounds: 1,
                maximum_rounds: if case.adjudicate { 1 } else { 2 },
                maximum_cycles: 10,
                maximum_population: 8,
                restart_on_extinction: true,
                energy_handicap: 0,
                source_species: Some(species),
                selection_population_weight: 0.5,
                selection_energy_weight: 0.5,
                restrictions: LocalModeRestrictions {
                    prohibit_information_shots: true,
                    prohibit_viruses: true,
                    prohibit_sexual_reproduction: true,
                    ..LocalModeRestrictions::default()
                },
                ..LocalModeConfig::default()
            })
            .unwrap();
        assert_eq!(
            engine.snapshot().local_mode.status,
            LocalModeStatus::Configured
        );

        engine.start_local_mode().unwrap();
        assert_eq!(
            engine.snapshot().local_mode.status,
            LocalModeStatus::Running
        );

        if case.adjudicate {
            engine
                .adjudicate_local_round(Some(species), "operator decision".to_owned())
                .unwrap();
            assert_eq!(
                engine.snapshot().local_mode.status,
                LocalModeStatus::Completed
            );
            assert_eq!(
                engine.snapshot().local_mode.results[0].winner,
                Some(species)
            );
        } else {
            engine.remove_many(&ids).unwrap();
            engine.tick().unwrap();
            assert_eq!(
                engine.snapshot().local_mode.status,
                LocalModeStatus::Running
            );
            assert_eq!(engine.snapshot().local_mode.restart_count, 1);
            assert_eq!(engine.snapshot().local_mode.current_round, 2);
            assert_eq!(engine.snapshot().organisms.len(), 1);
            assert_eq!(engine.snapshot().organisms[0].species, SpeciesId(species.0));
        }
    }
}

#[test]
fn survival_restart_carries_the_selected_winning_lineage_dna() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let species = engine.register_species(SpeciesDefinition {
        name: "Evolved entrant".to_owned(),
        minimum_population: 1,
        ..SpeciesDefinition::default()
    });
    let ids = engine
        .spawn_species_batch(
            LegacyDna::parse("start\nstop").unwrap(),
            species,
            [[100.0, 100.0]],
            1_000,
        )
        .unwrap();
    let evolved = LegacyDna::parse("start\n777 .up store\nstop").unwrap();
    engine.replace_dna(ids[0], evolved.clone()).unwrap();
    engine
        .configure_local_mode(LocalModeConfig {
            kind: LocalModeKind::SimpleSurvival,
            minimum_rounds: 1,
            maximum_rounds: 2,
            maximum_cycles: 10,
            maximum_population: 8,
            source_species: Some(species),
            ..LocalModeConfig::default()
        })
        .unwrap();
    engine.start_local_mode().unwrap();
    engine
        .adjudicate_local_round(Some(species), "winner".to_owned())
        .unwrap();

    let restarted_id = engine.snapshot().organisms[0].id;
    assert_eq!(engine.dna(restarted_id).unwrap(), &evolved);
}

#[test]
fn extinction_records_authoritative_result_and_honors_round_cap() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let species = engine.register_species(SpeciesDefinition {
        name: "Extinction entrant".to_owned(),
        minimum_population: 1,
        ..SpeciesDefinition::default()
    });
    let ids = engine
        .spawn_species_batch(
            LegacyDna::parse("start\nstop").unwrap(),
            species,
            [[100.0, 100.0]],
            1_000,
        )
        .unwrap();
    engine
        .configure_local_mode(LocalModeConfig {
            kind: LocalModeKind::EcoSurvival,
            minimum_rounds: 1,
            maximum_rounds: 1,
            maximum_cycles: 10,
            maximum_population: 8,
            source_species: Some(species),
            ..LocalModeConfig::default()
        })
        .unwrap();
    engine.start_local_mode().unwrap();
    engine.remove_many(&ids).unwrap();
    engine.tick().unwrap();

    let local_mode = engine.snapshot().local_mode.clone();
    assert_eq!(
        (
            local_mode.status,
            local_mode.current_round,
            local_mode.results.len(),
            local_mode.results.first().map(|result| (
                result.round,
                result.reason.as_str(),
                result.automatic
            )),
            local_mode
                .adjudication
                .as_ref()
                .map(|adjudication| (adjudication.reason.as_str(), adjudication.automatic)),
        ),
        (
            LocalModeStatus::Completed,
            1,
            1,
            Some((1, "extinction", true)),
            Some(("extinction", true)),
        )
    );
}
