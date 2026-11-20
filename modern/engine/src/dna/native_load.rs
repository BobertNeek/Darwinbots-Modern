//! Original LoadDNA images for AssignSkin and Classic actor startup.
//!
//! This deliberately does not change LegacyDna's executable/semantic image.
//! AssignSkin loads robot zero: SysvarTok's `n > 0` branch never resolves its
//! private definitions. Real actors resolve them as each line is loaded and
//! expose cells 1..DnaLen to the existing VM/native-record representation.

use super::{
    DnaBlock, DnaMetadata, EngineError, LegacyDna, MAX_DNA_BLOCKS, TokenClass, block_token,
    parse_error, parse_metadata, token_block, validate_metadata,
};

struct LoadedSource {
    blocks: Vec<DnaBlock>,
    variables: Vec<(String, i32)>,
}

pub(crate) fn native_load_image(source: &str) -> Result<Vec<DnaBlock>, EngineError> {
    Ok(load_source(source, false)?.blocks)
}

pub(crate) fn native_load_robot_dna(source: &str) -> Result<LegacyDna, EngineError> {
    let loaded = load_source(source, true)?;
    // DNA.bas starts at a = 1. SaveRobotBody writes the same cells 1..DnaLen,
    // stopping at the first actual master end; it never saves cell zero or
    // the source tail used by AssignSkin. Keep the existing coding/file view.
    let end = loaded
        .blocks
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, block)| {
            (block.class == TokenClass::End && block.value == 1).then_some(index)
        })
        .ok_or_else(|| parse_error(1, "native actor DNA has no cell-one master-end image"))?;
    let blocks = loaded.blocks[1..=end].to_vec();
    if blocks.len() > MAX_DNA_BLOCKS {
        return Err(parse_error(
            1,
            "native actor DNA exceeds the supported record length",
        ));
    }
    let (metadata, new_move) = metadata_context(source)?;
    let mut dna = LegacyDna::from_legacy_blocks(blocks);
    dna.set_native_context(loaded.variables, new_move, metadata);
    Ok(dna)
}

fn load_source(source: &str, resolve_private_variables: bool) -> Result<LoadedSource, EngineError> {
    let mut blocks = vec![number(0)];
    let mut variables = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        // VB Line Input removes the line terminator; Replace(vbTab, " ")
        // and Trim remove ASCII spaces only. A BOM is not a lexer directive.
        let line = line.replace('\t', " ");
        let line = match line.find('\'') {
            Some(position) if position > 0 => &line[..position],
            _ => &line,
        };
        let line = line.trim_matches(' ');
        if line.is_empty() || line.starts_with('\'') || line.starts_with('/') {
            continue;
        }
        // Source compares Left(a, 3) with lowercase "def", without LCase.
        if line.starts_with("def") {
            let definition = line
                .get(4..)
                .ok_or_else(|| parse_error(line_number, "invalid native private definition"))?;
            let (name, value) = definition
                .split_once(' ')
                .ok_or_else(|| parse_error(line_number, "invalid native private definition"))?;
            let value = native_integer(value, line_number)?;
            variables.push((name.to_owned(), value));
            // Scratch rob(0).vnum starts at zero; real robots start at one.
            let capacity = if resolve_private_variables {
                1000
            } else {
                1001
            };
            if variables.len() > capacity {
                return Err(parse_error(
                    line_number,
                    "native private-variable table overflow",
                ));
            }
            continue;
        }
        for token in line.split(' ').filter(|token| !token.is_empty()) {
            blocks.push(native_token(
                token,
                line_number,
                if resolve_private_variables {
                    Some(&variables)
                } else {
                    None
                },
            )?);
        }
    }
    blocks.push(DnaBlock {
        class: TokenClass::End,
        value: 1,
    });
    if !variables.is_empty() && blocks[1].class != TokenClass::Flow {
        blocks.remove(0);
    }
    Ok(LoadedSource { blocks, variables })
}

