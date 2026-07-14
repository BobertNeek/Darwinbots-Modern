use serde::{Deserialize, Serialize};

use crate::{SkinPoint, default_skin};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpeciesId(pub u32);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpeciesDefinition {
    pub name: String,
    pub vegetable: bool,
    pub color: u32,
    pub minimum_population: usize,
    pub reseed: bool,
    #[serde(default)]
    pub mutation_rate: f32,
    #[serde(default = "default_skin")]
    pub skin: [SkinPoint; 4],
    #[serde(default)]
    pub lineage_id: u64,
}

impl Default for SpeciesDefinition {
    fn default() -> Self {
        Self {
            name: "Unassigned".to_owned(),
            vegetable: false,
            color: 0xff62a844,
            minimum_population: 0,
            reseed: false,
            mutation_rate: 0.0,
            skin: default_skin(),
            lineage_id: 0,
        }
    }
}
