//! Match desktop LegacyDnaText: explicit BOM, strict UTF-8, then CP1252.
//! BOM-less bytes valid as UTF-8 are interpreted as UTF-8; encoding intent
//! cannot be recovered from an ambiguous byte sequence. Preserve CR/LF text.
use darwinbots_engine::{Engine, LegacyDna, StandardMutationProfile};
use std::{
    error::Error,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

pub(crate) struct BotSource {
    pub dna: LegacyDna,
    pub source: String,
    pub name: String,
    pub mutation_profile: Option<StandardMutationProfile>,
}

pub(crate) fn read(engine: &Engine, path: &Path) -> Result<BotSource, Box<dyn Error>> {
    let source = decode(&fs::read(path)?)?;
    let dna = engine.load_dna_source(&source)?;
    let sidecar = resolve_sidecar(path)?;
    if sidecar.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{}: mutation-rate sidecar is a directory",
                sidecar.display()
            ),
        )
        .into());
    }
    let mutation_profile = match fs::read(&sidecar) {
        Ok(bytes) => Some(
            StandardMutationProfile::from_mrate(&decode(&bytes)?)
                .map_err(|error| format!("{}: {error}", sidecar.display()))?,
        ),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    Ok(BotSource {
        dna,
        source,
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        mutation_profile,
    })
}

// Match Windows-style case-insensitive adjacent names on case-sensitive hosts.
// A single Unicode uppercase scalar covers CP1252 names (Café/CAFÉ) without
// expanding names such as Straße to STRASSE. This is a simple ordinal policy,
// not an attempt to reproduce every filesystem's exotic Unicode collation.
fn filename_eq_ignore_case(left: &OsStr, right: &OsStr) -> bool {
    fn uppercase(character: char) -> char {
        let mut upper = character.to_uppercase();
        match (upper.next(), upper.next()) {
            (Some(character), None) => character,
            _ => character,
        }
    }
    match (left.to_str(), right.to_str()) {
        (Some(left), Some(right)) => left.chars().map(uppercase).eq(right.chars().map(uppercase)),
        _ => left == right,
    }
}

fn sidecar_directory(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

fn resolve_sidecar(dna: &Path) -> io::Result<PathBuf> {
    let expected = dna.with_extension("mrate");
    let name = expected
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "DNA path has no filename"))?;
    if dna
        .file_name()
        .is_some_and(|dna_name| filename_eq_ignore_case(dna_name, name))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "DNA and mutation-rate sidecar must have distinct filenames",
        ));
    }
    let directory = sidecar_directory(&expected);
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(expected),
        Err(error) => return Err(error),
    };
    let mut found = None;
    for entry in entries {
        let entry = entry?;
        if filename_eq_ignore_case(&entry.file_name(), name) {
            if found.is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{}: ambiguous mutation-rate sidecars differing only by case",
                        expected.display()
                    ),
                ));
            }
            found = Some(entry.path());
        }
    }
    Ok(found.unwrap_or(expected))
}

fn invalid_unicode() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid Unicode DNA file")
}

