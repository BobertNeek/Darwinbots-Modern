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
    /// Retained save token for SG, the community short-bot alias (at most six genes).
    SingleGene,
    SingleStore,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeagueWorkflow {
    #[default]
    Direct,
    Ladder,
    Tournament,
    Seeding,
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
    pub staged_evolution: Option<crate::EvolutionConfig>,
    pub workflow: LeagueWorkflow,
    pub entrant_order: Vec<SpeciesId>,
    pub initial_ladder: Vec<SpeciesId>,
    pub seeding_opponent: Option<SpeciesId>,
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
            staged_evolution: None,
            workflow: LeagueWorkflow::Direct,
            entrant_order: Vec::new(),
            initial_ladder: Vec::new(),
            seeding_opponent: None,
            kind: LocalModeKind::League,
            preset: LeaguePreset::F1,
            minimum_rounds: 5,
            maximum_rounds: 1,
            maximum_cycles: 15_000,
            maximum_population: 700,
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
        if let Some(evolution) = &self.staged_evolution {
            evolution.validate()?;
            let expected = match evolution.mode {
                crate::EvolutionMode::SimpleSurvival => LocalModeKind::SimpleSurvival,
                crate::EvolutionMode::Eco => LocalModeKind::EcoSurvival,
                crate::EvolutionMode::Zerobot => LocalModeKind::Zerobot,
            };
            if self.kind != expected {
                return Err("evolution settings must match the selected survival mode".into());
            }
        }
        if self.workflow != LeagueWorkflow::Direct
            && !matches!(self.kind, LocalModeKind::League | LocalModeKind::Contest)
        {
            return Err("league schedules require League or Contest mode".to_owned());
        }
        if self.workflow == LeagueWorkflow::Seeding && self.seeding_opponent.is_none() {
            return Err("seeding requires a reference opponent".to_owned());
        }
        if self.minimum_rounds == 0 {
            return Err("minimum rounds must be greater than zero".to_owned());
        }
        if self.maximum_rounds == 0
            && !matches!(self.kind, LocalModeKind::League | LocalModeKind::Contest)
        {
            return Err("maximum rounds must be greater than zero".to_owned());
        }
        if self.maximum_cycles == 0
            && !matches!(self.kind, LocalModeKind::League | LocalModeKind::Contest)
        {
            return Err("maximum cycles must be greater than zero".to_owned());
        }
        if self.maximum_population == 0
            && !matches!(self.kind, LocalModeKind::League | LocalModeKind::Contest)
        {
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisqualificationRecord {
    pub tick: u64,
    pub round: u32,
    pub species: SpeciesId,
    pub file_name: String,
    pub tag: String,
    pub reason: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ContestRuntime {
    pub effective_cycle_limit: u64,
    pub cycles_since_population_sample: u64,
    pub previous_population: std::collections::BTreeMap<SpeciesId, usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalModeState {
    pub evolution: Option<crate::EvolutionState>,
    pub series: crate::league::LeagueSeries,
    pub config: Option<LocalModeConfig>,
    #[serde(default)]
    pub contest_runtime: ContestRuntime,
    pub status: LocalModeStatus,
    pub current_round: u32,
    pub cycle_in_round: u64,
    pub restart_count: u32,
    pub results: Vec<LocalModeResult>,
    pub disqualifications: Vec<DisqualificationRecord>,
    pub adjudication: Option<LocalModeAdjudication>,
}

impl Default for LocalModeState {
    fn default() -> Self {
        Self {
            evolution: None,
            series: crate::league::LeagueSeries::default(),
            config: None,
            contest_runtime: ContestRuntime::default(),
            status: LocalModeStatus::Idle,
            current_round: 0,
            cycle_in_round: 0,
            restart_count: 0,
            results: Vec::new(),
            disqualifications: Vec::new(),
            adjudication: None,
        }
    }
}

impl LocalModeState {
    pub(crate) fn presentation(&self) -> Self {
        Self {
            config: self.config.clone(),
            series: self.series.clone(),
            evolution: self
                .evolution
                .as_ref()
                .map(crate::EvolutionState::presentation),
            contest_runtime: self.contest_runtime.clone(),
            status: self.status,
            current_round: self.current_round,
            cycle_in_round: self.cycle_in_round,
            restart_count: self.restart_count,
            results: self.results.clone(),
            disqualifications: self.disqualifications.clone(),
            adjudication: self.adjudication.clone(),
        }
    }

    pub fn configured(config: LocalModeConfig) -> Self {
        Self {
            config: Some(config),
            status: LocalModeStatus::Configured,
            ..Self::default()
        }
    }
}
