use darwinbots_cli::run_from;
use serde_json::Value;
use std::{fs, io::Cursor};

#[test]
fn suite_replay_and_compare_preserve_inputs_and_fail_for_changed_state() {
    let temp = tempfile::tempdir().unwrap();
    let suite = temp.path().join("oracle");
    let mut output = Cursor::new(Vec::new());
    run_from(
        [
            "darwinbots",
            "native-suite",
            "--directory",
            suite.to_str().unwrap(),
        ],
        &mut output,
    )
    .unwrap();
    let manifest: Value = serde_json::from_slice(output.get_ref()).unwrap();
    assert_eq!(manifest["cases"].as_array().unwrap().len(), 7);
    let start = suite.join("movement-start.sim");
    let input = fs::read(&start).unwrap();
    let initial_file = darwinbots_engine::native_binary::NativeRecordFile::decode(&input).unwrap();
    let initial_tail =
        darwinbots_engine::native_binary::NativeSimulationTail::decode(&initial_file.tail).unwrap();
    assert_eq!(initial_tail.integer("total_cycles").unwrap(), 0);
    let end = suite.join("movement-rust-cycle1.sim");
    output.get_mut().clear();
    output.set_position(0);
    run_from(
        [
            "darwinbots",
            "native-replay",
            "--simulation",
            start.to_str().unwrap(),
            "--until-cycle",
            "1",
            "--output-sim",
            end.to_str().unwrap(),
        ],
        &mut output,
    )
    .unwrap();
    let result: Value = serde_json::from_slice(output.get_ref()).unwrap();
    assert_eq!(result["end_cycle"], 1);
    assert_eq!(fs::read(&start).unwrap(), input);
    let mut identical = Vec::new();
    run_from(
        [
            "darwinbots",
            "native-compare",
            "--left",
            end.to_str().unwrap(),
            "--right",
            end.to_str().unwrap(),
        ],
        &mut identical,
    )
    .unwrap();
    let result: Value = serde_json::from_slice(&identical).unwrap();
    assert_eq!(result["passed"], true);
    assert!(result["compared_fields"].as_u64().unwrap() > 1000);
    let mut changed = Vec::new();
    assert!(
        run_from(
            [
                "darwinbots",
                "native-compare",
                "--left",
                start.to_str().unwrap(),
                "--right",
                end.to_str().unwrap()
            ],
            &mut changed
        )
        .is_err()
    );
    let result: Value = serde_json::from_slice(&changed).unwrap();
    assert_eq!(result["passed"], false);
    assert!(result["difference_count"].as_u64().unwrap() > 0);
    // Neither repeated suite preparation nor a replay may replace receipts.
    assert!(
        run_from(
            [
                "darwinbots",
                "native-suite",
                "--directory",
                suite.to_str().unwrap()
            ],
            Vec::new()
        )
        .is_err()
    );
    let receipt = fs::read(&end).unwrap();
    assert!(
        run_from(
            [
                "darwinbots",
                "native-replay",
                "--simulation",
                start.to_str().unwrap(),
                "--ticks",
                "2",
                "--output-sim",
                end.to_str().unwrap()
            ],
            Vec::new()
        )
        .is_err()
    );
    assert_eq!(fs::read(end).unwrap(), receipt);
}