fn metadata_context(source: &str) -> Result<(DnaMetadata, bool), EngineError> {
    // Share the established metadata/hash reader without running LegacyDna's
    // convenience lexer or global private-var pass. Original LoadDNA never
    // calls the commented-out interpretUSE path; fresh actors keep NewMove off.
    let mut metadata = DnaMetadata::default();
    let mut saw_metadata = false;
    let mut hash_validation = None;
    let mut prefix = String::new();
    for (index, segment) in source.split_inclusive('\n').enumerate() {
        let logical = segment.strip_suffix('\n').unwrap_or(segment);
        let logical = logical.strip_suffix('\r').unwrap_or(logical);
        let raw = logical.trim_start_matches('\u{feff}');
        let (is_metadata, validation) = parse_metadata(raw, index + 1, &prefix, &mut metadata)?;
        if is_metadata {
            saw_metadata = true;
            if validation.is_some() {
                hash_validation = validation;
            }
        }
        prefix.push_str(logical);
        prefix.push_str("\r\n");
    }
    validate_metadata(&mut metadata, saw_metadata, hash_validation);
    Ok((metadata, false))
}

fn number(value: i32) -> DnaBlock {
    DnaBlock {
        class: TokenClass::Number,
        value,
    }
}

fn native_token(
    token: &str,
    line: usize,
    private_variables: Option<&[(String, i32)]>,
) -> Result<DnaBlock, EngineError> {
    if let Some(block) = token_block(token) {
        // The modern parser also accepts convenience aliases that the VB6
        // source does not. Reuse only original commands and its five aliases.
        if block_token(block).is_some_and(|name| token.eq_ignore_ascii_case(name))
            || ["dupint", "dropint", "clearint", "swapint", "overint"]
                .iter()
                .any(|name| token.eq_ignore_ascii_case(name))
        {
            return Ok(block);
        }
    }
    let (class, value) = token
        .strip_prefix('*')
        .map_or((TokenClass::Number, token), |value| {
            (TokenClass::Dereference, value)
        });
    let value = if let Some(name) = value.strip_prefix('.') {
        // System names compare without case; real-robot private names compare
        // exactly and later duplicate definitions replace earlier matches.
        // For scratch n = 0 this branch has no private-variable slice.
        if name.is_empty() {
            0
        } else {
            private_variables
                .and_then(|variables| {
                    variables
                        .iter()
                        .rev()
                        .find(|(candidate, _)| candidate.as_str() == name)
                })
                .map_or_else(
                    || crate::sysvar_address(name).unwrap_or(0),
                    |(_, value)| *value,
                )
        }
    } else {
        native_integer(value, line)?
    };
    Ok(DnaBlock { class, value })
}

fn native_integer(text: &str, line: usize) -> Result<i32, EngineError> {
    let compact: String = text
        .chars()
        .filter(|character| !matches!(character, ' ' | '\t' | '\r' | '\n'))
        .collect();
    let (sign, unsigned) = match compact.as_bytes().first() {
        Some(b'-') => (-1.0, &compact[1..]),
        Some(b'+') => (1.0, &compact[1..]),
        _ => (1.0, compact.as_str()),
    };
    let value = if unsigned.starts_with("&H") || unsigned.starts_with("&h") {
        radix_value(&unsigned[2..], 16, line)? * sign
    } else if unsigned.starts_with("&O") || unsigned.starts_with("&o") {
        radix_value(&unsigned[2..], 8, line)? * sign
    } else {
        decimal_value(unsigned, line)? * sign
    };
    let rounded = value.round_ties_even();
    if !rounded.is_finite() || !(-32768.0..=32767.0).contains(&rounded) {
        return Err(parse_error(line, "native DNA Integer overflow"));
    }
    Ok(rounded as i32)
}

