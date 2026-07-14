use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PhaseTimings {
    pub dna: f64,
    pub intent: f64,
    pub spatial: f64,
    pub sensing: f64,
    pub interactions: f64,
    pub physics: f64,
    #[serde(default)]
    pub projectiles: f64,
    #[serde(default)]
    pub vegetation: f64,
    pub lifecycle: f64,
    pub mutation: f64,
    pub snapshot: f64,
}
