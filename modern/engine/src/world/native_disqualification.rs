//! Action gates from Robots.bas, Shots.bas, NeoMutations.bas and Ties.bas.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum DisqualificationAction {
    Virus,
    Shell,
    Slime,
    Venom,
    Poison,
    Tie,
    InformationShot,
    SexualReproduction,
    GeneDeletion,
    OpponentEnergy(bool),
    OpponentBody(bool),
}

impl DisqualificationAction {
    fn reason(self) -> &'static str {
        match self {
            Self::Virus => "using a virus",
            Self::Shell => "making shell",
            Self::Slime => "making slime",
            Self::Venom => "making venom",
            Self::Poison => "making poison",
            Self::Tie => "making a tie",
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
    /// Effective SimOpts.F1, independent of the external x_restartmode byte.
    pub(super) fn native_f1_enabled(&self) -> bool {
        self.local_mode.config.as_ref().map_or_else(
            || self.native_f1.unwrap_or(false),
            |config| {
                let staged_test = self
                    .local_mode
                    .evolution
                    .as_ref()
                    .is_some_and(|state| state.phase == crate::EvolutionPhase::Testing);
                staged_test
                    || matches!(config.kind, LocalModeKind::League | LocalModeKind::Contest)
                        && config.workflow != crate::LeagueWorkflow::Seeding
            },
        )
    }

    pub fn update_disqualification_rules(&mut self, level: u8) -> Result<(), EngineError> {
        if level > 2 {
            return Err(EngineError::Invariant(
                "disqualification level must be 0, 1 or 2".into(),
            ));
        }
        self.config.disqualification = level;
        self.publish_snapshot();
        Ok(())
    }

    pub(super) fn native_disqualification_level(&self) -> u8 {
        if self.local_mode.status == LocalModeStatus::Running
            && let Some(config) = self.local_mode.config.as_ref()
            && matches!(config.kind, LocalModeKind::League | LocalModeKind::Contest)
        {
            return match config.preset {
                crate::LeaguePreset::F2 => 1,
                crate::LeaguePreset::F3 => 2,
                _ => self.config.disqualification,
            };
        }
        self.config.disqualification
    }

    pub(super) fn native_disqualify_action(
        &mut self,
        id: OrganismId,
        action: DisqualificationAction,
    ) {
        if !action.forbidden(self.native_disqualification_level()) {
            return;
        }
        let Ok(robot) = self.slots.get(id) else {
            return;
        };
        if !robot.flags.exists
            || robot.flags.corpse
            || (matches!(action, DisqualificationAction::Slime) && robot.flags.vegetable)
        {
            return;
        }
        // Original action gates use F1 OR x_restartmode=1. Seeding is allowed
        // to disqualify a file even though its SimOpts.F1 is explicitly false.
        let contest = self.native_f1_enabled()
            || self
                .local_mode
                .config
                .as_ref()
                .is_some_and(|config| config.workflow == crate::LeagueWorkflow::Seeding);
        if !contest {
            // Outside a contest only robots opted into dq_kill are killed.
            if robot.counters.disqualification == 1 {
                self.slots.get_mut(id).unwrap().flags.dead = true;
            }
            return;
        }
        let file_name = self.graph_species_name(robot);
        if !self.local_mode.disqualifications.iter().any(|record| {
            record.round == self.local_mode.current_round && record.file_name == file_name
        }) {
            let record = crate::modes::DisqualificationRecord {
                tick: self.tick,
                round: self.local_mode.current_round,
                species: robot.identity.species,
                file_name: file_name.clone(),
                tag: robot.identity.tag.clone(),
                reason: action.reason().into(),
            };
            self.local_mode.disqualifications.push(record);
        }
        // dreason removes current animals on every call. A prior reason does
        // not grant later arrivals immunity; queue each pending removal once.
        let mut pending: HashSet<_> = self.pending_disqualifications.iter().copied().collect();
        for member in self.slots.ids_in_slot_order() {
            let same_file = self.graph_species_name(self.slots.get(member).unwrap()) == file_name;
            let robot = self.slots.get_mut(member).unwrap();
            if robot.flags.exists
                && !robot.flags.vegetable
                && !robot.flags.corpse
                && same_file
                && pending.insert(member)
            {
                robot.flags.dead = true;
                self.pending_disqualifications.push(member);
            }
        }
    }

    pub(super) fn flush_native_disqualifications(&mut self) -> Result<(), EngineError> {
        let ids = std::mem::take(&mut self.pending_disqualifications);
        if ids.is_empty() {
            return Ok(());
        }
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

    fn named_bot(world: &mut Engine, species: SpeciesId, file: &str) -> OrganismId {
        let x = 500.0 + world.slots.len() as f32 * 200.0;
        let id = world
            .spawn_species_batch(
                LegacyDna::parse("start stop").unwrap(),
                species,
                [[x, 500.0]],
                3000,
            )
            .unwrap()[0];
        world.slots.get_mut(id).unwrap().identity.species_file = file.into();
        id
    }

    fn file_world() -> (Engine, SpeciesId, SpeciesId) {
        let mut world = Engine::new(EngineConfig {
            world_width: 5000.0,
            world_height: 4000.0,
            disqualification: 2,
            ..EngineConfig::testing()
        })
        .unwrap();
        let a = world.register_species(SpeciesDefinition {
            name: "registry-a.txt".into(),
            ..Default::default()
        });
        let b = world.register_species(SpeciesDefinition {
            name: "registry-b.txt".into(),
            ..Default::default()
        });
        (world, a, b)
    }

    fn evolution(phase: crate::EvolutionPhase) -> crate::EvolutionState {
        let mut state = crate::EvolutionState::new(
            crate::EvolutionConfig::default(),
            vec![crate::EvolutionGenome {
                dna: LegacyDna::parse("start stop").unwrap(),
                mutation_profile: StandardMutationProfile::default(),
            }],
        )
        .unwrap();
        state.phase = phase;
        state
    }

    #[test]
    fn imported_f1_disqualifies_live_animals_by_file_not_numeric_species() {
        let (mut world, a, b) = file_world();
        world.native_f1 = Some(true);
        let actor = named_bot(&mut world, a, "shared.txt");
        let same_file_other_species = named_bot(&mut world, b, "shared.txt");
        let same_species_other_file = named_bot(&mut world, a, "other.txt");
        let vegetable = named_bot(&mut world, b, "shared.txt");
        world.slots.get_mut(vegetable).unwrap().flags.vegetable = true;
        let corpse = named_bot(&mut world, b, "shared.txt");
        world.slots.get_mut(corpse).unwrap().flags.corpse = true;
        let absent = named_bot(&mut world, b, "shared.txt");
        world.slots.get_mut(absent).unwrap().flags.exists = false;
        let hidden = named_bot(&mut world, b, "shared.txt");
        world.slots.get_mut(hidden).unwrap().flags.hidden = true;
        assert!(world.local_mode.config.is_none());

        world.native_disqualify_action(actor, DisqualificationAction::Shell);

        assert_eq!(
            world.pending_disqualifications,
            [actor, same_file_other_species, hidden]
        );
        for id in [same_species_other_file, vegetable, corpse, absent] {
            assert!(!world.slots.get(id).unwrap().flags.dead);
        }
        assert_eq!(world.local_mode.disqualifications.len(), 1);
        assert_eq!(
            world.local_mode.disqualifications[0].file_name,
            "shared.txt"
        );
        world.flush_native_disqualifications().unwrap();
        for id in [actor, same_file_other_species, hidden] {
            assert!(world.slots.get(id).is_err());
        }
        for id in [same_species_other_file, vegetable, corpse, absent] {
            assert!(world.slots.get(id).is_ok());
        }
    }

    #[test]
    fn source_seeding_disqualifies_files_while_exporting_f1_false() {
        let (mut world, a, b) = file_world();
        let actor = named_bot(&mut world, a, "seed.txt");
        let same_file = named_bot(&mut world, b, "seed.txt");
        world.local_mode.config = Some(LocalModeConfig {
            workflow: crate::LeagueWorkflow::Seeding,
            ..Default::default()
        });
        world.local_mode.status = LocalModeStatus::Running;
        assert!(!world.native_f1_enabled());

        world.native_disqualify_action(actor, DisqualificationAction::Shell);
        assert_eq!(world.pending_disqualifications, [actor, same_file]);
        world.flush_native_disqualifications().unwrap();
        assert_eq!(world.population(), 0);
    }

    #[test]
    fn champion_testing_disqualifies_files_while_other_evolution_phases_use_dq_kill() {
        for phase in [
            crate::EvolutionPhase::Evolving,
            crate::EvolutionPhase::Testing,
            crate::EvolutionPhase::ZerobotTesting,
        ] {
            let (mut world, a, b) = file_world();
            let actor = named_bot(&mut world, a, "candidate.txt");
            let same_file = named_bot(&mut world, b, "candidate.txt");
            world
                .slots
                .get_mut(actor)
                .unwrap()
                .counters
                .disqualification = 1;
            world.native_f1 = Some(true);
            world.local_mode.config = Some(LocalModeConfig {
                kind: LocalModeKind::SimpleSurvival,
                ..Default::default()
            });
            world.local_mode.status = LocalModeStatus::Running;
            world.local_mode.evolution = Some(evolution(phase));
            assert_eq!(
                world.native_f1_enabled(),
                phase == crate::EvolutionPhase::Testing
            );

            world.native_disqualify_action(actor, DisqualificationAction::Shell);
            assert!(world.slots.get(actor).unwrap().flags.dead);
            if phase == crate::EvolutionPhase::Testing {
                assert_eq!(world.pending_disqualifications, [actor, same_file]);
                world.flush_native_disqualifications().unwrap();
                assert_eq!(world.population(), 0);
            } else {
                assert!(!world.slots.get(same_file).unwrap().flags.dead);
                assert!(world.pending_disqualifications.is_empty());
                assert!(world.local_mode.disqualifications.is_empty());
            }
        }
    }

    #[test]
    fn repeated_forbidden_actions_and_later_arrivals_do_not_duplicate_or_evade_removal() {
        let (mut world, a, b) = file_world();
        world.native_f1 = Some(true);
        let actor = named_bot(&mut world, a, "repeat.txt");
        let same_file = named_bot(&mut world, b, "repeat.txt");
        world.native_disqualify_action(actor, DisqualificationAction::Shell);
        world.native_disqualify_action(actor, DisqualificationAction::Venom);
        world.native_disqualify_action(same_file, DisqualificationAction::Poison);
        assert_eq!(world.pending_disqualifications, [actor, same_file]);
        assert_eq!(world.local_mode.disqualifications.len(), 1);
        world.flush_native_disqualifications().unwrap();

        let newcomer = named_bot(&mut world, b, "repeat.txt");
        world.native_disqualify_action(newcomer, DisqualificationAction::Shell);
        assert_eq!(world.pending_disqualifications, [newcomer]);
        assert_eq!(world.local_mode.disqualifications.len(), 1);
        world.flush_native_disqualifications().unwrap();
        assert_eq!(world.population(), 0);
    }

    #[test]
    fn source_slime_vegetable_and_corpse_actor_exceptions_remain_in_force() {
        let (mut world, a, b) = file_world();
        world.native_f1 = Some(true);
        let vegetable = named_bot(&mut world, a, "shared.txt");
        world.slots.get_mut(vegetable).unwrap().flags.vegetable = true;
        let animal = named_bot(&mut world, b, "shared.txt");
        let corpse = named_bot(&mut world, b, "shared.txt");
        world.slots.get_mut(corpse).unwrap().flags.corpse = true;

        world.native_disqualify_action(vegetable, DisqualificationAction::Slime);
        world.native_disqualify_action(corpse, DisqualificationAction::Shell);
        assert!(world.pending_disqualifications.is_empty());
        assert!(world.local_mode.disqualifications.is_empty());

        // Source makeshell has no vegetable exemption, but dreason still
        // excludes vegetables and corpses when removing the matching file.
        world.native_disqualify_action(vegetable, DisqualificationAction::Shell);
        assert_eq!(world.pending_disqualifications, [animal]);
        world.flush_native_disqualifications().unwrap();
        assert!(world.slots.get(vegetable).is_ok());
        assert!(world.slots.get(corpse).is_ok());
    }

    #[test]
    fn imported_f1_uses_the_global_disqualification_preference_and_modern_save_keeps_it() {
        let (mut world, a, b) = file_world();
        world.native_f1 = Some(true);
        let actor = named_bot(&mut world, a, "global.txt");
        let same_file = named_bot(&mut world, b, "global.txt");
        world.update_disqualification_rules(0).unwrap();
        world.native_disqualify_action(actor, DisqualificationAction::Shell);
        assert!(world.pending_disqualifications.is_empty());

        world.update_disqualification_rules(2).unwrap();
        let mut resumed =
            crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(resumed.config.disqualification, 2);
        assert!(resumed.native_f1_enabled());
        resumed.native_disqualify_action(actor, DisqualificationAction::Shell);
        assert_eq!(resumed.pending_disqualifications, [actor, same_file]);
        resumed.flush_native_disqualifications().unwrap();
        assert_eq!(resumed.population(), 0);
    }

    fn contest(preset: crate::LeaguePreset) -> (Engine, SpeciesId, SpeciesId) {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let a = world.register_species(SpeciesDefinition {
            name: "a.txt".into(),
            ..Default::default()
        });
        let b = world.register_species(SpeciesDefinition {
            name: "b.txt".into(),
            ..Default::default()
        });
        for (species, x) in [(a, 500.0), (b, 2000.0)] {
            world
                .spawn_species_batch(
                    LegacyDna::parse("start stop").unwrap(),
                    species,
                    [[x, 500.0]],
                    3000,
                )
                .unwrap();
        }
        world
            .configure_local_mode(LocalModeConfig {
                preset,
                ..Default::default()
            })
            .unwrap();
        world.start_local_mode().unwrap();
        (world, a, b)
    }

    #[test]
    fn f3_material_action_removes_the_entire_animal_file_and_keeps_reason_on_resume() {
        let (mut world, a, b) = contest(crate::LeaguePreset::F3);
        let id = world
            .slots
            .ids_in_slot_order()
            .into_iter()
            .find(|id| world.slots.get(*id).unwrap().identity.species == a)
            .unwrap();
        world.native_disqualify_action(id, DisqualificationAction::Shell);
        world.flush_native_disqualifications().unwrap();
        assert!(!world.contest_populations().contains_key(&a));
        assert_eq!(world.contest_populations().get(&b), Some(&5));
        assert_eq!(world.local_mode.disqualifications[0].reason, "making shell");
        let resumed = crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(
            resumed.local_mode.disqualifications,
            world.local_mode.disqualifications
        );
    }

    #[test]
    fn f2_allows_materials_but_disqualifies_virus_use() {
        let (mut world, a, _) = contest(crate::LeaguePreset::F2);
        let id = world
            .slots
            .ids_in_slot_order()
            .into_iter()
            .find(|id| world.slots.get(*id).unwrap().identity.species == a)
            .unwrap();
        world.native_disqualify_action(id, DisqualificationAction::Shell);
        assert!(world.pending_disqualifications.is_empty());
        world.native_disqualify_action(id, DisqualificationAction::Virus);
        world.flush_native_disqualifications().unwrap();
        assert!(!world.contest_populations().contains_key(&a));
    }

    #[test]
    fn outside_contests_only_dq_kill_robot_dies() {
        let mut world = Engine::new(EngineConfig {
            disqualification: 2,
            ..EngineConfig::testing()
        })
        .unwrap();
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
