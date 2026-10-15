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
