use crate::OrganismId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShotSnapshot {
    pub owner: OrganismId,
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub kind: i32,
    pub value: i32,
}
