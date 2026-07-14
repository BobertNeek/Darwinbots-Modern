use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistorySample {
    pub tick: u64,
    pub population: usize,
    pub total_energy: i64,
    pub births: u64,
    pub deaths: u64,
    pub mutations: u64,
    pub shots_fired: u64,
    #[serde(default)]
    pub projectile_impacts: u64,
    #[serde(default, alias = "plant_energy_produced")]
    pub plant_energy_generated: u64,
}
