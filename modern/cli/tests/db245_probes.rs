use darwinbots_cli::run_from;
use std::{fs, io::Cursor, path::Path, time::Instant};

#[test]
fn every_db245_bot_completes_the_standard_probe() {
    let cli_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let bots_root = cli_root.join("../../Installer/bots");
    let manifest =
        fs::read_to_string(cli_root.join("../tests/fixtures/db245-bots.sha256")).unwrap();
    let alga = bots_root.join("Alga_Minimalis.txt");
    let mut failures = Vec::new();

    for line in manifest.lines().filter(|line| !line.trim().is_empty()) {
        let (_, name) = line.split_once('|').unwrap();
        let bot = bots_root.join(name);
        eprintln!("DB2 probe starting: {name}");
        let started = Instant::now();
        let mut output = Cursor::new(Vec::new());
        let arguments = vec![
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
        ];
        match run_from(arguments, &mut output) {
            Ok(()) => {
                let report: serde_json::Value = serde_json::from_slice(output.get_ref()).unwrap();
                let candidate = report["candidate_final_population"].as_u64();
                let vegetables = report["vegetable_final_population"].as_u64();
                let total = report["population"].as_u64();
                let candidate_births = report["candidate_births"].as_u64();
                let births = report["births"].as_u64();
                let candidate_deaths = report["candidate_deaths"].as_u64();
                let deaths = report["deaths"].as_u64();
                let candidate_accounted = candidate_births
                    .and_then(|value| 5u64.checked_add(value))
                    .zip(candidate_deaths)
                    .and_then(|(value, deaths)| value.checked_sub(deaths));
                if report["loaded"] != true
                    || report["tick"] != 1000
                    || report["starting_population"] != 25
                    || report["candidate_starting_population"] != 5
                    || report["vegetable_starting_population"] != 20
                    || report["final_population"] != report["population"]
                    || candidate.zip(vegetables).map(|(left, right)| left + right) != total
                    || candidate_births.is_none()
                    || births.is_none()
                    || candidate_births > births
                    || candidate_deaths.is_none()
                    || deaths.is_none()
                    || candidate_deaths > deaths
                    || candidate_accounted != candidate
                {
                    failures.push(format!("{name}: unexpected report {report}"));
                }
            }
            Err(error) => failures.push(format!("{name}: {error}")),
        }
        eprintln!(
            "DB2 probe finished: {name} in {:.2}s",
            started.elapsed().as_secs_f64()
        );
    }

    assert!(
        failures.is_empty(),
        "DB2 probe failures:\n{}",
        failures.join("\n")
    );
}
