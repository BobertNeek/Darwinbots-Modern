//! Master.bas 203-231: separate high/low durations, 20^sin or 16/0.0625.
use super::*;

impl Engine {
    pub(super) fn update_native_mutation_oscillation(&mut self) {
        if self.native_random.is_none() { return; }
        let settings = self.config.advanced_mutation.oscillation;
        let period = settings.high_cycles.saturating_add(settings.low_cycles);
        if settings.mode == crate::MutationOscillationMode::Disabled || period == 0 { return; }
        let phase = self.tick.saturating_add(1) % period;
        self.global_mutation_multiplier = if settings.mode == crate::MutationOscillationMode::Sine {
            let angle = if phase < settings.high_cycles {
                phase as f64 / settings.high_cycles as f64 * std::f64::consts::PI
            } else {
                -((phase - settings.high_cycles) as f64) / settings.low_cycles as f64 * std::f64::consts::PI
            };
            20.0_f64.powf(angle.sin()) as f32
        } else if phase < settings.high_cycles { 16.0 } else { 0.0625 };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unequal_native_phases_use_original_exponential_and_square_values() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        world.config.advanced_mutation.oscillation = crate::MutationOscillationSettings {
            mode: crate::MutationOscillationMode::Sine, high_cycles: 4, low_cycles: 8, ..Default::default()
        };
        for (tick, expected) in [(1, 20.0), (3, 1.0), (7, 0.05), (11, 1.0)] {
            world.tick = tick;
            world.update_native_mutation_oscillation();
            assert!((world.global_mutation_multiplier - expected).abs() < 0.00001);
        }
        world.config.advanced_mutation.oscillation.mode = crate::MutationOscillationMode::Sawtooth;
        world.tick = 2;
        world.update_native_mutation_oscillation();
        assert_eq!(world.global_mutation_multiplier, 16.0);
        world.tick = 3;
        world.update_native_mutation_oscillation();
        assert_eq!(world.global_mutation_multiplier, 0.0625);
        let restored = Engine::from_native_sim(&world.export_native_sim().unwrap()).unwrap();
        assert_eq!(restored.config.advanced_mutation.oscillation, world.config.advanced_mutation.oscillation);
    }
}
