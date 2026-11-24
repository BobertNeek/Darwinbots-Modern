//! Original OptionsForm AddItem appearance and AssignSkin.
//!
//! Both entry points use raw Single Timer seconds. Staging uses the caller's
//! uniform stream, while the league's skin-only operation owns a private stream.

use crate::EngineError;
use crate::dna::{DnaBlock, native_load_image};
use crate::native_random::NativeRandom;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct StagedSpeciesAppearance {
    pub argb: u32,
    pub skin: [i16; 8],
}

/// Prepare AddItem's appearance from the original basename bytes and DNA text.
/// Validation precedes all draws. Only the supplied uniform stream is advanced.
pub(crate) fn assign_staged_species_appearance(
    random: &mut NativeRandom,
    filename: &[u8],
    source_text: &str,
    use_old_colors: bool,
    color_timer_seconds: f32,
    skin_timer_seconds: f32,
) -> Result<StagedSpeciesAppearance, EngineError> {
    validate_timer(color_timer_seconds, "native color")?;
    validate_timer(skin_timer_seconds, "native skin")?;
    let (name, dna) = skin_source(filename, source_text)?;
    let argb = add_item_color(random, filename, use_old_colors, color_timer_seconds);
    let skin = assign_skin_from_image(random, &name, dna, skin_timer_seconds);
    Ok(StagedSpeciesAppearance { argb, skin })
}

pub(crate) fn assign_skin(
    alias: &str,
    source_text: &str,
    timer_seconds: f32,
) -> Result<[i16; 8], EngineError> {
    validate_timer(timer_seconds, "native skin")?;
    if !alias.is_ascii() {
        return Err(EngineError::Invariant(
            "native skin aliases must contain a nonempty ASCII name".into(),
        ));
    }
    let (name, dna) = skin_source(alias.as_bytes(), source_text)?;
    Ok(assign_skin_from_image(
        &mut NativeRandom::default(),
        &name,
        dna,
        timer_seconds,
    ))
}

fn validate_timer(seconds: f32, operation: &str) -> Result<(), EngineError> {
    if !(0.0..86400.0).contains(&seconds) {
        return Err(EngineError::Invariant(format!(
            "{operation} Timer must be finite and within [0, 86400) seconds"
        )));
    }
    Ok(())
}

fn skin_source(
    filename: &[u8],
    source_text: &str,
) -> Result<(Vec<u8>, Vec<DnaBlock>), EngineError> {
    // Replace removes every literal ".txt" before VB Asc reads the name's bytes.
    let extension = b".txt";
    let mut name = Vec::with_capacity(filename.len());
    let mut remainder = filename;
    while let Some(index) = remainder
        .windows(extension.len())
        .position(|bytes| bytes == extension)
    {
        name.extend_from_slice(&remainder[..index]);
        remainder = &remainder[index + extension.len()..];
    }
    name.extend_from_slice(remainder);
    if name.is_empty() {
        return Err(EngineError::Invariant(
            "native skin aliases must contain a nonempty name".into(),
        ));
    }
    // Original LoadDNA failure can display a modal before returning False.
    // Reject unsupported malformed raw inputs atomically instead of silently
    // pretending that modal workflow completed with an alias-only profile.
    Ok((name, native_load_image(source_text)?))
}

fn add_item_color(
    random: &mut NativeRandom,
    filename: &[u8],
    use_old_colors: bool,
    timer_seconds: f32,
) -> u32 {
    fn channel(draw: f32, scale: f64, offset: f64) -> u32 {
        // DARWIN.EXE passes the widened x87 expression to VB's __vbaFpI2.
        (f64::from(draw) * scale + offset).round_ties_even() as u32
    }

    fn argb(red: u32, green: u32, blue: u32) -> u32 {
        0xff00_0000 | (red << 16) | (green << 8) | blue
    }

    random.randomize_timer(timer_seconds);
    // AddItem's IIf evaluates both RGB arms in order even for a named override.
    let old_color = argb(
        channel(random.unit(), 200.0, 55.0),
        channel(random.unit(), 200.0, 55.0),
        channel(random.unit(), 255.0, 0.0),
    );
    let full_color = argb(
        channel(random.unit(), 255.0, 0.0),
        channel(random.unit(), 255.0, 0.0),
        channel(random.unit(), 255.0, 0.0),
    );
    match filename {
        b"Base.txt" => 0xff00_00ff,
        b"Mutate.txt" | b"Test.txt" => 0xffff_0000,
        b"robotA.txt" => 0xffff_8000,
        b"robotB.txt" => 0xff00_80ff,
        _ if use_old_colors => old_color,
        _ => full_color,
    }
}

