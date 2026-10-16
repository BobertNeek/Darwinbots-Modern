use super::*;
use crate::{
    DnaBlock, EvolutionGenome, EvolutionMode, EvolutionPhase, EvolutionState, MAX_DNA_BLOCKS,
};

fn genome_of(robot: &RobotState) -> EvolutionGenome {
    let mut dna = robot.program.dna.clone();
    dna.synchronize_runtime_metadata(
        robot.counters.generation,
        robot.program.mutation_metadata.total_mutations,
        &robot.identity.tag,
    );
    EvolutionGenome {
        dna,
        mutation_profile: robot.program.mutation_profile.clone(),
    }
}

impl Engine {
    pub(super) fn staged_test_round(
        &mut self,
        winner: Option<SpeciesId>,
        complete: bool,
    ) -> Result<(), EngineError> {
        let Some(mut state) = self.local_mode.evolution.take() else {
            return Ok(());
        };
        let result: Result<(), EngineError> = (|| {
            if complete {
                if winner.is_some_and(|id| state.mutator_species.contains(&id)) {
                    state.test_won().map_err(EngineError::Invariant)?;
                } else {
                    let random = self.native_random.get_or_insert_with(|| {
                        crate::native_random::NativeRandom::with_user_seed(self.config.seed)
                    });
                    state.test_lost(random);
                }
                state.test_result_cursor = self.local_mode.results.len();
            }
            Ok(())
        })();
        self.local_mode.evolution = Some(state);
        result?;
        self.local_mode.current_round = self.local_mode.current_round.saturating_add(1);
        self.local_mode.status = LocalModeStatus::Running;
        self.restart_staged_world(complete)?;
        self.publish_snapshot();
        Ok(())
    }

    pub(super) fn advance_staged_test(&mut self) -> Result<(), EngineError> {
        let Some(state) = self.local_mode.evolution.as_ref() else {
            return Ok(());
        };
        if self.local_mode.status != LocalModeStatus::Running
            || state.phase != EvolutionPhase::Testing
        {
            return Ok(());
        }
        self.local_mode.cycle_in_round = self.local_mode.cycle_in_round.saturating_add(1);
        if !self.local_mode.cycle_in_round.is_multiple_of(10) {
            return Ok(());
        }
        let baselines = self.staged_role_count(&state.baseline_species);
        let mutators = self.staged_role_count(&state.mutator_species);
        if baselines + mutators == 0 && self.native_cycle.is_some() {
            return self.retry_native_contest_round();
        }
        let limit = self
            .local_mode
            .config
            .as_ref()
            .map_or(15_000, |config| config.maximum_cycles);
        let source_cycle = self
            .native_cycle
            .map_or(self.local_mode.cycle_in_round, |cycle| cycle.max(0) as u64);
        let winner = if baselines == 0 && mutators > 0 {
            state.mutator_species.first().copied()
        } else if mutators == 0 && baselines > 0 {
            state.baseline_species.first().copied()
        } else if limit > 0 && source_cycle > limit {
            if mutators > baselines {
                state.mutator_species.first().copied()
            } else if baselines > mutators {
                state.baseline_species.first().copied()
            } else {
                None
            }
        } else if baselines + mutators == 0 {
            None
        } else {
            return Ok(());
        };
        self.finish_local_mode_round(winner, "staged champion test".into(), true)
    }

    pub(super) fn prepare_staged_evolution(
        &self,
        config: &LocalModeConfig,
    ) -> Result<Option<EvolutionState>, EngineError> {
        let Some(mut settings) = config.staged_evolution.clone() else {
            return Ok(None);
        };
        if self.config.advanced_mutation.normalization_enabled
            && settings.mutation_rate_limit.is_none()
        {
            settings.mutation_rate_limit = Some(f64::from(
                self.config
                    .advanced_mutation
                    .native_delta2
                    .maximum_rate_per_base,
            ));
        }
        let dna = if settings.mode == EvolutionMode::Zerobot {
            LegacyDna::from_legacy_blocks(vec![
                DnaBlock {
                    class: TokenClass::Number,
                    value: 0
                };
                settings
                    .zerobot_initial_dna_length
                    .min(MAX_DNA_BLOCKS - 1)
            ])
        } else if !settings.source_dna.trim().is_empty() {
            self.load_dna_source(&settings.source_dna)?
        } else {
            let source = config.source_species.map(|id| id.0 as usize).or_else(|| {
                self.species
                    .iter()
                    .enumerate()
                    .position(|(index, definition)| {
                        !definition.vegetable
                            && self
                                .species_templates
                                .get(index)
                                .and_then(Option::as_ref)
                                .is_some()
                    })
            });
            source
                .and_then(|index| self.species_templates.get(index))
                .and_then(Option::as_ref)
                .map(|template| template.dna.clone())
                .ok_or_else(|| {
                    EngineError::Invariant(
                        "staged evolution needs a source bot or registered startup genome".into(),
                    )
                })?
        };
        let source_profile =
            if settings.mode != EvolutionMode::Zerobot && !settings.source_dna.trim().is_empty() {
                settings.source_mutation_profile.clone()
            } else {
                None
            };
        let profile = source_profile
            .or_else(|| {
                config
                    .source_species
                    .and_then(|id| self.species.get(id.0 as usize))
                    .and_then(|definition| definition.mutation_profile.clone())
            })
            .unwrap_or_else(|| {
                if self.default_mutation_profile == StandardMutationProfile::default() {
                    self.native_default_mutation_profile(dna.blocks().len())
                } else {
                    self.default_mutation_profile.clone()
                }
            });
        let count = match settings.mode {
            EvolutionMode::SimpleSurvival => 1,
            EvolutionMode::Eco => 15,
            EvolutionMode::Zerobot => 8,
        };
        EvolutionState::new(
            settings,
            vec![
                EvolutionGenome {
                    dna,
                    mutation_profile: profile
                };
                count
            ],
        )
        .map(Some)
        .map_err(EngineError::Invariant)
    }

