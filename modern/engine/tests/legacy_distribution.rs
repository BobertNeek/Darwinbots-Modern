use darwinbots_engine::LegacyDna;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
fn every_shipped_robot_dna_round_trips_through_text_export() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Installer/bots");
    let mut files = Vec::new();
    collect_robot_files(&root, &mut files);
    files.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    assert!(files.len() >= 40, "expected the installer robot library");

    let mut failures = Vec::new();
    for path in files {
        let result = (|| -> Result<(), String> {
            let source = fs::read_to_string(&path).map_err(|error| format!("read failed: {error}"))?;
            let original =
                LegacyDna::parse(&source).map_err(|error| format!("parse failed: {error}"))?;
            let exported = original.to_source();
            let reparsed = LegacyDna::parse(&exported)
                .map_err(|error| format!("reparse failed: {error}"))?;

            if original.blocks() != reparsed.blocks() {
                return Err("executable blocks changed after export".to_owned());
            }
            if original.uses_new_move() != reparsed.uses_new_move() {
                return Err("NewMove program mode changed after export".to_owned());
            }
            Ok(())
        })();
        if let Err(error) = result {
            failures.push(format!("{}: {error}", path.display()));
        }
    }

    assert!(
        failures.is_empty(),
        "legacy imports failed:\n{}",
        failures.join("\n")
    );
}

fn collect_robot_files(directory: &Path, output: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            collect_robot_files(&path, output);
        } else if path.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("txt") || extension.eq_ignore_ascii_case("dbo")
        }) {
            output.push(path);
        }
    }
}
