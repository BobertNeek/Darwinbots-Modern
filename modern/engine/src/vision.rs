use serde::{Deserialize, Serialize};

use crate::VmMemory;

pub const EYE_VALUE_START: i32 = 501;
pub const FOCUS_EYE: i32 = 511;
pub const EYE_DIRECTION_START: i32 = 521;
pub const EYE_WIDTH_START: i32 = 531;

const EYE_COUNT: usize = 9;
const STANDARD_EYE_WIDTH: i32 = 35;
const AIM_MODULUS: i32 = 1256;
const AIM_UNITS_PER_RADIAN: f32 = 200.0;
const STANDARD_SIGHT_DISTANCE: f32 = 1_440.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EyeSnapshot {
    pub direction: i32,
    pub width: i32,
    pub center_radians: f32,
    pub half_width_radians: f32,
    pub range: f32,
    pub value: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VisionSnapshot {
    pub focus_eye: u8,
    pub eyes: [EyeSnapshot; EYE_COUNT],
}

impl VisionSnapshot {
    pub fn from_memory(memory: &VmMemory, aim: i32) -> Self {
        let focus_value = memory.read(FOCUS_EYE) as i64 + 4;
        let focus_eye = (focus_value.unsigned_abs() % EYE_COUNT as u64) as u8;
        let eyes = std::array::from_fn(|index| {
            let direction = memory.read(EYE_DIRECTION_START + index as i32);
            let width = memory.read(EYE_WIDTH_START + index as i32);
            EyeSnapshot {
                direction,
                width,
                center_radians: eye_center_radians(aim, index, direction),
                half_width_radians: absolute_eye_width(width) as f32
                    / AIM_UNITS_PER_RADIAN
                    / 2.0,
                range: eye_sight_distance(width),
                value: memory.read(EYE_VALUE_START + index as i32),
            }
        });
        Self { focus_eye, eyes }
    }

    pub fn max_range(&self) -> f32 {
        self.eyes
            .iter()
            .map(|eye| eye.range)
            .fold(0.0_f32, f32::max)
    }

    pub fn matching_eyes(
        &self,
        observer: [f32; 2],
        target: [f32; 2],
        observer_radius: f32,
        target_radius: f32,
    ) -> [bool; EYE_COUNT] {
        let delta = [target[0] - observer[0], target[1] - observer[1]];
        let center_distance = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
        if !center_distance.is_finite() {
            return [false; EYE_COUNT];
        }
        let edge_distance = (center_distance - observer_radius - target_radius).max(0.0);
        let target_angle = delta[0].atan2(delta[1]);
        let target_half_width = if center_distance <= target_radius || center_distance == 0.0 {
            std::f32::consts::PI
        } else {
            (target_radius / center_distance).clamp(0.0, 1.0).asin()
        };

        std::array::from_fn(|index| {
            let eye = self.eyes[index];
            let angular_distance = signed_angle(target_angle - eye.center_radians).abs();
            edge_distance <= eye.range
                && angular_distance <= eye.half_width_radians + target_half_width
        })
    }
}

impl Default for VisionSnapshot {
    fn default() -> Self {
        Self::from_memory(&VmMemory::default(), 0)
    }
}

pub fn absolute_eye_width(width: i32) -> i32 {
    if width == 0 {
        return STANDARD_EYE_WIDTH;
    }
    let mut absolute = (width % AIM_MODULUS) + STANDARD_EYE_WIDTH;
    if absolute <= 0 {
        absolute += AIM_MODULUS;
    }
    absolute
}

pub fn eye_sight_distance(width: i32) -> f32 {
    let absolute = absolute_eye_width(width);
    let range = if absolute == STANDARD_EYE_WIDTH {
        STANDARD_SIGHT_DISTANCE
    } else {
        STANDARD_SIGHT_DISTANCE
            * (1.0 - ((absolute as f32 / STANDARD_EYE_WIDTH as f32).ln() / 4.0))
    };
    if range.is_finite() { range.max(0.0) } else { 0.0 }
}

pub fn eye_value(range: f32, edge_distance: f32) -> i32 {
    if edge_distance <= 0.0 {
        return 32_000;
    }
    if !range.is_finite() || range <= 0.0 || edge_distance > range {
        return 0;
    }
    let percent_distance = (edge_distance + 10.0) / range;
    (1.0 / (percent_distance * percent_distance))
        .round()
        .clamp(0.0, 32_000.0) as i32
}

fn eye_center_radians(aim: i32, index: usize, direction: i32) -> f32 {
    let standard_offset = (4.0 - index as f32) * std::f32::consts::PI / 18.0;
    (aim as f32 / AIM_UNITS_PER_RADIAN
        + (direction % AIM_MODULUS) as f32 / AIM_UNITS_PER_RADIAN
        + standard_offset)
        .rem_euclid(std::f32::consts::TAU)
}

fn signed_angle(angle: f32) -> f32 {
    (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI
}
