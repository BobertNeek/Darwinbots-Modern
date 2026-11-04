//! NeoMutations.mutate forks by total Mutations, with a strict threshold.
use super::*;

impl Engine {
    pub(super) fn native_default_mutation_profile(&self, length: usize) -> StandardMutationProfile {
        let mut profile = StandardMutationProfile::default();
        profile.point2.mean = 1.0;
        profile.point2.std_dev = 0.0;
        if self.config.advanced_mutation.normalization_enabled {
            let rate = f64::from(
                (length as f64 * self.config.advanced_mutation.normalization_dna_length as f64)
                    as f32,
            );
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
                profile.rate_mut(kind).one_in = rate;
            }
        }
        profile
    }

    pub(super) fn native_auto_fork_species(&mut self, id: OrganismId, original_color: u32) {
        if !self.config.auto_speciation || self.species.len() >= 49 {
            return;
        }
        let Ok(robot) = self.slots.get(id) else {
            return;
        };
        let length = robot.program.dna.blocks().len();
        let threshold =
            length as f64 * f64::from(self.config.speciation_genetic_distance_percent) / 100.0;
        if robot.program.mutation_metadata.total_mutations as f64 <= threshold {
            return;
        }
        let Some(mut definition) = self.species.get(robot.identity.species.0 as usize).cloned()
        else {
            return;
        };
        self.speciation_fork_interval = self.speciation_fork_interval.saturating_add(1);
        let old_name = if robot.identity.species_file.is_empty() {
            &definition.name
        } else {
            &robot.identity.species_file
        };
        definition.name = format!(
            "({}){}",
            self.speciation_fork_interval,
            strip_numeric_species_prefix(old_name)
        );
        definition.minimum_population = 0;
        definition.reseed = false;
        definition.placement_bounds = [0.0, 1.0, 0.0, 1.0];
        definition.color = original_color;
        definition.skin = robot.phenotype.skin;
        definition.custom_skin = true;
        definition.lineage_id = 0;
        definition.mutation_profile = Some(self.native_default_mutation_profile(length));
        definition.mutation_rate = 0.0;
        let new_species = self.register_species_unpublished(definition);
        let definition = &self.species[new_species.0 as usize];
        let robot = self.slots.get_mut(id).unwrap();
        robot.identity.species = new_species;
        robot.identity.species_file = definition.name.clone();
        robot.phenotype.lineage_id = definition.lineage_id;
        robot.phenotype.accumulated_mutations = 0;
        robot.program.mutation_metadata.total_mutations = 0;
        robot.program.dna.synchronize_runtime_metadata(
            robot.counters.generation,
            0,
            &robot.identity.tag,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_defaults_are_dna_length_times_the_native_per_base_setting() {
        let mut config = EngineConfig::testing();
        config.advanced_mutation.normalization_enabled = true;
        let mut world = Engine::new(config).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2])
            .unwrap();
        let profile = &world.slots.get(id).unwrap().program.mutation_profile;
        assert_eq!(profile.point.one_in, 3_213.0);
        assert_eq!(profile.point2.one_in, 3_213.0);
        assert_eq!(profile.point2.mean, 1.0);
        assert_eq!(profile.point2.std_dev, 0.0);
    }

    #[test]
    fn strict_threshold_preserves_pose_and_skin_and_resets_native_mutations() {
        let mut world = Engine::new(EngineConfig {
            auto_speciation: true,
            speciation_genetic_distance_percent: 100.0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2])
            .unwrap();
        let old = world.slots.get(id).unwrap().clone();
        let length = old.program.dna.blocks().len() as u64;
        world
            .slots
            .get_mut(id)
            .unwrap()
            .program
            .mutation_metadata
            .total_mutations = length;
        world.native_auto_fork_species(id, old.phenotype.color);
        assert_eq!(
            world.slots.get(id).unwrap().identity.species,
            old.identity.species
        );
        world
            .slots
            .get_mut(id)
            .unwrap()
            .program
            .mutation_metadata
            .total_mutations += 1;
        let random = world.native_random.clone();
        world.native_auto_fork_species(id, old.phenotype.color);
        let robot = world.slots.get(id).unwrap();
        assert_ne!(robot.identity.species, old.identity.species);
        assert!(robot.identity.species_file.starts_with("(1)"));
        assert_eq!(robot.program.mutation_metadata.total_mutations, 0);
        assert_eq!(robot.kinematics, old.kinematics);
        assert_eq!(robot.phenotype.skin, old.phenotype.skin);
        assert_eq!(world.native_random, random);
    }
}
