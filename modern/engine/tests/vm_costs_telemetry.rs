use darwinbots_engine::{
    DnaCosts, DnaVm, Engine, EngineConfig, LegacyDna, VmContext, VmMemory, VmTermination,
};

fn run_report(
    source: &str,
    costs: &DnaCosts,
    record: bool,
) -> (VmMemory, darwinbots_engine::VmReport) {
    let program = LegacyDna::parse(source).unwrap();
    let mut memory = VmMemory::default();
    let report = DnaVm::new(7)
        .execute(
            &program,
            &mut memory,
            VmContext {
                position: [0.0, 0.0],
                costs,
                record_activations: record,
            },
        )
        .unwrap();
    (memory, report)
}

#[test]
fn gene_numbers_conditions_and_activations_match_flow() {
    let source = "cond 1 0 > start 11 100 store stop\ncond 0 1 > start 22 101 store stop\ncond 0 1 > start 33 102 store else 44 103 store stop\nstart 55 104 store stop";
    let (memory, report) = run_report(source, &DnaCosts::default(), true);

    assert_eq!(memory.read(100), 11);
    assert_eq!(memory.read(101), 0);
    assert_eq!(memory.read(102), 0);
    assert_eq!(memory.read(103), 44);
    assert_eq!(memory.read(104), 55);
    assert_eq!(memory.read_sysvar("thisgene"), 4);
    assert_eq!(report.conditions_evaluated, 3);
    assert_eq!(report.current_gene, 4);
    assert_eq!(report.activations, vec![false, true, false, true, true]);
    assert_eq!(report.termination, VmTermination::End);

    let (_, quiet) = run_report(source, &DnaCosts::default(), false);
    assert!(quiet.activations.is_empty());
    assert!(quiet.debug_text.is_empty());

    let (_, no_store) = run_report("start stop", &DnaCosts::default(), true);
    assert_eq!(no_store.stores_applied, 0);
    assert_eq!(no_store.activations, vec![false, true]);

    let direct_else_cases = [
        ("cond false else 66 200 store stop", 66),
        ("cond else 77 200 store stop", 0),
        ("cond true else 88 200 store stop", 0),
    ];
    for (source, expected) in direct_else_cases {
        let (memory, _) = run_report(source, &DnaCosts::default(), true);
        assert_eq!(memory.read(200), expected, "{source}");
    }
}

#[test]
fn normal_tick_publishes_vm_activation_telemetry_for_live_diagnostics() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let robot = engine
        .spawn_at(
            LegacyDna::parse("cond 1 0 > start 11 100 store stop").unwrap(),
            [500.0, 500.0],
        )
        .unwrap();

    engine.tick().unwrap();

    let program = &engine.robot_state(robot).unwrap().program;
    assert_eq!(program.current_gene, 1);
    assert_eq!(program.activation_bitmap, vec![false, true]);

    let snapshot = serde_json::to_value(engine.snapshot()).unwrap();
    assert_eq!(
        snapshot["organisms"][0]["activation_history"],
        serde_json::json!([{"tick": 1, "activations": [false, true]}])
    );
}

