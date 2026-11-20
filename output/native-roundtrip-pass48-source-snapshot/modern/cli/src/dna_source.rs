//! Match desktop LegacyDnaText: explicit BOM, strict UTF-8, then CP1252.
//! BOM-less bytes valid as UTF-8 are interpreted as UTF-8; encoding intent
//! cannot be recovered from an ambiguous byte sequence. Preserve CR/LF text.
use darwinbots_engine::{Engine, LegacyDna};
use std::{error::Error, fs, io, path::Path};

pub(crate) fn read(engine: &Engine, path: &Path) -> Result<(LegacyDna, String), Box<dyn Error>> {
    let source = decode(&fs::read(path)?)?;
    let dna = engine.load_dna_source(&source)?;
    Ok((dna, source))
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
        let (dna, source) = read(&native, &path).unwrap();
        assert!(source.starts_with("'é€\r\n"));
        assert_eq!(
            dna.blocks()
                .iter()
                .filter(|b| b.class == TokenClass::Store)
                .count(),
            0
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
        let modern = Engine::new(EngineConfig::testing()).unwrap();
        let (dna, retained) = read(&modern, &path).unwrap();
        assert_eq!(retained, source);
        assert_eq!(
            dna.blocks()
                .iter()
                .filter(|b| b.class == TokenClass::Store)
                .count(),
            1
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
