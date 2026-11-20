//! Master.bas 243-302: population history, incremental costs and low-water latch.
use super::*;

impl DynamicCostRuntimeState {
    fn advance(&mut self, costs: &DnaCosts, population: usize, cycle: u64) {
        let population = population.min(MAX_ROBOTS) as u32;
        if cycle.is_multiple_of(10) {
            self.population_history.copy_within(0..9, 1);
            self.population_history[0] = population;
        }
        let mut multiplier = self.active_multiplier.unwrap_or(costs.global_multiplier) as f32;
        if costs.dynamic_cost_multiplier != 0.0 {
            let amount_off =
                (f64::from(population) - costs.dynamic_target_population as f64) as f32;
            let upper = (costs.dynamic_upper_range_percent
                * 0.01
                * costs.dynamic_target_population as f64) as f32;
            let lower = (costs.dynamic_lower_range_percent
                * 0.01
                * costs.dynamic_target_population as f64) as f32;
            let old_population = self.population_history[9];
            if population == old_population {
                self.countdown = (self.countdown - 1).max(-10);
            } else {
                self.countdown = 10;
            }
            if (amount_off > upper && (old_population < population || self.countdown <= 0))
                || (amount_off < -lower && (old_population > population || self.countdown <= 0))
            {
                let correction = if amount_off > upper {
                    amount_off - upper
                } else {
                    amount_off.abs() - lower
                };
                multiplier = (f64::from(multiplier)
                    + 0.0000001
                        * f64::from(correction)
                        * f64::from(amount_off.signum())
                        * costs.dynamic_sensitivity) as f32;
                if !costs.dynamic_allow_negative {
                    multiplier = multiplier.max(0.0);
                }
                self.countdown = 10;
            }
        }
        if f64::from(population) < costs.dynamic_lower_zero_threshold && multiplier != 0.0 {
            self.zeroed = true;
            self.saved_multiplier = f64::from(multiplier);
            multiplier = 0.0;
        } else if f64::from(population) > costs.dynamic_lower_reinstate_threshold && self.zeroed {
            self.zeroed = false;
            multiplier = self.saved_multiplier as f32;
        }
        self.active_multiplier = Some(f64::from(multiplier));
    }
}

impl Engine {
    pub(super) fn update_dynamic_cost_state(&mut self) {
        let population = self.cost_population();
        let cycle = self.next_native_cycle_or_tick();
        self.dynamic_cost_state
            .advance(&self.config.dna_costs, population, cycle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn costs() -> DnaCosts {
        DnaCosts {
            global_multiplier: 1.0,
            dynamic_cost_multiplier: 1.0,
            dynamic_target_population: 100,
            dynamic_upper_range_percent: 10.0,
            dynamic_lower_range_percent: 20.0,
            dynamic_sensitivity: 10.0,
            ..Default::default()
        }
    }

    #[test]
    fn cost_ratcheting_uses_direction_deadband_and_stagnation() {
        let c = costs();
        let mut state = DynamicCostRuntimeState {
            population_history: [100; 10],
            ..Default::default()
        };
        state.advance(&c, 120, 1);
        assert_eq!(state.active_multiplier, Some(f64::from(1.00001_f32)));
        state.population_history = [130; 10];
        state.advance(&c, 120, 2);
        assert_eq!(
            state.active_multiplier,
            Some(f64::from(1.00001_f32)),
            "recovering population does not raise costs"
        );
        state.population_history = [120; 10];
        state.countdown = 1;
        state.advance(&c, 120, 3);
        assert_eq!(state.active_multiplier, Some(f64::from(1.00002_f32)));
        let before = state.active_multiplier;
        state.advance(&c, 105, 4);
        assert_eq!(state.active_multiplier, before);
    }

    #[test]
    fn lower_band_decreases_costs_and_negative_costs_are_explicit() {
        let mut c = costs();
        c.global_multiplier = 0.000001;
        let initial = DynamicCostRuntimeState {
            population_history: [100; 10],
            ..Default::default()
        };
        let mut state = initial.clone();
        state.advance(&c, 60, 1);
        assert_eq!(state.active_multiplier, Some(0.0));
        c.dynamic_allow_negative = true;
        let mut state = initial;
        state.advance(&c, 60, 1);
        assert!(state.active_multiplier.unwrap() < 0.0);
    }

    #[test]
    fn history_shifts_each_tenth_cycle_and_low_water_saves_the_adjusted_cost() {
        let mut c = costs();
        c.dynamic_lower_zero_threshold = 200.0;
        c.dynamic_lower_reinstate_threshold = 210.0;
        let mut state = DynamicCostRuntimeState::default();
        state.advance(&c, 120, 9);
        assert_eq!(state.population_history, [0; 10]);
        assert_eq!(state.active_multiplier, Some(0.0));
        assert_eq!(state.saved_multiplier, f64::from(1.00001_f32));
        c.dynamic_cost_multiplier = 0.0;
        state.advance(&c, 205, 10);
        assert_eq!(state.population_history[0], 205);
        assert_eq!(state.active_multiplier, Some(0.0));
        state.advance(&c, 211, 11);
        assert_eq!(state.active_multiplier, Some(f64::from(1.00001_f32)));
        assert!(!state.zeroed);
    }

    #[test]
    fn modern_resume_keeps_cost_history_and_corpses_do_not_enter_the_population() {
        let mut world = Engine::new(EngineConfig {
            dna_costs: costs(),
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        world.become_native_corpse(id).unwrap();
        assert_eq!(world.cost_population(), 0);
        world.dynamic_cost_state.population_history = [120; 10];
        world.dynamic_cost_state.countdown = 2;
        world.dynamic_cost_state.active_multiplier = Some(f64::from(1.25_f32));
        let resumed = crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(resumed.dynamic_cost_state.population_history, [120; 10]);
        assert_eq!(resumed.dynamic_cost_state.countdown, 2);
        assert_eq!(resumed.active_dna_costs().dynamic_multiplier(999), 1.25);
        let native = Engine::from_native_sim(&world.export_native_sim().unwrap()).unwrap();
        assert_eq!(native.config.dna_costs.global_multiplier, 1.25);
    }
}
