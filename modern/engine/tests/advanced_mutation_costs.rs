use darwinbots_engine::{
    AdvancedMutationSettings, Delta2Settings, DnaCosts, EpigeneticPolicyAction, EpigeneticSettings,
    GenomeMutator, LegacyDna, MutationOscillationMode, MutationOscillationSettings,
    MutationPolicyInput, MutationPolicyReport, SimulationCostAction, SimulationCostInput,
    StandardMutationProfile, apply_advanced_mutation_policies,
};

#[test]
fn advanced_mutation_policies_and_simulation_costs_apply_in_rust_authority() {
    struct MutationCase {
        name: &'static str,
        settings: AdvancedMutationSettings,
        input: MutationPolicyInput,
        expected_multiplier: f64,
        expected_one_in: f64,
        expected_delta2: bool,
        expected_epigenetic: EpigeneticPolicyAction,
        expected_save_as_temporary_gene: bool,
        expected_staging: [i32; 15],
    }

    let cases = [
        MutationCase {
            name: "delta2",
            settings: AdvancedMutationSettings {
                delta2: Delta2Settings {
                    enabled: true,
                    interval: 1,
                    rate_step: -500.0,
                    distribution_step: 0.0,
                    preference_step_percent: -10,
                },
                ..Default::default()
            },
            input: MutationPolicyInput {
                cycle: 1,
                dna_len: 100,
                mutation_count: 0,
                base_multiplier: 1.0,
            },
            expected_multiplier: 1.0,
            expected_one_in: 4_500.0,
            expected_delta2: true,
            expected_epigenetic: EpigeneticPolicyAction::None,
            expected_save_as_temporary_gene: false,
            expected_staging: [2; 15],
        },
        MutationCase {
            name: "normalization",
            settings: AdvancedMutationSettings {
                normalization_enabled: true,
                normalization_dna_length: 10,
                ..Default::default()
            },
            input: MutationPolicyInput {
                cycle: 0,
                dna_len: 100,
                mutation_count: 0,
                base_multiplier: 1.0,
            },
            expected_multiplier: 1.0,
            expected_one_in: 50_000.0,
            expected_delta2: false,
            expected_epigenetic: EpigeneticPolicyAction::None,
            expected_save_as_temporary_gene: false,
            expected_staging: [2; 15],
        },
        MutationCase {
            name: "oscillation",
            settings: AdvancedMutationSettings {
                oscillation: MutationOscillationSettings {
                    mode: MutationOscillationMode::Sawtooth,
                    period: 4,
                    amplitude: 0.5,
                },
                ..Default::default()
            },
            input: MutationPolicyInput {
                cycle: 1,
                dna_len: 100,
                mutation_count: 0,
                base_multiplier: 2.0,
            },
            expected_multiplier: 1.5,
            expected_one_in: 3_333.3333333333335,
            expected_delta2: false,
            expected_epigenetic: EpigeneticPolicyAction::None,
            expected_save_as_temporary_gene: false,
            expected_staging: [2; 15],
        },
        MutationCase {
            name: "epigenetics",
            settings: AdvancedMutationSettings {
                epigenetic: EpigeneticSettings {
                    reset_enabled: false,
                    amplification: 2.0,
                    overload_threshold: 3,
                    save_as_temporary_gene: true,
                },
                ..Default::default()
            },
            input: MutationPolicyInput {
                cycle: 0,
                dna_len: 100,
                mutation_count: 4,
                base_multiplier: 1.0,
            },
            expected_multiplier: 1.0,
            expected_one_in: 5_000.0,
            expected_delta2: false,
            expected_epigenetic: EpigeneticPolicyAction::Amplified,
            expected_save_as_temporary_gene: true,
            expected_staging: [4; 15],
        },
    ];

    for case in cases {
        let mut profile = StandardMutationProfile::default();
        let mut staging = [2; 15];
        let report = apply_advanced_mutation_policies(
            &mut profile,
            &case.settings,
            case.input,
            &mut staging,
        );

        assert_eq!(
            report.effective_multiplier, case.expected_multiplier,
            "{}",
            case.name
        );
        assert_eq!(
            report.effective_one_in, case.expected_one_in,
            "{}",
            case.name
        );
        assert_eq!(report.delta2_applied, case.expected_delta2, "{}", case.name);
        assert_eq!(
            report.epigenetic_action, case.expected_epigenetic,
            "{}",
            case.name
        );
        assert_eq!(
            report.save_as_temporary_gene, case.expected_save_as_temporary_gene,
            "{}",
            case.name
        );
        assert_eq!(staging, case.expected_staging, "{}", case.name);
    }

    let mut dna = LegacyDna::parse("1 2 3 4").unwrap();
    let mut mutator = GenomeMutator::new(1);
    let no_mutation = MutationPolicyReport {
        effective_multiplier: 1.0,
        effective_one_in: f64::MAX,
        delta2_applied: false,
        epigenetic_action: EpigeneticPolicyAction::None,
        save_as_temporary_gene: false,
    };
    assert_eq!(
        mutator
            .mutate_with_policy(&mut dna, &StandardMutationProfile::default(), no_mutation)
            .changes,
        0
    );

    let costs = DnaCosts {
        movement: 2.0,
        body_upkeep: 0.5,
        age_base: 1.0,
        age_start: 2.0,
        age_linear: 0.25,
        global_multiplier: 2.0,
        dynamic_cost_multiplier: 1.0,
        dynamic_target_population: 10,
        dynamic_population_range: 4,
        dynamic_sensitivity: 1.0,
        ..Default::default()
    };
    let cost = costs.calculate_cost(SimulationCostInput {
        body: 10.0,
        dna_len: 5,
        age: 4,
        population: 8,
        action: Some((SimulationCostAction::Movement, 3.0)),
    });

    assert_eq!(cost.upkeep, 5.0);
    assert_eq!(cost.age, 1.5);
    assert_eq!(cost.action, 6.0);
    assert_eq!(cost.dynamic_multiplier, 3.0);
    assert_eq!(cost.total, 37.5);

    let dna_upkeep = DnaCosts {
        dna_upkeep: 2.0,
        ..DnaCosts::default()
    };
    assert_eq!(dna_upkeep.upkeep_cost(0.0, 5, 0), 8.0);
}
