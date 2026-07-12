use darwinbots_engine::LegacyDna;
use std::{fs, path::{Path, PathBuf}};

#[test]
fn imports_every_robot_shipped_with_the_legacy_distribution() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Installer/bots");
    let mut files = Vec::new();
    collect_robot_files(&root, &mut files);
    assert!(files.len() >= 40, "expected the installer robot library");

    let mut failures = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path).unwrap();
        if let Err(error) = LegacyDna::parse(&source) {
            failures.push(format!("{}: {error}", path.display()));
        }
    }

    assert!(failures.is_empty(), "legacy imports failed:\n{}", failures.join("\n"));
}

fn collect_robot_files(directory: &Path, output: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            collect_robot_files(&path, output);
        } else if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("txt")) {
            output.push(path);
        }
    }
}
