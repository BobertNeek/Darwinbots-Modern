use serde::{Deserialize, Serialize};

const MIN_SKIN_RADIUS: f32 = 0.15;
const MAX_SKIN_RADIUS: f32 = 0.82;
const AIM_UNITS_PER_TURN: i32 = 1257;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkinPoint {
    pub radius: f32,
    pub angle: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VisualPhenotype {
    pub lineage_id: u64,
    pub color: u32,
    pub skin: [SkinPoint; 4],
    pub accumulated_mutations: u32,
}

impl VisualPhenotype {
    pub fn new(lineage_id: u64, color: u32, skin: [SkinPoint; 4]) -> Self {
        Self {
            lineage_id,
            color,
            skin,
            accumulated_mutations: 0,
        }
    }

    pub fn rgb(&self) -> [u8; 3] {
        [
            ((self.color >> 16) & 0xff) as u8,
            ((self.color >> 8) & 0xff) as u8,
            (self.color & 0xff) as u8,
        ]
    }

    pub fn apply_color_mutation(
        &mut self,
        changes: u32,
        autotroph: bool,
        random_state: &mut u64,
    ) {
        if changes == 0 {
            return;
        }

        let alpha = self.color & 0xff00_0000;
        let mut rgb = self.rgb();
        for _ in 0..changes {
            let channel = (next_random(random_state) % 3) as usize;
            let increase = next_random(random_state) & 1 == 0;
            rgb[channel] = if increase {
                rgb[channel].saturating_add(20)
            } else {
                rgb[channel].saturating_sub(20)
            };
        }

        if autotroph {
            rgb = clamp_to_autotroph_green(rgb);
        }
        self.color = alpha | ((rgb[0] as u32) << 16) | ((rgb[1] as u32) << 8) | rgb[2] as u32;
    }

    pub fn apply_speciation(&mut self, lineage_id: u64, random_state: &mut u64) {
        let point_index = (next_random(random_state) % self.skin.len() as u64) as usize;
        let original = self.skin[point_index].radius;
        let magnitude = 0.01 + (next_random(random_state) % 12) as f32 * 0.01;
        let signed = if next_random(random_state) & 1 == 0 {
            magnitude
        } else {
            -magnitude
        };
        let mut varied = (original + signed).clamp(MIN_SKIN_RADIUS, MAX_SKIN_RADIUS);
        if varied == original {
            varied = (original - signed).clamp(MIN_SKIN_RADIUS, MAX_SKIN_RADIUS);
        }

        self.skin[point_index].radius = varied;
        self.lineage_id = lineage_id;
        self.accumulated_mutations = 0;
    }
}

impl Default for VisualPhenotype {
    fn default() -> Self {
        Self::new(0, 0xff62_a844, default_skin())
    }
}

pub fn default_skin() -> [SkinPoint; 4] {
    generated_skin("Unassigned", 0)
}

pub fn generated_skin(species_name: &str, seed: u64) -> [SkinPoint; 4] {
    let mut state = fnv1a(species_name.as_bytes()) ^ seed.rotate_left(23);
    if state == 0 {
        state = 0x9e37_79b9_7f4a_7c15;
    }

    std::array::from_fn(|index| {
        let radius_step = (next_random(&mut state) % 68) as f32;
        let radius = MIN_SKIN_RADIUS + radius_step * 0.01;
        let jitter = (next_random(&mut state) % 141) as i32 - 70;
        let angle = ((index as i32 * 314) + jitter).rem_euclid(AIM_UNITS_PER_TURN);
        SkinPoint { radius, angle }
    })
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn next_random(state: &mut u64) -> u64 {
    if *state == 0 {
        *state = 0x9e37_79b9_7f4a_7c15;
    }
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn clamp_to_autotroph_green(rgb: [u8; 3]) -> [u8; 3] {
    let [mut hue, saturation, value] = rgb_to_hsv(rgb);
    if !(85.0..=155.0).contains(&hue) {
        let distance_to_low = angular_distance(hue, 85.0);
        let distance_to_high = angular_distance(hue, 155.0);
        hue = if distance_to_low <= distance_to_high {
            85.0
        } else {
            155.0
        };
    }
    hsv_to_rgb(
        hue,
        saturation.clamp(0.45, 0.90),
        value.clamp(0.40, 0.85),
    )
}

fn rgb_to_hsv(rgb: [u8; 3]) -> [f32; 3] {
    let red = rgb[0] as f32 / 255.0;
    let green = rgb[1] as f32 / 255.0;
    let blue = rgb[2] as f32 / 255.0;
    let maximum = red.max(green).max(blue);
    let minimum = red.min(green).min(blue);
    let delta = maximum - minimum;

    let hue = if delta == 0.0 {
        120.0
    } else if maximum == red {
        60.0 * ((green - blue) / delta).rem_euclid(6.0)
    } else if maximum == green {
        60.0 * (((blue - red) / delta) + 2.0)
    } else {
        60.0 * (((red - green) / delta) + 4.0)
    };
    let saturation = if maximum == 0.0 { 0.0 } else { delta / maximum };
    [hue, saturation, maximum]
}

fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> [u8; 3] {
    let chroma = value * saturation;
    let sector = hue / 60.0;
    let secondary = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = match sector.floor() as i32 {
        0 => (chroma, secondary, 0.0),
        1 => (secondary, chroma, 0.0),
        2 => (0.0, chroma, secondary),
        3 => (0.0, secondary, chroma),
        4 => (secondary, 0.0, chroma),
        _ => (chroma, 0.0, secondary),
    };
    let match_value = value - chroma;
    [
        ((red + match_value) * 255.0).round() as u8,
        ((green + match_value) * 255.0).round() as u8,
        ((blue + match_value) * 255.0).round() as u8,
    ]
}

fn angular_distance(left: f32, right: f32) -> f32 {
    let direct = (left - right).abs();
    direct.min(360.0 - direct)
}
