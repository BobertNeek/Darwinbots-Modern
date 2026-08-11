use darwinbots_engine::{
    DnaBlock, Engine, EngineConfig, LegacyDna, MAX_DNA_BLOCKS, MutationPreference, MutationRate,
    PointMutationSchedule, StandardMutationEvent, StandardMutationKind, StandardMutationProfile,
    TokenClass, apply_standard_mutation,
};

fn number(value: i32) -> DnaBlock {
    DnaBlock {
        class: TokenClass::Number,
        value,
    }
}

fn master_end() -> DnaBlock {
    DnaBlock {
        class: TokenClass::End,
        value: 1,
    }
}

fn assert_exact_dna(dna: &LegacyDna, expected: &[DnaBlock]) {
    assert_eq!(dna.blocks(), expected);
    assert_eq!(
        dna.blocks()
            .iter()
            .filter(|block| block.class == TokenClass::End)
            .count(),
        1
    );
    assert_eq!(dna.blocks().last(), Some(&master_end()));
}

fn assert_event_rejected_atomically(event: StandardMutationEvent) {
    let mut dna = LegacyDna::parse("1 2 3 4 5").unwrap();
    let mut profile = StandardMutationProfile::default();
    let before_dna = dna.clone();
    let before_profile = profile.clone();
    let description = format!("{event:?}");

    assert!(
        apply_standard_mutation(&mut dna, &mut profile, event).is_err(),
        "accepted {description}"
    );
    assert_eq!(dna, before_dna, "DNA changed after rejecting {description}");
    assert_eq!(
        profile, before_profile,
        "profile changed after rejecting {description}"
    );
}

#[test]
fn standard_profile_defaults_match_db24832() {
    let profile = StandardMutationProfile::default();

    assert_eq!(profile.point2.one_in, 5_000.0);
    assert_eq!(profile.point2.mean, 3.0);
    assert_eq!(profile.point2.std_dev, 1.0);
    assert_eq!(profile.copy_error2.one_in, 5_000.0);
    assert_eq!(profile.copy_error2.mean, 1.0);
    assert_eq!(profile.copy_error2.std_dev, 0.0);
    assert_eq!(profile.amplification.one_in, 5_000.0);
    assert_eq!(profile.amplification.mean, 250.0);
    assert_eq!(profile.amplification.std_dev, 75.0);
    assert_eq!(profile.translocation.one_in, 5_000.0);
    assert_eq!(profile.translocation.mean, 250.0);
    assert_eq!(profile.translocation.std_dev, 75.0);

    assert_eq!(
        profile.point,
        MutationRate {
            one_in: 5_000.0,
            mean: 3.0,
            std_dev: 1.0,
        }
    );
    assert_eq!(
        profile.delta,
        MutationRate {
            one_in: 5_000.0,
            mean: 500.0,
            std_dev: 150.0,
        }
    );
    assert_eq!(
        profile.copy_error,
        MutationRate {
            one_in: 5_000.0,
            mean: 1.0,
            std_dev: 0.0,
        }
    );
    assert_eq!(
        profile.insertion,
        MutationRate {
            one_in: 5_000.0,
            mean: 1.0,
            std_dev: 0.0,
        }
    );
    assert_eq!(
        profile.minor_deletion,
        MutationRate {
            one_in: 5_000.0,
            mean: 1.0,
            std_dev: 0.0,
        }
    );
    assert_eq!(
        profile.major_deletion,
        MutationRate {
            one_in: 5_000.0,
            mean: 3.0,
            std_dev: 1.0,
        }
    );
    assert_eq!(
        profile.reversal,
        MutationRate {
            one_in: 5_000.0,
            mean: 3.0,
            std_dev: 1.0,
        }
    );
    assert_eq!(profile.point_value_change_percent, 80);
    assert_eq!(profile.copy_error_value_change_percent, 80);
}

#[test]
fn detailed_profile_serializes_all_db24832_classes_and_category_distributions() {
    let profile = StandardMutationProfile::default();
    let value = serde_json::to_value(profile).unwrap();

    for class in [
        "point",
        "point2",
        "delta",
        "copy_error",
        "copy_error2",
        "insertion",
        "reversal",
        "minor_deletion",
        "major_deletion",
        "amplification",
        "translocation",
    ] {
        assert!(value.get(class).is_some(), "missing mutation class {class}");
    }
    assert!(value.get("point_category_distribution").is_some());
    assert!(value.get("copy_error_category_distribution").is_some());
}

#[test]
fn multiplier_divides_the_robots_one_in_rate() {
    let profile = StandardMutationProfile::default();

    assert_eq!(
        profile.effective_one_in(StandardMutationKind::Point, 4.0),
        1_250.0
    );
}

