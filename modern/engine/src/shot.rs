use crate::OrganismId;
use serde::{Deserialize, Serialize};

mod effects;
mod projectile;

pub(crate) use effects::{
    ProjectileEffect, ProjectileImpact, ProjectileTarget, projectile_effect,
};
pub(crate) use projectile::{ProjectilePool, ProjectileSpawn};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShotSnapshot {
    pub owner: OrganismId,
    pub start: [f32; 2],
    pub end: [f32; 2],
    #[serde(default)]
    pub velocity: [f32; 2],
    #[serde(default)]
    pub age: u32,
    #[serde(default)]
    pub range: u32,
    #[serde(default)]
    pub energy: f32,
    pub kind: i32,
    pub value: i32,
    #[serde(default)]
    pub impact_flash: bool,
}