#[test]
fn token_and_relative_store_costs_are_charged_exactly() {
    let category_cases = [
        (
            "start 5 0 store stop",
            DnaCosts {
                number: 3.0,
                ..DnaCosts::default()
            },
            6.0,
        ),
        (
            "start *1 stop",
            DnaCosts {
                dereference: 4.0,
                ..DnaCosts::default()
            },
            4.0,
        ),
        (
            "start add stop",
            DnaCosts {
                basic: 5.0,
                ..DnaCosts::default()
            },
            5.0,
        ),
        (
            "start sqr stop",
            DnaCosts {
                advanced: 6.0,
                ..DnaCosts::default()
            },
            6.0,
        ),
        (
            "start debugint stop",
            DnaCosts {
                advanced: 6.0,
                ..DnaCosts::default()
            },
            0.0,
        ),
        (
            "start sqr stop",
            DnaCosts {
                advanced: -0.25,
                global_multiplier: 2.0,
                ..DnaCosts::default()
            },
            -0.5,
        ),
        (
            "start ~ stop",
            DnaCosts {
                bitwise: 7.0,
                ..DnaCosts::default()
            },
            7.0,
        ),
        (
            "cond < start stop",
            DnaCosts {
                condition: 8.0,
                ..DnaCosts::default()
            },
            8.0,
        ),
        (
            "cond and start stop",
            DnaCosts {
                logic: 9.0,
                ..DnaCosts::default()
            },
            9.0,
        ),
        (
            "start 0 1 store stop",
            DnaCosts {
                store: 10.0,
                ..DnaCosts::default()
            },
            10.0,
        ),
        (
            "start stop",
            DnaCosts {
                flow: 11.0,
                ..DnaCosts::default()
            },
            22.0,
        ),
    ];
    for (source, costs, expected) in category_cases {
        assert_eq!(
            run_report(source, &costs, false).1.energy_cost,
            expected,
            "{source}"
        );
    }

    let relative_cases = [
        ("start 1 41 store stop", 280.0),
        ("start 41 inc stop", 28.0),
        ("start 41 dec stop", 28.0),
        ("start 1 41 addstore stop", 56.0),
        ("start 1 41 substore stop", 56.0),
        ("start 1 41 multstore stop", 56.0),
        ("start 1 41 divstore stop", 56.0),
        ("start 1 41 ceilstore stop", 56.0),
        ("start 1 41 floorstore stop", 56.0),
        ("start 41 rndstore stop", 40.0),
        ("start 41 sgnstore stop", 40.0),
        ("start 41 absstore stop", 35.0),
        ("start 41 sqrstore stop", 40.0),
        ("start 41 negstore stop", 35.0),
    ];
    for (source, expected) in relative_cases {
        let costs = DnaCosts {
            store: 280.0,
            ..DnaCosts::default()
        };
        assert_eq!(
            run_report(source, &costs, false).1.energy_cost,
            expected,
            "{source}"
        );
    }

    let upkeep = DnaCosts {
        dna_upkeep: 2.5,
        ..DnaCosts::default()
    };
    assert_eq!(
        run_report("start 1 drop stop", &upkeep, false)
            .1
            .energy_cost,
        10.0
    );

    let multiplied = DnaCosts {
        number: 1.5,
        global_multiplier: 2.0,
        ..DnaCosts::default()
    };
    assert_eq!(
        run_report("start 1 drop stop", &multiplied, false)
            .1
            .energy_cost,
        3.0
    );
}

#[test]
fn zero_destination_store_has_zero_store_cost() {
    let stores = [
        "store",
        "inc",
        "dec",
        "addstore",
        "substore",
        "multstore",
        "divstore",
        "ceilstore",
        "floorstore",
        "rndstore",
        "sgnstore",
        "absstore",
        "sqrstore",
        "negstore",
    ];
    let costs = DnaCosts {
        store: 280.0,
        ..DnaCosts::default()
    };
    for command in stores {
        let source = format!("start 123 0 {command} stop");
        let (memory, report) = run_report(&source, &costs, false);
        assert_eq!(memory.read(123), 0, "{command}");
        assert_eq!(report.stores_applied, 0, "{command}");
        assert_eq!(report.energy_cost, 0.0, "{command}");
    }
}

#[test]
fn fractional_cost_remainders_settle_positive_and_negative_energy() {
    for (dna_upkeep, expected_energy, expected_remainder) in [(0.3, 999, 0.2), (-0.3, 1001, -0.2)] {
        let mut config = EngineConfig::testing();
        config.dna_costs = DnaCosts {
            dna_upkeep,
            ..DnaCosts::default()
        };
        let mut engine = Engine::new(config).unwrap();
        let id = engine
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0, 500.0])
            .unwrap();

        engine.tick().unwrap();
        engine.tick().unwrap();

        let robot = engine.robot_state(id).unwrap();
        assert_eq!(robot.materials.energy, expected_energy);
        assert!((robot.program.fractional_cost_remainder - expected_remainder).abs() < 1e-12);
        assert!(robot.program.activation_bitmap.is_empty());
        assert!(robot.program.debug_text.is_empty());
    }
}
