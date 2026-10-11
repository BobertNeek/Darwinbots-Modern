use serde::{Deserialize, Serialize};

use crate::{RestrictionColumn, SkinPoint, default_skin};

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
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
    #[serde(default)]
    pub mutation_profile: Option<crate::StandardMutationProfile>,
    #[serde(default = "default_skin")]
    pub skin: [SkinPoint; 4],
    #[serde(default)]
    pub lineage_id: u64,
    #[serde(default)]
    pub restrictions: Option<RestrictionColumn>,
    #[serde(default)]
    pub custom_skin: bool,
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
            mutation_profile: None,
            skin: default_skin(),
            lineage_id: 0,
            restrictions: None,
            custom_skin: false,
        }
    }
}
