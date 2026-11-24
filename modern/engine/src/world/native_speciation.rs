//! NeoMutations.mutate forks by total Mutations, with a strict threshold.
use super::*;

impl Engine {
    pub fn fork_robot_and_descendants(&mut self, id: OrganismId) -> Result<String, EngineError> {
        let robot = self.slots.get(id)?;
        if !robot.flags.exists || robot.flags.corpse {
            return Err(EngineError::Invariant(
                "a new species requires a living robot".to_owned(),
            ));
        }
        let species = robot.identity.species;
        let mut definition = self
            .species
            .get(species.0 as usize)
            .cloned()
            .ok_or_else(|| {
                EngineError::Invariant(format!("species {} does not exist", species.0))
            })?;
        let founder = if robot.identity.species_file.is_empty() {
            &definition.name
        } else {
            &robot.identity.species_file
        };
        let base: String = founder
            .strip_suffix(".txt")
            .unwrap_or(founder)
            .chars()
            .take(28)
            .collect();
        definition.vegetable = robot.flags.vegetable;
        definition.color = robot.phenotype.color;
        definition.skin = robot.phenotype.skin;
        definition.custom_skin = true;
        definition.minimum_population = 0;
        definition.reseed = false;
        definition.placement_bounds = [0.0, 1.0, 0.0, 1.0];
        definition.lineage_id = 0;
        definition.mutation_rate = 0.0;
        definition.mutation_profile =
            Some(self.native_default_mutation_profile(robot.program.dna.blocks().len()));

        let descendants = self.living_fork_slots(id.slot() as usize);
        let robot = self.slots.get_mut(id)?;
        let suffix = world_random_integer(
            &mut self.native_random,
            &mut robot.program.rng_state,
            1,
            10_000,
        );
        let name = format!("{base} {suffix}");
        definition.name = name.clone();
        let new_species = self.register_species_unpublished(definition);
        for slot in descendants {
            if let Some(robot) = &mut self.slots[slot].organism {
                robot.identity.species = new_species;
                robot.identity.species_file.clone_from(&name);
            }
        }
        self.publish_snapshot();
        Ok(name)
    }

    fn living_fork_slots(&self, root: usize) -> HashSet<usize> {
        let mut children = HashMap::<u64, Vec<usize>>::new();
        let mut older_children = HashMap::<OrganismId, Vec<usize>>::new();
        for (slot, entry) in self.slots.iter().enumerate() {
            let Some(robot) = entry
                .organism
                .as_ref()
                .filter(|robot| robot.flags.exists && !robot.flags.corpse)
            else {
                continue;
            };
            if robot.identity.parent_absolute_id != 0 {
                children
                    .entry(robot.identity.parent_absolute_id)
                    .or_default()
                    .push(slot);
            } else if let Some(parent) = robot.identity.parents[0] {
                // Older Modern saves may only retain the stable slot/generation key.
                older_children.entry(parent).or_default().push(slot);
            }
        }

        let mut pending = vec![root];
        let mut visited = HashSet::new();
        while let Some(slot) = pending.pop() {
            if !visited.insert(slot) {
                continue;
            }
            let entry = &self.slots[slot];
            let Some(robot) = entry
                .organism
                .as_ref()
                .filter(|robot| robot.counters.offspring > 0)
            else {
                continue;
            };
            if let Some(next) = children.get(&robot.identity.absolute_id) {
                pending.extend(next);
            }
            if let Some(next) = older_children.get(&OrganismId::new(slot as u32, entry.generation))
            {
                pending.extend(next);
            }
        }
        visited
    }

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
    fn manual_fork_preserves_live_state_and_follows_only_extant_parent_chains() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let root = world.spawn_at(dna.clone(), [200.0, 200.0]).unwrap();
        world.slots.get_mut(root).unwrap().identity.species_file =
            "Long founder filename for the selected robot.txt".to_owned();
        let child = world.clone_organism(root, [400.0, 200.0]).unwrap();
        let grandchild = world.clone_organism(child, [600.0, 200.0]).unwrap();
        let corpse = world.clone_organism(root, [200.0, 400.0]).unwrap();
        let past_corpse = world.clone_organism(corpse, [400.0, 400.0]).unwrap();
        world.slots.get_mut(corpse).unwrap().flags.corpse = true;
        let stranger = world.spawn_at(dna, [600.0, 400.0]).unwrap();
        let pre_fork = world.slots.get(child).unwrap().clone();
        let old_species = pre_fork.identity.species;
        let tick = world.snapshot().tick;

        assert!(world.fork_robot_and_descendants(corpse).is_err());
        let name = world.fork_robot_and_descendants(root).unwrap();
        let (base, suffix) = name.rsplit_once(' ').unwrap();
        assert_eq!(base, "Long founder filename for th");
        assert!((1..=10_000).contains(&suffix.parse::<i32>().unwrap()));
        let new_species = world.slots.get(root).unwrap().identity.species;
        assert_ne!(old_species, new_species);
        assert_eq!(world.species[new_species.0 as usize].name, name);
        for id in [root, child, grandchild] {
            let robot = world.slots.get(id).unwrap();
            assert_eq!(robot.identity.species, new_species);
            assert_eq!(robot.identity.species_file, name);
        }
        for id in [corpse, past_corpse, stranger] {
            assert_eq!(world.slots.get(id).unwrap().identity.species, old_species);
        }
        let mut after = world.slots.get(child).unwrap().clone();
        after.identity = pre_fork.identity.clone();
        assert_eq!(after, pre_fork);
        assert_eq!(world.snapshot().tick, tick);
    }

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
