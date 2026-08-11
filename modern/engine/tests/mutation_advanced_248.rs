use darwinbots_engine::{
    AdvancedMutationSettings, DnaBlock, Engine, EngineConfig, EpigeneticPolicyAction,
    GenomeMutator, LegacyDna, LocalModeConfig, LocalModeKind, MutationContext,
    MutationPolicyReport, SpeciesDefinition, SpeciesId, StandardMutationEvent,
    StandardMutationProfile, TokenClass, apply_standard_mutation,
};

fn number(value: i32) -> DnaBlock {
    DnaBlock {
        class: TokenClass::Number,
        value,
    }
}

fn end() -> DnaBlock {
    DnaBlock {
        class: TokenClass::End,
        value: 1,
    }
}

fn always_mutate_policy() -> MutationPolicyReport {
    MutationPolicyReport {
        effective_multiplier: 1.0,
        effective_one_in: 1.0,
        delta2_applied: false,
        epigenetic_action: EpigeneticPolicyAction::None,
        save_as_temporary_gene: false,
    }
}

fn sysvar_biased_profile() -> StandardMutationProfile {
    let mut profile = StandardMutationProfile::default();
    for rate in [
        &mut profile.point,
        &mut profile.delta,
        &mut profile.copy_error,
        &mut profile.insertion,
        &mut profile.reversal,
        &mut profile.minor_deletion,
        &mut profile.major_deletion,
        &mut profile.point2,
        &mut profile.copy_error2,
        &mut profile.amplification,
        &mut profile.translocation,
    ] {
        rate.one_in = f64::MAX;
    }
    profile.point2.one_in = 1.0;
    profile.copy_error2.one_in = 1.0;
    profile
}

#[test]
fn point2_and_copyerror2_are_seeded_and_sysvar_biased() {
    let original = LegacyDna::parse(&"1 ".repeat(24)).unwrap();
    let settings = AdvancedMutationSettings {
        point2_enabled: true,
        copy_error2_enabled: true,
        ..Default::default()
    };
    let profile = sysvar_biased_profile();

    let mut point_left = original.clone();
    let mut point_right = original.clone();
    let left = GenomeMutator::new(91).mutate_with_advanced_policy(
        &mut point_left,
        &profile,
        always_mutate_policy(),
        &settings,
        MutationContext::Lifelong,
    );
    let right = GenomeMutator::new(91).mutate_with_advanced_policy(
        &mut point_right,
        &profile,
        always_mutate_policy(),
        &settings,
        MutationContext::Lifelong,
    );
    assert_eq!(left, right);
    assert!(left.changes > 0);
    assert_eq!(point_left, point_right);
    assert_ne!(point_left, original);

    let mut copy = original.clone();
    let copy_report = GenomeMutator::new(92).mutate_with_advanced_policy(
        &mut copy,
        &profile,
        always_mutate_policy(),
        &settings,
        MutationContext::Reproduction { forced: true },
    );
    assert!(copy_report.changes > 0);
    assert!(
        copy.blocks().windows(2).any(|pair| {
            pair[0].class == TokenClass::Number
                && pair[1].class == TokenClass::Store
                && [1, 2, 3, 4, 7, 8, 19, 300, 301, 330, 800, 801, 802].contains(&pair[0].value)
        }) || copy
            .blocks()
            .iter()
            .any(|block| block.class == TokenClass::Dereference)
    );
    assert_eq!(copy.blocks().last(), Some(&end()));
}

#[test]
fn amplification_copies_and_translocation_moves_coding_segments() {
    let mut amplified = LegacyDna::parse("1 2 3 4 5 6").unwrap();
    let mut profile = StandardMutationProfile::default();
    let report = apply_standard_mutation(
        &mut amplified,
        &mut profile,
        StandardMutationEvent::Amplification {
            source_start: 2,
            length: 3,
            after_location: 5,
        },
    )
    .unwrap();
    assert_eq!(report.changes, 3);
    assert_eq!(
        amplified.blocks(),
        &[
            number(1),
            number(2),
            number(3),
            number(4),
            number(5),
            number(2),
            number(3),
            number(4),
            number(6),
            end(),
        ]
    );

    let mut translocated = LegacyDna::parse("1 2 3 4 5 6").unwrap();
    let report = apply_standard_mutation(
        &mut translocated,
        &mut profile,
        StandardMutationEvent::Translocation {
            source_start: 2,
            length: 2,
            after_location: 4,
        },
    )
    .unwrap();
    assert_eq!(report.changes, 1);
    assert_eq!(
        translocated.blocks(),
        &[
            number(1),
            number(4),
            number(5),
            number(6),
            number(2),
            number(3),
            end()
        ]
    );
}