#[test]
fn robots_of_one_species_own_isolated_mutation_profiles() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let first = engine.spawn(LegacyDna::parse("1 2 3 4").unwrap()).unwrap();
    let second = engine.spawn(LegacyDna::parse("1 2 3 4").unwrap()).unwrap();

    let mut changed = engine.mutation_profile(first).unwrap().clone();
    changed.point.one_in = 1_250.0;
    engine.set_mutation_profile(first, changed).unwrap();

    assert_eq!(
        engine.mutation_profile(first).unwrap().point.one_in,
        1_250.0
    );
    assert_eq!(
        engine.mutation_profile(second).unwrap().point.one_in,
        5_000.0
    );
}

#[test]
fn changing_a_robot_point_rate_reschedules_its_next_mutation() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let id = engine
        .spawn_at(LegacyDna::parse("1 2 3 4").unwrap(), [500.0, 500.0])
        .unwrap();

    let mut slow_profile = engine.mutation_profile(id).unwrap().clone();
    slow_profile.point.one_in = 1_000_000_000.0;
    engine.set_mutation_profile(id, slow_profile).unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.snapshot().stats.mutations, 0);

    let mut fast_profile = engine.mutation_profile(id).unwrap().clone();
    fast_profile.point.one_in = 1.0;
    engine.set_mutation_profile(id, fast_profile).unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.snapshot().stats.mutations, 1);
    assert_eq!(engine.organism(id).unwrap().total_mutations, 1);
}

#[test]
fn point_schedule_uses_coding_length_and_excludes_master_end() {
    let dna = LegacyDna::parse("1 2 3 4").unwrap();
    assert_exact_dna(
        &dna,
        &[number(1), number(2), number(3), number(4), master_end()],
    );

    let schedule = PointMutationSchedule::from_geometric(40, dna.blocks().len(), 8).unwrap();

    assert_eq!(schedule.location, 1);
    assert_eq!(schedule.cycle, 42);
    assert!(schedule.location < dna.blocks().len() - 1);
}

#[test]
fn value_preference_uses_80_as_the_first_type_change_roll() {
    let profile = StandardMutationProfile::default();

    assert_eq!(profile.point_preference(79), MutationPreference::Value);
    assert_eq!(profile.point_preference(80), MutationPreference::Type);
    assert_eq!(profile.copy_error_preference(79), MutationPreference::Value);
    assert_eq!(profile.copy_error_preference(80), MutationPreference::Type);
}

#[test]
fn scripted_point_changes_three_blocks_without_touching_master_end() {
    let mut dna = LegacyDna::parse("1 2 3 4").unwrap();
    let mut profile = StandardMutationProfile::default();

    let report = apply_standard_mutation(
        &mut dna,
        &mut profile,
        StandardMutationEvent::Point {
            location: 1,
            replacements: vec![number(11), number(22), number(33)],
        },
    )
    .unwrap();

    assert_eq!(report.kind, StandardMutationKind::Point);
    assert_eq!(report.changes, 3);
    assert_exact_dna(
        &dna,
        &[number(11), number(22), number(33), number(4), master_end()],
    );
}

#[test]
fn scripted_delta_changes_only_the_selected_robot_rate() {
    let mut dna = LegacyDna::parse("1 2 3 4").unwrap();
    let original = StandardMutationProfile::default();
    let mut changed = original.clone();

    let report = apply_standard_mutation(
        &mut dna,
        &mut changed,
        StandardMutationEvent::Delta {
            target: StandardMutationKind::Point,
            new_one_in: 4_200.0,
        },
    )
    .unwrap();

    assert_eq!(report.kind, StandardMutationKind::Delta);
    assert_eq!(report.changes, 1);
    assert_eq!(changed.point.one_in, 4_200.0);
    assert_eq!(changed.delta, original.delta);
    assert_eq!(changed.copy_error, original.copy_error);
    assert_eq!(changed.insertion, original.insertion);
    assert_eq!(changed.reversal, original.reversal);
    assert_eq!(changed.minor_deletion, original.minor_deletion);
    assert_eq!(changed.major_deletion, original.major_deletion);
    assert_exact_dna(
        &dna,
        &[number(1), number(2), number(3), number(4), master_end()],
    );
}

#[test]
fn invalid_delta_one_in_is_rejected_atomically() {
    for new_one_in in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut dna = LegacyDna::parse("1 2 3 4").unwrap();
        let mut profile = StandardMutationProfile::default();
        let before_dna = dna.clone();
        let before_profile = profile.clone();

        assert!(
            apply_standard_mutation(
                &mut dna,
                &mut profile,
                StandardMutationEvent::Delta {
                    target: StandardMutationKind::Point,
                    new_one_in,
                },
            )
            .is_err(),
            "accepted Delta one-in {new_one_in:?}"
        );
        assert_eq!(dna, before_dna);
        assert_eq!(profile, before_profile);
    }
}