fn radix_value(text: &str, radix: u32, line: usize) -> Result<f64, EngineError> {
    let end = text
        .char_indices()
        .find_map(|(index, character)| (!character.is_digit(radix)).then_some(index))
        .unwrap_or(text.len());
    if end == 0 {
        return Ok(0.0);
    }
    let value = u32::from_str_radix(&text[..end], radix)
        .map_err(|_| parse_error(line, "native numeric literal overflow"))?;
    let explicitly_long = text[end..].starts_with('&');
    let value = if value <= u16::MAX as u32 && !explicitly_long {
        f64::from(value as u16 as i16)
    } else {
        f64::from(value as i32)
    };
    Ok(value)
}

fn decimal_value(text: &str, line: usize) -> Result<f64, EngineError> {
    let bytes = text.as_bytes();
    let mut end = 0;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    if bytes.get(end) == Some(&b'.') {
        end += 1;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
    }
    if end == 0 || (end == 1 && bytes[0] == b'.') {
        return Ok(0.0);
    }
    if bytes
        .get(end)
        .is_some_and(|character| matches!(character, b'e' | b'E' | b'd' | b'D'))
    {
        let mut exponent_end = end + 1;
        if bytes
            .get(exponent_end)
            .is_some_and(|character| matches!(character, b'+' | b'-'))
        {
            exponent_end += 1;
        }
        let exponent_start = exponent_end;
        while bytes.get(exponent_end).is_some_and(u8::is_ascii_digit) {
            exponent_end += 1;
        }
        if exponent_end > exponent_start {
            end = exponent_end;
        }
    }
    let literal = text[..end].replace(['d', 'D'], "e");
    let mut value = literal
        .parse::<f64>()
        .map_err(|_| parse_error(line, "invalid native numeric literal"))?;
    match bytes.get(end) {
        Some(b'%' | b'&') if value.fract() != 0.0 => {
            return Err(parse_error(line, "native numeric type suffix mismatch"));
        }
        Some(b'!') => value = f64::from(value as f32),
        Some(b'@') => value = (value * 10000.0).round_ties_even() / 10000.0,
        Some(b'$') => {
            return Err(parse_error(line, "native numeric type suffix mismatch"));
        }
        _ => {}
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LegacyDna;

    fn words(source: &str) -> Vec<(u8, i32)> {
        native_load_image(source)
            .unwrap()
            .into_iter()
            .map(|block| (block.class as u8, block.value))
            .collect()
    }

    #[test]
    fn raw_load_keeps_dummy_zero_appended_end_and_explicit_end_trailing_tokens() {
        assert_eq!(words(""), [(0, 0), (10, 1)]);
        assert_eq!(words("' comment\r\n / comment\r\n"), [(0, 0), (10, 1)]);
        assert_eq!(words("start stop"), [(0, 0), (9, 2), (9, 4), (10, 1)]);
        assert_eq!(
            words("start end 7 .up stop end"),
            [
                (0, 0),
                (9, 2),
                (10, 1),
                (0, 7),
                (0, 1),
                (9, 4),
                (10, 1),
                (10, 1)
            ]
        );
    }

    #[test]
    fn scratch_zero_definitions_only_change_leading_zero_and_never_bind_private_names() {
        assert_eq!(
            words("def Foo 7\r\ndef Foo 8\r\ndef foo 9\r\ndef up 99\r\nstart .Foo .foo .up stop"),
            [(0, 0), (9, 2), (0, 0), (0, 0), (0, 1), (9, 4), (10, 1)]
        );
        assert_eq!(words("def Foo 7\n.Foo *.Foo"), [(0, 0), (1, 0), (10, 1)]);
        assert_eq!(words("def Foo 7\n2"), [(0, 2), (10, 1)]);
        assert_eq!(words("def Foo 7"), [(10, 1)]);
        assert_eq!(
            words("DEF Foo 7\n2"),
            [(0, 0), (0, 0), (0, 0), (0, 7), (0, 2), (10, 1)]
        );
    }

    #[test]
    fn native_lexer_keeps_source_tokens_instead_of_modern_convenience_expansions() {
        assert_eq!(
            words("\u{feff}start\t10.up .eye5>0 (2) `3 NewMove sqrt dupint stopd\t*1 ' ignored"),
            [
                (0, 0),
                (0, 0),
                (0, 10),
                (0, 0),
                (0, 0),
                (0, 0),
                (0, 0),
                (0, 0),
                (2, 10),
                (0, 0),
                (1, 1),
                (10, 1)
            ]
        );
        assert_eq!(
            words("START *.UP * END"),
            [(0, 0), (9, 2), (1, 1), (2, 6), (10, 1), (10, 1)]
        );
    }

    #[test]
    fn native_numbers_use_val_prefixes_integer_width_and_even_rounding() {
        assert_eq!(
            words("2.5 3.5 -2.5 -3.5 12tail 1D2 1e &HFFFF &O177777 32767 -32768 2.50001@"),
            [
                (0, 0),
                (0, 2),
                (0, 4),
                (0, -2),
                (0, -4),
                (0, 12),
                (0, 100),
                (0, 1),
                (0, -1),
                (0, -1),
                (0, 32767),
                (0, -32768),
                (0, 2),
                (10, 1)
            ]
        );
        for source in [
            "32768",
            "-32769",
            "1e999",
            "50.5%",
            "&HFFFF&",
            "def Foo 40000",
            "def",
        ] {
            assert!(native_load_image(source).is_err(), "{source}");
        }
    }

    #[test]
    fn raw_load_keeps_the_complete_source_array_beyond_the_executable_genome_limit() {
        let count = crate::dna::MAX_DNA_BLOCKS + 3;
        let source = format!("end {}", "1 ".repeat(count));
        let image = native_load_image(&source).unwrap();
        assert_eq!(image.len(), count + 3);
        assert_eq!(image[0], number(0));
        assert_eq!(image[1].class, TokenClass::End);
        assert!(image[2..count + 2].iter().all(|block| *block == number(1)));
        assert_eq!(image.last().unwrap().class, TokenClass::End);
        assert_eq!(LegacyDna::parse(&source).unwrap().blocks().len(), 1);
        assert!(native_load_image(&"def Foo 7\n".repeat(1002)).is_err());
    }

    #[test]
    fn raw_skin_image_does_not_change_ordinary_legacy_dna_semantics_or_equality() {
        let source = "def Foo 7\r\nstart .Foo end 999 stop";
        let before = LegacyDna::parse(source).unwrap();
        let encoded = serde_json::to_vec(&before).unwrap();
        let raw = words(source);
        assert_eq!(
            raw,
            [(0, 0), (9, 2), (0, 0), (10, 1), (0, 999), (9, 4), (10, 1)]
        );
        assert_eq!(LegacyDna::parse(source).unwrap(), before);
        assert_eq!(serde_json::to_vec(&before).unwrap(), encoded);
        assert_eq!(before.blocks().len(), 3);
        assert_eq!(before.blocks()[1].value, 7);
    }

    #[test]
    fn real_actor_load_resolves_private_names_and_projects_only_cells_one_through_master_end() {
        let source = "def Foo 7\nstart .Foo *.Foo .FOO .up stop end 999";
        let actor = native_load_robot_dna(source).unwrap();
        let blocks: Vec<_> = actor
            .blocks()
            .iter()
            .map(|b| (b.class as u8, b.value))
            .collect();
        assert_eq!(
            blocks,
            [(9, 2), (0, 7), (1, 7), (0, 0), (0, 1), (9, 4), (10, 1)]
        );
        assert_eq!(actor.native_user_variables(), [("Foo".to_owned(), 7)]);
        let encoded = serde_json::to_vec(&actor).unwrap();
        assert_eq!(
            serde_json::from_slice::<LegacyDna>(&encoded).unwrap(),
            actor
        );
        assert!(
            !serde_json::from_slice::<serde_json::Value>(&encoded)
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("raw_load_blocks")
        );
    }

    #[test]
    fn real_actor_definitions_take_effect_in_source_order_with_case_sensitive_last_matches() {
        let source = "start .Foo stop\ndef Foo 7\nstart .Foo .foo stop\ndef foo 8\ndef Foo 9\nstart .Foo .foo .FOO stop";
        let actor = native_load_robot_dna(source).unwrap();
        let blocks: Vec<_> = actor
            .blocks()
            .iter()
            .map(|b| (b.class as u8, b.value))
            .collect();
        assert_eq!(
            blocks,
            [
                (9, 2),
                (0, 0),
                (9, 4),
                (9, 2),
                (0, 7),
                (0, 0),
                (9, 4),
                (9, 2),
                (0, 9),
                (0, 8),
                (0, 0),
                (9, 4),
                (10, 1),
            ]
        );
        assert_eq!(
            actor.native_user_variables(),
            [
                ("Foo".to_owned(), 7),
                ("foo".to_owned(), 8),
                ("Foo".to_owned(), 9),
            ]
        );
    }

    #[test]
    fn actor_projection_skips_cell_zero_after_the_original_def_correction() {
        let source = "def up 77\n999 start .up .UP stop end 123";
        let actor = native_load_robot_dna(source).unwrap();
        let blocks: Vec<_> = actor
            .blocks()
            .iter()
            .map(|b| (b.class as u8, b.value))
            .collect();
        assert_eq!(blocks, [(9, 2), (0, 77), (0, 1), (9, 4), (10, 1)]);
        assert_eq!(native_load_image(source).unwrap()[0], number(999));
        let ordinary = LegacyDna::parse(source).unwrap();
        assert_eq!(ordinary.blocks()[0], number(999));
        assert_eq!(ordinary.blocks()[3], number(77));
    }

    #[test]
    fn actor_load_retains_metadata_without_enabling_the_modern_newmove_directive() {
        let mut source_dna = LegacyDna::parse("NewMove\ndef Foo 7\nstart .Foo stop").unwrap();
        source_dna.synchronize_runtime_metadata(23, 41, "source tag");
        let source = source_dna.to_source();
        let ordinary = LegacyDna::parse(&source).unwrap();
        let actor = native_load_robot_dna(&source).unwrap();
        assert_eq!(actor.metadata(), ordinary.metadata());
        assert_eq!(
            actor.native_user_variables(),
            ordinary.native_user_variables()
        );
        assert!(!actor.uses_new_move());
        assert!(ordinary.uses_new_move());
        assert_eq!(actor.blocks(), ordinary.blocks());
        let before = serde_json::to_vec(&ordinary).unwrap();
        assert_eq!(
            serde_json::to_vec(&LegacyDna::parse(&source).unwrap()).unwrap(),
            before
        );
    }

    #[test]
    fn original_actor_newmove_words_are_zero_tokens_and_never_enable_the_flag() {
        let source = "NewMove\nuse NewMove\nstart stop";
        let actor = native_load_robot_dna(source).unwrap();
        assert!(!actor.uses_new_move());
        assert_eq!(
            actor
                .blocks()
                .iter()
                .map(|b| (b.class as u8, b.value))
                .collect::<Vec<_>>(),
            [(0, 0), (0, 0), (0, 0), (9, 2), (9, 4), (10, 1),]
        );
        assert!(LegacyDna::parse(source).unwrap().uses_new_move());
    }

    #[test]
    fn actor_load_rejects_an_unrepresentable_source_instead_of_truncating_or_changing_indexing() {
        assert!(native_load_robot_dna("def Foo 7").is_err());
        assert!(native_load_robot_dna(&"def Foo 7\n".repeat(1001)).is_err());
        let definitions = format!("{}start .Foo stop", "def Foo 7\n".repeat(1000));
        assert_eq!(
            native_load_robot_dna(&definitions)
                .unwrap()
                .native_user_variables()
                .len(),
            1000
        );
        let count = MAX_DNA_BLOCKS - 3;
        let maximum = format!("start {}stop", "1 ".repeat(count));
        assert_eq!(
            native_load_robot_dna(&maximum).unwrap().blocks().len(),
            MAX_DNA_BLOCKS
        );
        let oversized = format!("start {}stop", "1 ".repeat(count + 1));
        assert!(native_load_robot_dna(&oversized).is_err());
    }
}