#[test]
fn replay_uses_signed_original_cycles_and_preserves_the_pending_first_update() {
    use darwinbots_engine::native_binary::{NativeRecordFile, NativeSimulationTail};
    use darwinbots_engine::{Engine, EngineConfig, LegacyDna};

    let temp = tempfile::tempdir().unwrap();
    let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
    world.update_mutation_settings(false, 1.0, &[]).unwrap();
    world
        .spawn_at(
            LegacyDna::parse("start 7 900 store stop").unwrap(),
            [500.0; 2],
        )
        .unwrap();
    let start = temp.path().join("pending.sim");
    let input = world.export_native_sim().unwrap();
    fs::write(&start, &input).unwrap();
    for (target, updates) in [(-1, 0_u64), (0, 1), (1, 2), (10, 11)] {
        let end = temp.path().join(format!("cycle-{target}.sim"));
        let mut output = Vec::new();
        run_from(
            [
                "darwinbots",
                "native-replay",
                "--simulation",
                start.to_str().unwrap(),
                "--until-cycle",
                &target.to_string(),
                "--output-sim",
                end.to_str().unwrap(),
            ],
            &mut output,
        )
        .unwrap();
        let report: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(report["start_cycle"], -1);
        assert_eq!(report["end_cycle"], target);
        assert_eq!(report["ticks"], updates);
        assert_eq!(report["processed_tick"], updates);
        let saved = NativeRecordFile::decode(&fs::read(end).unwrap()).unwrap();
        let tail = NativeSimulationTail::decode(&saved.tail).unwrap();
        assert_eq!(tail.integer("total_cycles").unwrap(), target);
        let memory = saved.robots[0].bytes("memory").unwrap();
        // The saved array includes cell zero; cycle zero must execute DNA once.
        let word = i16::from_le_bytes(memory[1800..1802].try_into().unwrap());
        assert_eq!(word, if updates == 0 { 0 } else { 7 });
    }
    assert_eq!(fs::read(&start).unwrap(), input);

    for arguments in [["--until-cycle", "-2"], ["--ticks", "2147483649"]] {
        let end = temp.path().join("rejected.sim");
        let mut args = vec![
            "darwinbots",
            "native-replay",
            "--simulation",
            start.to_str().unwrap(),
        ];
        args.extend(arguments);
        args.extend(["--output-sim", end.to_str().unwrap()]);
        assert!(run_from(args, Vec::new()).is_err());
        assert!(!end.exists());
        assert_eq!(fs::read(&start).unwrap(), input);
    }
}

#[test]
fn comparison_reports_actual_fields_and_applies_tolerance_only_to_floats() {
    use darwinbots_engine::native_binary::NativeRecordFile;
    use darwinbots_engine::{Engine, EngineConfig, LegacyDna};
    let temp = tempfile::tempdir().unwrap();
    let mut world = Engine::new(EngineConfig::testing()).unwrap();
    world
        .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2])
        .unwrap();
    let left = world.export_native_sim().unwrap();
    let mut file = NativeRecordFile::decode(&left).unwrap();
    file.robots[0].set_single("position_x", 500.00003).unwrap();
    let a = temp.path().join("a.sim");
    let b = temp.path().join("b.sim");
    fs::write(&a, left).unwrap();
    fs::write(&b, file.encode().unwrap()).unwrap();
    let mut output = Vec::new();
    run_from(
        [
            "darwinbots",
            "native-compare",
            "--left",
            a.to_str().unwrap(),
            "--right",
            b.to_str().unwrap(),
        ],
        &mut output,
    )
    .unwrap();
    output.clear();
    assert!(
        run_from(
            [
                "darwinbots",
                "native-compare",
                "--left",
                a.to_str().unwrap(),
                "--right",
                b.to_str().unwrap(),
                "--absolute-tolerance",
                "0",
                "--relative-tolerance",
                "0"
            ],
            &mut output
        )
        .is_err()
    );
    let result: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(result["difference_count"], 1);
    assert!(
        result["differences"][0]["field"]
            .as_str()
            .unwrap()
            .ends_with(".position_x")
    );
}

