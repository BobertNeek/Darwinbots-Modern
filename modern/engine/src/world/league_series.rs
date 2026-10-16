use super::*;

impl Engine {
    pub(super) fn classic_native_species_aliases(&self) -> Option<Vec<(SpeciesId, &'static str)>> {
        use crate::PopulationSeedingPhase as Phase;
        let series = &self.local_mode.series;
        if series.workflow != crate::LeagueWorkflow::PopulationSeeding {
            return None;
        }
        match series.population_seeding_phase? {
            Phase::Seeding => series.active_seed.map(|id| vec![(id, "Test.txt")]),
            Phase::Tournament | Phase::Ladder => series
                .active_match
                .map(|pair| vec![(pair[0], "robotA.txt"), (pair[1], "robotB.txt")]),
            Phase::Complete => series
                .matches
                .last()
                .map(|m| vec![(m.entrants[0], "robotA.txt"), (m.entrants[1], "robotB.txt")])
                .or_else(|| {
                    series
                        .population_seeds
                        .last()
                        .map(|s| vec![(s.species, "Test.txt")])
                }),
        }
    }

    pub(super) fn classic_native_species_table(&self) -> Option<Vec<SpeciesId>> {
        let aliases = self.classic_native_species_aliases()?;
        let mut table: Vec<_> = self
            .species
            .iter()
            .enumerate()
            .filter_map(|(index, definition)| {
                definition.vegetable.then_some(SpeciesId(index as u32))
            })
            .collect();
        table.extend(aliases.into_iter().map(|(id, _)| id));
        Some(table)
    }

    pub(super) fn classic_competitive_world(&self) -> bool {
        self.classic_native_species_aliases()
            .is_some_and(|aliases| aliases.len() == 2)
    }

    pub(super) fn classic_native_species_color(&self, species: SpeciesId) -> Option<u32> {
        self.classic_native_species_aliases()?
            .into_iter()
            .find(|(id, _)| *id == species)
            .map(|(_, alias)| match alias {
                "Test.txt" => 0xffff0000,
                "robotA.txt" => 0xffff8000,
                _ => 0xff0080ff,
            })
    }

    pub(super) fn classic_native_species_skin(&self, species: SpeciesId) -> Option<[SkinPoint; 4]> {
        let profile = self
            .local_mode
            .series
            .classic_skin_profiles
            .iter()
            .find(|p| p.species == species)?;
        Some(std::array::from_fn(|i| SkinPoint {
            radius: f32::from(profile.skin[i * 2]) / 60.0,
            angle: 314 - i32::from(profile.skin[i * 2 + 1]) * 2,
        }))
    }

    pub(super) fn classic_native_species_restrictions(
        &self,
        species: SpeciesId,
    ) -> Option<RestrictionColumn> {
        let _ = self.classic_native_species_aliases()?;
        let definition = self.species.get(species.0 as usize)?;
        let animal = self.classic_native_species_color(species).is_some();
        if !animal && !definition.vegetable {
            return None;
        }
        let mut restrictions = if definition.vegetable {
            self.restriction_options.repopulating.clone()
        } else {
            self.restriction_options.non_repopulating.clone()
        };
        if animal {
            restrictions.disable_mutations = true;
        } else {
            restrictions.disable_mutations = definition
                .restrictions
                .as_ref()
                .is_some_and(|r| r.disable_mutations);
        }
        Some(restrictions)
    }

    pub fn set_native_restart_inputs(
        &mut self,
        seed: u64,
        skin_timer_seconds: Option<[f32; 2]>,
    ) -> Result<(), EngineError> {
        if skin_timer_seconds.is_some_and(|pair| {
            pair.iter()
                .any(|t| !t.is_finite() || !(0.0..86_400.0).contains(t))
        }) {
            return Err(EngineError::Invariant(
                "native skin Timer samples must be finite seconds in [0,86400)".into(),
            ));
        }
        self.set_native_restart_seed(seed);
        if let Some(pair) = skin_timer_seconds {
            self.native_skin_timer_seconds = Some(pair);
        }
        Ok(())
    }

    pub(super) fn prepare_classic_skin_profiles(&mut self) -> Result<(), EngineError> {
        let Some(aliases) = self.classic_native_species_aliases() else {
            return Ok(());
        };
        let timers = self.native_skin_timer_seconds.unwrap_or([0.0; 2]);
        let mut profiles = Vec::with_capacity(aliases.len());
        for (index, (species, alias)) in aliases.into_iter().enumerate() {
            let template = self
                .species_templates
                .get(species.0 as usize)
                .and_then(Option::as_ref)
                .ok_or_else(|| {
                    EngineError::Invariant("classic entrant has no startup genome".into())
                })?;
            let source = template
                .source_text
                .clone()
                .unwrap_or_else(|| template.dna.to_source());
            let skin = crate::native_skin::assign_skin(alias, &source, timers[index])?;
            profiles.push(crate::ClassicSkinProfile {
                species,
                alias: alias.into(),
                skin,
                timer_seconds: timers[index],
                synthetic_timer: self.native_skin_timer_seconds.is_none(),
                canonical_source: template.source_text.is_none(),
            });
        }
        self.local_mode.series.classic_skin_profiles = profiles;
        Ok(())
    }

    pub(super) fn classic_native_start_dna(
        &self,
        species: SpeciesId,
        fallback: LegacyDna,
    ) -> Result<LegacyDna, EngineError> {
        if !self
            .classic_native_species_table()
            .is_some_and(|table| table.contains(&species))
        {
            return Ok(fallback);
        }
        let source = self
            .species_templates
            .get(species.0 as usize)
            .and_then(Option::as_ref)
            .and_then(|template| template.source_text.as_deref())
            .map(str::to_owned)
            .unwrap_or_else(|| fallback.to_source());
        crate::dna::native_load_robot_dna(&source)
    }