#[test]
fn structural_mutations_reject_zero_and_out_of_range_locations_atomically() {
    for event in [
        StandardMutationEvent::Insertion {
            after_location: 0,
            blocks: vec![number(9)],
        },
        StandardMutationEvent::Insertion {
            after_location: 6,
            blocks: vec![number(9)],
        },
        StandardMutationEvent::Reversal {
            center: 0,
            sampled_length: 3,
        },
        StandardMutationEvent::Reversal {
            center: 6,
            sampled_length: 3,
        },
        StandardMutationEvent::MinorDeletion {
            location: 0,
            length: 1,
        },
        StandardMutationEvent::MinorDeletion {
            location: 6,
            length: 1,
        },
        StandardMutationEvent::MajorDeletion {
            location: 0,
            length: 3,
        },
        StandardMutationEvent::MajorDeletion {
            location: 6,
            length: 3,
        },
    ] {
        assert_event_rejected_atomically(event);
    }
}

#[test]
fn insertion_rejects_insufficient_capacity_without_truncating_payload() {
    let mut dna = LegacyDna::parse(&"1 ".repeat(MAX_DNA_BLOCKS - 2)).unwrap();
    assert_eq!(dna.blocks().len(), MAX_DNA_BLOCKS - 1);
    let mut profile = StandardMutationProfile::default();
    let before_dna = dna.clone();
    let before_profile = profile.clone();

    assert!(
        apply_standard_mutation(
            &mut dna,
            &mut profile,
            StandardMutationEvent::Insertion {
                after_location: 1,
                blocks: vec![number(8), number(9)],
            },
        )
        .is_err()
    );
    assert_eq!(dna, before_dna);
    assert_eq!(profile, before_profile);
}

#[test]
fn mutation_payloads_reject_embedded_end_without_truncating_dna() {
    for event in [
        StandardMutationEvent::Point {
            location: 2,
            replacements: vec![master_end()],
        },
        StandardMutationEvent::CopyError {
            location: 2,
            replacements: vec![master_end()],
        },
        StandardMutationEvent::Insertion {
            after_location: 2,
            blocks: vec![master_end()],
        },
    ] {
        assert_event_rejected_atomically(event);
    }
}

#[test]
fn engine_rejects_every_malformed_profile_field_atomically() {
    let mut invalid_profiles = Vec::new();
    macro_rules! invalid_rate {
        ($label:literal, $rate:ident, $field:ident, $value:expr) => {{
            let mut profile = StandardMutationProfile::default();
            profile.$rate.$field = $value;
            invalid_profiles.push(($label, profile));
        }};
    }

    invalid_rate!("point one_in", point, one_in, 0.0);
    invalid_rate!("delta one_in", delta, one_in, -1.0);
    invalid_rate!("copy one_in", copy_error, one_in, f64::NAN);
    invalid_rate!("insertion one_in", insertion, one_in, f64::INFINITY);
    invalid_rate!("reversal one_in", reversal, one_in, 0.0);
    invalid_rate!("minor deletion one_in", minor_deletion, one_in, -1.0);
    invalid_rate!("major deletion one_in", major_deletion, one_in, f64::NAN);

    invalid_rate!("point mean", point, mean, f64::NAN);
    invalid_rate!("delta mean", delta, mean, f64::INFINITY);
    invalid_rate!("copy mean", copy_error, mean, f64::NAN);
    invalid_rate!("insertion mean", insertion, mean, f64::INFINITY);
    invalid_rate!("reversal mean", reversal, mean, f64::NAN);
    invalid_rate!("minor deletion mean", minor_deletion, mean, f64::INFINITY);
    invalid_rate!("major deletion mean", major_deletion, mean, f64::NAN);

    invalid_rate!("point std_dev", point, std_dev, -1.0);
    invalid_rate!("delta std_dev", delta, std_dev, f64::NAN);
    invalid_rate!("copy std_dev", copy_error, std_dev, f64::INFINITY);
    invalid_rate!("insertion std_dev", insertion, std_dev, -1.0);
    invalid_rate!("reversal std_dev", reversal, std_dev, f64::NAN);
    invalid_rate!(
        "minor deletion std_dev",
        minor_deletion,
        std_dev,
        f64::INFINITY
    );
    invalid_rate!("major deletion std_dev", major_deletion, std_dev, -1.0);

    let invalid_point_percent = StandardMutationProfile {
        point_value_change_percent: 101,
        ..StandardMutationProfile::default()
    };
    invalid_profiles.push(("point preference", invalid_point_percent));
    let invalid_copy_percent = StandardMutationProfile {
        copy_error_value_change_percent: 101,
        ..StandardMutationProfile::default()
    };
    invalid_profiles.push(("copy preference", invalid_copy_percent));

    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let id = engine.spawn(LegacyDna::parse("1 2 3 4").unwrap()).unwrap();
    let original = engine.mutation_profile(id).unwrap().clone();
    for (label, invalid) in invalid_profiles {
        assert!(
            engine.set_mutation_profile(id, invalid).is_err(),
            "accepted invalid {label}"
        );
        assert_eq!(
            engine.mutation_profile(id).unwrap(),
            &original,
            "installed invalid {label}"
        );
    }
}