#[test]
fn zero_step_replay_preserves_validated_original_bytes_and_create_new_outputs() {
    let original = include_bytes!("../../engine/tests/fixtures/db24832-original-user.sim");
    // The runtime loader deliberately normalizes this old source image. Zero
    // file updates must nevertheless preserve its complete original byte stream.
    let loaded = darwinbots_engine::Engine::from_native_sim(original).unwrap();
    assert_ne!(loaded.export_native_sim().unwrap(), original.as_slice());
    let temp = tempfile::tempdir().unwrap();
    let start = temp.path().join("original.sim");
    fs::write(&start, original).unwrap();
    for (option, value, name) in [
        ("--ticks", "0", "ticks-zero.sim"),
        ("--until-cycle", "281", "same-cycle.sim"),
    ] {
        let end = temp.path().join(name);
        let mut output = Vec::new();
        let arguments = [
            "darwinbots",
            "native-replay",
            "--simulation",
            start.to_str().unwrap(),
            option,
            value,
            "--output-sim",
            end.to_str().unwrap(),
        ];
        run_from(arguments, &mut output).unwrap();
        let report: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(report["start_cycle"], 281);
        assert_eq!(report["end_cycle"], 281);
        assert_eq!(report["ticks"], 0);
        assert_eq!(report["processed_tick"], 281);
        assert_eq!(report["output_image"], "unchanged_input");
        assert_eq!(fs::read(&end).unwrap(), original.as_slice());
        assert_eq!(fs::read(&start).unwrap(), original.as_slice());
        assert!(run_from(arguments, Vec::new()).is_err());
        assert_eq!(fs::read(&end).unwrap(), original.as_slice());
    }
    assert!(
        run_from(
            [
                "darwinbots",
                "native-replay",
                "--simulation",
                start.to_str().unwrap(),
                "--ticks",
                "0",
                "--output-sim",
                start.to_str().unwrap(),
            ],
            Vec::new()
        )
        .is_err()
    );
    assert_eq!(fs::read(&start).unwrap(), original.as_slice());

    // Byte preservation does not bypass native stream/runtime validation.
    let malformed = temp.path().join("malformed.sim");
    let rejected = temp.path().join("rejected.sim");
    fs::write(&malformed, b"not a native simulation").unwrap();
    assert!(
        run_from(
            [
                "darwinbots",
                "native-replay",
                "--simulation",
                malformed.to_str().unwrap(),
                "--ticks",
                "0",
                "--output-sim",
                rejected.to_str().unwrap(),
            ],
            Vec::new()
        )
        .is_err()
    );
    assert!(!rejected.exists());
    assert_eq!(fs::read(&malformed).unwrap(), b"not a native simulation");
}

#[test]
fn positive_replay_exports_actual_runtime_and_matches_the_genuine_first_update() {
    let original_start =
        include_bytes!("../../engine/tests/fixtures/db24832-movement-start-48032.sim");
    let genuine_c1 =
        include_bytes!("../../engine/tests/fixtures/db24832-genuine-movement-cycle1-20261016.sim");
    let temp = tempfile::tempdir().unwrap();
    let start = temp.path().join("start.sim");
    let expected = temp.path().join("genuine-c1.sim");
    let end = temp.path().join("rust-c1.sim");
    fs::write(&start, original_start).unwrap();
    fs::write(&expected, genuine_c1).unwrap();
    let mut output = Vec::new();
    run_from(
        [
            "darwinbots",
            "native-replay",
            "--simulation",
            start.to_str().unwrap(),
            "--ticks",
            "1",
            "--output-sim",
            end.to_str().unwrap(),
        ],
        &mut output,
    )
    .unwrap();
    let report: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(report["start_cycle"], 0);
    assert_eq!(report["end_cycle"], 1);
    assert_eq!(report["ticks"], 1);
    assert_eq!(report["output_image"], "engine_native_export");
    assert_eq!(fs::read(&start).unwrap(), original_start.as_slice());
    output.clear();
    run_from(
        [
            "darwinbots",
            "native-compare",
            "--left",
            expected.to_str().unwrap(),
            "--right",
            end.to_str().unwrap(),
            "--absolute-tolerance",
            "0",
            "--relative-tolerance",
            "0",
            "--max-differences",
            "100",
        ],
        &mut output,
    )
    .unwrap();
    let comparison: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(comparison["compared_fields"], 1379);
    assert_eq!(comparison["difference_count"], 0, "{comparison}");
    assert_eq!(comparison["passed"], true);
}