#[test]
fn mrepro_uses_reproductive_mutation_context() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let mut settings = AdvancedMutationSettings::default();
    settings.copy_error2_enabled = true;
    engine.update_advanced_mutation_settings(settings).unwrap();
    let parent = engine
        .spawn_at(
            LegacyDna::parse("start\n10 .up store\n50 .mrepro store\nstop").unwrap(),
            [500.0, 500.0],
        )
        .unwrap();
    let mut profile = engine.mutation_profile(parent).unwrap().clone();
    profile.copy_error2.one_in = 1.0;
    engine.set_mutation_profile(parent, profile).unwrap();
    let parent_dna = engine.dna(parent).unwrap().clone();

    engine.tick().unwrap();

    let child = engine
        .snapshot()
        .organisms
        .iter()
        .find(|organism| organism.id != parent)
        .unwrap();
    assert_eq!(engine.dna(parent).unwrap(), &parent_dna);
    assert_ne!(engine.dna(parent).unwrap(), engine.dna(child.id).unwrap());
    assert!(child.total_mutations > 0);
    assert!(engine.snapshot().stats.mutations > 0);
}

#[test]
fn automatic_named_species_fork_uses_prefix_and_49_species_limit() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let source = engine.register_species(SpeciesDefinition {
        name: "Animal Minimalis".to_owned(),
        minimum_population: 1,
        ..Default::default()
    });
    engine
        .update_speciation_settings(Some(true), Some(0.1))
        .unwrap();
    let parent = engine
        .spawn_species_batch(
            LegacyDna::parse("start\n10 .up store\n50 .mrepro store\nstop").unwrap(),
            source,
            [[500.0, 500.0]],
            10_000,
        )
        .unwrap()[0];
    let mut profile = engine.mutation_profile(parent).unwrap().clone();
    profile.copy_error2.one_in = 1.0;
    engine.set_mutation_profile(parent, profile).unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.snapshot().species.len(), 3);
    assert!(
        engine.snapshot().species[2]
            .name
            .starts_with("(5001)Animal Minimalis"),
        "species: {:?}",
        engine.snapshot().species
    );
    let child = engine
        .snapshot()
        .organisms
        .iter()
        .find(|organism| organism.id != parent)
        .unwrap();
    assert_eq!(child.species.0, 2);

    let mut capped = Engine::new(EngineConfig::testing()).unwrap();
    let mut capped_source = SpeciesId::default();
    for index in 0..48 {
        capped_source = capped.register_species(SpeciesDefinition {
            name: if index == 47 {
                "Animal Minimalis".to_owned()
            } else {
                format!("Species {index}")
            },
            minimum_population: 1,
            ..Default::default()
        });
    }
    capped
        .update_speciation_settings(Some(true), Some(0.1))
        .unwrap();
    let capped_parent = capped
        .spawn_species_batch(
            LegacyDna::parse("start\n10 .up store\n50 .mrepro store\nstop").unwrap(),
            capped_source,
            [[500.0, 500.0]],
            10_000,
        )
        .unwrap()[0];
    let mut profile = capped.mutation_profile(capped_parent).unwrap().clone();
    profile.copy_error2.one_in = 1.0;
    capped.set_mutation_profile(capped_parent, profile).unwrap();
    capped.tick().unwrap();
    assert_eq!(capped.snapshot().species.len(), 49);
}

