//! Native multibot survival requirement and contest population/cycle controls.
use super::*;

impl Engine {
    pub(super) fn update_multibot_requirements(&mut self) {
        for id in self.phase_robot_ids() {
            let robot = self.slots.get(id).unwrap();
            let timer = robot.counters.multibot_time;
            if timer == 0 { continue; }
            let tied = robot.counters.multibot && self.tie_indices_for(id).into_iter().any(|index| {
                let tie = &self.ties[index];
                let other = if tie.first == id { tie.second } else { tie.first };
                self.slots.get(other).is_ok_and(|other| other.identity.species_file == robot.identity.species_file)
            });
            let robot = self.slots.get_mut(id).unwrap();
            robot.counters.multibot_time = if tied { timer.saturating_add(1).min(210) } else { timer.saturating_sub(1) };
            if robot.counters.multibot_time < 10 { robot.flags.dead = true; }
        }
    }

    pub(super) fn contest_populations(&self) -> BTreeMap<SpeciesId, usize> {
        let mut counts = BTreeMap::new();
        for robot in self.slots.iter().filter_map(|s| s.organism.as_ref())
            .filter(|r| !r.flags.vegetable && !r.flags.corpse && !r.flags.dead) {
            *counts.entry(robot.identity.species).or_default() += 1;
        }
        counts
    }

    pub(super) fn update_contest_cycle_limit(&mut self, counts: &BTreeMap<SpeciesId, usize>) {
        let Some(config) = self.local_mode.config.as_ref() else { return; };
        if config.maximum_cycles == 0 || counts.len() != 2 { return; }
        let current: Vec<_> = counts.iter().map(|(id, n)| (*id, *n)).collect();
        let state = &mut self.local_mode.contest_runtime;
        if state.effective_cycle_limit == 0 { state.effective_cycle_limit = config.maximum_cycles; }
        if state.previous_population.is_empty() && self.local_mode.cycle_in_round < 500 {
            state.previous_population = counts.clone();
        }
        state.cycles_since_population_sample = state.cycles_since_population_sample.saturating_add(1);
        if state.cycles_since_population_sample <= 1000 || !self.local_mode.cycle_in_round.is_multiple_of(10) { return; }
        for index in 0..2 {
            let (leading, population) = current[index];
            let (trailing, trailing_population) = current[1 - index];
            let old_leading = state.previous_population.get(&leading).copied().unwrap_or(population);
            let old_trailing = state.previous_population.get(&trailing).copied().unwrap_or(trailing_population);
            if population > trailing_population && trailing_population > 10
                && (population as i64 - old_leading as i64) < (trailing_population as i64 - old_trailing as i64) {
                let ratio = population as f64 / trailing_population as f64;
                let extension = (1000.0 / (1.0 / (ratio - 1.0) + 1.0)).round_ties_even() as u64;
                state.effective_cycle_limit = state.effective_cycle_limit.saturating_add(extension);
            }
        }
        state.previous_population = counts.clone();
        state.cycles_since_population_sample = 0;
    }

    pub(super) fn enforce_contest_population_cap(&mut self, maximum: usize) -> Result<(), EngineError> {
        let counts = self.contest_populations();
        if maximum == 0 || counts.len() > 2 { return Ok(()); }
        let mut remove = Vec::new();
        for (species, population) in counts {
            let mut members: Vec<_> = self.slots.ids_in_slot_order().into_iter().filter_map(|id| {
                let robot = self.slots.get(id).ok()?;
                (robot.identity.species == species && !robot.flags.vegetable && !robot.flags.corpse)
                    .then_some((id, robot.materials.energy + robot.materials.body * 10.0))
            }).collect();
            members.sort_by(|a,b| a.1.total_cmp(&b.1).then_with(|| a.0.slot().cmp(&b.0.slot())));
            remove.extend(members.into_iter().take(population.saturating_sub(maximum)).map(|(id, _)| id));
        }
        if !remove.is_empty() { self.remove_many(&remove)?; }
        Ok(())
    }
}