#[test]
fn stale_generation_profile_access_cannot_touch_reused_slot() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let stale = engine.spawn(LegacyDna::parse("1 2 3 4").unwrap()).unwrap();
    engine.remove(stale).unwrap();
    let replacement = engine.spawn(LegacyDna::parse("5 6 7 8").unwrap()).unwrap();
    assert_eq!(stale.slot(), replacement.slot());
    assert_ne!(stale.generation(), replacement.generation());
    let replacement_profile = engine.mutation_profile(replacement).unwrap().clone();

    assert!(engine.mutation_profile(stale).is_err());
    let mut changed = StandardMutationProfile::default();
    changed.point.one_in = 1_250.0;
    assert!(engine.set_mutation_profile(stale, changed).is_err());
    assert_eq!(
        engine.mutation_profile(replacement).unwrap(),
        &replacement_profile
    );
}

#[test]
fn scripted_one_block_reproduction_mutations_are_exact() {
    let original = LegacyDna::parse("1 2 3 4 5").unwrap();

    let mut copied = original.clone();
    let mut copied_profile = StandardMutationProfile::default();
    let copy_report = apply_standard_mutation(
        &mut copied,
        &mut copied_profile,
        StandardMutationEvent::CopyError {
            location: 2,
            replacements: vec![number(9)],
        },
    )
    .unwrap();
    assert_eq!(copy_report.kind, StandardMutationKind::CopyError);
    assert_exact_dna(
        &copied,
        &[
            number(1),
            number(9),
            number(3),
            number(4),
            number(5),
            master_end(),
        ],
    );

    let mut inserted = original.clone();
    let mut inserted_profile = StandardMutationProfile::default();
    let insertion_report = apply_standard_mutation(
        &mut inserted,
        &mut inserted_profile,
        StandardMutationEvent::Insertion {
            after_location: 2,
            blocks: vec![number(9)],
        },
    )
    .unwrap();
    assert_eq!(insertion_report.kind, StandardMutationKind::Insertion);
    assert_exact_dna(
        &inserted,
        &[
            number(1),
            number(2),
            number(9),
            number(3),
            number(4),
            number(5),
            master_end(),
        ],
    );

    let mut deleted = original;
    let mut deleted_profile = StandardMutationProfile::default();
    let deletion_report = apply_standard_mutation(
        &mut deleted,
        &mut deleted_profile,
        StandardMutationEvent::MinorDeletion {
            location: 2,
            length: 1,
        },
    )
    .unwrap();
    assert_eq!(deletion_report.kind, StandardMutationKind::MinorDeletion);
    assert_exact_dna(
        &deleted,
        &[number(1), number(3), number(4), number(5), master_end()],
    );
}

#[test]
fn scripted_major_deletion_removes_exactly_three_blocks() {
    let mut dna = LegacyDna::parse("1 2 3 4 5").unwrap();
    let mut profile = StandardMutationProfile::default();

    let report = apply_standard_mutation(
        &mut dna,
        &mut profile,
        StandardMutationEvent::MajorDeletion {
            location: 2,
            length: 3,
        },
    )
    .unwrap();

    assert_eq!(report.kind, StandardMutationKind::MajorDeletion);
    assert_eq!(report.changes, 1);
    assert_exact_dna(&dna, &[number(1), number(5), master_end()]);
}

#[test]
fn scripted_reversal_reverses_an_odd_three_block_run() {
    let mut dna = LegacyDna::parse("1 2 3 4 5").unwrap();
    let mut profile = StandardMutationProfile::default();

    let report = apply_standard_mutation(
        &mut dna,
        &mut profile,
        StandardMutationEvent::Reversal {
            center: 3,
            sampled_length: 3,
        },
    )
    .unwrap();

    assert_eq!(report.kind, StandardMutationKind::Reversal);
    assert_eq!(report.changes, 1);
    assert_exact_dna(
        &dna,
        &[
            number(1),
            number(4),
            number(3),
            number(2),
            number(5),
            master_end(),
        ],
    );
}
