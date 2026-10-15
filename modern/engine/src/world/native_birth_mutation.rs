//! Robots.bas reproductive Delta2 boot and MREPRO's temporary tenfold rates.
use super::*;

impl Engine {
    pub(super) fn native_birth_mutations(&mut self, child: OrganismId, mutated_reproduction: bool) {
        let settings = self.config.advanced_mutation.clone();
        let base_multiplier = f64::from(self.global_mutation_multiplier);
        if !settings.delta2.enabled {
            if self.mutations_enabled {
                self.apply_mutation_to_organism(
                    child,
                    &settings,
                    base_multiplier,
                    MutationContext::Reproduction {
                        forced: mutated_reproduction,
                    },
                    None,
                );
            }
            return;
        }
        let Ok(robot) = self.slots.get(child) else {
            return;
        };
        let length = robot.program.dna.blocks().len();
        let (dmoc, zerobot) = self
            .local_mode
            .evolution
            .as_ref()
            .map_or((1.0, false), |state| {
                (
                    if state.config.normalize_dna_size {
                        (1.0 + (length as f64 - state.current_dna_size as f64) / 500.0).max(0.01)
                    } else {
                        1.0
                    },
                    state.config.mode == crate::EvolutionMode::Zerobot
                        && state.phase == crate::EvolutionPhase::Evolving
                        && state.mutator_species.contains(&robot.identity.species),
                )
            });
        if zerobot {
            let maximum = if settings.normalization_enabled {
                length as f64 * f64::from(settings.native_delta2.maximum_rate_per_base)
            } else {
                2_000_000_000.0
            };
            let profile = &mut self.slots.get_mut(child).unwrap().program.mutation_profile;
            for kind in [StandardMutationKind::Point, StandardMutationKind::Point2] {
                let rate = f64::from((profile.rate(kind).one_in * 1.75).min(maximum) as f32);
                profile.rate_mut(kind).one_in = rate;
            }
        }
        // VB evaluates the For bound's Rnd even when MREPRO is false.
        let bound = (self.native_random.as_mut().unwrap().unit() * 3.0).floor() as usize + 1;
        let passes = if mutated_reproduction { bound + 1 } else { 1 };
        for _ in 0..passes {
            let mut mutator =
                GenomeMutator::with_native_random(self.native_random.clone().unwrap());
            let profile = &mut self.slots.get_mut(child).unwrap().program.mutation_profile;
            mutator.native_delta2_rates(profile, &settings, length, true, dmoc);
            self.native_random = mutator.native_random();
            if self.mutations_enabled {
                // Complete mutation/colour/subspecies processing before the next
                // rate boot so all passes share the original global draw order.
                self.apply_mutation_to_organism(
                    child,
                    &settings,
                    base_multiplier,
                    MutationContext::Reproduction { forced: false },
                    None,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reproductive_delta2_consumes_repeat_bound_with_global_gene_mutation_disabled() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2])
            .unwrap();
        world.config.advanced_mutation.delta2.enabled = true;
        world.mutations_enabled = false;
        let profile = &mut world.slots.get_mut(id).unwrap().program.mutation_profile;
        for kind in [
            StandardMutationKind::Point,
            StandardMutationKind::Point2,
            StandardMutationKind::Delta,
            StandardMutationKind::CopyError,
            StandardMutationKind::CopyError2,
            StandardMutationKind::Insertion,
            StandardMutationKind::Reversal,
            StandardMutationKind::MinorDeletion,
            StandardMutationKind::MajorDeletion,
            StandardMutationKind::Amplification,
            StandardMutationKind::Translocation,
        ] {
            profile.rate_mut(kind).one_in = 0.0;
        }
        let mut expected = world.native_random.clone().unwrap();
        expected.unit(); // bound is evaluated although MREPRO is false
        expected.unit(); // CopyErrorWhatToChange update after the skipped rate loop
        world.native_birth_mutations(id, false);
        assert_eq!(world.native_random.as_ref().unwrap(), &expected);
    }
}
