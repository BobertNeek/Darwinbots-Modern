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
                let invested =
                    f64::from(robot.materials.energy) + f64::from(robot.materials.body) * 10.0;
                let descendants = children_by_parent
                    .get(&robot.identity.absolute_id)
                    .copied()
                    .unwrap_or_default();
                let entry = next.entry(parent).or_default();
                entry.0 = entry.0.saturating_add(1).saturating_add(descendants.0);
                entry.1 += invested + descendants.1;
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
                        + f64::from(robot.materials.body) * 10.0,
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
}
