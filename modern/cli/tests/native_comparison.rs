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
