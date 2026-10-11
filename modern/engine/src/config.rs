use crate::{AdvancedMutationSettings, BackendPreference, DnaCosts};
use serde::{Deserialize, Serialize};

pub const MAX_ROBOTS: usize = 32_000;

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
    #[serde(default)]
    pub planet_eaters: bool,
    #[serde(default)]
    pub planet_eaters_g: f32,
    #[serde(default)]
    pub zero_momentum: bool,
}

impl Default for PhysicsSettings {
    fn default() -> Self {
        Self {
            max_velocity: 60.0,
            movement_efficiency: 0.66,
            surface_gravity: 0.0,
            static_friction: 0.0,
            kinetic_friction: 0.0,
            density: 0.0,
            viscosity: 0.0,
            elasticity: 0.0,
            planet_eaters: false,
            planet_eaters_g: 0.0,
            zero_momentum: false,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorpseDecayMode {
    None,
    WasteShot,
    #[default]
    EnergyShot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CorpseDecaySettings {
    pub enabled: bool,
    pub death_energy_threshold: i32,
    pub mode: CorpseDecayMode,
    pub packet_size: i32,
    pub period: u64,
}

impl Default for CorpseDecaySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            death_energy_threshold: 0,
            mode: CorpseDecayMode::None,
            packet_size: 0,
            period: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WasteSettings {
    pub alzheimer_threshold: i32,
    pub defecation_threshold: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecordingSettings {
    pub safemode_backup: bool,
    pub dead_snapshot_enabled: bool,
    pub exclude_repopulating_robots: bool,
    pub autosave_interval: u64,
    #[serde(default = "default_sampling_interval")]
    pub sampling_interval: u64,
}

impl Default for RecordingSettings {
    fn default() -> Self {
        Self {
            safemode_backup: false,
            dead_snapshot_enabled: false,
            exclude_repopulating_robots: false,
            autosave_interval: 0,
            sampling_interval: default_sampling_interval(),
        }
    }
}

fn default_sampling_interval() -> u64 {
    200
}

impl Default for WasteSettings {
    fn default() -> Self {
        Self {
            alzheimer_threshold: 400,
            defecation_threshold: 32_000,
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
    pub sun_up_enabled: bool,
    pub sun_up_threshold: i32,
    pub sun_down_enabled: bool,
    pub sun_down_threshold: i32,
    pub sun_threshold_mode: i32,
    pub weather_enabled: bool,
    pub pond_mode: bool,
    pub light_intensity: f32,
    pub gradient: f32,
    pub tides: u32,
    pub tides_offset: i32,
    #[serde(default)]
    pub sun_on_random: bool,
}

impl Default for VegetationSettings {
    fn default() -> Self {
        Self {
            start_chloroplasts: 16_000,
            max_energy_per_tick: 100,
            minimum_chloroplast_equivalents: 50,
            repopulation_amount: 10,
            repopulation_cooldown: 10,
            feeding_to_body: 0.75,
            daytime: true,
            day_night_enabled: false,
            cycle_length: 10_000,
            sun_up_enabled: false,
            sun_up_threshold: 500_000,
            sun_down_enabled: false,
            sun_down_threshold: 1_000_000,
            sun_threshold_mode: 0,
            weather_enabled: false,
            pond_mode: false,
            light_intensity: 100.0,
            gradient: 1.02,
            tides: 0,
            tides_offset: 0,
            sun_on_random: false,
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
    #[serde(default)]
    pub fixed_bot_radii: bool,
    #[serde(default)]
    pub axis_topology: Option<[bool; 2]>,
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
    pub corpse_decay: CorpseDecaySettings,
    #[serde(default)]
    pub waste: WasteSettings,
    #[serde(default)]
    pub vegetation: VegetationSettings,
    #[serde(default)]
    pub recording: RecordingSettings,
    #[serde(default)]
    pub dna_costs: DnaCosts,
    #[serde(default)]
    pub advanced_mutation: AdvancedMutationSettings,
    #[serde(default)]
    pub auto_speciation: bool,
    #[serde(default = "default_speciation_genetic_distance_percent")]
    pub speciation_genetic_distance_percent: f32,
    #[serde(default = "default_low_memory_dna_limit")]
    pub low_memory_dna_limit: usize,
    #[serde(default = "default_low_memory_detail_limit")]
    pub low_memory_detail_limit: usize,
    #[serde(default)]
    pub greedy_reproduction: bool,
    #[serde(default)]
    pub body_cutoff: i32,
    #[serde(skip)]
    pub force_gpu_unavailable_for_tests: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            seed: 1,
            organism_capacity: MAX_ROBOTS,
            vegetable_population_cap: default_vegetable_population_cap(),
            world_width: 16_000.0,
            world_height: 12_000.0,
            toroidal_world: default_toroidal_world(),
            fixed_bot_radii: false,
            axis_topology: None,
            backend: BackendPreference::Auto,
            allow_cpu_fallback: true,
            metabolism_cost: default_metabolism_cost(),
            vegetable_energy_per_tick: default_vegetable_energy_per_tick(),
            sunlight_energy: default_sunlight_energy(),
            gravity: [0.0, 0.0],
            drag: 0.0,
            brownian_motion: 0.5,
            physics: PhysicsSettings::default(),
            shots: ShotSettings::default(),
            corpse_decay: CorpseDecaySettings::default(),
            waste: WasteSettings::default(),
            vegetation: VegetationSettings::default(),
            recording: RecordingSettings::default(),
            dna_costs: DnaCosts::default(),
            advanced_mutation: AdvancedMutationSettings::default(),
            auto_speciation: false,
            speciation_genetic_distance_percent: default_speciation_genetic_distance_percent(),
            low_memory_dna_limit: default_low_memory_dna_limit(),
            low_memory_detail_limit: default_low_memory_detail_limit(),
            greedy_reproduction: false,
            body_cutoff: 0,
            force_gpu_unavailable_for_tests: false,
        }
    }
}

fn default_metabolism_cost() -> i32 {
    0
}
fn default_toroidal_world() -> bool {
    true
}
fn default_vegetable_energy_per_tick() -> i32 {
    0
}
fn default_sunlight_energy() -> i32 {
    100
}
fn default_vegetable_population_cap() -> usize {
    500
}
fn default_speciation_genetic_distance_percent() -> f32 {
    20.0
}

fn default_low_memory_dna_limit() -> usize {
    4_000_000
}

fn default_low_memory_detail_limit() -> usize {
    3_000_000
}

impl EngineConfig {
    pub fn axis_topology(&self) -> [bool; 2] {
        self.axis_topology.unwrap_or([self.toroidal_world; 2])
    }

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
