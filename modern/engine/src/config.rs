use crate::BackendPreference;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EngineConfig {
    pub seed: u64,
    pub organism_capacity: usize,
    #[serde(default = "default_vegetable_population_cap")]
    pub vegetable_population_cap: usize,
    pub world_width: f32,
    pub world_height: f32,
    pub backend: BackendPreference,
    pub allow_cpu_fallback: bool,
    #[serde(default = "default_metabolism_cost")]
    pub metabolism_cost: i32,
    #[serde(default = "default_vegetable_energy_per_tick")]
    pub vegetable_energy_per_tick: i32,
    #[serde(default = "default_sunlight_energy")]
    pub sunlight_energy: i32,
    #[serde(default)]
    pub gravity: [f32; 2],
    #[serde(default)]
    pub drag: f32,
    #[serde(default)]
    pub brownian_motion: f32,
    #[serde(skip)]
    pub force_gpu_unavailable_for_tests: bool,
    #[serde(skip)]
    pub force_gpu_runtime_failure_for_tests: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            seed: 1,
            organism_capacity: 100_000,
            vegetable_population_cap: default_vegetable_population_cap(),
            world_width: 16_000.0,
            world_height: 12_000.0,
            backend: BackendPreference::Auto,
            allow_cpu_fallback: true,
            metabolism_cost: default_metabolism_cost(),
            vegetable_energy_per_tick: default_vegetable_energy_per_tick(),
            sunlight_energy: default_sunlight_energy(),
            gravity: [0.0, 0.0],
            drag: 0.0,
            brownian_motion: 0.0,
            force_gpu_unavailable_for_tests: false,
            force_gpu_runtime_failure_for_tests: false,
        }
    }
}

fn default_metabolism_cost() -> i32 { 1 }
fn default_vegetable_energy_per_tick() -> i32 { 4 }
fn default_sunlight_energy() -> i32 { 100 }
fn default_vegetable_population_cap() -> usize { 500 }

impl EngineConfig {
    pub fn testing() -> Self {
        Self {
            organism_capacity: 32,
            world_width: 1_000.0,
            world_height: 1_000.0,
            backend: BackendPreference::Cpu,
            ..Self::default()
        }
    }
}