    pub(super) fn apply_classic_start_actor(
        &mut self,
        id: OrganismId,
        species: SpeciesId,
    ) -> Result<(), EngineError> {
        let Some(aliases) = self.classic_native_species_aliases() else {
            return Ok(());
        };
        let alias = aliases
            .iter()
            .find(|(selected, _)| *selected == species)
            .map(|(_, alias)| *alias);
        let color = self.classic_native_species_color(species);
        let skin = self.classic_native_species_skin(species);
        let restrictions = self.classic_native_species_restrictions(species);
        let name = if alias.is_some() {
            let source_name = self.species[species.0 as usize]
                .name
                .rsplit(['\\', '/'])
                .next()
                .unwrap_or("")
                .trim_matches(' ');
            native_export::ansi(source_name)?
                .into_iter()
                .take(45)
                .map(|byte| {
                    if byte < 32 || (127..255).contains(&byte) {
                        '?'
                    } else {
                        char::from(byte)
                    }
                })
                .collect::<String>()
        } else {
            String::new()
        };
        let robot = self.slots.get_mut(id)?;
        if let Some(restrictions) = restrictions {
            apply_restriction_column(robot, &restrictions);
            robot.program.initialize_native_dna_context();
        }
        if let Some(alias) = alias {
            robot.identity.species_file = alias.into();
            robot.identity.tag = name;
            robot.program.mutation_profile = StandardMutationProfile::default();
            robot.flags.can_mutate = false;
            if let Some(color) = color {
                robot.phenotype.color = color;
            }
            if let Some(skin) = skin {
                robot.phenotype.skin = skin;
            }
        }
        Ok(())
    }

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
                    source_text: None,
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
        if config.workflow == crate::LeagueWorkflow::PopulationSeeding {
            for species in &self.species {
                let base = species
                    .name
                    .rsplit(['\\', '/'])
                    .next()
                    .unwrap_or("")
                    .trim_end_matches([' ', '.']);
                if species.vegetable
                    && ["Test.txt", "robotA.txt", "robotB.txt"]
                        .iter()
                        .any(|alias| base.eq_ignore_ascii_case(alias))
                {
                    return Err(EngineError::Invariant(
                        "classic vegetation conflicts with a reserved Test/robotA/robotB filename"
                            .into(),
                    ));
                }
            }
        }
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
            .chain(
                config
                    .seeding_opponent
                    .iter()
                    .filter(|_| config.workflow == crate::LeagueWorkflow::Seeding),
            )
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
            let native_dna = if config.workflow == LeagueWorkflow::PopulationSeeding {
                let source = self
                    .species_templates
                    .get(species.0 as usize)
                    .and_then(Option::as_ref)
                    .and_then(|template| template.source_text.as_deref())
                    .map(str::to_owned)
                    .unwrap_or_else(|| dna.to_source());
                Some(crate::dna::native_load_robot_dna(&source)?)
            } else {
                None
            };
            let dna = native_dna.as_ref().unwrap_or(dna);
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
        if self.local_mode.series.population_seeding_phase
            == Some(crate::PopulationSeedingPhase::Ladder)
            && self.local_mode.series.ladder.is_empty()
        {
            let mut order: Vec<_> = self.local_mode.series.pending.iter().copied().collect();
            order.sort_by_key(|id| {
                self.species[id.0 as usize]
                    .name
                    .rsplit(['\\', '/'])
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase()
            });
            self.local_mode.series.initialize_classic_ladder(order);
        }
        if self.local_mode.series.active_match.is_none() {
            self.local_mode.status = LocalModeStatus::Completed;
        } else {
            self.local_mode.status = LocalModeStatus::RoundComplete;
            let boundary = if match_winner.is_some() {
                NativeStartBoundary::ExternalProcess
            } else {
                NativeStartBoundary::SameProcessRound
            };
            self.restart_mode_world(None, boundary)?;
            self.local_mode.current_round = self.local_mode.current_round.saturating_add(1);
            self.local_mode.cycle_in_round = 0;
            self.local_mode.contest_runtime = Default::default();
            self.local_mode.status = LocalModeStatus::Running;
            self.apply_mode_restrictions(config);
        }
        self.publish_snapshot();
        Ok(())
    }

    pub(super) fn advance_population_seeding(&mut self) -> Result<(), EngineError> {
        let due = self
            .native_cycle
            .map_or(self.local_mode.cycle_in_round == 2_000, |cycle| {
                cycle == 2_000
            });
        if !due {
            return Ok(());
        }
        // Master uses totnvegsDisplayed, the previous UpdateCounters census,
        // not a winner, current end-of-cycle population or a cached UI snapshot.
        let population = self
            .native_population_census
            .map_or_else(|| self.mode_heterotroph_population(), |census| census[1]);
        self.local_mode
            .series
            .record_population_seed(population, 2_000)
            .map_err(EngineError::Invariant)?;
        if self.local_mode.series.active_seed.is_none()
            && self.local_mode.series.active_match.is_none()
        {
            self.local_mode.status = LocalModeStatus::Completed;
        } else {
            self.local_mode.status = LocalModeStatus::RoundComplete;
            self.restart_mode_world(None, NativeStartBoundary::ExternalProcess)?;
            self.local_mode.current_round = self.local_mode.current_round.saturating_add(1);
            self.local_mode.cycle_in_round = 0;
            self.local_mode.contest_runtime = Default::default();
            self.local_mode.status = LocalModeStatus::Running;
            if let Some(config) = self.local_mode.config.clone() {
                self.apply_mode_restrictions(&config);
            }
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
