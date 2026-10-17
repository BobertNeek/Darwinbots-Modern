//! main.frm score(..., 1, 10, 0), using persistent native parent numbers.
use super::*;

impl Engine {
    pub(super) fn native_family_scores(&self) -> Vec<(u64, f64, bool)> {
        let mut children_by_parent = HashMap::<u64, (u64, f64, bool)>::new();
        // Ten bounded dynamic-programming passes reproduce the native recursive
        // depth, including retained corpses and duplicate absolute numbers.
        // This also bounds work for cyclic or densely repeated ancestry.
        for _ in 0..10 {
            let mut next = HashMap::<u64, (u64, f64, bool)>::new();
            for robot in self.slots.iter().filter_map(|s| s.organism.as_ref()) {
                let parent = if robot.identity.parent_absolute_id != 0 {
                    robot.identity.parent_absolute_id
                } else {
                    robot.identity.parents[0]
                        .and_then(|id| self.slots.get(id).ok())
                        .map_or(0, |state| state.identity.absolute_id)
                };
                if parent == 0 {
                    continue;
                }
                // main.frm3077-3081: both operands are Single. Round the
                // product and sum before assigning InvestedEnergy to Double.
                let invested = f64::from(robot.materials.energy + robot.materials.body * 10.0_f32);
                let descendants = children_by_parent
                    .get(&robot.identity.absolute_id)
                    .copied()
                    .unwrap_or_default();
                let entry = next.entry(parent).or_default();
                entry.0 = entry.0.saturating_add(1).saturating_add(descendants.0);
                // score3053-3054 adds the recursive Double result first,
                // then this child's already-rounded InvestedEnergy.
                entry.1 += descendants.1;
                entry.1 += invested;
                entry.2 |= invested < 1000.0 || descendants.2;
            }
            children_by_parent = next;
        }
        self.slots
            .iter()
            .map(|slot| {
                let Some(robot) = slot.organism.as_ref() else {
                    return (0, 0.0, false);
                };
                let descendants = children_by_parent
                    .get(&robot.identity.absolute_id)
                    .copied()
                    .unwrap_or_default();
                (
                    descendants.0.saturating_add(1),
                    descendants.1
                        + f64::from(robot.materials.energy)
                        // main.frm3008 keeps the root sum Double, while
                        // body * Integer10 is still a Single product.
                        + f64::from(robot.materials.body * 10.0_f32),
                    descendants.2,
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ancestry_cycles_remain_bounded_at_the_original_ten_descendant_levels() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        let robot = world.slots.get_mut(id).unwrap();
        robot.identity.parent_absolute_id = robot.identity.absolute_id;
        let invested = f64::from(robot.materials.energy) + f64::from(robot.materials.body) * 10.0;
        let scores = world.native_family_scores();
        assert_eq!(scores[id.slot() as usize].0, 11);
        assert_eq!(scores[id.slot() as usize].1, 11.0 * invested);
    }
    #[test]
    fn manual_zerobot_selection_rounds_child_investment_before_the_cancer_threshold() {
        let mut world = Engine::new_native(EngineConfig {
            organism_capacity: 4,
            ..EngineConfig::testing()
        })
        .unwrap();
        world.set_native_restart_seed(42);
        world
            .configure_local_mode(LocalModeConfig {
                kind: LocalModeKind::Zerobot,
                staged_evolution: Some(crate::EvolutionConfig {
                    mode: crate::EvolutionMode::Zerobot,
                    ..Default::default()
                }),
                ..Default::default()
            })
            .unwrap();
        world.start_local_mode().unwrap();
        let (mutators, baselines): (Vec<_>, Vec<_>) =
            world.slots.ids_in_slot_order().into_iter().partition(|id| {
                native_census_filename(world.slots.get(*id).unwrap(), &world.species)
                    == "Mutate.txt"
            });
        world.remove_many(&baselines).unwrap();
        assert_eq!(mutators.len(), 2);
        let parent = mutators[0];
        let rival = mutators[1];
        for (id, energy) in [(parent, 10_000.0), (rival, 5_000.0)] {
            let robot = world.slots.get_mut(id).unwrap();
            robot.materials.energy = energy;
            robot.materials.body = 0.0;
            robot.program.mutation_metadata.native_last_mutations = 1;
        }
        let parent_absolute_id = world.slots.get(parent).unwrap().identity.absolute_id;
        let child = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        let robot = world.slots.get_mut(child).unwrap();
        robot.identity.parent_absolute_id = parent_absolute_id;
        robot.flags.vegetable = true; // ancestry includes it, candidate selection does not
        robot.materials.body = 99.99999_f32;
        robot.materials.energy = 0.00004_f32;
        // Original Single product + Single sum yields exactly 1000. Widening
        // the inputs first yields 999.9999637060537 and wrongly marks cancer.
        let family = world.native_family_scores()[parent.slot() as usize];
        assert_eq!(family, (2, 11_000.0, false));
        assert_eq!(world.find_best_robot_weighted(29), Some(parent));
        assert_eq!(
            world.select_best_robot(200, Some(777)).unwrap(),
            (Some(parent), false, 29)
        );
        assert_eq!(
            world
                .local_mode
                .evolution
                .as_ref()
                .unwrap()
                .zerobot_previous_id,
            parent_absolute_id
        );
    }

    #[test]
    fn native_family_scores_preserve_single_products_and_double_accumulation_order() {
        let mut world = Engine::new_native(EngineConfig {
            organism_capacity: 4,
            ..EngineConfig::testing()
        })
        .unwrap();
        let root = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        let robot = world.slots.get_mut(root).unwrap();
        robot.materials.body = 99.99999_f32;
        robot.materials.energy = 0.00004_f32;
        // main.frm3008 starts with a Double score: only body*10 is Single,
        // so the complete root expression must not be rounded to Single.
        assert_eq!(
            world.native_family_scores()[root.slot() as usize],
            (1, 999.9999789648427, false)
        );

        let root_absolute_id = world.slots.get(root).unwrap().identity.absolute_id;
        let large = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [600.0; 2])
            .unwrap();
        let branch = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [700.0; 2])
            .unwrap();
        let leaf = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [800.0; 2])
            .unwrap();
        let branch_absolute_id = world.slots.get(branch).unwrap().identity.absolute_id;
        for (id, energy, parent) in [
            (root, 0.0, 0),
            (large, f32::from_bits(0x5a000000), root_absolute_id), // exactly 2^53, a finite native Single
            (branch, 1.0, root_absolute_id),
            (leaf, 1.0, branch_absolute_id),
        ] {
            let robot = world.slots.get_mut(id).unwrap();
            robot.materials.energy = energy;
            robot.materials.body = 0.0;
            robot.identity.parent_absolute_id = parent;
        }
        // score3053-3054 adds descendant 1, then invested 1. Combining those
        // first produces 2^53+2; the original two Double additions retain 2^53.
        assert_eq!(
            world.native_family_scores()[root.slot() as usize],
            (4, 9_007_199_254_740_992.0, true)
        );
    }

