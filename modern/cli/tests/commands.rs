use darwinbots_cli::run_from;
use std::{fs, io::Cursor};

#[test]
fn run_imports_legacy_bot_advances_ticks_and_emits_json() {
    let directory = tempfile::tempdir().unwrap();
    let bot = directory.path().join("minimal.txt");
    fs::write(&bot, "start\n10 .up store\nstop\n").unwrap();
    let mut output = Cursor::new(Vec::new());

    run_from(
        ["darwinbots", "run", "--bot", bot.to_str().unwrap(), "--ticks", "3", "--backend", "cpu"],
        &mut output,
    ).unwrap();

    let report: serde_json::Value = serde_json::from_slice(output.get_ref()).unwrap();
    assert_eq!(report["tick"], 3);
    assert_eq!(report["population"], 1);
    assert_eq!(report["backend"], "Cpu");
}

#[test]
fn benchmark_populates_requested_number_of_organisms() {
    let directory = tempfile::tempdir().unwrap();
    let bot = directory.path().join("minimal.txt");
    fs::write(&bot, "start\nstop\n").unwrap();
    let mut output = Cursor::new(Vec::new());

    run_from(
        [
            "darwinbots", "bench", "--bot", bot.to_str().unwrap(), "--population", "128",
            "--ticks", "2", "--backend", "cpu",
        ],
        &mut output,
    ).unwrap();

    let report: serde_json::Value = serde_json::from_slice(output.get_ref()).unwrap();
    assert_eq!(report["population"], 128);
    assert_eq!(report["ticks"], 2);
    assert_eq!(report["warmup_ticks"], 3);
    assert_eq!(report["layout"], "uniform-grid");
    assert!(report["ticks_per_second"].as_f64().unwrap() > 0.0);
    assert!(report["phase_ms"]["dna"].as_f64().is_some());
}

#[test]
fn stress_command_checks_invariants_and_emits_reproducible_hash() {
    let directory = tempfile::tempdir().unwrap();
    let bot = directory.path().join("minimal.txt");
    fs::write(&bot, "start\nstop\n").unwrap();
    let arguments = [
        "darwinbots", "stress", "--bot", bot.to_str().unwrap(), "--population", "4",
        "--ticks", "25", "--invariant-interval", "5", "--backend", "cpu", "--seed", "9",
    ];
    let mut first = Cursor::new(Vec::new());
    let mut second = Cursor::new(Vec::new());
    run_from(arguments, &mut first).unwrap();
    run_from(arguments, &mut second).unwrap();
    let first: serde_json::Value = serde_json::from_slice(first.get_ref()).unwrap();
    let second: serde_json::Value = serde_json::from_slice(second.get_ref()).unwrap();
    assert_eq!(first["ticks"], 25);
    assert_eq!(first["invariant_checks"], 5);
    assert_eq!(first["final_state_hash"], second["final_state_hash"]);
}