    pub(super) fn restart_staged_world(&mut self, external_start: bool) -> Result<(), EngineError> {
        let Some(mut state) = self.local_mode.evolution.take() else {
            return Ok(());
        };
        let result: Result<(), EngineError> = (|| {
            for id in self.slots.ids_in_slot_order() {
                self.slots.remove(id)?;
            }
            self.slots
                .free_slots
                .sort_unstable_by(|first, second| second.cmp(first));
            self.mutations_enabled = true;
            self.global_mutation_multiplier = self.global_mutation_multiplier.max(1.0);
            self.native_random.get_or_insert_with(|| {
                crate::native_random::NativeRandom::with_user_seed(self.config.seed)
            });
            self.prepare_native_restart(if external_start {
                NativeStartBoundary::ExternalProcess
            } else {
                NativeStartBoundary::SameProcessRound
            });
            self.ties.clear();
            self.native_tie_index = None;
            self.native_buckets = native_collisions::NativeBuckets::default();
            self.projectiles = ProjectilePool::default();
            self.corpses.clear();
            self.pending_mutations.clear();
            self.pending_projectile_requests.clear();
            self.pending_gpu_render_instances = None;
            self.pending_gpu_collision_pairs = None;
            let zero_test = state.phase == EvolutionPhase::ZerobotTesting;
            let testing = matches!(
                state.phase,
                EvolutionPhase::Testing | EvolutionPhase::ZerobotTesting
            );
            let baselines = if zero_test {
                Vec::new()
            } else {
                state.stages[&state.opponent_stage].clone()
            };
            let mutators = if testing {
                state.candidate.clone()
            } else {
                state.mutators.clone()
            };
            let groups = baselines.len().max(mutators.len());
            while state.baseline_species.len() < groups {
                state
                    .baseline_species
                    .push(self.register_species_unpublished(SpeciesDefinition {
                        name: "Base.txt".into(),
                        color: 0xffda6c59,
                        ..Default::default()
                    }));
            }
            while state.mutator_species.len() < groups {
                state
                    .mutator_species
                    .push(self.register_species_unpublished(SpeciesDefinition {
                        name: "Mutate.txt".into(),
                        color: 0xff62a844,
                        ..Default::default()
                    }));
            }
            let quantity = if zero_test {
                5
            } else {
                match state.config.mode {
                    EvolutionMode::Eco => 1,
                    EvolutionMode::SimpleSurvival => 5,
                    EvolutionMode::Zerobot => ((f64::from(self.config.world_width)
                        * f64::from(self.config.world_height))
                    .sqrt()
                        / (80.0 * 8.0 * f64::from(state.stage + 1).sqrt()))
                    .round_ties_even()
                    .max(1.0) as usize,
                }
            };
            let mut remaining = self.config.organism_capacity;
            if state.config.mode == EvolutionMode::Zerobot && !testing {
                // MDIForm1 computes this before StartSimul reseeds. Its negative
                // Rnd must not replace the new simulation's placement stream.
                let mut random = crate::native_random::NativeRandom::default();
                state.fitness_weight = (20.0 + random.rnd(-((state.stage + 1) as f32)) * 40.0)
                    .round_ties_even() as u16;
            }
            for group in 0..groups {
                for (genomes, ids, baseline) in [
                    (&baselines, &state.baseline_species, true),
                    (&mutators, &state.mutator_species, false),
                ] {
                    let Some(genome) = genomes.get(group) else {
                        continue;
                    };
                    let species = ids[group];
                    self.species[species.0 as usize].name = if baseline {
                        "Base.txt"
                    } else if testing {
                        "Test.txt"
                    } else {
                        "Mutate.txt"
                    }
                    .into();
                    self.species[species.0 as usize].mutation_profile =
                        Some(genome.mutation_profile.clone());
                    self.species_templates[species.0 as usize] = Some(SpeciesTemplate {
                        source_text: None,
                        dna: genome.dna.clone(),
                        initial_energy: 3_000,
                        initial_population: quantity,
                    });
                    for ordinal in 0..quantity {
                        if remaining == 0 {
                            break;
                        }
                        remaining -= 1;
                        let id = self.spawn_native_start_robot(
                            genome.dna.clone(),
                            species,
                            self.config.seed ^ ordinal as u64,
                            None,
                        )?;
                        let robot = self.slots.get_mut(id)?;
                        robot.flags.can_mutate = !baseline && !testing;
                        robot.program.mutation_profile = genome.mutation_profile.clone();
                    }
                }
            }
            state.reset_attempt();
            state.test_start_energy = self.staged_role_energy(&state.mutator_species);
            self.refresh_native_chloroplast_population();
            self.local_mode.restart_count = self.local_mode.restart_count.saturating_add(1);
            self.local_mode.cycle_in_round = 0;
            self.local_mode.contest_runtime = Default::default();
            Ok(())
        })();
        self.local_mode.evolution = Some(state);
        result
    }

