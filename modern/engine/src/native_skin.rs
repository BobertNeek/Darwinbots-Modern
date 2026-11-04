//! Original OptionsForm.AssignSkin with an isolated VB6 random stream.
//!
//! Clock-dependent slot six uses explicitly supplied raw Single Timer seconds.
//! No simulation RNG, Gaussian cache, DNA definition, or live actor is touched.

use crate::EngineError;
use crate::dna::native_load_image;
use crate::native_random::NativeRandom;

pub(crate) fn assign_skin(
    alias: &str,
    source_text: &str,
    timer_seconds: f32,
) -> Result<[i16; 8], EngineError> {
    if !timer_seconds.is_finite() || !(0.0..86400.0).contains(&timer_seconds) {
        return Err(EngineError::Invariant(
            "native skin Timer must be finite and within [0, 86400) seconds".into(),
        ));
    }
    let name = alias.replace(".txt", "");
    if name.is_empty() || !name.is_ascii() {
        return Err(EngineError::Invariant(
            "native skin aliases must contain a nonempty ASCII name".into(),
        ));
    }
    // Original LoadDNA failure can display a modal before returning False.
    // Reject unsupported malformed raw inputs atomically instead of silently
    // pretending that modal workflow completed with an alias-only profile.
    let dna = native_load_image(source_text)?;
    let mut random = NativeRandom::default();
    random.randomize(0.0);
    let name_draws: Vec<_> = name
        .bytes()
        .map(|character| f64::from(random.rnd(-f32::from(character))))
        .collect();
    let mut next = 0.0_f64;
    for draw in name_draws {
        next = folded_draw(&mut random, next, draw);
    }
    let name_draw = next;

    next = 0.0;
    // This is the SAME random stream: Randomize(0) preserves its low byte.
    random.randomize(0.0);
    let mut dna_draws = Vec::with_capacity(dna.len());
    for block in dna {
        // Spell out the nested source expression's value-before-tipo order.
        // This order remains subject to genuine original stable-skin capture;
        // a source-derived vector is not evidence of executable evaluation.
        let value_draw = f64::from(random.rnd(-(block.value as f32)));
        let type_draw = f64::from(random.rnd(-(block.class as u8 as f32)));
        dna_draws.push(folded_draw(&mut random, value_draw, type_draw));
    }
    for draw in dna_draws {
        next = folded_draw(&mut random, next, draw);
    }

    random.randomize(next * 1000.0);
    let mut skin = [0_i16; 8];
    for index in (0..8).step_by(2) {
        skin[index] = (random.unit() * 61.0_f32).floor() as i16;
        if index == 4 {
            random.randomize(name_draw * 1000.0);
        }
        skin[index + 1] = (random.unit() * 629.0_f32).floor() as i16;
    }
    random.randomize_timer(timer_seconds);
    let radius = (random.unit() * 61.0_f32).floor() as i16;
    skin[6] = (f32::from(skin[6] + radius * 2) / 3.0_f32).round_ties_even() as i16;
    Ok(skin)
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

    #[test]
    fn original_source_skin_vectors_include_the_full_load_image_and_raw_single_timer() {
        // Independently calculated by output/native_skin_reference47.py from
        // literal source LoadDNA images, not by importing/running this port.
        // Authority: original revision467a8a0 OptionsForm3411-3472,
        // Physics607-624, and Microsoft's VBMath Rnd/Randomize equivalents.
        // These pin the declared value-before-tipo interpretation. Original
        // executable evaluation order still needs a genuine stable-skin capture.
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