    #[test]
    fn manual_find_best_excludes_literal_corpse_filename_without_filtering_descendants() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        let living = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        let named_corpse = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [600.0; 2])
            .unwrap();
        for (id, energy) in [(living, 1_000.0), (named_corpse, 2_000.0)] {
            let robot = world.slots.get_mut(id).unwrap();
            robot.materials.energy = energy;
            robot.materials.body = 0.0;
        }
        world
            .slots
            .get_mut(named_corpse)
            .unwrap()
            .identity
            .species_file = "Corpse".into();
        assert!(!world.slots.get(named_corpse).unwrap().flags.corpse);
        world.publish_snapshot();
        // Keep the existing explicit query contract; only the manual source
        // action adds main.frm3005's literal filename candidate exclusion.
        let before = crate::SaveFile::encode(&world).unwrap();
        assert_eq!(world.find_best_robot_weighted(200), Some(named_corpse));
        assert_eq!(
            world.select_best_robot(200, Some(777)).unwrap(),
            (Some(living), false, 200)
        );
        assert_eq!(crate::SaveFile::encode(&world).unwrap(), before);
        assert_eq!(world.find_best_robot_weighted(200), Some(named_corpse));
        let living_absolute_id = world.slots.get(living).unwrap().identity.absolute_id;
        world
            .slots
            .get_mut(named_corpse)
            .unwrap()
            .identity
            .parent_absolute_id = living_absolute_id;
        assert_eq!(
            world.native_family_scores()[living.slot() as usize],
            (2, 3_000.0, false)
        );
    }
}