    fn staged_role_count(&self, ids: &[SpeciesId]) -> usize {
        self.slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .filter(|robot| !robot.flags.corpse && ids.contains(&robot.identity.species))
            .count()
    }

    fn staged_role_energy(&self, ids: &[SpeciesId]) -> f64 {
        self.slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .filter(|robot| !robot.flags.corpse && ids.contains(&robot.identity.species))
            .map(|robot| f64::from(robot.materials.energy) + f64::from(robot.materials.body) * 10.0)
            .sum()
    }

    fn staged_average_energy(&self, ids: &[SpeciesId]) -> Option<f64> {
        let mut count = 0;
        let mut energy = 0.0;
        for robot in self.slots.iter().filter_map(|slot| slot.organism.as_ref()) {
            if ids.contains(&robot.identity.species)
                && robot.program.mutation_metadata.native_last_mutations > 0
            {
                count += 1;
                energy += f64::from(robot.materials.energy);
            }
        }
        (count > 0).then(|| energy / f64::from(count))
    }

    fn staged_candidates(&mut self, eco: bool, weight: u16) -> Vec<EvolutionGenome> {
        if !eco {
            return self
                .find_best_robot_weighted(weight)
                .and_then(|id| self.slots.get(id).ok())
                .map(genome_of)
                .into_iter()
                .collect();
        }
        let ids = self.slots.ids_in_slot_order();
        let mut diversity = HashMap::new();
        for &id in &ids {
            let robot = self.slots.get(id).unwrap();
            if robot.flags.corpse || robot.flags.vegetable || robot.flags.hidden {
                continue;
            }
            let name = &self.species[robot.identity.species.0 as usize].name;
            let distance: f32 = ids
                .iter()
                .copied()
                .filter(|other| *other != id)
                .filter_map(|other| self.slots.get(other).ok())
                .filter(|target| {
                    !target.flags.corpse
                        && self.species[target.identity.species.0 as usize].name == *name
                })
                .map(|target| robot.program.dna.genetic_distance(&target.program.dna))
                .sum();
            diversity.insert(id, distance);
        }
        let population_exponent = if weight < 100 {
            1.0
        } else {
            f64::from(200 - weight) / 100.0
        };
        let energy_exponent = f64::from(weight.min(100)) / 100.0;
        let mut selected = Vec::new();
        for _ in 0..15 {
            // Deleting a selected family member changes later family scores.
            let families = self.native_family_scores();
            let mut best = None;
            let mut best_score = 0.0;
            for id in self.slots.ids_in_slot_order() {
                let Some(distance) = diversity.get(&id) else {
                    continue;
                };
                let family = families[id.slot() as usize];
                let score = (family.0 as f64).powf(population_exponent)
                    * family.1.max(0.0).powf(energy_exponent)
                    * f64::from(*distance);
                if score >= best_score {
                    best_score = score;
                    best = Some(id);
                }
            }
            let Some(id) = best else { break };
            selected.push(genome_of(self.slots.get(id).unwrap()));
            self.slots
                .remove(id)
                .expect("selected evolution candidate must exist");
            diversity.remove(&id);
        }
        selected
    }

