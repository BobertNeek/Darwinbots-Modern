use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationStats {
    pub population: usize,
    pub births: u64,
    pub deaths: u64,
    pub shots_fired: u64,
    #[serde(default)]
    pub energy_harvested: u64,
    #[serde(default)]
    pub energy_donated: u64,
    pub mutations: u64,
    pub ties_created: u64,
    #[serde(default)]
    pub reseeds: u64,
    #[serde(default)]
    pub self_reproductions: u64,
    #[serde(default)]
    pub feeding_events: u64,
    #[serde(default)]
    pub intentional_movement_events: u64,
    pub total_energy: i64,
}
