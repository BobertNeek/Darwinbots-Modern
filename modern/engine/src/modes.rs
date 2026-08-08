use serde::{Deserialize, Serialize};

use crate::SpeciesId;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalModeKind {
    #[default]
    League,
    Contest,
    SimpleSurvival,
    EcoSurvival,
    Zerobot,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaguePreset {
    #[default]
    F1,
    F2,
    F3,
    Multibot,
    ShortBot,
    SingleGene,
    SingleStore,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalModeStatus {
    #[default]
    Idle,
    Configured,
    Running,
    RoundComplete,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalModeRestrictions {
    pub prohibit_information_shots: bool,
    pub prohibit_viruses: bool,
    pub prohibit_sexual_reproduction: bool,
    pub prohibit_cross_species_tie_transfer: bool,
    pub robot_fixing_allowed: bool,
    pub asexual_reproduction_allowed: bool,
}

impl Default for LocalModeRestrictions {
    fn default() -> Self {
        Self {
            prohibit_information_shots: false,
            prohibit_viruses: false,
            prohibit_sexual_reproduction: false,
            prohibit_cross_species_tie_transfer: false,
            robot_fixing_allowed: true,
            asexual_reproduction_allowed: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalModeConfig {
    pub kind: LocalModeKind,
    pub preset: LeaguePreset,
    pub minimum_rounds: u32,
    pub maximum_rounds: u32,
    pub maximum_cycles: u64,
    pub maximum_population: usize,
    pub restart_on_extinction: bool,
    pub energy_handicap: i32,
    pub source_species: Option<SpeciesId>,
    pub selection_population_weight: f32,
    pub selection_energy_weight: f32,
    pub restrictions: LocalModeRestrictions,
}

impl Default for LocalModeConfig {
    fn default() -> Self {
        Self {
            kind: LocalModeKind::League,
            preset: LeaguePreset::F1,
            minimum_rounds: 1,
            maximum_rounds: 3,
            maximum_cycles: 10_000,
            maximum_population: 25_000,
            restart_on_extinction: true,
            energy_handicap: 0,
            source_species: None,
            selection_population_weight: 0.5,
            selection_energy_weight: 0.5,
            restrictions: LocalModeRestrictions::default(),
        }
    }
}

impl LocalModeConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.minimum_rounds == 0 {
            return Err("minimum rounds must be greater than zero".to_owned());
        }
        if self.maximum_rounds < self.minimum_rounds {
            return Err("maximum rounds must be at least the minimum rounds".to_owned());
        }
        if self.maximum_cycles == 0 {
            return Err("maximum cycles must be greater than zero".to_owned());
        }
        if self.maximum_population == 0 {
            return Err("maximum population must be greater than zero".to_owned());
        }
        if !self.selection_population_weight.is_finite()
            || !self.selection_energy_weight.is_finite()
            || self.selection_population_weight < 0.0
            || self.selection_energy_weight < 0.0
            || self.selection_population_weight + self.selection_energy_weight <= 0.0
        {
            return Err("survival selection weights must be finite and positive".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocalModeResult {
    pub round: u32,
    pub winner: Option<SpeciesId>,
    pub winning_population: usize,
    pub winning_energy: i64,
    pub reason: String,
    pub automatic: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocalModeAdjudication {
    pub winner: Option<SpeciesId>,
    pub reason: String,
    pub automatic: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalModeState {
    pub config: Option<LocalModeConfig>,
    pub status: LocalModeStatus,
    pub current_round: u32,
    pub cycle_in_round: u64,
    pub restart_count: u32,
    pub results: Vec<LocalModeResult>,
    pub adjudication: Option<LocalModeAdjudication>,
}

impl Default for LocalModeState {
    fn default() -> Self {
        Self {
            config: None,
            status: LocalModeStatus::Idle,
            current_round: 0,
            cycle_in_round: 0,
            restart_count: 0,
            results: Vec::new(),
            adjudication: None,
        }
    }
}

impl LocalModeState {
    pub fn configured(config: LocalModeConfig) -> Self {
        Self {
            config: Some(config),
            status: LocalModeStatus::Configured,
            ..Self::default()
        }
    }
}
