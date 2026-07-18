use crate::BackendPreference;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PhysicsSettings {
    pub max_velocity: f32,
    pub movement_efficiency: f32,
    pub surface_gravity: f32,
    pub static_friction: f32,
    pub kinetic_friction: f32,
    pub density: f64,
    pub viscosity: f64,
    pub elasticity: f32,
}

impl Default for PhysicsSettings {
    fn default() -> Self {
        Self {
            max_velocity: 180.0,
            movement_efficiency: 0.66,
            surface_gravity: 2.0,
            static_friction: 0.6,
            kinetic_friction: 0.4,
            density: 0.0,
            viscosity: 0.0,
            elasticity: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShotSettings {
    pub speed: f32,
    pub range_multiplier: f32,
    pub decay: f32,
    pub energy_shots_do_not_decay: bool,
    pub waste_shots_do_not_decay: bool,
}

impl Default for ShotSettings {
    fn default() -> Self {
        Self {
            speed: 40.0,
            range_multiplier: 1.0,
            decay: 40.0,
            energy_shots_do_not_decay: false,
            waste_shots_do_not_decay: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VegetationSettings {
    pub start_chloroplasts: i32,
    pub max_energy_per_tick: i32,
    pub minimum_chloroplast_equivalents: usize,
    pub repopulation_amount: usize,
    pub repopulation_cooldown: u64,
    pub feeding_to_body: f32,
    pub daytime: bool,
    pub day_night_enabled: bool,
    pub cycle_length: u64,
}

impl Default for VegetationSettings {
    fn default() -> Self {
        Self {
            start_chloroplasts: 16_000,
            max_energy_per_tick: 40,
            minimum_chloroplast_equivalents: 10,
            repopulation_amount: 10,
            repopulation_cooldown: 25,
            feeding_to_body: 0.5,
            daytime: true,
            day_night_enabled: false,
            cycle_length: 10_000,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EngineConfig {
    pub seed: u64,
    pub organism_capacity: usize,
    #[serde(default = "default_vegetable_population_cap")]
    pub vegetable_population_cap: usize,
    pub world_width: f32,
    pub world_height: f32,
    #[serde(default = "default_toroidal_world")]
    pub toroidal_world: bool,
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
    #[serde(default)]
    pub physics: PhysicsSettings,
    #[serde(default)]
    pub shots: ShotSettings,
    #[serde(default)]
    pub vegetation: VegetationSettings,
    #[serde(default)]
    pub auto_speciation: bool,
    #[serde(default = "default_speciation_genetic_distance_percent")]
    pub speciation_genetic_distance_percent: f32,
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
            toroidal_world: default_toroidal_world(),
            backend: BackendPreference::Auto,
            allow_cpu_fallback: true,
            metabolism_cost: default_metabolism_cost(),
            vegetable_energy_per_tick: default_vegetable_energy_per_tick(),
            sunlight_energy: default_sunlight_energy(),
            gravity: [0.0, 0.0],
            drag: 0.0,
            brownian_motion: 0.0,
            physics: PhysicsSettings::default(),
            shots: ShotSettings::default(),
            vegetation: VegetationSettings::default(),
            auto_speciation: false,
            speciation_genetic_distance_percent: default_speciation_genetic_distance_percent(),
            force_gpu_unavailable_for_tests: false,
            force_gpu_runtime_failure_for_tests: false,
        }
    }
}

fn default_metabolism_cost() -> i32 { 0 }
fn default_toroidal_world() -> bool { true }
fn default_vegetable_energy_per_tick() -> i32 { 0 }
fn default_sunlight_energy() -> i32 { 100 }
fn default_vegetable_population_cap() -> usize { 500 }
fn default_speciation_genetic_distance_percent() -> f32 { 20.0 }

impl EngineConfig {
    pub fn testing() -> Self {
        Self {
            organism_capacity: 32,
            world_width: 1_000.0,
            world_height: 1_000.0,
            backend: BackendPreference::Cpu,
            brownian_motion: 0.0,
            physics: PhysicsSettings {
                surface_gravity: 0.0,
                static_friction: 0.0,
                kinetic_friction: 0.0,
                ..PhysicsSettings::default()
            },
            ..Self::default()
        }
    }
}
