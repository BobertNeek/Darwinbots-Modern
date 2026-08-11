use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DnaCosts {
    pub number: f64,
    pub dereference: f64,
    pub basic: f64,
    pub advanced: f64,
    pub bitwise: f64,
    pub condition: f64,
    pub logic: f64,
    pub store: f64,
    pub flow: f64,
    pub dna_upkeep: f64,
    pub dna_copy: f64,
    pub movement: f64,
    pub rotation: f64,
    pub tie_formation: f64,
    pub shot_formation: f64,
    pub venom: f64,
    pub poison: f64,
    pub slime: f64,
    pub shell: f64,
    pub body_upkeep: f64,
    pub chloroplast_construction: f64,
    pub virus: f64,
    pub age_base: f64,
    pub age_start: f64,
    pub age_linear: f64,
    pub age_logarithmic: f64,
    pub global_multiplier: f64,
    pub dynamic_cost_multiplier: f64,
    pub dynamic_all_robots: bool,
    pub dynamic_lower_zero_threshold: f64,
    pub dynamic_upper_zero_threshold: f64,
    pub dynamic_lower_reinstate_threshold: f64,
    pub dynamic_upper_reinstate_threshold: f64,
    pub dynamic_allow_negative: bool,
    pub dynamic_target_population: usize,
    pub dynamic_population_range: usize,
    pub dynamic_sensitivity: f64,
}

impl Default for DnaCosts {
    fn default() -> Self {
        Self {
            number: 0.0,
            dereference: 0.0,
            basic: 0.0,
            advanced: 0.0,
            bitwise: 0.0,
            condition: 0.0,
            logic: 0.0,
            store: 0.0,
            flow: 0.0,
            dna_upkeep: 0.0,
            dna_copy: 0.0,
            movement: 0.0,
            rotation: 0.0,
            tie_formation: 0.0,
            shot_formation: 0.0,
            venom: 0.0,
            poison: 0.0,
            slime: 0.0,
            shell: 0.0,
            body_upkeep: 0.0,
            chloroplast_construction: 0.0,
            virus: 0.0,
            age_base: 0.0,
            age_start: 0.0,
            age_linear: 0.0,
            age_logarithmic: 0.0,
            global_multiplier: 1.0,
            dynamic_cost_multiplier: 0.0,
            dynamic_all_robots: true,
            dynamic_lower_zero_threshold: 0.0,
            dynamic_upper_zero_threshold: 0.0,
            dynamic_lower_reinstate_threshold: 0.0,
            dynamic_upper_reinstate_threshold: 0.0,
            dynamic_allow_negative: false,
            dynamic_target_population: 0,
            dynamic_population_range: 0,
            dynamic_sensitivity: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimulationCostAction {
    Movement,
    Rotation,
    TieFormation,
    ShotFormation,
    Venom,
    Poison,
    Slime,
    Shell,
    BodyUpkeep,
    DnaCopy,
    ChloroplastConstruction,
    Virus,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationCostInput {
    pub body: f64,
    pub dna_len: usize,
    pub age: u64,
    pub population: usize,
    pub action: Option<(SimulationCostAction, f64)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationCostReport {
    pub dna: f64,
    pub action: f64,
    pub upkeep: f64,
    pub age: f64,
    pub dynamic_multiplier: f64,
    pub total: f64,
}

impl DnaCosts {
    pub fn upkeep_cost(&self, body: f64, dna_len: usize, population: usize) -> f64 {
        (body.max(0.0) * self.body_upkeep + dna_len.saturating_sub(1) as f64 * self.dna_upkeep)
            * self.dynamic_multiplier(population)
    }

    pub fn age_cost(&self, age: u64, population: usize) -> f64 {
        let age = age as f64;
        if age <= 0.0 || age <= self.age_start {
            return 0.0;
        }
        let elapsed = age - self.age_start;
        (self.age_base
            + elapsed.max(0.0) * self.age_linear
            + (elapsed + 1.0).ln() * self.age_logarithmic)
            * self.dynamic_multiplier(population)
    }

    pub fn action_cost_for(
        &self,
        action: SimulationCostAction,
        amount: f64,
        population: usize,
    ) -> f64 {
        self.action_cost(action) * amount.max(0.0) * self.dynamic_multiplier(population)
    }

    pub fn calculate_cost(&self, input: SimulationCostInput) -> SimulationCostReport {
        let dna = input.dna_len.saturating_sub(1) as f64 * self.dna_upkeep;
        let upkeep = input.body.max(0.0) * self.body_upkeep + dna;
        let age = if input.age as f64 > 0.0 && input.age as f64 > self.age_start {
            let elapsed = input.age as f64 - self.age_start;
            self.age_base
                + elapsed.max(0.0) * self.age_linear
                + (elapsed + 1.0).ln() * self.age_logarithmic
        } else {
            0.0
        };
        let action = input
            .action
            .map(|(kind, amount)| self.action_cost(kind) * amount.max(0.0))
            .unwrap_or(0.0);
        let dynamic_multiplier = self.dynamic_multiplier(input.population);
        SimulationCostReport {
            dna,
            action,
            upkeep,
            age,
            dynamic_multiplier,
            total: (dna + action + upkeep + age) * dynamic_multiplier,
        }
    }

    pub fn dynamic_multiplier(&self, population: usize) -> f64 {
        let global = if self.global_multiplier.is_finite() {
            self.global_multiplier
        } else {
            1.0
        };
        if self.dynamic_cost_multiplier == 0.0
            || self.dynamic_population_range == 0
            || !self.dynamic_sensitivity.is_finite()
        {
            return global;
        }
        let distance = self.dynamic_target_population as f64 - population as f64;
        let population_factor = distance / self.dynamic_population_range as f64;
        let mut multiplier = global
            * (1.0 + self.dynamic_cost_multiplier * population_factor * self.dynamic_sensitivity);
        if !self.dynamic_allow_negative {
            multiplier = multiplier.max(0.0);
        }
        if self.dynamic_lower_zero_threshold > 0.0 && distance >= self.dynamic_lower_zero_threshold
            || self.dynamic_upper_zero_threshold > 0.0
                && -distance >= self.dynamic_upper_zero_threshold
        {
            multiplier = 0.0;
        }
        multiplier
    }

    fn action_cost(&self, action: SimulationCostAction) -> f64 {
        match action {
            SimulationCostAction::Movement => self.movement,
            SimulationCostAction::Rotation => self.rotation,
            SimulationCostAction::TieFormation => self.tie_formation,
            SimulationCostAction::ShotFormation => self.shot_formation,
            SimulationCostAction::Venom => self.venom,
            SimulationCostAction::Poison => self.poison,
            SimulationCostAction::Slime => self.slime,
            SimulationCostAction::Shell => self.shell,
            SimulationCostAction::BodyUpkeep => self.body_upkeep,
            SimulationCostAction::DnaCopy => self.dna_copy,
            SimulationCostAction::ChloroplastConstruction => self.chloroplast_construction,
            SimulationCostAction::Virus => self.virus,
        }
    }
}