    /// Returns true when a process-style restart replaced the world this cycle.
    pub(super) fn staged_evolution_before_cycle(
        &mut self,
    ) -> Result<(bool, Option<f64>), EngineError> {
        if self.local_mode.status != LocalModeStatus::Running {
            return Ok((false, None));
        }
        let Some(mut state) = self.local_mode.evolution.take() else {
            return Ok((false, None));
        };
        let mut restart = false;
        state.cycle_in_attempt = state.cycle_in_attempt.saturating_add(1);
        let source_cycle = self
            .native_cycle
            .map_or(state.cycle_in_attempt, |cycle| cycle.max(0) as u64);
        let evolving = state.phase == EvolutionPhase::Evolving;
        let baselines = self.staged_role_count(&state.baseline_species);
        let mutators = self.staged_role_count(&state.mutator_species);
        if evolving && state.config.mode != EvolutionMode::Zerobot {
            let random = self.native_random.get_or_insert_with(|| {
                crate::native_random::NativeRandom::with_user_seed(self.config.seed)
            });
            if mutators == 0 {
                state.decrease_difficulty(random);
                restart = true;
            } else if baselines == 0 {
                let changed = self
                    .find_best_robot_weighted(state.fitness_weight)
                    .and_then(|id| self.slots.get(id).ok())
                    .is_some_and(|robot| robot.program.mutation_metadata.total_mutations > 0);
                if changed && (state.config.mode != EvolutionMode::Eco || mutators >= 15) {
                    let candidates = self.staged_candidates(
                        state.config.mode == EvolutionMode::Eco,
                        state.fitness_weight,
                    );
                    state
                        .begin_test(candidates)
                        .map_err(EngineError::Invariant)?;
                } else {
                    state.scale_mutations();
                    state.increase_difficulty(self.native_random.as_mut().unwrap());
                }
                restart = true;
            }
        } else if evolving && mutators == 0 {
            restart = true;
        }
        if !restart && evolving && state.config.mode != EvolutionMode::Zerobot {
            state.mode_cycles = state.mode_cycles.saturating_add(1);
            if baselines > mutators {
                state.stagnant = false;
            }
            if source_cycle == 1_000_000 {
                state.stagnant = true;
            }
            let threshold = f64::from(state.hide_cycles) / 1.2 + f64::from(state.hide_offset);
            if f64::from(state.mode_cycles) > threshold {
                if state.reduction_unit == 150.0 && state.hidden && mutators < baselines {
                    state.mode_cycles = state.mode_cycles.saturating_sub(100);
                } else {
                    let delta = state.energy_delta / f64::from(state.mode_cycles);
                    state.inverse_delta += delta;
                    if state.hidden {
                        let next =
                            (state.previous_energy_delta - delta) / f64::from(state.reduction_unit);
                        state.handicap = if next < state.handicap {
                            next
                        } else {
                            (state.handicap * 9.0 + next) / 10.0
                        };
                        state.inverse_handicap = ((state.previous_inverse_delta
                            - state.inverse_delta)
                            / f64::from(state.reduction_unit))
                        .min(0.0);
                        if state.handicap - state.inverse_handicap > 0.1 {
                            state.inverse_handicap = state.handicap - 0.1;
                        }
                        state.previous_inverse_delta = state.inverse_delta;
                        state.inverse_delta = 0.0;
                        self.staged_separate_engagements(&state);
                        for slot in self.projectiles.live_slots() {
                            if matches!(self.projectiles.kinds[slot as usize], -1 | -6) {
                                self.projectiles.deactivate(slot);
                            }
                        }
                    }
                    state.previous_energy_delta = delta;
                    state.energy_delta = 0.0;
                    state.hidden = !state.hidden;
                    state.hide_offset = state.hide_cycles as f32 / 3.0
                        * self.native_random.as_mut().unwrap().unit();
                    state.mode_cycles = 0;
                }
            }
        }
        let handicap = if self.native_cycle.is_some() {
            (state.handicap - state.inverse_handicap)
                * (source_cycle as f64 / (f64::from(state.hide_cycles) * 8.0)).min(1.0)
        } else {
            state.current_handicap()
        };
        for robot in self
            .slots
            .iter_mut()
            .filter_map(|slot| slot.organism.as_mut())
        {
            robot.flags.hidden = state.hidden
                && !robot.flags.corpse
                && state.baseline_species.contains(&robot.identity.species);
            if state.hidden && state.mutator_species.contains(&robot.identity.species) {
                let scale = if robot.program.mutation_metadata.native_last_mutations > 0 {
                    1.0
                } else {
                    0.5
                };
                robot.materials.energy =
                    (f64::from(robot.materials.energy) + handicap * scale) as f32;
            }
        }
        let average = self.staged_average_energy(&state.mutator_species);
        self.local_mode.evolution = Some(state);
        if restart {
            self.restart_staged_world(true)?;
        }
        Ok((restart, average))
    }

    fn staged_separate_engagements(&mut self, state: &EvolutionState) {
        let mut moved = 0;
        let limit = 3_200.0 + self.staged_role_count(&state.mutator_species) as f64 * 0.9;
        loop {
            let mut changed = false;
            let ids = self.slots.ids_in_slot_order();
            for &base in &ids {
                let robot = self.slots.get(base).unwrap();
                if !state.baseline_species.contains(&robot.identity.species) {
                    continue;
                }
                let base_position = robot.kinematics.position;
                let base_radius = robot.materials.radius;
                let base_body = robot.materials.body;
                for &mutator in &ids {
                    let robot = self.slots.get(mutator).unwrap();
                    if !state.mutator_species.contains(&robot.identity.species) {
                        continue;
                    }
                    let body = base_body.max(robot.materials.body);
                    let range = base_radius
                        + robot.materials.radius
                        + if body > 10.0 {
                            (f64::from(body).ln() * 60.0 + 41.0) as f32
                        } else {
                            40.0
                        }
                        + 40.0;
                    let delta = std::array::from_fn::<_, 2, _>(|axis| {
                        base_position[axis] - robot.kinematics.position[axis]
                    });
                    let distance = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
                    if distance >= range {
                        continue;
                    }
                    let offset = if distance == 0.0 {
                        [0.0; 2]
                    } else {
                        delta.map(|value| -(value / distance) * (range - distance))
                    };
                    let group = self
                        .file_transfer_group(mutator)
                        .unwrap_or_else(|_| vec![mutator]);
                    for member in group {
                        let robot = self.slots.get_mut(member).unwrap();
                        if !state.mutator_species.contains(&robot.identity.species) {
                            continue;
                        }
                        for (axis, displacement) in offset.into_iter().enumerate() {
                            robot.kinematics.position[axis] += displacement;
                        }
                    }
                    changed = true;
                    moved += 1;
                }
            }
            if !changed || f64::from(moved) > limit {
                break;
            }
        }
    }