pub(super) fn child_multibot_timer(parent: u8) -> u8 {
    if parent == 0 { 0 } else { (f64::from(parent) / 2.0 + 2.0).round_ties_even() as u8 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_contest_round_uses_five_per_species_and_caps_each_competitor_separately() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let mut species = Vec::new();
        for name in ["animal-a", "animal-b", "plant"] {
            let id = world.register_species(SpeciesDefinition { name: name.into(), vegetable: name == "plant", ..Default::default() });
            world.spawn_species_batch(LegacyDna::parse("stop").unwrap(), id, [[500.0; 2]], 3000).unwrap();
            species.push(id);
        }
        world.configure_local_mode(LocalModeConfig { kind: LocalModeKind::Contest, maximum_population: 100, ..Default::default() }).unwrap();
        world.start_local_mode().unwrap();
        assert_eq!(world.contest_populations(), BTreeMap::from([(species[0], 5), (species[1], 5)]));
        assert_eq!(world.vegetable_population(), 5);
        world.enforce_contest_population_cap(3).unwrap();
        assert_eq!(world.contest_populations(), BTreeMap::from([(species[0], 3), (species[1], 3)]));
        assert_eq!(world.vegetable_population(), 5);
    }

    #[test]
    fn multibot_requirement_counts_down_without_hardened_same_species_ties() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let a = world.spawn_at(LegacyDna::parse("stop").unwrap(), [100.0; 2]).unwrap();
        let b = world.spawn_at(LegacyDna::parse("stop").unwrap(), [600.0; 2]).unwrap();
        world.slots.get_mut(a).unwrap().counters.multibot_time = 10;
        world.update_multibot_requirements();
        assert!(world.slots.get(a).unwrap().flags.dead);
        let robot = world.slots.get_mut(a).unwrap(); robot.flags.dead = false; robot.counters.multibot = true; robot.counters.multibot_time = 209;
        world.ties.push(TieState::legacy(a, b, 500.0, 1, 1));
        world.update_multibot_requirements();
        assert_eq!(world.slots.get(a).unwrap().counters.multibot_time, 210);
        assert_eq!(child_multibot_timer(207), 106);
        assert_eq!(child_multibot_timer(0), 0);
    }

    #[test]
    fn contest_adaptation_extends_a_leaders_limit_when_the_trailer_recovers() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        world.local_mode = LocalModeState::configured(LocalModeConfig { maximum_cycles: 15000, ..Default::default() });
        world.local_mode.cycle_in_round = 1010;
        world.local_mode.contest_runtime.cycles_since_population_sample = 1000;
        world.local_mode.contest_runtime.previous_population = BTreeMap::from([(SpeciesId(0), 30), (SpeciesId(1), 10)]);
        world.update_contest_cycle_limit(&BTreeMap::from([(SpeciesId(0), 30), (SpeciesId(1), 20)]));
        assert_eq!(world.local_mode.contest_runtime.effective_cycle_limit, 15333);
        let resumed = crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(resumed.local_mode.contest_runtime.effective_cycle_limit, 15333);
    }

    #[test]
    fn family_fitness_follows_native_parent_numbers_after_slot_reuse() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let a = world.spawn_at(LegacyDna::parse("stop").unwrap(), [100.0; 2]).unwrap();
        let b = world.spawn_at(LegacyDna::parse("stop").unwrap(), [600.0; 2]).unwrap();
        let c = world.spawn_at(LegacyDna::parse("stop").unwrap(), [900.0; 2]).unwrap();
        let parent_number = world.slots.get(a).unwrap().identity.absolute_id;
        let child = world.slots.get_mut(c).unwrap();
        child.identity.parent_absolute_id = parent_number;
        child.identity.parents[0] = Some(b); // a stale projection must not replace the native number.
        assert_eq!(world.find_best_robot_weighted(0), Some(a));
        world.become_native_corpse(c).unwrap();
        assert_eq!(world.find_best_robot_weighted(0), Some(a), "retained corpse descendants remain in the native family score");
    }
}
