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
    #[serde(default = "default_placement_bounds")]
    pub placement_bounds: [f32; 4],
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
            placement_bounds: default_placement_bounds(),
        }
    }
}

fn default_placement_bounds() -> [f32; 4] { [0.0, 1.0, 0.0, 1.0] }

impl SpeciesDefinition {
    pub(crate) fn validate_placement_bounds(&self) -> Result<(), String> {
        let [left, right, top, bottom] = self.placement_bounds;
        if self.placement_bounds.iter().any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
            || left > right || top > bottom {
            return Err("species placement bounds must be ordered and within 0..1".into());
        }
        Ok(())
    }
}