    pub(super) fn staged_evolution_after_cycle(
        &mut self,
        average_before: Option<f64>,
    ) -> Result<(), EngineError> {
        let Some(mut state) = self.local_mode.evolution.take() else {
            return Ok(());
        };
        let source_cycle = self
            .native_cycle
            .map_or(state.cycle_in_attempt, |cycle| cycle.max(0) as u64);
        let mut restart = false;
        if state.phase == EvolutionPhase::Evolving {
            if let Some(after) = self.staged_average_energy(&state.mutator_species) {
                state.energy_delta += after - average_before.unwrap_or(0.0);
            }
            if state.config.mode == EvolutionMode::Zerobot
                && (self.native_cycle.is_none() || source_cycle > 0)
                && source_cycle.is_multiple_of(50)
                && let Some(id) = self.find_best_robot_weighted(state.fitness_weight)
            {
                let robot = self.slots.get(id)?;
                if state.mutator_species.contains(&robot.identity.species) {
                    let family = self.native_family_scores()[id.slot() as usize];
                    let weight = f64::from(state.fitness_weight);
                    let score = if family.2 {
                        0.0
                    } else {
                        (family.0 as f64).powf(if weight < 100.0 {
                            1.0
                        } else {
                            (200.0 - weight) / 100.0
                        }) * family.1.max(0.0).powf(weight.min(100.0) / 100.0)
                    };
                    let mut genome = genome_of(robot);
                    restart = state.observe_zerobot(
                        robot.identity.absolute_id,
                        score,
                        robot.program.mutation_metadata.native_last_mutations > 0,
                        &mut genome,
                    );
                    self.slots.get_mut(id)?.program.mutation_profile = genome.mutation_profile;
                }
            }
        } else if state.phase == EvolutionPhase::ZerobotTesting && source_cycle == 1 {
            state.test_start_energy = self.staged_role_energy(&state.mutator_species);
        } else if state.phase == EvolutionPhase::ZerobotTesting
            && if self.native_cycle.is_some() {
                source_cycle == 8_000
            } else {
                source_cycle >= 8_000
            }
        {
            if self.staged_role_count(&state.mutator_species) > 10
                && self.staged_role_energy(&state.mutator_species) > state.test_start_energy * 2.0
            {
                state.phase = EvolutionPhase::Complete;
                self.local_mode.status = LocalModeStatus::Completed;
            } else {
                state.phase = EvolutionPhase::Evolving;
                state.candidate.clear();
                if state.stage > 500 {
                    let seed = EvolutionGenome {
                        dna: LegacyDna::from_legacy_blocks(vec![
                            DnaBlock {
                                class: TokenClass::Number,
                                value: 0,
                            };
                            state
                                .config
                                .zerobot_initial_dna_length
                                .min(MAX_DNA_BLOCKS - 1)
                        ]),
                        mutation_profile: self.default_mutation_profile.clone(),
                    };
                    state
                        .retired_stages
                        .extend(std::mem::take(&mut state.stages).into_values());
                    state.stages.insert(0, vec![seed.clone()]);
                    state.mutators = vec![seed; 8];
                    state.stage = 0;
                    state.opponent_stage = 0;
                }
                restart = true;
            }
        }
        self.local_mode.cycle_in_round = state.cycle_in_attempt;
        self.local_mode.evolution = Some(state);
        if restart {
            self.restart_staged_world(true)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_staged_file_profile_and_native_private_genome_reach_live_bots_and_saves() {
        let selected = StandardMutationProfile {
            point: crate::MutationRate {
                one_in: 37.0,
                mean: 3.0,
                std_dev: 0.25,
            },
            point_value_change_percent: 13,
            ..Default::default()
        };
        let selected = StandardMutationProfile::from_mrate(&selected.to_mrate().unwrap()).unwrap();
        let previous = StandardMutationProfile {
            point_value_change_percent: 97,
            ..Default::default()
        };
        let mut world = Engine::new_native(EngineConfig {
            organism_capacity: 16,
            ..EngineConfig::testing()
        })
        .unwrap();
        let source = world.register_species(SpeciesDefinition {
            name: "Configured.txt".into(),
            mutation_profile: Some(previous.clone()),
            ..Default::default()
        });
        world
            .spawn_species_batch(
                LegacyDna::parse("start stop").unwrap(),
                source,
                std::iter::empty::<[f32; 2]>(),
                1000,
            )
            .unwrap();
        let raw = "def Marker 900\r\ndef marker 901\r\n11 start 17 .Marker store 19 .marker store 1 999 += stop end 123\r\nNewMove\r\n";
        let expected_dna = world.load_dna_source(raw).unwrap();
        world
            .configure_local_mode(LocalModeConfig {
                kind: LocalModeKind::SimpleSurvival,
                source_species: Some(source),
                staged_evolution: Some(crate::EvolutionConfig {
                    source_dna: raw.into(),
                    source_mutation_profile: Some(selected.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .unwrap();
        world.start_local_mode().unwrap();
        let state = world.local_mode.evolution.as_ref().unwrap();
        assert_eq!(state.config.source_mutation_profile, Some(selected.clone()));
        assert!(
            state
                .mutators
                .iter()
                .all(|genome| genome.mutation_profile == selected && genome.dna == expected_dna)
        );
        let mutator_species = state.mutator_species.clone();
        let ids: Vec<_> = world
            .slots
            .ids_in_slot_order()
            .into_iter()
            .filter(|id| mutator_species.contains(&world.slots.get(*id).unwrap().identity.species))
            .collect();
        assert!(!ids.is_empty());
        for &id in &ids {
            assert_eq!(world.mutation_profile(id).unwrap(), &selected);
            assert_eq!(world.dna(id).unwrap(), &expected_dna);
            assert!(!world.dna(id).unwrap().uses_new_move());
            assert_eq!(
                world.dna(id).unwrap().blocks()[0],
                DnaBlock {
                    class: TokenClass::Flow,
                    value: 2
                },
                "original VM coding excludes the first raw numeric cell"
            );
        }
        assert_eq!(
            world.species[source.0 as usize].mutation_profile,
            Some(previous)
        );
        // Native startup enables evolution mutations; disable them only for this
        // deterministic VM check, without changing the imported profile itself.
        world.update_mutation_settings(false, 1.0, &[]).unwrap();
        world.tick().unwrap();
        for &id in &ids {
            assert_eq!(world.memory_at(id, 900).unwrap(), 17);
            assert_eq!(world.memory_at(id, 901).unwrap(), 19);
            assert_eq!(
                world.memory_at(id, 999).unwrap(),
                0,
                "native += is a numeric zero, not an addstore"
            );
        }
        let saved = crate::SaveFile::encode(&world).unwrap();
        let mut restored = crate::SaveFile::decode(&saved).unwrap();
        assert_eq!(
            restored
                .local_mode
                .evolution
                .as_ref()
                .unwrap()
                .config
                .source_mutation_profile,
            Some(selected.clone())
        );
        for &id in &ids {
            assert_eq!(restored.mutation_profile(id).unwrap(), &selected);
        }
        world.tick().unwrap();
        restored.tick().unwrap();
        assert_eq!(
            crate::SaveFile::encode(&world).unwrap(),
            crate::SaveFile::encode(&restored).unwrap()
        );
    }

    #[test]
    fn staged_missing_sidecar_and_registered_source_keep_the_configured_profile() {
        let configured = StandardMutationProfile {
            point_value_change_percent: 91,
            ..Default::default()
        };
        let unrelated = StandardMutationProfile {
            point_value_change_percent: 12,
            ..Default::default()
        };
        for explicit_dna in [false, true] {
            let mut world = Engine::new_native(EngineConfig {
                organism_capacity: 16,
                ..EngineConfig::testing()
            })
            .unwrap();
            let source = world.register_species(SpeciesDefinition {
                name: "Settings.txt".into(),
                mutation_profile: Some(configured.clone()),
                ..Default::default()
            });
            world
                .spawn_species_batch(
                    LegacyDna::parse("start 42 900 store stop").unwrap(),
                    source,
                    std::iter::empty::<[f32; 2]>(),
                    1000,
                )
                .unwrap();
            world
                .configure_local_mode(LocalModeConfig {
                    kind: LocalModeKind::SimpleSurvival,
                    source_species: Some(source),
                    staged_evolution: Some(crate::EvolutionConfig {
                        source_dna: if explicit_dna {
                            "start 43 900 store stop".into()
                        } else {
                            String::new()
                        },
                        source_mutation_profile: if explicit_dna {
                            None
                        } else {
                            Some(unrelated.clone())
                        },
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .unwrap();
            world.start_local_mode().unwrap();
            let state = world.local_mode.evolution.as_ref().unwrap();
            assert!(
                state
                    .mutators
                    .iter()
                    .all(|genome| genome.mutation_profile == configured)
            );
            for bot in world.slots.iter().filter_map(|slot| slot.organism.as_ref()) {
                assert_eq!(bot.program.mutation_profile, configured);
            }
            let restored =
                crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
            assert_eq!(restored.local_mode.evolution, world.local_mode.evolution);
        }
    }

    #[test]
    fn staged_source_profile_defaults_and_invalid_configuration_are_atomic() {
        let mut legacy = serde_json::to_value(crate::EvolutionConfig::default()).unwrap();
        legacy
            .as_object_mut()
            .unwrap()
            .remove("source_mutation_profile");
        let legacy: crate::EvolutionConfig = serde_json::from_value(legacy).unwrap();
        assert!(legacy.source_mutation_profile.is_none());
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        let before = crate::SaveFile::encode(&world).unwrap();
        let invalid = StandardMutationProfile {
            point: crate::MutationRate {
                one_in: 1.0,
                mean: 1.0,
                std_dev: -1.0,
            },
            ..Default::default()
        };
        assert!(
            world
                .configure_local_mode(LocalModeConfig {
                    kind: LocalModeKind::SimpleSurvival,
                    staged_evolution: Some(crate::EvolutionConfig {
                        source_dna: "start stop".into(),
                        source_mutation_profile: Some(invalid),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .is_err()
        );
        assert_eq!(crate::SaveFile::encode(&world).unwrap(), before);
    }

    #[test]
    fn zerobot_stage_weight_does_not_replace_the_seeded_placement_stream() {
        let mut world = Engine::new_native(EngineConfig {
            seed: 0,
            organism_capacity: 1,
            ..EngineConfig::testing()
        })
        .unwrap();
        world.set_native_restart_seed(42);
        world.set_native_gaussian_spare(Some(1.25)).unwrap();
        world
            .configure_local_mode(LocalModeConfig {
                kind: LocalModeKind::Zerobot,
                staged_evolution: Some(crate::EvolutionConfig {
                    mode: EvolutionMode::Zerobot,
                    ..Default::default()
                }),
                ..Default::default()
            })
            .unwrap();
        world.start_local_mode().unwrap();
        assert_eq!(world.read_native_gaussian_spare(), None);
        assert_eq!(
            world.local_mode.evolution.as_ref().unwrap().fitness_weight,
            29
        );
        assert_eq!(world.population(), 1);
        // Header4 followed by preparerob6, x/y and timer; stage Rnd is separate.
        assert_eq!(world.native_random.as_ref().unwrap().state(), 0x003ac52d);
        assert_eq!(world.config.seed, 42);
        let robot = world
            .slots
            .iter()
            .find_map(|slot| slot.organism.as_ref())
            .unwrap();
        assert_eq!(robot.program.rng_state, 0x003ac52d);
    }

    fn simple() -> Engine {
        let mut world = Engine::new(EngineConfig {
            metabolism_cost: 0,
            ..EngineConfig::testing()
        })
        .unwrap();
        world
            .configure_local_mode(LocalModeConfig {
                kind: LocalModeKind::SimpleSurvival,
                minimum_rounds: 1,
                maximum_rounds: 1,
                staged_evolution: Some(crate::EvolutionConfig {
                    source_dna: "start 111 901 store stop".into(),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .unwrap();
        world.start_local_mode().unwrap();
        world
    }

    #[test]
    fn native_cycle_staged_handicap_and_stagnancy_do_not_use_attempt_call_count() {
        let mut world = simple();
        world.native_cycle = Some(0);
        world.tick = 500;
        let mutators = {
            let state = world.local_mode.evolution.as_mut().unwrap();
            state.hidden = true;
            state.hide_cycles = 1000;
            state.handicap = -8000.0;
            state.inverse_handicap = 0.0;
            state.cycle_in_attempt = 10;
            state.mutator_species.clone()
        };
        let id = world
            .slots
            .ids_in_slot_order()
            .into_iter()
            .find(|id| mutators.contains(&world.slots.get(*id).unwrap().identity.species))
            .unwrap();
        let robot = world.slots.get_mut(id).unwrap();
        robot.materials.energy = 3000.0;
        robot.program.mutation_metadata.native_last_mutations = 1;
        world.staged_evolution_before_cycle().unwrap();
        assert_eq!(world.slots.get(id).unwrap().materials.energy, 3000.0);
        assert_eq!(
            world
                .local_mode
                .evolution
                .as_ref()
                .unwrap()
                .cycle_in_attempt,
            11
        );
        world.native_cycle = Some(1);
        world.staged_evolution_before_cycle().unwrap();
        assert_eq!(world.slots.get(id).unwrap().materials.energy, 2999.0);
        {
            let state = world.local_mode.evolution.as_mut().unwrap();
            state.cycle_in_attempt = 999_999;
            state.stagnant = false;
        }
        world.native_cycle = Some(999_999);
        world.staged_evolution_before_cycle().unwrap();
        assert!(!world.local_mode.evolution.as_ref().unwrap().stagnant);
        world.native_cycle = Some(1_000_000);
        world.staged_evolution_before_cycle().unwrap();
        assert!(world.local_mode.evolution.as_ref().unwrap().stagnant);
    }

    #[test]
    fn native_cycle_zerobot_test_records_one_and_checks_exactly_eight_thousand() {
        let mut world = simple();
        {
            let state = world.local_mode.evolution.as_mut().unwrap();
            state.config.mode = EvolutionMode::Zerobot;
            state.phase = EvolutionPhase::ZerobotTesting;
            state.test_start_energy = 17.0;
            state.cycle_in_attempt = 1;
        }
        world.native_cycle = Some(0);
        world.staged_evolution_after_cycle(None).unwrap();
        assert_eq!(
            world
                .local_mode
                .evolution
                .as_ref()
                .unwrap()
                .test_start_energy,
            17.0
        );
        world.native_cycle = Some(1);
        world.staged_evolution_after_cycle(None).unwrap();
        let state = world.local_mode.evolution.as_ref().unwrap();
        assert_eq!(
            state.test_start_energy,
            world.staged_role_energy(&state.mutator_species)
        );
        for cycle in [7999, 8001] {
            world.native_cycle = Some(cycle);
            world
                .local_mode
                .evolution
                .as_mut()
                .unwrap()
                .cycle_in_attempt = 8000;
            world.staged_evolution_after_cycle(None).unwrap();
            assert_eq!(
                world.local_mode.evolution.as_ref().unwrap().phase,
                EvolutionPhase::ZerobotTesting
            );
        }
        world.native_cycle = Some(8000);
        world.staged_evolution_after_cycle(None).unwrap();
        assert_eq!(
            world.local_mode.evolution.as_ref().unwrap().phase,
            EvolutionPhase::Evolving
        );
        assert_eq!(world.native_cycle, Some(-1));
    }

    #[test]
    fn native_cycle_staged_champion_both_dead_retries_without_a_loss_or_round_advance() {
        let mut world = simple();
        {
            let state = world.local_mode.evolution.as_mut().unwrap();
            state.phase = EvolutionPhase::Testing;
            state.candidate = state.mutators.clone();
        }
        world.native_cycle = Some(29);
        world.set_native_gaussian_spare(Some(2.5)).unwrap();
        world.local_mode.cycle_in_round = 9;
        let round = world.local_mode.current_round;
        let restart_count = world.local_mode.restart_count;
        let state = world.local_mode.evolution.as_ref().unwrap();
        let stage = state.stage;
        let test_cursor = state.test_result_cursor;
        let ids = world.slots.ids_in_slot_order();
        world.remove_many(&ids).unwrap();
        world.advance_staged_test().unwrap();
        let state = world.local_mode.evolution.as_ref().unwrap();
        assert_eq!(state.phase, EvolutionPhase::Testing);
        assert_eq!(state.stage, stage);
        assert_eq!(state.test_result_cursor, test_cursor);
        assert_eq!(world.local_mode.current_round, round);
        assert_eq!(world.local_mode.restart_count, restart_count);
        assert!(world.local_mode.results.is_empty());
        assert_eq!(world.local_mode.adjudication, None);
        assert_eq!(world.native_cycle, Some(29));
        assert_eq!(world.read_native_gaussian_spare(), Some(2.5));
        assert!(world.population() > 0);
    }

    #[test]
    fn native_cycle_zerobot_observation_excludes_zero_despite_elapsed_multiple_of_fifty() {
        let mut world = simple();
        let mutators = {
            let state = world.local_mode.evolution.as_mut().unwrap();
            state.config.mode = EvolutionMode::Zerobot;
            state.cycle_in_attempt = 50;
            state.mutator_species.clone()
        };
        for robot in world
            .slots
            .iter_mut()
            .filter_map(|slot| slot.organism.as_mut())
        {
            if mutators.contains(&robot.identity.species) {
                robot.program.mutation_metadata.native_last_mutations = 1;
                robot.materials.energy = 30_000.0;
            }
        }
        world.native_cycle = Some(0);
        world.staged_evolution_after_cycle(None).unwrap();
        assert_eq!(
            world
                .local_mode
                .evolution
                .as_ref()
                .unwrap()
                .zerobot_previous_id,
            0
        );
        world.native_cycle = Some(50);
        world
            .local_mode
            .evolution
            .as_mut()
            .unwrap()
            .cycle_in_attempt = 51;
        world.staged_evolution_after_cycle(None).unwrap();
        assert_ne!(
            world
                .local_mode
                .evolution
                .as_ref()
                .unwrap()
                .zerobot_previous_id,
            0
        );
    }

    #[test]
    fn hidden_baseline_skips_dna_movement_collision_grid_and_rendering() {
        let mut world = simple();
        let state = world.local_mode.evolution.as_mut().unwrap();
        state.hidden = true;
        state.hide_cycles = 15_000;
        let baseline = state.baseline_species[0];
        let id = world
            .slots
            .ids_in_slot_order()
            .into_iter()
            .find(|id| world.slots.get(*id).unwrap().identity.species == baseline)
            .unwrap();
        world.slots.get_mut(id).unwrap().kinematics.velocity = [1.0, 0.0];
        let position = world.slots.get(id).unwrap().kinematics.position;
        world.tick().unwrap();
        let robot = world.slots.get(id).unwrap();
        assert!(robot.flags.hidden);
        assert_eq!(robot.kinematics.position, position);
        assert_eq!(robot.counters.age, 0);
        assert_eq!(robot.program.memory.read(901), 0);
        assert!(
            !world
                .snapshot
                .render_instances
                .iter()
                .any(|instance| instance.slot == id.slot())
        );
        assert!(
            !world
                .sensing_spatial
                .neighbors(position, 1.0)
                .contains(&(id.slot() as usize))
        );
        assert!(
            world
                .slots
                .iter()
                .filter_map(|slot| slot.organism.as_ref())
                .filter(|robot| !robot.flags.hidden)
                .all(|robot| robot.program.memory.read(901) == 111)
        );
    }

    #[test]
    fn candidate_test_promotes_stage_zero_without_overwriting_the_archive() {
        let mut world = simple();
        let state = world.local_mode.evolution.as_ref().unwrap();
        let baseline = state.baseline_species.clone();
        let original = state.stages[&0].clone();
        for id in world.slots.ids_in_slot_order() {
            let robot = world.slots.get_mut(id).unwrap();
            if baseline.contains(&robot.identity.species) {
                world.slots.remove(id).unwrap();
            } else {
                robot.program.mutation_metadata.total_mutations = 1;
            }
        }
        world.tick().unwrap();
        let state = world.local_mode.evolution.as_ref().unwrap();
        assert_eq!(state.phase, EvolutionPhase::Testing);
        let candidate = state.mutator_species[0];
        world
            .adjudicate_local_round(Some(candidate), "champion test".into())
            .unwrap();
        let state = world.local_mode.evolution.as_ref().unwrap();
        assert_eq!(state.phase, EvolutionPhase::Evolving);
        assert_eq!(state.stage, 1);
        assert_eq!(state.stages[&0], original);
        assert_eq!(state.stages[&1][0].dna.metadata().mutations, 1);
    }

    #[test]
    fn modern_save_retains_archive_and_rng_while_presentation_omits_genome_copies() {
        let mut world = simple();
        world.tick().unwrap();
        let state = world.local_mode.evolution.as_ref().unwrap();
        assert!(!state.stages.is_empty());
        assert!(
            world
                .snapshot
                .local_mode
                .evolution
                .as_ref()
                .unwrap()
                .stages
                .is_empty()
        );
        let mut restored =
            crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(restored.local_mode.evolution.as_ref().unwrap(), state);
        assert_eq!(world.native_random, restored.native_random);
        world.tick().unwrap();
        restored.tick().unwrap();
        assert_eq!(world.local_mode.evolution, restored.local_mode.evolution);
        assert_eq!(world.native_random, restored.native_random);
        for id in world.slots.ids_in_slot_order() {
            assert_eq!(
                world.slots.get(id).unwrap().kinematics.position,
                restored.slots.get(id).unwrap().kinematics.position
            );
        }
    }

    #[test]
    fn completed_zerobot_stops_advancing_world_and_trace_stays_paused() {
        let mut world = simple();
        world.local_mode.evolution.as_mut().unwrap().phase = EvolutionPhase::Complete;
        world.local_mode.status = LocalModeStatus::Completed;
        let tick = world.tick;
        let random = world.native_random.clone();
        world.tick_many(10).unwrap();
        let trace = world.tick_with_trace().unwrap();
        assert_eq!(trace.tick, tick);
        assert_eq!(world.tick, tick);
        assert_eq!(world.native_random, random);
    }
}
