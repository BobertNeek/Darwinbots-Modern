use super::*;
use crate::{DnaBlock, MAX_DNA_BLOCKS, EvolutionGenome, EvolutionMode, EvolutionPhase, EvolutionState};

fn genome_of(robot: &RobotState) -> EvolutionGenome {
    let mut dna = robot.program.dna.clone();
    dna.synchronize_runtime_metadata(robot.counters.generation,
        robot.program.mutation_metadata.total_mutations, &robot.identity.tag);
    EvolutionGenome { dna, mutation_profile: robot.program.mutation_profile.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple() -> Engine {
        let mut world = Engine::new(EngineConfig { metabolism_cost: 0, ..EngineConfig::testing() }).unwrap();
        world.configure_local_mode(LocalModeConfig { kind: LocalModeKind::SimpleSurvival,
            minimum_rounds: 1, maximum_rounds: 1,
            staged_evolution: Some(crate::EvolutionConfig { source_dna: "start 111 901 store stop".into(),
                ..Default::default() }), ..Default::default() }).unwrap();
        world.start_local_mode().unwrap();
        world
    }

    #[test]
    fn hidden_baseline_skips_dna_movement_collision_grid_and_rendering() {
        let mut world = simple();
        let state = world.local_mode.evolution.as_mut().unwrap();
        state.hidden = true;
        state.hide_cycles = 15_000;
        let baseline = state.baseline_species[0];
        let id = world.slots.ids_in_slot_order().into_iter()
            .find(|id| world.slots.get(*id).unwrap().identity.species == baseline).unwrap();
        world.slots.get_mut(id).unwrap().kinematics.velocity = [1.0, 0.0];
        let position = world.slots.get(id).unwrap().kinematics.position;
        world.tick().unwrap();
        let robot = world.slots.get(id).unwrap();
        assert!(robot.flags.hidden);
        assert_eq!(robot.kinematics.position, position);
        assert_eq!(robot.counters.age, 0);
        assert_eq!(robot.program.memory.read(901), 0);
        assert!(!world.snapshot.render_instances.iter().any(|instance| instance.slot == id.slot()));
        assert!(!world.sensing_spatial.neighbors(position, 1.0).contains(&(id.slot() as usize)));
        assert!(world.slots.iter().filter_map(|slot| slot.organism.as_ref())
            .filter(|robot| !robot.flags.hidden).all(|robot| robot.program.memory.read(901) == 111));
    }

    #[test]
    fn candidate_test_promotes_stage_zero_without_overwriting_the_archive() {
        let mut world = simple();
        let state = world.local_mode.evolution.as_ref().unwrap();
        let baseline = state.baseline_species.clone();
        let original = state.stages[&0].clone();
        for id in world.slots.ids_in_slot_order() {
            let robot = world.slots.get_mut(id).unwrap();
            if baseline.contains(&robot.identity.species) { world.slots.remove(id).unwrap(); }
            else { robot.program.mutation_metadata.total_mutations = 1; }
        }
        world.tick().unwrap();
        let state = world.local_mode.evolution.as_ref().unwrap();
        assert_eq!(state.phase, EvolutionPhase::Testing);
        let candidate = state.mutator_species[0];
        world.adjudicate_local_round(Some(candidate), "champion test".into()).unwrap();
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
        assert!(world.snapshot.local_mode.evolution.as_ref().unwrap().stages.is_empty());
        let mut restored = crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(restored.local_mode.evolution.as_ref().unwrap(), state);
        assert_eq!(world.native_random, restored.native_random);
        world.tick().unwrap();
        restored.tick().unwrap();
        assert_eq!(world.local_mode.evolution, restored.local_mode.evolution);
        assert_eq!(world.native_random, restored.native_random);
        for id in world.slots.ids_in_slot_order() {
            assert_eq!(world.slots.get(id).unwrap().kinematics.position, restored.slots.get(id).unwrap().kinematics.position);
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

impl Engine {
    pub(super) fn staged_test_round(&mut self, winner: Option<SpeciesId>, complete: bool) -> Result<(), EngineError> {
        let Some(mut state) = self.local_mode.evolution.take() else { return Ok(()) };
        let result: Result<(), EngineError> = (|| {
            if complete {
                if winner.is_some_and(|id| state.mutator_species.contains(&id)) {
                    state.test_won().map_err(EngineError::Invariant)?;
                } else {
                    let random = self.native_random.get_or_insert_with(|| crate::native_random::NativeRandom::with_user_seed(self.config.seed));
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
        self.restart_staged_world()?;
        self.publish_snapshot();
        Ok(())
    }

    pub(super) fn advance_staged_test(&mut self) -> Result<(), EngineError> {
        let Some(state) = self.local_mode.evolution.as_ref() else { return Ok(()) };
        if self.local_mode.status != LocalModeStatus::Running || state.phase != EvolutionPhase::Testing { return Ok(()); }
        self.local_mode.cycle_in_round = self.local_mode.cycle_in_round.saturating_add(1);
        if !self.local_mode.cycle_in_round.is_multiple_of(10) { return Ok(()); }
        let baselines = self.staged_role_count(&state.baseline_species);
        let mutators = self.staged_role_count(&state.mutator_species);
        let limit = self.local_mode.config.as_ref().map_or(15_000, |config| config.maximum_cycles);
        let winner = if baselines == 0 && mutators > 0 { state.mutator_species.first().copied() }
            else if mutators == 0 && baselines > 0 { state.baseline_species.first().copied() }
            else if limit > 0 && self.local_mode.cycle_in_round > limit {
                if mutators > baselines { state.mutator_species.first().copied() }
                else if baselines > mutators { state.baseline_species.first().copied() } else { None }
            } else if baselines + mutators == 0 { None } else { return Ok(()) };
        self.finish_local_mode_round(winner, "staged champion test".into(), true)
    }

    pub(super) fn prepare_staged_evolution(&self, config: &LocalModeConfig) -> Result<Option<EvolutionState>, EngineError> {
        let Some(mut settings) = config.staged_evolution.clone() else { return Ok(None) };
        if self.config.advanced_mutation.normalization_enabled && settings.mutation_rate_limit.is_none() {
            settings.mutation_rate_limit = Some(f64::from(self.config.advanced_mutation.native_delta2.maximum_rate_per_base));
        }
        let dna = if settings.mode == EvolutionMode::Zerobot {
            LegacyDna::from_legacy_blocks(vec![DnaBlock { class: TokenClass::Number, value: 0 };
                settings.zerobot_initial_dna_length.min(MAX_DNA_BLOCKS - 1)])
        } else if !settings.source_dna.trim().is_empty() { LegacyDna::parse(&settings.source_dna)? }
        else {
            let source = config.source_species.map(|id| id.0 as usize).or_else(||
                self.species.iter().enumerate().position(|(index, definition)| !definition.vegetable
                    && self.species_templates.get(index).and_then(Option::as_ref).is_some()));
            source.and_then(|index| self.species_templates.get(index)).and_then(Option::as_ref)
                .map(|template| template.dna.clone()).ok_or_else(||
                    EngineError::Invariant("staged evolution needs a source bot or registered startup genome".into()))?
        };
        let profile = config.source_species.and_then(|id| self.species.get(id.0 as usize))
            .and_then(|definition| definition.mutation_profile.clone()).unwrap_or_else(|| {
                if self.default_mutation_profile == StandardMutationProfile::default() { self.native_default_mutation_profile(dna.blocks().len()) }
                else { self.default_mutation_profile.clone() }
            });
        let count = match settings.mode { EvolutionMode::SimpleSurvival => 1, EvolutionMode::Eco => 15, EvolutionMode::Zerobot => 8 };
        EvolutionState::new(settings, vec![EvolutionGenome { dna, mutation_profile: profile }; count])
            .map(Some).map_err(EngineError::Invariant)
    }

    pub(super) fn restart_staged_world(&mut self) -> Result<(), EngineError> {
        let Some(mut state) = self.local_mode.evolution.take() else { return Ok(()) };
        let result: Result<(), EngineError> = (|| {
            for id in self.slots.ids_in_slot_order() { self.slots.remove(id)?; }
            self.slots.free_slots.sort_unstable_by(|first, second| second.cmp(first));
            self.mutations_enabled = true;
            self.global_mutation_multiplier = self.global_mutation_multiplier.max(1.0);
            self.native_random.get_or_insert_with(|| crate::native_random::NativeRandom::with_user_seed(self.config.seed));
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
            let testing = matches!(state.phase, EvolutionPhase::Testing | EvolutionPhase::ZerobotTesting);
            let baselines = if zero_test { Vec::new() } else { state.stages[&state.opponent_stage].clone() };
            let mutators = if testing { state.candidate.clone() } else { state.mutators.clone() };
            let groups = baselines.len().max(mutators.len());
            while state.baseline_species.len() < groups {
                state.baseline_species.push(self.register_species_unpublished(SpeciesDefinition {
                    name: "Base.txt".into(), color: 0xffda6c59, ..Default::default()
                }));
            }
            while state.mutator_species.len() < groups {
                state.mutator_species.push(self.register_species_unpublished(SpeciesDefinition {
                    name: "Mutate.txt".into(), color: 0xff62a844, ..Default::default()
                }));
            }
            let quantity = if zero_test { 5 } else { match state.config.mode {
                EvolutionMode::Eco => 1,
                EvolutionMode::SimpleSurvival => 5,
                EvolutionMode::Zerobot => ((f64::from(self.config.world_width) * f64::from(self.config.world_height)).sqrt()
                    / (80.0 * 8.0 * f64::from(state.stage + 1).sqrt())).round_ties_even().max(1.0) as usize,
            }};
            let mut remaining = self.config.organism_capacity;
            if state.config.mode == EvolutionMode::Zerobot && !testing {
                let random = self.native_random.get_or_insert_with(|| crate::native_random::NativeRandom::with_user_seed(self.config.seed));
                state.fitness_weight = (20.0 + random.rnd(-((state.stage + 1) as f32)) * 40.0).round_ties_even() as u16;
            }
            for group in 0..groups {
                for (genomes, ids, baseline) in [(&baselines, &state.baseline_species, true), (&mutators, &state.mutator_species, false)] {
                    let Some(genome) = genomes.get(group) else { continue };
                    let species = ids[group];
                    self.species[species.0 as usize].name = if baseline { "Base.txt" } else if testing { "Test.txt" } else { "Mutate.txt" }.into();
                    self.species[species.0 as usize].mutation_profile = Some(genome.mutation_profile.clone());
                    self.species_templates[species.0 as usize] = Some(SpeciesTemplate {
                        dna: genome.dna.clone(), initial_energy: 3_000, initial_population: quantity,
                    });
                    for ordinal in 0..quantity {
                        if remaining == 0 { break; }
                        remaining -= 1;
                        let position = self.native_placement_position(species, self.config.seed ^ ordinal as u64);
                        let id = self.spawn_species_at_unpublished(genome.dna.clone(), species, position)?;
                        self.native_placement_heading(id)?;
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
        self.slots.iter().filter_map(|slot| slot.organism.as_ref())
            .filter(|robot| !robot.flags.corpse && ids.contains(&robot.identity.species)).count()
    }

    fn staged_role_energy(&self, ids: &[SpeciesId]) -> f64 {
        self.slots.iter().filter_map(|slot| slot.organism.as_ref())
            .filter(|robot| !robot.flags.corpse && ids.contains(&robot.identity.species))
            .map(|robot| f64::from(robot.materials.energy) + f64::from(robot.materials.body) * 10.0).sum()
    }

    fn staged_average_energy(&self, ids: &[SpeciesId]) -> Option<f64> {
        let mut count = 0;
        let mut energy = 0.0;
        for robot in self.slots.iter().filter_map(|slot| slot.organism.as_ref()) {
            if ids.contains(&robot.identity.species) && robot.program.mutation_metadata.native_last_mutations > 0 {
                count += 1; energy += f64::from(robot.materials.energy);
            }
        }
        (count > 0).then(|| energy / f64::from(count))
    }

    fn staged_candidates(&mut self, eco: bool, weight: u16) -> Vec<EvolutionGenome> {
        if !eco { return self.find_best_robot_weighted(weight).and_then(|id| self.slots.get(id).ok()).map(genome_of).into_iter().collect(); }
        let ids = self.slots.ids_in_slot_order();
        let mut diversity = HashMap::new();
        for &id in &ids {
            let robot = self.slots.get(id).unwrap();
            if robot.flags.corpse || robot.flags.vegetable || robot.flags.hidden { continue; }
            let name = &self.species[robot.identity.species.0 as usize].name;
            let distance: f32 = ids.iter().copied().filter(|other| *other != id)
                .filter_map(|other| self.slots.get(other).ok()).filter(|target| !target.flags.corpse
                    && self.species[target.identity.species.0 as usize].name == *name)
                .map(|target| robot.program.dna.genetic_distance(&target.program.dna)).sum();
            diversity.insert(id, distance);
        }
        let population_exponent = if weight < 100 { 1.0 } else { f64::from(200 - weight) / 100.0 };
        let energy_exponent = f64::from(weight.min(100)) / 100.0;
        let mut selected = Vec::new();
        for _ in 0..15 {
            // Deleting a selected family member changes later family scores.
            let families = self.native_family_scores();
            let mut best = None;
            let mut best_score = 0.0;
            for id in self.slots.ids_in_slot_order() {
                let Some(distance) = diversity.get(&id) else { continue };
                let family = families[id.slot() as usize];
                let score = (family.0 as f64).powf(population_exponent)
                    * family.1.max(0.0).powf(energy_exponent) * f64::from(*distance);
                if score >= best_score { best_score = score; best = Some(id); }
            }
            let Some(id) = best else { break };
            selected.push(genome_of(self.slots.get(id).unwrap()));
            self.slots.remove(id).expect("selected evolution candidate must exist");
            diversity.remove(&id);
        }
        selected
    }

    /// Returns true when a process-style restart replaced the world this cycle.
    pub(super) fn staged_evolution_before_cycle(&mut self) -> Result<(bool, Option<f64>), EngineError> {
        if self.local_mode.status != LocalModeStatus::Running { return Ok((false, None)); }
        let Some(mut state) = self.local_mode.evolution.take() else { return Ok((false, None)) };
        let mut restart = false;
        state.cycle_in_attempt = state.cycle_in_attempt.saturating_add(1);
        let evolving = state.phase == EvolutionPhase::Evolving;
        let baselines = self.staged_role_count(&state.baseline_species);
        let mutators = self.staged_role_count(&state.mutator_species);
        if evolving && state.config.mode != EvolutionMode::Zerobot {
            let random = self.native_random.get_or_insert_with(|| crate::native_random::NativeRandom::with_user_seed(self.config.seed));
            if mutators == 0 { state.decrease_difficulty(random); restart = true; }
            else if baselines == 0 {
                let changed = self.find_best_robot_weighted(state.fitness_weight).and_then(|id| self.slots.get(id).ok())
                    .is_some_and(|robot| robot.program.mutation_metadata.total_mutations > 0);
                if changed && (state.config.mode != EvolutionMode::Eco || mutators >= 15) {
                    let candidates = self.staged_candidates(state.config.mode == EvolutionMode::Eco, state.fitness_weight);
                    state.begin_test(candidates).map_err(EngineError::Invariant)?;
                } else {
                    state.scale_mutations();
                    state.increase_difficulty(self.native_random.as_mut().unwrap());
                }
                restart = true;
            }
        } else if evolving && mutators == 0 { restart = true; }
        if !restart && evolving && state.config.mode != EvolutionMode::Zerobot {
            state.mode_cycles = state.mode_cycles.saturating_add(1);
            if baselines > mutators { state.stagnant = false; }
            if state.cycle_in_attempt == 1_000_000 { state.stagnant = true; }
            let threshold = f64::from(state.hide_cycles) / 1.2 + f64::from(state.hide_offset);
            if f64::from(state.mode_cycles) > threshold {
                if state.reduction_unit == 150.0 && state.hidden && mutators < baselines {
                    state.mode_cycles = state.mode_cycles.saturating_sub(100);
                } else {
                    let delta = state.energy_delta / f64::from(state.mode_cycles);
                    state.inverse_delta += delta;
                    if state.hidden {
                        let next = (state.previous_energy_delta - delta) / f64::from(state.reduction_unit);
                        state.handicap = if next < state.handicap { next } else { (state.handicap * 9.0 + next) / 10.0 };
                        state.inverse_handicap = ((state.previous_inverse_delta - state.inverse_delta)
                            / f64::from(state.reduction_unit)).min(0.0);
                        if state.handicap - state.inverse_handicap > 0.1 { state.inverse_handicap = state.handicap - 0.1; }
                        state.previous_inverse_delta = state.inverse_delta;
                        state.inverse_delta = 0.0;
                        self.staged_separate_engagements(&state);
                        for slot in self.projectiles.live_slots() {
                            if matches!(self.projectiles.kinds[slot as usize], -1 | -6) { self.projectiles.deactivate(slot); }
                        }
                    }
                    state.previous_energy_delta = delta;
                    state.energy_delta = 0.0;
                    state.hidden = !state.hidden;
                    state.hide_offset = state.hide_cycles as f32 / 3.0 * self.native_random.as_mut().unwrap().unit();
                    state.mode_cycles = 0;
                }
            }
        }
        for robot in self.slots.iter_mut().filter_map(|slot| slot.organism.as_mut()) {
            robot.flags.hidden = state.hidden && !robot.flags.corpse && state.baseline_species.contains(&robot.identity.species);
            if state.hidden && state.mutator_species.contains(&robot.identity.species) {
                let scale = if robot.program.mutation_metadata.native_last_mutations > 0 { 1.0 } else { 0.5 };
                robot.materials.energy = (f64::from(robot.materials.energy) + state.current_handicap() * scale) as f32;
            }
        }
        let average = self.staged_average_energy(&state.mutator_species);
        self.local_mode.evolution = Some(state);
        if restart { self.restart_staged_world()?; }
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
                if !state.baseline_species.contains(&robot.identity.species) { continue; }
                let base_position = robot.kinematics.position;
                let base_radius = robot.materials.radius;
                let base_body = robot.materials.body;
                for &mutator in &ids {
                    let robot = self.slots.get(mutator).unwrap();
                    if !state.mutator_species.contains(&robot.identity.species) { continue; }
                    let body = base_body.max(robot.materials.body);
                    let range = base_radius + robot.materials.radius + if body > 10.0 { (f64::from(body).ln() * 60.0 + 41.0) as f32 } else { 40.0 } + 40.0;
                    let delta = std::array::from_fn::<_, 2, _>(|axis| base_position[axis] - robot.kinematics.position[axis]);
                    let distance = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
                    if distance >= range { continue; }
                    let offset = if distance == 0.0 { [0.0; 2] } else { delta.map(|value| -(value / distance) * (range - distance)) };
                    let group = self.file_transfer_group(mutator).unwrap_or_else(|_| vec![mutator]);
                    for member in group {
                        let robot = self.slots.get_mut(member).unwrap();
                        if !state.mutator_species.contains(&robot.identity.species) { continue; }
                        for axis in 0..2 { robot.kinematics.position[axis] += offset[axis]; }
                    }
                    changed = true; moved += 1;
                }
            }
            if !changed || f64::from(moved) > limit { break; }
        }
    }

    pub(super) fn staged_evolution_after_cycle(&mut self, average_before: Option<f64>) -> Result<(), EngineError> {
        let Some(mut state) = self.local_mode.evolution.take() else { return Ok(()) };
        let mut restart = false;
        if state.phase == EvolutionPhase::Evolving {
            if let Some(after) = self.staged_average_energy(&state.mutator_species) {
                state.energy_delta += after - average_before.unwrap_or(0.0);
            }
            if state.config.mode == EvolutionMode::Zerobot && state.cycle_in_attempt.is_multiple_of(50) {
                if let Some(id) = self.find_best_robot_weighted(state.fitness_weight) {
                    let robot = self.slots.get(id)?;
                    if state.mutator_species.contains(&robot.identity.species) {
                        let family = self.native_family_scores()[id.slot() as usize];
                        let weight = f64::from(state.fitness_weight);
                        let score = if family.2 { 0.0 } else { (family.0 as f64).powf(if weight < 100.0 { 1.0 } else { (200.0 - weight) / 100.0 })
                            * family.1.max(0.0).powf(weight.min(100.0) / 100.0) };
                        let mut genome = genome_of(robot);
                        restart = state.observe_zerobot(robot.identity.absolute_id, score,
                            robot.program.mutation_metadata.native_last_mutations > 0, &mut genome);
                        self.slots.get_mut(id)?.program.mutation_profile = genome.mutation_profile;
                    }
                }
            }
        } else if state.phase == EvolutionPhase::ZerobotTesting && state.cycle_in_attempt == 1 {
            state.test_start_energy = self.staged_role_energy(&state.mutator_species);
        } else if state.phase == EvolutionPhase::ZerobotTesting && state.cycle_in_attempt >= 8_000 {
            if self.staged_role_count(&state.mutator_species) > 10
                && self.staged_role_energy(&state.mutator_species) > state.test_start_energy * 2.0 {
                state.phase = EvolutionPhase::Complete;
                self.local_mode.status = LocalModeStatus::Completed;
            } else {
                state.phase = EvolutionPhase::Evolving;
                state.candidate.clear();
                if state.stage > 500 {
                    let seed = EvolutionGenome { dna: LegacyDna::from_legacy_blocks(vec![DnaBlock {
                        class: TokenClass::Number, value: 0,
                    }; state.config.zerobot_initial_dna_length.min(MAX_DNA_BLOCKS - 1)]),
                        mutation_profile: self.default_mutation_profile.clone() };
                    state.retired_stages.extend(std::mem::take(&mut state.stages).into_values());
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
        if restart { self.restart_staged_world()?; }
        Ok(())
    }
}