fn assign_skin_from_image(
    random: &mut NativeRandom,
    name: &[u8],
    dna: Vec<DnaBlock>,
    timer_seconds: f32,
) -> [i16; 8] {
    random.randomize(0.0);
    let name_draws: Vec<_> = name
        .iter()
        .map(|&character| f64::from(random.rnd(-f32::from(character))))
        .collect();
    let mut next = 0.0_f64;
    for draw in name_draws {
        next = folded_draw(random, next, draw);
    }
    let name_draw = next;

    next = 0.0;
    // This is the SAME random stream: Randomize(0) preserves its low byte.
    random.randomize(0.0);
    let mut dna_draws = Vec::with_capacity(dna.len());
    for block in dna {
        // DARWIN.EXE loads value at 0x71a6b7 before tipo at 0x71a6f3.
        let value_draw = f64::from(random.rnd(-(block.value as f32)));
        let type_draw = f64::from(random.rnd(-(block.class as u8 as f32)));
        dna_draws.push(folded_draw(random, value_draw, type_draw));
    }
    for draw in dna_draws {
        next = folded_draw(random, next, draw);
    }

    random.randomize(next * 1000.0);
    let mut skin = [0_i16; 8];
    for index in (0..8).step_by(2) {
        skin[index] = scale_skin_draw(random.unit(), 61);
        if index == 4 {
            random.randomize(name_draw * 1000.0);
        }
        skin[index + 1] = scale_skin_draw(random.unit(), 629);
    }
    random.randomize_timer(timer_seconds);
    let radius = scale_skin_draw(random.unit(), 61);
    skin[6] = (f32::from(skin[6] + radius * 2) / 3.0_f32).round_ties_even() as i16;
    skin
}

fn scale_skin_draw(draw: f32, scale: u16) -> i16 {
    // AssignSkin passes the x87 product directly to VB's Int-to-Integer helper.
    (f64::from(draw) * f64::from(scale)).floor() as i16
}

fn folded_draw(random: &mut NativeRandom, x: f64, y: f64) -> f64 {
    f64::from(random.rnd(-source_angle((x - 0.5) as f32, (y - 0.5) as f32)))
}

