//! Action gates from Robots.bas, Shots.bas, NeoMutations.bas and Ties.bas.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum DisqualificationAction {
    Virus, Shell, Slime, Venom, Poison, Tie, InformationShot, SexualReproduction,
    GeneDeletion, OpponentEnergy(bool), OpponentBody(bool),
}

impl DisqualificationAction {
    fn reason(self) -> &'static str {
        match self {
            Self::Virus => "using a virus", Self::Shell => "making shell",
            Self::Slime => "making slime", Self::Venom => "making venom",
            Self::Poison => "making poison", Self::Tie => "making a tie",
            Self::InformationShot => "firing an info shot",
            Self::SexualReproduction => "attempting to reproduce sexually",
            Self::GeneDeletion => "deleting a gene",
            Self::OpponentEnergy(true) => "giving energy to opponent",
            Self::OpponentEnergy(false) => "taking energy from opponent",
            Self::OpponentBody(true) => "giving body to opponent",
            Self::OpponentBody(false) => "taking body from opponent",
        }
    }

    fn forbidden(self, level: u8) -> bool {
        match self {
            Self::Virus => level == 1 || level == 2,
            Self::OpponentEnergy(_) | Self::OpponentBody(_) => level == 1,
            _ => level == 2,
        }
    }
}

impl Engine {
    pub(super) fn native_disqualification_level(&self) -> u8 {
        if self.local_mode.status == LocalModeStatus::Running
            && let Some(config) = self.local_mode.config.as_ref()
            && matches!(config.kind, LocalModeKind::League | LocalModeKind::Contest) {
            return match config.preset { crate::LeaguePreset::F2 => 1,
                crate::LeaguePreset::F3 => 2, _ => self.config.disqualification };
        }
        self.config.disqualification
    }

    pub(super) fn native_disqualify_action(&mut self, id: OrganismId, action: DisqualificationAction) {
        if !action.forbidden(self.native_disqualification_level()) { return; }
        let Ok(robot) = self.slots.get(id) else { return };
        if robot.flags.corpse || (matches!(action, DisqualificationAction::Slime) && robot.flags.vegetable) {
            return;
        }
        let contest = self.local_mode.status == LocalModeStatus::Running
            && self.local_mode.config.as_ref().is_some_and(|config| {
                matches!(config.kind, LocalModeKind::League | LocalModeKind::Contest)
            });
        if !contest {
            // Outside a contest only robots opted into dq_kill are killed.
            if robot.counters.disqualification == 1 {
                self.slots.get_mut(id).unwrap().flags.dead = true;
            }
            return;
        }
        let file_name = robot.identity.species_file.clone();
        if self.local_mode.disqualifications.iter().any(|record| {
            record.round == self.local_mode.current_round && record.file_name == file_name
        }) { return; }
        let record = crate::modes::DisqualificationRecord {
            tick: self.tick, round: self.local_mode.current_round,
            species: robot.identity.species, file_name: file_name.clone(),
            tag: robot.identity.tag.clone(), reason: action.reason().into(),
        };
        self.local_mode.disqualifications.push(record);
        for member in self.slots.ids_in_slot_order() {
            let robot = self.slots.get_mut(member).unwrap();
            if !robot.flags.vegetable && !robot.flags.corpse && robot.identity.species_file == file_name {
                robot.flags.dead = true;
                self.pending_disqualifications.push(member);
            }
        }
    }

    pub(super) fn flush_native_disqualifications(&mut self) -> Result<(), EngineError> {
        let ids = std::mem::take(&mut self.pending_disqualifications);
        if ids.is_empty() { return Ok(()); }
        if let Some(phase) = self.native_phase.as_mut() {
            for id in &ids {
                if let Ok(robot) = self.slots.get(*id) {
                    if self.config.dna_costs.dynamic_all_robots || !robot.flags.vegetable {
                        phase.cost_population = phase.cost_population.saturating_sub(1);
                    }
                    phase.chloroplast_total -= f64::from(robot.materials.chloroplasts);
                }
            }
        }
        self.remove_many(&ids)?;
        self.rebuild_native_tie_index();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contest(preset: crate::LeaguePreset) -> (Engine, SpeciesId, SpeciesId) {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let a = world.register_species(SpeciesDefinition { name: "a.txt".into(), ..Default::default() });
        let b = world.register_species(SpeciesDefinition { name: "b.txt".into(), ..Default::default() });
        for (species, x) in [(a, 500.0), (b, 2000.0)] {
            world.spawn_species_batch(LegacyDna::parse("start stop").unwrap(), species, [[x, 500.0]], 3000).unwrap();
        }
        world.configure_local_mode(LocalModeConfig { preset, ..Default::default() }).unwrap();
        world.start_local_mode().unwrap();
        (world, a, b)
    }

    #[test]
    fn f3_material_action_removes_the_entire_animal_file_and_keeps_reason_on_resume() {
        let (mut world, a, b) = contest(crate::LeaguePreset::F3);
        let id = world.slots.ids_in_slot_order().into_iter().find(|id| world.slots.get(*id).unwrap().identity.species == a).unwrap();
        world.native_disqualify_action(id, DisqualificationAction::Shell);
        world.flush_native_disqualifications().unwrap();
        assert!(!world.contest_populations().contains_key(&a));
        assert_eq!(world.contest_populations().get(&b), Some(&5));
        assert_eq!(world.local_mode.disqualifications[0].reason, "making shell");
        let resumed = crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(resumed.local_mode.disqualifications, world.local_mode.disqualifications);
    }

    #[test]
    fn f2_allows_materials_but_disqualifies_virus_use() {
        let (mut world, a, _) = contest(crate::LeaguePreset::F2);
        let id = world.slots.ids_in_slot_order().into_iter().find(|id| world.slots.get(*id).unwrap().identity.species == a).unwrap();
        world.native_disqualify_action(id, DisqualificationAction::Shell);
        assert!(world.pending_disqualifications.is_empty());
        world.native_disqualify_action(id, DisqualificationAction::Virus);
        world.flush_native_disqualifications().unwrap();
        assert!(!world.contest_populations().contains_key(&a));
    }

    #[test]
    fn outside_contests_only_dq_kill_robot_dies() {
        let mut world = Engine::new(EngineConfig { disqualification: 2, ..EngineConfig::testing() }).unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let a = world.spawn_at(dna.clone(), [500.0; 2]).unwrap();
        let b = world.spawn_at(dna, [1500.0; 2]).unwrap();
        world.slots.get_mut(a).unwrap().counters.disqualification = 1;
        world.native_disqualify_action(a, DisqualificationAction::Shell);
        world.native_disqualify_action(b, DisqualificationAction::Shell);
        assert!(world.slots.get(a).unwrap().flags.dead);
        assert!(!world.slots.get(b).unwrap().flags.dead);
        assert!(world.pending_disqualifications.is_empty());
    }
}
