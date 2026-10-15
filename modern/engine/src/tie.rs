use crate::OrganismId;
use serde::{Deserialize, Serialize};

pub(crate) const MAX_TIES_PER_ROBOT: usize = 10;
pub(crate) const SOFT_TIE_LAST: i32 = -20;
pub(crate) const SOFT_TIE_B: f32 = 0.02;
pub(crate) const SOFT_TIE_K: f32 = 0.01;
pub(crate) const HARD_TIE_B: f32 = 0.1;
pub(crate) const HARD_TIE_K: f32 = 0.05;
pub(crate) const TIE_DEADBAND: f32 = 20.0;
pub(crate) const MAX_TIE_SURFACE_GAP: f32 = 1_000.0;
pub(crate) const TIE_ANGLE_SLACK: f32 = 5.0_f32.to_radians();

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct NativeTieEnd {
    pub rest_length: f32,
    pub last: i32,
    pub b: f32,
    pub k: f32,
    pub hardened: bool,
    pub angle: f32,
    pub angle_fixed: bool,
    pub angle_initialized: bool,
    pub bend: f32,
    #[serde(default)]
    pub shrink: i32,
    #[serde(default)]
    pub kind: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct TieState {
    #[serde(default)]
    pub(crate) native_ends: Option<[NativeTieEnd; 2]>,
    pub(crate) first: OrganismId,
    pub(crate) second: OrganismId,
    pub(crate) rest_length: f32,
    #[serde(default)]
    pub(crate) first_port: i32,
    #[serde(default)]
    pub(crate) second_port: i32,
    #[serde(default)]
    pub(crate) last: i32,
    #[serde(default = "legacy_tie_b")]
    pub(crate) b: f32,
    #[serde(default = "legacy_tie_k")]
    pub(crate) k: f32,
    #[serde(default = "legacy_tie_hardened")]
    pub(crate) hardened: bool,
    #[serde(default)]
    pub(crate) information: bool,
    #[serde(default)]
    pub(crate) sharing_first: bool,
    #[serde(default)]
    pub(crate) sharing_second: bool,
    #[serde(default)]
    pub(crate) requested_angle: f32,
    #[serde(default)]
    pub(crate) angle_fixed: bool,
    #[serde(default)]
    pub(crate) angle_initialized: bool,
}

impl TieState {
    pub(crate) fn new(
        first: OrganismId,
        second: OrganismId,
        rest_length: f32,
        first_port: i32,
        second_port: i32,
    ) -> Self {
        Self {
            native_ends: None,
            first,
            second,
            rest_length,
            first_port,
            second_port,
            last: SOFT_TIE_LAST,
            b: SOFT_TIE_B,
            k: SOFT_TIE_K,
            hardened: false,
            information: false,
            sharing_first: false,
            sharing_second: false,
            requested_angle: 0.0,
            angle_fixed: false,
            angle_initialized: false,
        }
    }

    pub(crate) fn legacy(
        first: OrganismId,
        second: OrganismId,
        rest_length: f32,
        first_port: i32,
        second_port: i32,
    ) -> Self {
        Self {
            native_ends: None,
            first,
            second,
            rest_length,
            first_port,
            second_port,
            last: 0,
            b: HARD_TIE_B,
            k: HARD_TIE_K,
            hardened: true,
            information: false,
            sharing_first: false,
            sharing_second: false,
            requested_angle: 0.0,
            angle_fixed: false,
            angle_initialized: false,
        }
    }

    pub(crate) fn contains(&self, id: OrganismId) -> bool {
        self.first == id || self.second == id
    }

    pub(crate) fn native_end_states(&self) -> [NativeTieEnd; 2] {
        self.native_ends.unwrap_or_else(|| {
            let first = NativeTieEnd {
                rest_length: self.rest_length,
                last: self.last,
                b: self.b,
                k: self.k,
                hardened: self.hardened,
                angle: self.requested_angle,
                angle_fixed: self.angle_fixed,
                angle_initialized: self.angle_initialized,
                bend: 0.0,
                shrink: 0,
                kind: if self.hardened { 3 } else { 0 },
            };
            [
                first,
                NativeTieEnd {
                    angle: 0.0,
                    angle_fixed: false,
                    angle_initialized: true,
                    ..first
                },
            ]
        })
    }

    pub(crate) fn sync_native_primary(&mut self) {
        let Some([first, _]) = self.native_ends else {
            return;
        };
        self.rest_length = first.rest_length;
        self.last = first.last;
        self.b = first.b;
        self.k = first.k;
        self.hardened = first.hardened;
        self.requested_angle = first.angle;
        self.angle_fixed = first.angle_fixed;
        self.angle_initialized = first.angle_initialized;
    }

    pub(crate) fn advance_lifecycle(&mut self, current_length: f32) -> bool {
        if self.last > 1 {
            self.last -= 1;
        }
        if self.last == 1 {
            return false;
        }
        if self.last < 0 {
            self.last += 1;
        }
        if self.last == -1 && !self.hardened {
            self.rest_length = current_length;
            self.b = HARD_TIE_B;
            self.k = HARD_TIE_K;
            self.hardened = true;
        }
        true
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TieSnapshot {
    pub first: OrganismId,
    pub second: OrganismId,
    pub rest_length: f32,
    #[serde(default)]
    pub first_port: i32,
    #[serde(default)]
    pub second_port: i32,
    #[serde(default)]
    pub last: i32,
    #[serde(default = "legacy_tie_b")]
    pub b: f32,
    #[serde(default = "legacy_tie_k")]
    pub k: f32,
    #[serde(default = "legacy_tie_hardened")]
    pub hardened: bool,
    #[serde(default)]
    pub requested_angle: f32,
    #[serde(default)]
    pub angle_fixed: bool,
    #[serde(default)]
    pub angle_initialized: bool,
}

impl From<&TieState> for TieSnapshot {
    fn from(value: &TieState) -> Self {
        Self {
            first: value.first,
            second: value.second,
            rest_length: value.rest_length,
            first_port: value.first_port,
            second_port: value.second_port,
            last: value.last,
            b: value.b,
            k: value.k,
            hardened: value.hardened,
            requested_angle: value.requested_angle,
            angle_fixed: value.angle_fixed,
            angle_initialized: value.angle_initialized,
        }
    }
}

pub(crate) const fn legacy_tie_b() -> f32 {
    HARD_TIE_B
}

pub(crate) const fn legacy_tie_k() -> f32 {
    HARD_TIE_K
}

pub(crate) const fn legacy_tie_hardened() -> bool {
    true
}
