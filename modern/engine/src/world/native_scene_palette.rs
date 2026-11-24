//! The seeded scene colors from main.frm and colors.bas.
use crate::native_random::NativeRandom;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeScenePalette {
    pub chart_argb: u32,
    pub arena_argb: u32,
}

impl NativeScenePalette {
    pub(super) fn draw(random: &mut NativeRandom) -> Self {
        let hue = (f64::from(random.unit()) * 240.0).floor() as u8;
        let saturation = 180.0 + (f64::from(random.unit()) * 60.0).floor();
        let lightness = 222.0 + (f64::from(random.unit()) * 2.0).floor() * 6.0;
        Self::from_hsl(hue, saturation, lightness)
    }

    pub(super) fn from_seed(seed: u64) -> Self {
        let mut random = NativeRandom::default();
        random.reseed_native_start(seed);
        Self::draw(&mut random)
    }

    fn from_hsl(hue: u8, saturation: f64, lightness: f64) -> Self {
        let hue_rgb = hue_rgb(hue);
        // The original assigns each 127.5 gray component to a VB Long.
        let saturated = mix(hue_rgb, [128; 3], 1.0 - saturation / 240.0);
        Self {
            chart_argb: argb(mix(saturated, [255; 3], (lightness - 120.0) / 120.0)),
            arena_argb: argb(mix([0; 3], saturated, (lightness - 195.0) / 120.0)),
        }
    }
}

fn hue_rgb(hue: u8) -> [u8; 3] {
    let delta = 255.0 / 40.0 * f64::from(hue % 40);
    let rgb = match hue / 40 {
        0 => [255.0, delta, 0.0],
        1 => [255.0 - delta, 255.0, 0.0],
        2 => [0.0, 255.0, delta],
        3 => [0.0, 255.0 - delta, 255.0],
        4 => [delta, 0.0, 255.0],
        _ => [255.0, 0.0, 255.0 - delta],
    };
    rgb.map(|channel| channel.round_ties_even() as u8)
}

fn mix(first: [u8; 3], second: [u8; 3], amount: f64) -> [u8; 3] {
    // mixrgb returns Long components before the next blend.
    std::array::from_fn(|i| {
        (f64::from(first[i]) * (1.0 - amount) + f64::from(second[i]) * amount).round_ties_even()
            as u8
    })
}

fn argb([red, green, blue]: [u8; 3]) -> u32 {
    u32::from_be_bytes([255, red, green, blue])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_scene_palette_keeps_long_rounding_between_original_color_blends() {
        // Independently evaluated channel values at the six hue boundaries,
        // including the Long midpoint on the hue and gray conversions.
        for (hue, saturation, lightness, chart_argb, arena_argb) in [
            (0, 240.0, 222.0, 0xffff_d9d9, 0xff39_0000),
            (20, 240.0, 228.0, 0xffff_f2e6, 0xff46_2300),
            (40, 180.0, 222.0, 0xfffa_fade, 0xff32_3207),
            (80, 180.0, 228.0, 0xffe9_fce9, 0xff09_3d09),
            (120, 180.0, 222.0, 0xffde_fafa, 0xff07_3232),
            (160, 180.0, 222.0, 0xffde_defa, 0xff07_0732),
            (200, 180.0, 222.0, 0xfffa_defa, 0xff32_0732),
            (239, 180.0, 228.0, 0xfffc_e9e9, 0xff3d_090a),
        ] {
            assert_eq!(
                NativeScenePalette::from_hsl(hue, saturation, lightness),
                NativeScenePalette {
                    chart_argb,
                    arena_argb
                },
            );
        }
    }
}
