use darwinbots_cli::run_from;
use std::{collections::HashSet, fs, io::Cursor, path::Path, path::PathBuf, time::Instant};

fn collect_bots(directory: &Path, bots: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            collect_bots(&entry.path(), bots);
        } else if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "txt")
        {
            bots.push(entry.path());
        }
    }
}

#[test]
fn remaining_bundled_bots_complete_the_standard_probe() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let bots_root = root.join("../../Installer/bots");
    let reference_manifest =
        fs::read_to_string(root.join("../tests/fixtures/db245-bots.sha256")).unwrap();
    let reference_bots: HashSet<_> = reference_manifest
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| bots_root.join(line.split_once('|').unwrap().1))
        .collect();
    let mut bots = Vec::new();
    collect_bots(&bots_root, &mut bots);
    assert_eq!(
        bots.len(),
        48,
        "the shipped corpus changed; update its acceptance contract"
    );
    bots.retain(|bot| !reference_bots.contains(bot));
    bots.sort();
    assert_eq!(
        bots.len(),
        25,
        "standard and additional probes must cover the whole corpus"
    );
    let alga = bots_root.join("Alga_Minimalis.txt");
    let mut failures = Vec::new();
    for bot in bots {
        let name = bot.strip_prefix(&bots_root).unwrap().display().to_string();
        eprintln!("Additional bundled probe starting: {name}");
        let started = Instant::now();
        let mut output = Cursor::new(Vec::new());
        let result = run_from(
            vec![
                "darwinbots".to_owned(),
                "run".to_owned(),
                "--bot".to_owned(),
                bot.to_string_lossy().into_owned(),
                "--population".to_owned(),
                "5".to_owned(),
                "--vegetable-bot".to_owned(),
                alga.to_string_lossy().into_owned(),
                "--vegetable-population".to_owned(),
                "20".to_owned(),
                "--initial-energy".to_owned(),
                "1000".to_owned(),
                "--ticks".to_owned(),
                "1000".to_owned(),
                "--seed".to_owned(),
                "24501".to_owned(),
                "--backend".to_owned(),
                "cpu".to_owned(),
            ],
            &mut output,
        );
        match result {
            Ok(()) => {
                let report: serde_json::Value = serde_json::from_slice(output.get_ref()).unwrap();
                let candidates = report["candidate_final_population"].as_u64();
                let vegetables = report["vegetable_final_population"].as_u64();
                let total = report["population"].as_u64();
                let births = report["candidate_births"].as_u64();
                let deaths = report["candidate_deaths"].as_u64();
                let accounted = births
                    .and_then(|births| births.checked_add(5))
                    .zip(deaths)
                    .and_then(|(born, dead)| born.checked_sub(dead));
                if report["loaded"] != true
                    || report["tick"] != 1000
                    || report["starting_population"] != 25
                    || report["candidate_starting_population"] != 5
                    || report["vegetable_starting_population"] != 20
                    || candidates.is_none()
                    || candidates != accounted
                    || candidates
                        .zip(vegetables)
                        .and_then(|(a, b)| a.checked_add(b))
                        != total
                    || report["final_population"] != report["population"]
                    || report["backend"] != "Cpu"
                {
                    failures.push(format!("{name}: unexpected report {report}"));
                }
                eprintln!(
                    "Additional bundled probe {name}: candidates={candidates:?}, births={births:?}, deaths={deaths:?}, warnings={}",
                    report["compatibility_warnings"]
                );
            }
            Err(error) => failures.push(format!("{name}: {error}")),
        }
        eprintln!(
            "Additional bundled probe finished: {name} in {:.2}s",
            started.elapsed().as_secs_f64()
        );
    }
    assert!(
        failures.is_empty(),
        "Additional bundled probe failures:\n{}",
        failures.join("\n")
    );
}