fn decode(bytes: &[u8]) -> Result<String, Box<dyn Error>> {
    if let Some(bytes) = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        return Ok(std::str::from_utf8(bytes)?.into());
    }
    for (bom, little) in [
        (&[0xff, 0xfe, 0x00, 0x00][..], true),
        (&[0x00, 0x00, 0xfe, 0xff][..], false),
    ] {
        if let Some(bytes) = bytes.strip_prefix(bom) {
            if !bytes.len().is_multiple_of(4) {
                return Err(invalid_unicode().into());
            }
            return bytes
                .chunks_exact(4)
                .map(|part| {
                    let value = if little {
                        u32::from_le_bytes(part.try_into().unwrap())
                    } else {
                        u32::from_be_bytes(part.try_into().unwrap())
                    };
                    char::from_u32(value).ok_or_else(|| invalid_unicode().into())
                })
                .collect();
        }
    }
    for (bom, little) in [(&[0xff, 0xfe][..], true), (&[0xfe, 0xff][..], false)] {
        if let Some(bytes) = bytes.strip_prefix(bom) {
            if !bytes.len().is_multiple_of(2) {
                return Err(invalid_unicode().into());
            }
            let words: Vec<_> = bytes
                .chunks_exact(2)
                .map(|part| {
                    if little {
                        u16::from_le_bytes(part.try_into().unwrap())
                    } else {
                        u16::from_be_bytes(part.try_into().unwrap())
                    }
                })
                .collect();
            return Ok(String::from_utf16(&words)?);
        }
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok(text.into());
    }
    const C1: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž',
        '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}',
        'ž', 'Ÿ',
    ];
    Ok(bytes
        .iter()
        .map(|&byte| {
            if (128..160).contains(&byte) {
                C1[(byte - 128) as usize]
            } else {
                char::from(byte)
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use darwinbots_engine::{EngineConfig, SaveFile, SpeciesId, TokenClass};
    #[test]
    fn adjacent_sidecar_uses_windows_case_for_cp1252_names_and_rejects_directories() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Café.txt");
        let sidecar = directory.path().join("CAFÉ.MrAtE");
        let source = "' named DNA source\r\nstart 7 900 store stop\r\n";
        fs::write(&path, source).unwrap();
        let profile = StandardMutationProfile {
            point_value_change_percent: 17,
            ..Default::default()
        };
        fs::write(&sidecar, profile.to_mrate().unwrap()).unwrap();
        let mut engine = Engine::new_native(EngineConfig::testing()).unwrap();
        let original = SaveFile::encode(&engine).unwrap();
        let bot = read(&engine, &path).unwrap();
        assert_eq!(bot.name, "Café.txt");
        assert_eq!(bot.source, source);
        assert_eq!(bot.mutation_profile.as_ref(), Some(&profile));
        assert_eq!(SaveFile::encode(&engine).unwrap(), original);
        crate::populate(&mut engine, &bot, 2).unwrap();
        for robot in &engine.snapshot().organisms {
            assert_eq!(engine.mutation_profile(robot.id).unwrap(), &profile);
            assert_eq!(
                engine.export_species_template_dna(robot.species).unwrap(),
                source
            );
        }
        let published = SaveFile::encode(&engine).unwrap();
        fs::remove_file(&sidecar).unwrap();
        fs::create_dir(&sidecar).unwrap();
        let error = read(&engine, &path).err().unwrap();
        assert!(error.to_string().contains("directory"));
        assert_eq!(SaveFile::encode(&engine).unwrap(), published);
        fs::remove_dir(&sidecar).unwrap();
        assert!(read(&engine, &path).unwrap().mutation_profile.is_none());
        // The user-named DNA path itself is never rediscovered by case.
        #[cfg(not(windows))]
        assert!(read(&engine, &directory.path().join("CAFÉ.TXT")).is_err());
        assert_eq!(sidecar_directory(Path::new("bot.mrate")), Path::new("."));
        assert_eq!(
            sidecar_directory(Path::new("nested/bot.mrate")),
            Path::new("nested")
        );
        assert!(filename_eq_ignore_case(
            OsStr::new("Café.mrate"),
            OsStr::new("CAFÉ.MRATE")
        ));
        assert!(!filename_eq_ignore_case(
            OsStr::new("Straße.mrate"),
            OsStr::new("STRASSE.MRATE")
        ));
        let same_path = directory.path().join("source.MRATE");
        fs::write(&same_path, "start stop").unwrap();
        assert!(
            read(&engine, &same_path)
                .err()
                .unwrap()
                .to_string()
                .contains("distinct")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn adjacent_sidecar_rejects_case_ambiguity_even_when_exact_spelling_exists() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mixed.txt");
        let exact = path.with_extension("mrate");
        let other = directory.path().join("MIXED.MRATE");
        fs::write(&path, "start stop").unwrap();
        let first = StandardMutationProfile {
            point_value_change_percent: 13,
            ..Default::default()
        };
        let second = StandardMutationProfile {
            point_value_change_percent: 27,
            ..Default::default()
        };
        fs::write(&exact, first.to_mrate().unwrap()).unwrap();
        fs::write(&other, second.to_mrate().unwrap()).unwrap();
        let engine = Engine::new_native(EngineConfig::testing()).unwrap();
        let original = SaveFile::encode(&engine).unwrap();
        let error = read(&engine, &path).err().unwrap();
        assert!(error.to_string().contains("ambiguous"));
        assert_eq!(SaveFile::encode(&engine).unwrap(), original);
        fs::remove_file(exact).unwrap();
        assert_eq!(
            read(&engine, &path).unwrap().mutation_profile.as_ref(),
            Some(&second)
        );
    }

    #[test]
    fn original_ansi_and_strict_bom_decoding_preserve_source_without_guessing() {
        assert_eq!(decode(&[0x27, 0xe9, 0x80, 13, 10]).unwrap(), "'é€\r\n");
        for bytes in [
            vec![0xc3, 0xa9],
            vec![0xef, 0xbb, 0xbf, 0xc3, 0xa9],
            vec![0xff, 0xfe, 0xe9, 0],
            vec![0xfe, 0xff, 0, 0xe9],
            vec![0xff, 0xfe, 0, 0, 0xe9, 0, 0, 0],
            vec![0, 0, 0xfe, 0xff, 0, 0, 0, 0xe9],
        ] {
            assert_eq!(decode(&bytes).unwrap(), "é");
        }
        for bytes in [
            vec![0xef, 0xbb, 0xbf, 0xe9],
            vec![0xff, 0xfe, 0],
            vec![0xff, 0xfe, 0, 0, 0],
            vec![0, 0, 0xfe, 0xff, 0, 0x11, 0, 0],
        ] {
            assert!(decode(&bytes).is_err());
        }
    }
    #[test]
    fn actual_file_loader_uses_engine_native_boundary_and_retains_exact_ansi_input() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("original.txt");
        let mut bytes = b"'".to_vec();
        bytes.extend([0xe9, 0x80]);
        bytes.extend_from_slice(b"\r\ndef Foo 7\r\ndef foo 9\r\n11 start .Foo .foo 1 999 += stop end 123\r\nNewMove\r\n");
        fs::write(&path, &bytes).unwrap();
        let mut native = Engine::new_native(EngineConfig::testing()).unwrap();
        let BotSource { dna, source, .. } = read(&native, &path).unwrap();
        assert!(source.starts_with("'é€\r\n"));
        assert_eq!(
            dna.blocks()
                .iter()
                .map(|b| (b.class as u8, b.value))
                .collect::<Vec<_>>(),
            vec![
                (9, 2),
                (0, 7),
                (0, 9),
                (0, 1),
                (0, 999),
                (0, 0),
                (9, 4),
                (10, 1)
            ]
        );
        assert!(!dna.uses_new_move());
        assert_eq!(dna.blocks()[0].class, TokenClass::Flow);
        assert_eq!(dna.blocks()[1].value, 7);
        assert_eq!(dna.blocks()[2].value, 9);
        native
            .spawn_species_batch(dna, SpeciesId::default(), [[500.0; 2]], 1000)
            .unwrap();
        native
            .retain_species_template_source(SpeciesId::default(), source.clone())
            .unwrap();
        let loaded = SaveFile::decode(&SaveFile::encode(&native).unwrap()).unwrap();
        assert_eq!(
            loaded
                .export_species_template_dna(SpeciesId::default())
                .unwrap(),
            source
        );
        // The bare constructor also uses the native stream. Only an explicit
        // legacy save state selects the preserved convenience parser.
        let mut legacy_state =
            serde_json::to_value(Engine::new(EngineConfig::testing()).unwrap()).unwrap();
        legacy_state["native_random"] = serde_json::Value::Null;
        let legacy: Engine = serde_json::from_value(legacy_state).unwrap();
        let BotSource {
            dna,
            source: retained,
            ..
        } = read(&legacy, &path).unwrap();
        assert_eq!(retained, source);
        assert_eq!(
            dna.blocks()
                .iter()
                .map(|b| (b.class as u8, b.value))
                .collect::<Vec<_>>(),
            vec![
                (0, 11),
                (9, 2),
                (0, 9),
                (0, 9),
                (0, 1),
                (0, 999),
                (7, 4),
                (9, 4),
                (10, 1)
            ]
        );
        assert!(dna.uses_new_move());
        assert_eq!(dna.blocks()[0].value, 11);
        for command in ["run", "bench", "stress"] {
            let mut arguments = vec![
                "darwinbots".to_owned(),
                command.into(),
                "--bot".into(),
                path.to_string_lossy().into_owned(),
                "--population".into(),
                "1".into(),
                "--ticks".into(),
                "0".into(),
                "--backend".into(),
                "cpu".into(),
            ];
            if command == "bench" {
                arguments.extend(["--warmup-ticks".into(), "0".into()]);
            }
            let mut output = Vec::new();
            crate::run_from(arguments, &mut output).unwrap();
            let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
            assert_eq!(report["population"], 1);
            assert_eq!(report["backend"], "Cpu");
        }
    }
}
