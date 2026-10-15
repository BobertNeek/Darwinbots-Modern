use super::*;

impl Engine {
    pub(super) fn capture_missing_league_templates(&mut self) {
        for index in 0..self.species.len() {
            if self.species[index].vegetable || self.species_templates[index].is_some() {
                continue;
            }
            if let Some(robot) = self
                .slots
                .iter()
                .filter_map(|slot| slot.organism.as_ref())
                .find(|robot| {
                    robot.identity.species == SpeciesId(index as u32) && !robot.flags.corpse
                })
            {
                self.species_templates[index] = Some(SpeciesTemplate {
                    dna: robot.program.dna.clone(),
                    initial_energy: resource_readback(robot.materials.energy).max(1),
                    initial_population: 5,
                });
            }
        }
    }
    pub(super) fn prepare_league_series(
        &self,
        config: &LocalModeConfig,
    ) -> Result<crate::LeagueSeries, EngineError> {
        let registered: Vec<_> = self
            .species
            .iter()
            .enumerate()
            .filter_map(|(index, definition)| {
                (!definition.vegetable
                    && (self
                        .species_templates
                        .get(index)
                        .and_then(Option::as_ref)
                        .is_some()
                        || self
                            .slots
                            .iter()
                            .filter_map(|slot| slot.organism.as_ref())
                            .any(|robot| {
                                robot.identity.species == SpeciesId(index as u32)
                                    && !robot.flags.corpse
                            })))
                .then_some(SpeciesId(index as u32))
            })
            .collect();
        for id in config
            .entrant_order
            .iter()
            .chain(&config.initial_ladder)
            .chain(&config.seeding_opponent)
        {
            if !registered.contains(id) {
                return Err(EngineError::Invariant(
                    "league entrant must be an animal with a saved startup genome".into(),
                ));
            }
        }
        self.validate_community_league_entrants(config, &registered)?;
        crate::LeagueSeries::start(config, registered).map_err(EngineError::Invariant)
    }

    fn validate_community_league_entrants(
        &self,
        config: &LocalModeConfig,
        registered: &[SpeciesId],
    ) -> Result<(), EngineError> {
        use crate::{LeaguePreset, LeagueWorkflow, TokenClass};
        if !matches!(config.kind, LocalModeKind::League | LocalModeKind::Contest)
            || !matches!(
                config.preset,
                LeaguePreset::ShortBot | LeaguePreset::SingleGene | LeaguePreset::SingleStore
            )
        {
            return Ok(());
        }
        // Official wiki: Short_Bot_League (revision 3444), League (4374),
        // Single_Store_league (4373). SG is another name for short bots,
        // with at most six genes; SS has one store site, including inc/dec.
        let mut entrants: std::collections::BTreeSet<_> =
            if config.workflow == LeagueWorkflow::Direct || config.entrant_order.is_empty() {
                registered.iter().copied().collect()
            } else {
                config.entrant_order.iter().copied().collect()
            };
        if config.workflow == LeagueWorkflow::Ladder {
            entrants.extend(config.initial_ladder.iter().copied());
        }
        if config.workflow == LeagueWorkflow::Seeding {
            entrants.extend(config.seeding_opponent);
        }
        for species in entrants {
            // Inspect the actual startup genome that a round will restart.
            let dna = self
                .species_templates
                .get(species.0 as usize)
                .and_then(Option::as_ref)
                .map(|template| &template.dna)
                .or_else(|| {
                    self.slots
                        .iter()
                        .filter_map(|slot| slot.organism.as_ref())
                        .find(|robot| robot.identity.species == species && !robot.flags.corpse)
                        .map(|robot| &robot.program.dna)
                })
                .ok_or_else(|| {
                    EngineError::Invariant("league entrant has no startup genome".into())
                })?;
            let name = &self.species[species.0 as usize].name;
            if matches!(
                config.preset,
                LeaguePreset::ShortBot | LeaguePreset::SingleGene
            ) {
                let genes = dna.gene_count();
                if genes > 6 {
                    return Err(EngineError::Invariant(format!(
                        "{name} cannot enter the short bot / SG league: at most 6 genes are allowed (found {genes})"
                    )));
                }
            } else {
                let stores = dna
                    .blocks()
                    .iter()
                    .filter(|block| block.class == TokenClass::Store)
                    .count();
                if stores != 1 {
                    return Err(EngineError::Invariant(format!(
                        "{name} cannot enter the single store league: exactly 1 store instruction is required, including inc and dec (found {stores})"
                    )));
                }
            }
        }
        Ok(())
    }

    pub(super) fn advance_league_series(
        &mut self,
        config: &LocalModeConfig,
        match_winner: Option<SpeciesId>,
    ) -> Result<(), EngineError> {
        if let Some(winner) = match_winner {
            self.local_mode
                .series
                .settle(winner, self.local_mode.results.len())
                .map_err(EngineError::Invariant)?;
        }
        if self.local_mode.series.active_match.is_none() {
            self.local_mode.status = LocalModeStatus::Completed;
        } else {
            self.local_mode.status = LocalModeStatus::RoundComplete;
            self.restart_mode_world(None)?;
            self.local_mode.current_round = self.local_mode.current_round.saturating_add(1);
            self.local_mode.cycle_in_round = 0;
            self.local_mode.contest_runtime = Default::default();
            self.local_mode.status = LocalModeStatus::Running;
            self.apply_mode_restrictions(config);
        }
        self.publish_snapshot();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tournament_restarts_only_active_pair_and_preserves_unplayed_genomes_on_save() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let first = world.spawn_at(dna.clone(), [200.0; 2]).unwrap();
        let first_species = world.slots.get(first).unwrap().identity.species;
        let second_species = world.register_species(SpeciesDefinition {
            name: "second".into(),
            ..Default::default()
        });
        let third_species = world.register_species(SpeciesDefinition {
            name: "third".into(),
            ..Default::default()
        });
        world
            .spawn_species_at(dna.clone(), second_species, [400.0; 2])
            .unwrap();
        world
            .spawn_species_at(dna, third_species, [700.0; 2])
            .unwrap();
        world
            .configure_local_mode(LocalModeConfig {
                workflow: crate::LeagueWorkflow::Tournament,
                minimum_rounds: 1,
                maximum_rounds: 1,
                ..Default::default()
            })
            .unwrap();
        world.start_local_mode().unwrap();
        assert_eq!(
            world.local_mode.series.active_match,
            Some([first_species, second_species])
        );
        assert_eq!(world.mode_species_score(third_species).0, 0);
        world.remove_extinct_species_phase();
        let saved = crate::SaveFile::encode(&world).unwrap();
        let mut resumed = crate::SaveFile::decode(&saved).unwrap();
        resumed
            .adjudicate_local_round(Some(second_species), "first match".into())
            .unwrap();
        assert_eq!(
            resumed.local_mode.series.active_match,
            Some([second_species, third_species])
        );
        assert_eq!(resumed.mode_species_score(third_species).0, 5);
        resumed
            .adjudicate_local_round(Some(third_species), "final".into())
            .unwrap();
        assert_eq!(resumed.local_mode.status, LocalModeStatus::Completed);
        assert_eq!(resumed.local_mode.series.ladder, vec![third_species]);
    }
}
