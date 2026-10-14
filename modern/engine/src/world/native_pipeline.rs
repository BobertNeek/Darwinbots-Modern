//! Robots.bas UpdateRobots: finish each robot before visiting the next slot.

use super::*;

pub(super) struct NativeRobotPhase {
    pub actor: OrganismId,
    pub cost_population: usize,
    pub chloroplast_total: f64,
    pub radius_bound: f32,
}

impl Engine {
    pub(super) fn rebuild_native_tie_index(&mut self) {
        if self.native_tie_index.is_none() { return; }
        let mut index: HashMap<OrganismId, Vec<usize>> = HashMap::new();
        for (ordinal, tie) in self.ties.iter().enumerate() {
            index.entry(tie.first).or_default().push(ordinal);
            index.entry(tie.second).or_default().push(ordinal);
        }
        self.native_tie_index = Some(index);
    }

    pub(super) fn tie_indices_for(&self, id: OrganismId) -> Vec<usize> {
        if let Some(index) = &self.native_tie_index { return index.get(&id).cloned().unwrap_or_default(); }
        self.ties.iter().enumerate().filter_map(|(index, tie)| tie.contains(id).then_some(index)).collect()
    }

    pub(super) fn phase_tie_indices(&self) -> Vec<usize> {
        match &self.native_phase {
            Some(phase) => self.tie_indices_for(phase.actor),
            None => (0..self.ties.len()).collect(),
        }
    }

    pub(super) fn native_mid_robot_pipeline(&mut self, trace: &mut CycleRecorder) -> Result<(), EngineError> {
        let started = Instant::now();
        // GPU integration computes independent results once. Publish each result
        // at the original robot's UpdatePosition point in the slot-ordered pass.
        let batch = self.prepare_physics_step()?;
        self.phase_timings.physics = elapsed_ms(started);
        self.native_tie_index = Some(HashMap::new());
        self.rebuild_native_tie_index();
        let population = self.cost_population();
        let chloroplast_total = self.phase_chloroplast_total();
        let result = (|| {
            for (index, &id) in batch.ids.iter().enumerate() {
                if self.slots.get(id).is_err() { continue; }
                self.native_phase = Some(NativeRobotPhase {
                    actor: id, cost_population: population, chloroplast_total, radius_bound: 415.0,
                });
                self.trace_robot_phase(trace, CyclePhase::Tie);
                let started = Instant::now();
                self.tie_exchange_phase(trace);
                self.flush_native_disqualifications()?;
                self.phase_timings.interactions += elapsed_ms(started);
                if self.slots.get(id).is_err() { continue; }
                self.trace_robot_phase(trace, CyclePhase::BirthTieHeredity);
                self.birth_tie_heredity_phase(trace);
                self.trace_robot_phase(trace, CyclePhase::Aim);
                self.aim_commands_phase();
                self.trace_robot_phase(trace, CyclePhase::Virus);
                self.virus_commands_phase();
                self.flush_native_disqualifications()?;
                if self.slots.get(id).is_err() { continue; }
                self.trace_robot_phase(trace, CyclePhase::PositionIntegration);
                let robot = self.slots.get_mut(id)?;
                robot.kinematics.position = batch.batch.positions[index];
                robot.kinematics.velocity = batch.batch.velocities[index];
                self.finish_native_integration(id)?;
                self.trace_robot_phase(trace, CyclePhase::StoreClamps);
                self.store_clamps_phase();
            }
            Ok(())
        })();
        self.native_phase = None;
        result
    }

    pub(super) fn phase_slot_range(&self) -> std::ops::Range<usize> {
        match &self.native_phase {
            Some(phase) if self.slots.get(phase.actor).is_ok() => {
                let slot = phase.actor.slot() as usize;
                slot..slot + 1
            }
            Some(_) => 0..0,
            None => 0..self.slots.len(),
        }
    }

    pub(super) fn phase_robot_ids(&self) -> Vec<OrganismId> {
        self.phase_slot_range()
            .filter_map(|slot| {
                self.slots[slot].organism.as_ref()?;
                Some(OrganismId::new(slot as u32, self.slots[slot].generation))
            })
            .collect()
    }

    pub(super) fn phase_chloroplast_total(&self) -> f64 {
        if self.native_random.is_some() { return self.vegetation_runtime.native_chloroplast_population() as f64 * 16_000.0; }
        self.native_phase.as_ref().map_or_else(
            || self.slots.iter().filter_map(|slot| slot.organism.as_ref())
                .map(|robot| f64::from(robot.materials.chloroplasts.max(0.0))).sum(),
            |phase| phase.chloroplast_total,
        )
    }

    fn flush_disqualification_for_actor(
        &mut self, population: &mut usize, chloroplast_total: &mut f64,
    ) -> Result<bool, EngineError> {
        self.flush_native_disqualifications()?;
        let phase = self.native_phase.as_ref().unwrap();
        *population = phase.cost_population;
        *chloroplast_total = phase.chloroplast_total;
        Ok(self.slots.get(phase.actor).is_ok())
    }

