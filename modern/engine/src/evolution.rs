//! Evo.bas stage archives and difficulty controller. Runtime wiring lives in world.
use crate::{
    LegacyDna, StandardMutationKind as Kind, StandardMutationProfile, native_random::NativeRandom,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const KINDS: [Kind; 11] = [
    Kind::Point,
    Kind::Point2,
    Kind::Delta,
    Kind::CopyError,
    Kind::CopyError2,
    Kind::Insertion,
    Kind::Reversal,
    Kind::MinorDeletion,
    Kind::MajorDeletion,
    Kind::Amplification,
    Kind::Translocation,
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvolutionMode {
    #[default]
    SimpleSurvival,
    Eco,
    Zerobot,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EvolutionConfig {
    pub mode: EvolutionMode,
    pub fitness_weight: u16,
    pub source_dna: String,
    pub reduction_unit: f32,
    pub cycle_length: u32,
    pub normalize_dna_size: bool,
    pub zerobot_initial_dna_length: usize,
    pub mutation_rate_limit: Option<f64>,
    pub auto_graphs: bool,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            mode: EvolutionMode::SimpleSurvival,
            fitness_weight: 100,
            source_dna: String::new(),
            reduction_unit: 10.0,
            cycle_length: 100,
            normalize_dna_size: false,
            zerobot_initial_dna_length: 32_000,
            mutation_rate_limit: None,
            auto_graphs: false,
        }
    }
}

impl EvolutionConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.fitness_weight > 200 {
            return Err("evolution fitness weight must be 0..200".into());
        }
        if !self.reduction_unit.is_finite() || !(0.01..=150.0).contains(&self.reduction_unit) {
            return Err("evolution reduction unit must be 0.01..150".into());
        }
        if self.cycle_length == 0 || self.cycle_length > 15_000 {
            return Err("evolution cycle length must be 1..15000".into());
        }
        if !(1..=32_000).contains(&self.zerobot_initial_dna_length) {
            return Err("zerobot starting DNA length must be 1..32000".into());
        }
        if self
            .mutation_rate_limit
            .is_some_and(|limit| !limit.is_finite() || limit <= 0.0)
        {
            return Err("evolution mutation normalization limit must be positive".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvolutionGenome {
    pub dna: LegacyDna,
    pub mutation_profile: StandardMutationProfile,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvolutionPhase {
    #[default]
    Evolving,
    Testing,
    ZerobotTesting,
    Complete,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvolutionState {
    pub baseline_species: Vec<crate::SpeciesId>,
    pub mutator_species: Vec<crate::SpeciesId>,
    pub test_start_energy: f64,
    pub fitness_weight: u16,
    pub config: EvolutionConfig,
    pub phase: EvolutionPhase,
    pub stage: u32,
    pub stages: BTreeMap<u32, Vec<EvolutionGenome>>,
    pub retired_stages: Vec<Vec<EvolutionGenome>>,
    pub candidate: Vec<EvolutionGenome>,
    pub mutators: Vec<EvolutionGenome>,
    pub test_wins: u32,
    pub test_result_cursor: usize,
    pub opponent_stage: u32,
    pub reduction_unit: f32,
    pub decreasing_difficulty: bool,
    pub correction: f32,
    pub hide_cycles: u32,
    pub initial_hide_cycles: u32,
    pub current_dna_size: usize,
    pub target_dna_size: usize,
    pub hidden: bool,
    pub mode_cycles: u32,
    pub hide_offset: f32,
    pub energy_delta: f64,
    pub previous_energy_delta: f64,
    pub inverse_delta: f64,
    pub previous_inverse_delta: f64,
    pub handicap: f64,
    pub inverse_handicap: f64,
    pub cycle_in_attempt: u64,
    pub stagnant: bool,
    pub zerobot_previous_id: u64,
    pub zerobot_previous_score: f64,
    pub events: Vec<String>,
}

// VB integer division first rounds BOTH operands, then truncates the quotient.
// Replacing it with floor(log10(...)) changes difficulty near the boundaries.
fn n10(value: f32) -> f32 {
    if value > 1.0 {
        return 1.0;
    }
    let numerator = f64::from(value * 2.0).ln().round_ties_even() as i32;
    let denominator = 10.0_f64.ln().round_ties_even() as i32;
    10.0_f32.powi(1 - numerator / denominator)
}

fn target_size(length: usize) -> usize {
    let mut target = 250;
    for index in 1..=length {
        if index > target {
            let rate = ((5_000.0 - target as f64) / 4_750.0).max(0.0);
            target = (target as f64 + 10.0 + rate * 250.0).round_ties_even() as usize;
        }
    }
    target
}

impl EvolutionState {
    /// UI publication must not deep-copy all archived genomes every frame.
    pub(crate) fn presentation(&self) -> Self {
        Self {
            config: self.config.clone(),
            baseline_species: self.baseline_species.clone(),
            mutator_species: self.mutator_species.clone(),
            test_start_energy: self.test_start_energy,
            fitness_weight: self.fitness_weight,
            phase: self.phase,
            stage: self.stage,
            stages: BTreeMap::new(),
            retired_stages: Vec::new(),
            candidate: Vec::new(),
            mutators: Vec::new(),
            test_wins: self.test_wins,
            test_result_cursor: self.test_result_cursor,
            opponent_stage: self.opponent_stage,
            reduction_unit: self.reduction_unit,
            decreasing_difficulty: self.decreasing_difficulty,
            correction: self.correction,
            hide_cycles: self.hide_cycles,
            initial_hide_cycles: self.initial_hide_cycles,
            current_dna_size: self.current_dna_size,
            target_dna_size: self.target_dna_size,
            hidden: self.hidden,
            mode_cycles: self.mode_cycles,
            hide_offset: self.hide_offset,
            energy_delta: self.energy_delta,
            previous_energy_delta: self.previous_energy_delta,
            inverse_delta: self.inverse_delta,
            previous_inverse_delta: self.previous_inverse_delta,
            handicap: self.handicap,
            inverse_handicap: self.inverse_handicap,
            cycle_in_attempt: self.cycle_in_attempt,
            stagnant: self.stagnant,
            zerobot_previous_id: self.zerobot_previous_id,
            zerobot_previous_score: self.zerobot_previous_score,
            events: self
                .events
                .iter()
                .rev()
                .take(20)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect(),
        }
    }

    pub fn new(config: EvolutionConfig, seeds: Vec<EvolutionGenome>) -> Result<Self, String> {
        config.validate()?;
        if seeds.is_empty() {
            return Err("staged evolution needs a source genome".into());
        }
        let length = seeds[0].dna.blocks().len();
        let reduction_unit = config.reduction_unit;
        let hide_cycles = config.cycle_length;
        let fitness_weight = config.fitness_weight;
        Ok(Self {
            baseline_species: Vec::new(),
            mutator_species: Vec::new(),
            test_start_energy: 0.0,
            fitness_weight,
            config,
            phase: EvolutionPhase::Evolving,
            stage: 0,
            stages: BTreeMap::from([(0, seeds.clone())]),
            mutators: seeds,
            retired_stages: Vec::new(),
            candidate: Vec::new(),
            test_wins: 0,
            test_result_cursor: 0,
            opponent_stage: 0,
            reduction_unit,
            decreasing_difficulty: true,
            correction: 5.0,
            hide_cycles,
            initial_hide_cycles: hide_cycles,
            current_dna_size: length + 5,
            target_dna_size: target_size(length),
            hidden: false,
            mode_cycles: 0,
            hide_offset: 0.0,
            energy_delta: 0.0,
            previous_energy_delta: 0.0,
            inverse_delta: 0.0,
            previous_inverse_delta: 0.0,
            handicap: 0.0,
            inverse_handicap: 0.0,
            cycle_in_attempt: 0,
            stagnant: false,
            zerobot_previous_id: 0,
            zerobot_previous_score: 0.0,
            events: Vec::new(),
        })
    }

    fn jitter_cycles(&mut self, random: &mut NativeRandom) {
        self.hide_cycles = (f64::from(self.initial_hide_cycles) + 300.0 * f64::from(random.unit())
            - 150.0)
            .round_ties_even()
            .clamp(150.0, 15_000.0) as u32;
    }

    pub(crate) fn increase_difficulty(&mut self, random: &mut NativeRandom) {
        if self.decreasing_difficulty {
            self.decreasing_difficulty = false;
            self.correction /= 2.0;
        }
        let scale = n10(self.reduction_unit);
        self.reduction_unit = (self.reduction_unit - self.correction / scale)
            .max(1.0 / scale)
            .max(0.01);
        self.jitter_cycles(random);
        if self.reduction_unit == 0.01 {
            self.hide_cycles = 150;
            self.initial_hide_cycles = 150;
        }
    }

    pub(crate) fn decrease_difficulty(&mut self, random: &mut NativeRandom) {
        self.renormalize_mutations();
        if !self.decreasing_difficulty {
            self.decreasing_difficulty = true;
            self.correction /= 2.0;
        }
        self.reduction_unit =
            (self.reduction_unit + self.correction / n10(self.reduction_unit)).min(150.0);
        self.jitter_cycles(random);
        if self.correction < 0.00000005 && self.config.mode != EvolutionMode::Eco && self.stage > 0
        {
            if let Some(retired) = self.stages.remove(&self.stage) {
                self.retired_stages.push(retired);
            }
            self.stage -= 1;
            self.opponent_stage = self.stage;
            self.correction = 5.0;
            self.reduction_unit = (self.reduction_unit + 10.0) / 2.0;
            self.mutators = self.stages[&self.stage].clone();
            self.current_dna_size = self.stages[&self.stage][0].dna.blocks().len() + 5;
            self.events.push("Reverting one stage".into());
        }
    }

    pub fn scale_mutations(&mut self) {
        let blend = f64::from((5.0 / self.correction) * 5.0);
        for genome in &mut self.mutators {
            let positive: Vec<_> = KINDS
                .iter()
                .map(|kind| genome.mutation_profile.rate(*kind).one_in)
                .filter(|rate| *rate > 0.0)
                .collect();
            if positive.is_empty() {
                continue;
            }
            let average = positive.iter().sum::<f64>() / positive.len() as f64;
            for kind in KINDS {
                let rate = genome.mutation_profile.rate_mut(kind);
                if rate.one_in <= 0.0 {
                    continue;
                }
                let threshold = 6.0_f64.ln() / 4.0_f64.ln() * average;
                let held = if rate.one_in >= threshold {
                    threshold - 1.0
                } else {
                    rate.one_in
                };
                let held = (held / 6.0 * 4.0_f64.powf(held / average)).max(1.0) as f32;
                rate.one_in =
                    f64::from(((rate.one_in * (blend - 1.0) + f64::from(held)) / blend) as f32);
            }
        }
    }

    fn renormalize_mutations(&mut self) {
        let blend = f64::from((5.0 / self.correction) * 90.0);
        let mut defaults = StandardMutationProfile::default();
        defaults.point2.mean = 1.0;
        defaults.point2.std_dev = 0.0;
        for genome in &mut self.mutators {
            let positive: Vec<_> = KINDS
                .iter()
                .map(|kind| genome.mutation_profile.rate(*kind).one_in)
                .filter(|rate| *rate > 0.0)
                .collect();
            if positive.is_empty() {
                continue;
            }
            let maximum = self
                .config
                .mutation_rate_limit
                .map_or(2_000_000_000.0, |limit| {
                    genome.dna.blocks().len() as f64 * limit
                });
            let normalized =
                (positive.iter().sum::<f64>() / positive.len() as f64 * 3.0).min(maximum);
            let profile = &mut genome.mutation_profile;
            profile.point_value_change_percent =
                ((f64::from(profile.point_value_change_percent) * (blend - 1.0) + 80.0) / blend)
                    .round_ties_even() as u8;
            profile.copy_error_value_change_percent =
                ((f64::from(profile.copy_error_value_change_percent) * (blend - 1.0) + 80.0)
                    / blend)
                    .round_ties_even() as u8;
            for kind in KINDS {
                let standard = defaults.rate(kind);
                let rate = profile.rate_mut(kind);
                if rate.one_in > 0.0 {
                    rate.one_in =
                        f64::from(((rate.one_in * (blend - 1.0) + normalized) / blend) as f32);
                }
                rate.mean = f64::from(((rate.mean * (blend - 1.0) + standard.mean) / blend) as f32);
                rate.std_dev =
                    f64::from(((rate.std_dev * (blend - 1.0) + standard.std_dev) / blend) as f32);
            }
        }
    }

    pub fn begin_test(&mut self, candidate: Vec<EvolutionGenome>) -> Result<(), String> {
        if candidate.is_empty() {
            return Err("evolution test needs candidate genomes".into());
        }
        self.candidate = candidate;
        self.phase = EvolutionPhase::Testing;
        self.opponent_stage = self.stage;
        self.reset_attempt();
        Ok(())
    }

    pub fn test_won(&mut self) -> Result<(), String> {
        if self.candidate.is_empty() {
            return Err("cannot promote an empty evolution candidate".into());
        }
        self.test_wins = self.test_wins.saturating_add(1);
        let opponent = (f64::from(self.stage)
            - f64::from(self.test_wins) * f64::from(self.stage).powf(1.0 / 3.0))
        .round_ties_even() as i64;
        if opponent < 0 || self.stage == 0 || self.config.mode == EvolutionMode::Eco {
            self.promote();
        } else {
            self.opponent_stage = opponent as u32;
        }
        self.reset_attempt();
        Ok(())
    }

    fn promote(&mut self) {
        self.test_wins = 0;
        self.decreasing_difficulty = !self.decreasing_difficulty;
        self.correction = 5.0;
        self.initial_hide_cycles = self.hide_cycles;
        if self.config.normalize_dna_size {
            let length = self.candidate[0].dna.blocks().len();
            let target = self.target_dna_size;
            let rate = ((5_000.0 - target as f64) / 4_750.0).max(0.0);
            self.current_dna_size = if length < target { length + 5 } else { target };
            if length as f64 >= target as f64 - 15.0 && length as f64 <= target as f64 + rate * 75.0
            {
                self.target_dna_size =
                    (target as f64 + rate * 250.0 + 10.0).round_ties_even() as usize;
            }
        }
        self.stage += 1;
        self.mutators = std::mem::take(&mut self.candidate);
        if let Some(retired) = self.stages.insert(self.stage, self.mutators.clone()) {
            self.retired_stages.push(retired);
        }
        self.opponent_stage = self.stage;
        self.phase = EvolutionPhase::Evolving;
        self.events.push(format!("Starting stage {}", self.stage));
    }

    pub(crate) fn test_lost(&mut self, random: &mut NativeRandom) {
        self.candidate.clear();
        if self.config.mode != EvolutionMode::Eco {
            self.test_wins = 0;
        }
        self.opponent_stage = self.stage;
        self.phase = EvolutionPhase::Evolving;
        self.increase_difficulty(random);
        self.reset_attempt();
    }

    pub fn reset_attempt(&mut self) {
        self.hidden = false;
        self.mode_cycles = 0;
        self.hide_offset = 0.0;
        self.energy_delta = 0.0;
        self.previous_energy_delta = 0.0;
        self.inverse_delta = 0.0;
        self.previous_inverse_delta = 0.0;
        self.handicap = 0.0;
        self.inverse_handicap = 0.0;
        self.cycle_in_attempt = 0;
        self.stagnant = false;
        self.zerobot_previous_id = 0;
        self.zerobot_previous_score = 0.0;
    }

    pub fn current_handicap(&self) -> f64 {
        (self.handicap - self.inverse_handicap)
            * (self.cycle_in_attempt as f64 / (f64::from(self.hide_cycles) * 8.0)).min(1.0)
    }

    /// calculateZB tracks the absolute identity of a mutated best robot, then
    /// compares its later score. A different best robot alone is not progress.
    pub fn observe_zerobot(
        &mut self,
        absolute_id: u64,
        score: f64,
        mutated: bool,
        genome: &mut EvolutionGenome,
    ) -> bool {
        if !mutated {
            return false;
        }
        let maximum = self
            .config
            .mutation_rate_limit
            .map_or(2_000_000_000.0, |limit| {
                genome.dna.blocks().len() as f64 * limit
            });
        let progress =
            self.zerobot_previous_id == absolute_id && score > self.zerobot_previous_score;
        let factor = if progress {
            1.75
        } else if self.zerobot_previous_id != 0 && self.zerobot_previous_id != absolute_id {
            1.15
        } else {
            1.0
        };
        if factor != 1.0 {
            genome.mutation_profile.point.one_in =
                f64::from((genome.mutation_profile.point.one_in * factor).min(maximum) as f32);
            genome.mutation_profile.point2.one_in =
                f64::from((genome.mutation_profile.point2.one_in * factor).min(maximum) as f32);
        }
        self.zerobot_previous_id = absolute_id;
        self.zerobot_previous_score = score;
        if !progress {
            return false;
        }
        self.stage += 1;
        if let Some(retired) = self.stages.insert(self.stage, vec![genome.clone()]) {
            self.retired_stages.push(retired);
        }
        self.candidate = vec![genome.clone()];
        let lowest = self.stage.saturating_sub(7);
        self.mutators = (0..8)
            .map(|index| self.stages[&(lowest + index % (self.stage + 1))][0].clone())
            .collect();
        self.phase = EvolutionPhase::ZerobotTesting;
        self.reset_attempt();
        true
    }

    pub fn validate(&self) -> Result<(), String> {
        self.config.validate()?;
        if !self.reduction_unit.is_finite()
            || !(0.01..=150.0).contains(&self.reduction_unit)
            || !self.correction.is_finite()
            || self.correction <= 0.0
        {
            return Err("invalid saved evolution difficulty".into());
        }
        if self.stages.get(&self.stage).is_none_or(Vec::is_empty)
            || !self.stages.contains_key(&self.opponent_stage)
            || self.mutators.is_empty()
        {
            return Err("saved evolution stage or opponent genome is missing".into());
        }
        if matches!(
            self.phase,
            EvolutionPhase::Testing | EvolutionPhase::ZerobotTesting
        ) && self.candidate.is_empty()
        {
            return Err("saved evolution test has no candidate genome".into());
        }
        if self.hide_cycles == 0
            || self.hide_cycles > 15_000
            || self.initial_hide_cycles == 0
            || !self.hide_offset.is_finite()
            || self.hide_offset < 0.0
            || [
                self.energy_delta,
                self.previous_energy_delta,
                self.inverse_delta,
                self.previous_inverse_delta,
                self.handicap,
                self.inverse_handicap,
                self.zerobot_previous_score,
            ]
            .iter()
            .any(|value| !value.is_finite())
        {
            return Err("invalid saved evolution cycle or handicap state".into());
        }
        for genome in self
            .stages
            .values()
            .flatten()
            .chain(&self.mutators)
            .chain(&self.candidate)
        {
            genome
                .mutation_profile
                .validate()
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state() -> EvolutionState {
        EvolutionState::new(
            EvolutionConfig::default(),
            vec![EvolutionGenome {
                dna: LegacyDna::parse("start stop").unwrap(),
                mutation_profile: StandardMutationProfile::default(),
            }],
        )
        .unwrap()
    }

    #[test]
    fn native_range_scaling_rounds_before_integer_division() {
        assert_eq!(n10(1.0), 10.0);
        assert_eq!(n10(0.1), 100.0);
        assert_eq!(n10(0.01), 1_000.0);
        assert_eq!(n10(1.01), 1.0);
    }

    #[test]
    fn difficulty_reversal_halves_correction_and_preserves_random_continuation() {
        let mut state = state();
        let mut random = NativeRandom::with_user_seed(42);
        state.increase_difficulty(&mut random);
        assert_eq!(state.correction, 2.5);
        assert_eq!(state.reduction_unit, 7.5);
        let mut resumed: EvolutionState =
            serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
        let mut resumed_random = random.clone();
        state.decrease_difficulty(&mut random);
        resumed.decrease_difficulty(&mut resumed_random);
        assert_eq!(state, resumed);
        assert_eq!(state.correction, 1.25);
        assert_eq!(state.reduction_unit, 8.75);
        assert_eq!(random, resumed_random);
    }

    #[test]
    fn stage_zero_advances_and_later_tests_visit_older_stages() {
        let mut state = state();
        let seed = state.stages[&0].clone();
        state.begin_test(seed.clone()).unwrap();
        state.test_won().unwrap();
        assert_eq!(state.stage, 1);
        assert_eq!(state.stages[&0], seed);
        state.begin_test(seed).unwrap();
        state.test_won().unwrap();
        assert_eq!(state.phase, EvolutionPhase::Testing);
        assert_eq!(state.opponent_stage, 0);
        state.test_won().unwrap();
        assert_eq!(state.stage, 2);
        assert_eq!(state.stages.len(), 3);
    }

    #[test]
    fn handicap_ramps_over_eight_hide_intervals() {
        let mut state = state();
        state.hide_cycles = 200;
        state.handicap = -4.0;
        state.inverse_handicap = -1.0;
        state.cycle_in_attempt = 800;
        assert_eq!(state.current_handicap(), -1.5);
        state.cycle_in_attempt = 2_000;
        assert_eq!(state.current_handicap(), -3.0);
    }

    #[test]
    fn mutation_adjustments_leave_archived_baseline_rates_unchanged() {
        let mut state = state();
        let archive = state.stages[&0].clone();
        state.scale_mutations();
        assert_ne!(state.mutators, archive);
        state.renormalize_mutations();
        assert_eq!(state.stages[&0], archive);
    }

    #[test]
    fn zerobot_requires_improvement_by_the_same_mutated_robot_and_builds_eight_groups() {
        let mut state = state();
        let mut genome = state.mutators[0].clone();
        assert!(!state.observe_zerobot(1, 500.0, false, &mut genome));
        assert!(!state.observe_zerobot(1, 500.0, true, &mut genome));
        assert!(!state.observe_zerobot(2, 700.0, true, &mut genome));
        assert!(state.observe_zerobot(2, 701.0, true, &mut genome));
        assert_eq!(state.phase, EvolutionPhase::ZerobotTesting);
        assert_eq!(state.mutators.len(), 8);
        assert_eq!(state.stage, 1);
        assert!(state.validate().is_ok());
    }
}