fn source_angle(x: f32, y: f32) -> f32 {
    // Physics.angle(0,0,x,y): retain every Single narrowing and the original
    // vertical/negative-x branches. atan2 would change its angle range.
    let dx = x - 0.0_f32;
    let dy = 0.0_f32 - y;
    if dx == 0.0 {
        let angle = std::f32::consts::PI / 2.0_f32;
        if dy < 0.0 { angle * 3.0_f32 } else { angle }
    } else {
        let mut angle = f64::from(dy / dx).atan() as f32;
        if dx < 0.0 {
            angle += std::f32::consts::PI;
        }
        angle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn appearance_random() -> NativeRandom {
        serde_json::from_value(serde_json::json!({
            "state": 0x08_d82a,
            "gaussian_spare": 0.125,
        }))
        .unwrap()
    }

    #[test]
    fn original_add_item_uses_both_palettes_before_its_exact_filename_override() {
        // DARWIN.EXE 2.48.32, 0x717500–0x7176d2. The two RGB calls both run
        // before IIf. The last of the six Rnd states is 0x2a9cf8.
        let cases: &[(&[u8], bool, u32)] = &[
            (b"Palette.txt", true, 0xff45_ed29),
            (b"Palette.txt", false, 0xff0a_b72a),
            (b"Base.txt", true, 0xff00_00ff),
            (b"Mutate.txt", false, 0xffff_0000),
            (b"Test.txt", true, 0xffff_0000),
            (b"robotA.txt", true, 0xffff_8000),
            (b"robotB.txt", false, 0xff00_80ff),
            (b"base.txt", false, 0xff0a_b72a),
        ];
        for &(filename, use_old_colors, expected) in cases {
            let mut random = appearance_random();
            assert_eq!(
                add_item_color(&mut random, filename, use_old_colors, 1.25),
                expected
            );
            assert_eq!(random.state(), 0x2a_9cf8);
        }
    }

    #[test]
    fn original_add_item_appearance_uses_two_timers_and_one_uniform_stream() {
        let cases: &[(&[u8], u32, [i16; 8])] = &[
            (
                b"Test.txt",
                0xffff_0000,
                [36, 504, 10, 212, 26, 622, 16, 287],
            ),
            (
                b"Palette.txt",
                0xff45_ed29,
                [36, 504, 10, 212, 26, 4, 24, 444],
            ),
        ];
        for &(name, argb, skin) in cases {
            let mut random = appearance_random();
            let appearance = assign_staged_species_appearance(
                &mut random,
                name,
                "start stop",
                true,
                1.25,
                12_345.5,
            )
            .unwrap();
            assert_eq!(appearance, StagedSpeciesAppearance { argb, skin });
            assert_eq!(random.state(), 0x31_2200);
            assert_eq!(random.gaussian_spare(), Some(0.125));
        }
    }

    #[test]
    fn invalid_add_item_inputs_do_not_consume_the_uniform_stream() {
        let original = appearance_random();
        let cases: &[(&[u8], &str, f32, f32)] = &[
            (b"Test.txt", "start stop", f32::NAN, 0.0),
            (b"Test.txt", "start stop", 0.0, 86_400.0),
            (b".txt", "start stop", 0.0, 0.0),
            (b"Test.txt", "start 32768 stop", 0.0, 0.0),
        ];
        for &(name, source, color_timer, skin_timer) in cases {
            let mut random = original.clone();
            assert!(
                assign_staged_species_appearance(
                    &mut random,
                    name,
                    source,
                    false,
                    color_timer,
                    skin_timer
                )
                .is_err()
            );
            assert_eq!(random, original);
        }
    }

    #[test]
    fn original_skin_draws_floor_without_rounding_the_scaled_product_to_single() {
        // DARWIN.EXE 2.48.32, 0x71a99d and 0x71aa7d: FMUL is followed by
        // __vbaR8IntI2 without a Single spill. These exact Rnd states would
        // round the f32 products up to 19 and 31 before applying Int.
        for (state, scale, expected) in [(0x4f_bcda, 61, 18), (0x0c_9dea, 629, 30)] {
            let draw = state as f32 / 16_777_216.0;
            assert_eq!(scale_skin_draw(draw, scale), expected);
        }
    }

    #[test]
    fn original_source_skin_vectors_include_the_full_load_image_and_raw_single_timer() {
        // Independently calculated by output/native_skin_reference47.py from
        // literal source LoadDNA images, not by importing/running this port.
        // Authority: original revision467a8a0 OptionsForm3411-3472,
        // Physics607-624, and Microsoft's VBMath Rnd/Randomize equivalents.
        // The executable confirms the value-before-tipo evaluation order.
        let vectors = [
            (
                "Test.txt",
                "start stop",
                0x00000000,
                [36, 504, 10, 212, 26, 622, 11, 287],
            ),
            (
                "robotA.txt",
                "start stop",
                0x00000000,
                [36, 504, 10, 212, 26, 103, 7, 39],
            ),
            (
                "robotB.txt",
                "start stop",
                0x4640e600,
                [36, 504, 10, 212, 26, 124, 8, 517],
            ),
            (
                "Test.txt",
                "start stop",
                0x3f800000,
                [36, 504, 10, 212, 26, 622, 21, 287],
            ),
            (
                "Test.txt",
                "start stop",
                0x4640e600,
                [36, 504, 10, 212, 26, 622, 16, 287],
            ),
            (
                "Test.txt",
                "start end 7 .up stop end",
                0x00000000,
                [39, 516, 39, 224, 6, 407, 24, 143],
            ),
            (
                "Test.txt",
                "def Foo 7\r\nstart .Foo stop",
                0x00000000,
                [37, 387, 32, 613, 16, 390, 38, 415],
            ),
        ];
        for (alias, source, timer_bits, expected) in vectors {
            assert_eq!(
                assign_skin(alias, source, f32::from_bits(timer_bits)).unwrap(),
                expected,
                "{alias} timer={timer_bits:08x} source={source:?}"
            );
        }
    }

    #[test]
    fn timer_changes_only_slot_six_and_does_not_change_local_deterministic_output() {
        for alias in ["Test.txt", "robotA.txt", "robotB.txt"] {
            let initial = assign_skin(alias, "start stop", 0.0).unwrap();
            for timer in [1.0, 12345.5, 86399.5] {
                let actual = assign_skin(alias, "start stop", timer).unwrap();
                for index in [0, 1, 2, 3, 4, 5, 7] {
                    assert_eq!(actual[index], initial[index], "{alias} slot {index}");
                }
                for index in (0..8).step_by(2) {
                    assert!((0..=60).contains(&actual[index]));
                    assert!((0..=628).contains(&actual[index + 1]));
                }
                assert_eq!(assign_skin(alias, "start stop", timer).unwrap(), actual);
            }
        }
    }

    #[test]
    fn raw_dna_and_explicit_trailing_end_tokens_affect_the_profile_without_mutating_source() {
        let source = "start end 7 .up stop end".to_owned();
        let initial = assign_skin("Test.txt", &source, 0.0).unwrap();
        assert_eq!(source, "start end 7 .up stop end");
        assert_eq!(assign_skin("Test.txt", &source, 0.0).unwrap(), initial);
        let short = assign_skin("Test.txt", "start stop", 0.0).unwrap();
        assert_ne!(initial[..5], short[..5]);
        assert_eq!(assign_skin("Test", &source, 0.0).unwrap(), initial);
        assert_eq!(assign_skin("Test.txt.txt", &source, 0.0).unwrap(), initial);
    }

    #[test]
    fn invalid_clock_alias_or_native_integer_source_returns_an_error() {
        for timer in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0, 86400.0] {
            assert!(assign_skin("Test.txt", "start stop", timer).is_err());
        }
        for alias in ["", ".txt", "røbøt.txt"] {
            assert!(assign_skin(alias, "start stop", 0.0).is_err());
        }
        assert!(assign_skin("Test.txt", "start 32768 stop", 0.0).is_err());
    }

    #[test]
    fn source_angle_keeps_single_vertical_and_negative_x_branches() {
        assert_eq!(source_angle(0.0, 0.0).to_bits(), 0x3fc90fdb);
        assert_eq!(source_angle(0.0, 1.0).to_bits(), 0x4096cbe4);
        assert_eq!(source_angle(-1.0, 0.0).to_bits(), 0x40490fdb);
        assert_eq!(source_angle(1.0, 1.0).to_bits(), 0xbf490fdb);
    }
}