#[test]
fn zerobot_mode_mutates_selects_and_rolls_back_unstable_rounds() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let species = engine.register_species(SpeciesDefinition {
        name: "Zerobot".to_owned(),
        minimum_population: 1,
        ..Default::default()
    });
    let id = engine
        .spawn_species_batch(
            LegacyDna::parse("0 0 0 0 0 0 0 0").unwrap(),
            species,
            [[500.0, 500.0]],
            10_000,
        )
        .unwrap()[0];
    let mut profile = engine.mutation_profile(id).unwrap().clone();
    profile.point.one_in = 1.0;
    engine.set_mutation_profile(id, profile).unwrap();
    engine
        .configure_local_mode(LocalModeConfig {
            kind: LocalModeKind::Zerobot,
            source_species: Some(species),
            minimum_rounds: 1,
            maximum_rounds: 2,
            maximum_cycles: 1,
            restart_on_extinction: false,
            ..Default::default()
        })
        .unwrap();
    engine.start_local_mode().unwrap();
    engine.tick().unwrap();
    engine.tick().unwrap();

    assert_eq!(
        engine
            .snapshot()
            .local_mode
            .config
            .as_ref()
            .map(|config| config.kind),
        Some(LocalModeKind::Zerobot)
    );
    assert_eq!(engine.snapshot().local_mode.results.len(), 2);
    assert!(engine.snapshot().stats.mutations > 0);
    assert!(
        engine.snapshot().local_mode.results[1]
            .reason
            .contains("rollback")
    );
}

#[test]
fn mrepro_auto_forked_zerobot_checkpoint_preserves_mutation_lineage_on_rollback() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let mut settings = AdvancedMutationSettings::default();
    settings.copy_error2_enabled = true;
    engine.update_advanced_mutation_settings(settings).unwrap();
    let source = engine.register_species(SpeciesDefinition {
        name: "Animal Minimalis".to_owned(),
        minimum_population: 1,
        ..Default::default()
    });
    engine
        .update_speciation_settings(Some(true), Some(0.1))
        .unwrap();
    let parent = engine
        .spawn_species_batch(
            LegacyDna::parse("start\n10 .up store\n50 .mrepro store\nstop").unwrap(),
            source,
            [[500.0, 500.0]],
            10_000,
        )
        .unwrap()[0];
    let mut profile = engine.mutation_profile(parent).unwrap().clone();
    profile.copy_error2.one_in = 1.0;
    engine.set_mutation_profile(parent, profile).unwrap();
    engine
        .configure_local_mode(LocalModeConfig {
            kind: LocalModeKind::Zerobot,
            source_species: Some(source),
            minimum_rounds: 1,
            maximum_rounds: 2,
            maximum_cycles: 2,
            restart_on_extinction: false,
            ..Default::default()
        })
        .unwrap();
    engine.start_local_mode().unwrap();

    engine.tick().unwrap();

    let child = engine
        .snapshot()
        .organisms
        .iter()
        .find(|organism| organism.id != parent)
        .unwrap();
    let child_id = child.id;
    let child_species = child.species;
    let child_dna = engine.dna(child_id).unwrap().clone();
    let child_log = child.mutation_log.clone();
    let child_subspecies = child.subspecies;
    let child_species_name = engine.snapshot().species[child_species.0 as usize]
        .name
        .clone();
    assert!(child.total_mutations > 0);
    assert!(!child_log.is_empty());
    assert_ne!(child_subspecies, 0);
    assert!(child_species_name.starts_with("(5001)Animal Minimalis"));

    engine
        .adjudicate_local_round(
            Some(child_species),
            "mutated reproduction checkpoint".to_owned(),
        )
        .unwrap();
    let checkpoint = engine
        .snapshot()
        .organisms
        .iter()
        .find(|organism| organism.species == child_species)
        .unwrap();
    let checkpoint_id = checkpoint.id;
    assert_eq!(
        engine.snapshot().local_mode.results[0].winner,
        Some(child_species)
    );
    assert_eq!(engine.dna(checkpoint_id).unwrap(), &child_dna);
    assert_eq!(checkpoint.mutation_log, child_log);
    assert_eq!(checkpoint.subspecies, child_subspecies);
    assert_eq!(
        engine.snapshot().species[child_species.0 as usize].name,
        child_species_name
    );

    engine
        .adjudicate_local_round(None, "unstable reproduction round".to_owned())
        .unwrap();
    let local_mode = engine.snapshot().local_mode.clone();
    assert_eq!(local_mode.results.len(), 2);
    assert!(local_mode.results[1].reason.contains("zerobot rollback"));
    let restored = engine.snapshot().organisms.first().unwrap();
    let restored_id = restored.id;
    assert_eq!(restored.species, child_species);
    assert_eq!(engine.dna(restored_id).unwrap(), &child_dna);
    assert_eq!(restored.mutation_log, child_log);
    assert_eq!(restored.subspecies, child_subspecies);
    assert_eq!(
        engine.snapshot().species[child_species.0 as usize].name,
        child_species_name
    );
}