    pub(super) fn native_late_robot_pipeline(
        &mut self,
        trace: &mut CycleRecorder,
    ) -> Result<LifecycleQueues, EngineError> {
        self.spatial_index_phase();
        // Angles depend on the completed movement pass. New ties refresh their
        // projections when created; resource changes do not change geometry.
        self.tie_readback_phase();
        let mut population = self.cost_population();
        let mut chloroplast_total = self.phase_chloroplast_total();
        let radius_bound = self.slots.iter().filter_map(|slot| slot.organism.as_ref())
            .map(|robot| robot.materials.radius).fold(415.0_f32, f32::max);
        let ids = self.slots.ids_in_slot_order();
        let mut births = LifecycleQueues::default();
        let result = (|| {
            for id in ids {
                let Ok(robot) = self.slots.get(id) else { continue };
                let old_chloroplasts = f64::from(robot.materials.chloroplasts);
                let counted = !robot.flags.corpse
                    && (self.config.dna_costs.dynamic_all_robots || !robot.flags.vegetable);
                self.native_phase = Some(NativeRobotPhase {
                    actor: id, cost_population: population, chloroplast_total, radius_bound,
                });
                self.trace_robot_phase(trace, CyclePhase::TieReadback);
                self.trace_robot_phase(trace, CyclePhase::Mutation);
                let started = Instant::now();
                self.mutation_phase(MutationContext::Lifelong);
                self.phase_timings.mutation += elapsed_ms(started);
                self.trace_robot_phase(trace, CyclePhase::Material);
                self.material_commands_phase();
                if !self.flush_disqualification_for_actor(&mut population, &mut chloroplast_total)? { continue; }
                self.trace_robot_phase(trace, CyclePhase::Waste);
                self.waste_phase();
                self.trace_robot_phase(trace, CyclePhase::Shot);
                self.prepare_projectile_requests_phase();
                self.projectile_spawn_phase();
                if !self.flush_disqualification_for_actor(&mut population, &mut chloroplast_total)? { continue; }
                self.trace_robot_phase(trace, CyclePhase::Chloroplast);
                self.chloroplast_phase();
                chloroplast_total += f64::from(self.slots.get(id)?.materials.chloroplasts)
                    - old_chloroplasts;
                self.native_phase.as_mut().unwrap().chloroplast_total = chloroplast_total;
                self.trace_robot_phase(trace, CyclePhase::Body);
                self.body_commands_phase();
                self.trace_robot_phase(trace, CyclePhase::Buoyancy);
                self.buoyancy_commands_phase();
                self.trace_robot_phase(trace, CyclePhase::Reproduction);
                let started = Instant::now();
                self.advance_reproductive_timers();
                let mut lifecycle = self.prepare_lifecycle_queues(trace);
                self.phase_timings.lifecycle += elapsed_ms(started);
                births.births.append(&mut lifecycle.births);
                self.trace_robot_phase(trace, CyclePhase::Shock);
                self.shock_phase();
                self.trace_robot_phase(trace, CyclePhase::Senses);
                let started = Instant::now();
                self.sensing_phase()?;
                self.publish_core_senses_phase();
                self.phase_timings.sensing += elapsed_ms(started);
                self.trace_robot_phase(trace, CyclePhase::TieFire);
                let started = Instant::now();
                self.tie_fire_phase(trace);
                self.phase_timings.interactions += elapsed_ms(started);
                if !self.flush_disqualification_for_actor(&mut population, &mut chloroplast_total)? { continue; }
                self.trace_robot_phase(trace, CyclePhase::AgeTimer);
                self.age_timer_phase();
                self.trace_robot_phase(trace, CyclePhase::CorpseRemoval);
                self.queue_deaths(&mut lifecycle, trace);
                if !lifecycle.deaths.is_empty() {
                    let before_death = f64::from(self.slots.get(id)?.materials.chloroplasts);
                    self.commit_lifecycle_queues(lifecycle, trace)?;
                    if counted && self.slots.get(id).map_or(true, |robot| robot.flags.corpse) {
                        population = population.saturating_sub(1);
                    }
                    let current = self.slots.get(id).map_or(0.0, |robot| {
                        f64::from(robot.materials.chloroplasts)
                    });
                    chloroplast_total += current - before_death;
                    // Removed/corpse robots no longer contribute chloroplasts.
                    chloroplast_total = chloroplast_total.max(0.0);
                }
                self.trace_robot_phase(trace, CyclePhase::TotalEnergy);
            }
            Ok(births)
        })();
        self.native_phase = None;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_robot_observes_earlier_body_command_but_not_the_reverse() {
        let mut world = Engine::new(EngineConfig {
            metabolism_cost: 0, ..EngineConfig::testing()
        }).unwrap();
        let dna = LegacyDna::parse("start 100 .strbody store stop").unwrap();
        let first = world.spawn_at(dna.clone(), [500.0, 500.0]).unwrap();
        let second = world.spawn_at(dna, [700.0, 500.0]).unwrap();
        world.slots.get_mut(first).unwrap().set_heading(0.0);
        world.slots.get_mut(second).unwrap().set_heading(std::f32::consts::PI);
        let first_body = world.slots.get(first).unwrap().materials.body;
        let second_body = world.slots.get(second).unwrap().materials.body;
        world.tick().unwrap();
        assert_eq!(world.memory_at(first, MEM_REF_BODY).unwrap(), resource_readback(second_body));
        assert_eq!(world.memory_at(second, MEM_REF_BODY).unwrap(), resource_readback(first_body + 10.0));
        assert!(world.native_phase.is_none());
    }

    #[test]
    fn native_late_trace_finishes_first_robot_before_second_material_phase() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let first = world.spawn_at(dna.clone(), [200.0; 2]).unwrap();
        let second = world.spawn_at(dna, [700.0; 2]).unwrap();
        let trace = world.tick_with_trace().unwrap();
        let visits: Vec<_> = trace.events.iter().filter_map(|event| match event {
            crate::CycleEvent::RobotVisited { phase, id, .. } => Some((*phase, *id)),
            _ => None,
        }).collect();
        let first_age = visits.iter().position(|visit| *visit == (CyclePhase::AgeTimer, first)).unwrap();
        let second_material = visits.iter().position(|visit| *visit == (CyclePhase::Material, second)).unwrap();
        assert!(first_age < second_material);
    }
}
