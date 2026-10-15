//! Robots.bas UpdateRobots: finish each robot before visiting the next slot.

use super::*;

pub(super) struct NativeRobotPhase {
    pub actor: OrganismId,
    pub cost_population: usize,
    pub chloroplast_total: f64,
    pub radius_bound: f32,
}

impl Engine {
    pub(super) fn native_pre_robot_pipeline(
        &mut self,
        trace: &mut CycleRecorder,
    ) -> Result<(), EngineError> {
        self.native_tie_index = Some(HashMap::new());
        self.rebuild_native_tie_index();
        self.native_buckets.synchronize(
            [self.config.world_width, self.config.world_height],
            &self.slots,
        );
        self.pending_gpu_collision_pairs = None;
        let mut contacts = native_collisions::NativeContacts::new(&self.slots);
        let population = self.cost_population();
        let chloroplast_total = self.phase_chloroplast_total();
        for tie in &mut self.ties {
            tie.information = false;
        }
        for id in self.slots.ids_in_slot_order() {
            if self.slots.get(id)?.flags.hidden {
                continue;
            }
            self.native_phase = Some(NativeRobotPhase {
                actor: id,
                cost_population: population,
                chloroplast_total,
                radius_bound: 415.0,
            });
            self.trace_robot_phase(trace, CyclePhase::Upkeep);
            self.upkeep_phase();
            self.trace_robot_phase(trace, CyclePhase::PoisonVenom);
            self.poison_venom_phase();
            self.trace_robot_phase(trace, CyclePhase::FixPosition);
            self.manage_native_fixed_state();
            self.trace_robot_phase(trace, CyclePhase::Mass);
            let robot = self.slots.get_mut(id)?;
            robot.materials.mass = derived_mass(
                robot.materials.body,
                robot.materials.shell,
                robot.materials.chloroplasts,
            )
            .clamp(1.0, 32_000.0);
            self.trace_robot_phase(trace, CyclePhase::ObstaclesBorders);
            self.apply_environment_features(true, false);
            let relocated = self.border_collision_phase();
            for member in relocated {
                contacts.relocate(member, self.slots.get(member)?.kinematics.position);
            }
            contacts.relocate(id, self.slots.get(id)?.kinematics.position);
            self.trace_robot_phase(trace, CyclePhase::TieSpring);
            self.native_tie_forces(id);
            self.trace_robot_phase(trace, CyclePhase::Movement);
            let started = Instant::now();
            self.movement_commands_phase();
            self.native_intent_phase();
            self.phase_timings.intent += elapsed_ms(started);
            self.trace_robot_phase(trace, CyclePhase::RobotCollisions);
            let started = Instant::now();
            self.native_collisions_for(id, &mut contacts);
            self.phase_timings.spatial += elapsed_ms(started);
            self.trace_robot_phase(trace, CyclePhase::StaticFriction);
            self.native_finalize_forces();
            self.trace_robot_phase(trace, CyclePhase::TieData);
            let started = Instant::now();
            self.tie_data_phase(trace);
            self.phase_timings.interactions += elapsed_ms(started);
        }
        self.native_phase = None;
        Ok(())
    }

    pub(super) fn rebuild_native_tie_index(&mut self) {
        if self.native_tie_index.is_none() {
            return;
        }
        let mut index: HashMap<OrganismId, Vec<usize>> = HashMap::new();
        for (ordinal, tie) in self.ties.iter().enumerate() {
            index.entry(tie.first).or_default().push(ordinal);
            index.entry(tie.second).or_default().push(ordinal);
        }
        for (&id, indices) in &mut index {
            self.order_native_tie_indices(id, indices);
        }
        self.native_tie_index = Some(index);
    }

    fn order_native_tie_indices(&self, id: OrganismId, indices: &mut [usize]) {
        if self.native_random.is_none() {
            return;
        }
        let Ok(robot) = self.slots.get(id) else {
            return;
        };
        // VB6 visits each robot's own compact tie array. Global pair order
        // cannot represent both endpoints' independently saved ordering.
        indices.sort_by_key(|&ordinal| {
            let tie = &self.ties[ordinal];
            let (other, port) = if tie.first == id {
                (tie.second, tie.first_port)
            } else {
                (tie.first, tie.second_port)
            };
            robot
                .ties
                .iter()
                .position(|endpoint| {
                    endpoint.is_some_and(|endpoint| {
                        endpoint.other == Some(other) && endpoint.port == port
                    })
                })
                .unwrap_or(MAX_TIES_PER_ROBOT)
        });
    }

    pub(super) fn tie_indices_for(&self, id: OrganismId) -> Vec<usize> {
        if let Some(index) = &self.native_tie_index {
            return index.get(&id).cloned().unwrap_or_default();
        }
        let mut indices = self
            .ties
            .iter()
            .enumerate()
            .filter_map(|(index, tie)| tie.contains(id).then_some(index))
            .collect::<Vec<_>>();
        self.order_native_tie_indices(id, &mut indices);
        indices
    }

    pub(super) fn phase_tie_indices(&self) -> Vec<usize> {
        match &self.native_phase {
            Some(phase) => self.tie_indices_for(phase.actor),
            None => (0..self.ties.len()).collect(),
        }
    }

    pub(super) fn native_mid_robot_pipeline(
        &mut self,
        trace: &mut CycleRecorder,
    ) -> Result<(), EngineError> {
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
                if self.slots.get(id).is_err() {
                    continue;
                }
                self.native_phase = Some(NativeRobotPhase {
                    actor: id,
                    cost_population: population,
                    chloroplast_total,
                    radius_bound: 415.0,
                });
                self.trace_robot_phase(trace, CyclePhase::Tie);
                let started = Instant::now();
                self.tie_exchange_phase(trace);
                self.flush_native_disqualifications()?;
                self.phase_timings.interactions += elapsed_ms(started);
                if self.slots.get(id).is_err() {
                    continue;
                }
                self.trace_robot_phase(trace, CyclePhase::BirthTieHeredity);
                self.birth_tie_heredity_phase(trace);
                self.trace_robot_phase(trace, CyclePhase::Aim);
                self.aim_commands_phase();
                self.trace_robot_phase(trace, CyclePhase::Virus);
                self.virus_commands_phase();
                self.flush_native_disqualifications()?;
                if self.slots.get(id).is_err() {
                    continue;
                }
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
        if self.native_random.is_some() {
            return self.vegetation_runtime.native_chloroplast_population() as f64 * 16_000.0;
        }
        self.native_phase.as_ref().map_or_else(
            || {
                self.slots
                    .iter()
                    .filter_map(|slot| slot.organism.as_ref())
                    .map(|robot| f64::from(robot.materials.chloroplasts.max(0.0)))
                    .sum()
            },
            |phase| phase.chloroplast_total,
        )
    }

    fn flush_disqualification_for_actor(
        &mut self,
        population: &mut usize,
        chloroplast_total: &mut f64,
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
        let radius_bound = self
            .slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .map(|robot| robot.materials.radius)
            .fold(415.0_f32, f32::max);
        let ids = self.slots.ids_in_slot_order();
        let mut births = LifecycleQueues::default();
        let result = (|| {
            for id in ids {
                let Ok(robot) = self.slots.get(id) else {
                    continue;
                };
                if robot.flags.hidden {
                    continue;
                }
                let old_chloroplasts = f64::from(robot.materials.chloroplasts);
                let counted = !robot.flags.corpse
                    && (self.config.dna_costs.dynamic_all_robots || !robot.flags.vegetable);
                self.native_phase = Some(NativeRobotPhase {
                    actor: id,
                    cost_population: population,
                    chloroplast_total,
                    radius_bound,
                });
                self.trace_robot_phase(trace, CyclePhase::TieReadback);
                self.trace_robot_phase(trace, CyclePhase::Mutation);
                let started = Instant::now();
                self.mutation_phase(MutationContext::Lifelong);
                self.phase_timings.mutation += elapsed_ms(started);
                self.trace_robot_phase(trace, CyclePhase::Material);
                self.material_commands_phase();
                if !self
                    .flush_disqualification_for_actor(&mut population, &mut chloroplast_total)?
                {
                    continue;
                }
                self.trace_robot_phase(trace, CyclePhase::Waste);
                self.waste_phase();
                self.trace_robot_phase(trace, CyclePhase::Shot);
                self.prepare_projectile_requests_phase();
                self.projectile_spawn_phase();
                if !self
                    .flush_disqualification_for_actor(&mut population, &mut chloroplast_total)?
                {
                    continue;
                }
                self.trace_robot_phase(trace, CyclePhase::Chloroplast);
                self.chloroplast_phase();
                chloroplast_total +=
                    f64::from(self.slots.get(id)?.materials.chloroplasts) - old_chloroplasts;
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
                if !self
                    .flush_disqualification_for_actor(&mut population, &mut chloroplast_total)?
                {
                    continue;
                }
                self.trace_robot_phase(trace, CyclePhase::AgeTimer);
                self.age_timer_phase();
                self.trace_robot_phase(trace, CyclePhase::CorpseRemoval);
                self.queue_deaths(&mut lifecycle, trace);
                if !lifecycle.deaths.is_empty() {
                    let robot = self.slots.get(id)?;
                    let converts_to_corpse = self.config.corpse_decay.enabled
                        && !robot.flags.corpse
                        && !robot.flags.dead
                        && robot.materials.body >= 0.5;
                    if !converts_to_corpse {
                        // ManageDeath queues KillRobot; ReproduceAndKill births
                        // first, then releases the queued death slots.
                        self.slots.get_mut(id)?.flags.dead = true;
                        births.deaths.append(&mut lifecycle.deaths);
                        self.trace_robot_phase(trace, CyclePhase::TotalEnergy);
                        continue;
                    }
                    let before_death = f64::from(self.slots.get(id)?.materials.chloroplasts);
                    self.commit_lifecycle_queues(lifecycle, trace)?;
                    if counted && self.slots.get(id).map_or(true, |robot| robot.flags.corpse) {
                        population = population.saturating_sub(1);
                    }
                    let current = self
                        .slots
                        .get(id)
                        .map_or(0.0, |robot| f64::from(robot.materials.chloroplasts));
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
    fn disable_ties_gates_dna_requests_while_preserving_existing_bonds() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let first = world.spawn_at(dna.clone(), [500.0; 2]).unwrap();
        let second = world.spawn_at(dna.clone(), [700.0, 500.0]).unwrap();
        let third = world.spawn_at(dna, [900.0, 500.0]).unwrap();
        world
            .ties
            .push(TieState::legacy(first, second, 200.0, 1, 1));
        world.refresh_tie_projections();
        world.set_ties_enabled(false);
        assert_eq!(world.ties.len(), 1);
        world
            .slots
            .get_mut(third)
            .unwrap()
            .program
            .memory
            .write(330, 7);
        world.tie_fire_phase(&mut CycleRecorder::disabled(0));
        assert_eq!(world.ties.len(), 1);
        assert_eq!(world.memory_at(third, 330).unwrap(), 0);
        world.set_ties_enabled(true);
        world
            .slots
            .get_mut(third)
            .unwrap()
            .program
            .memory
            .write(330, 7);
        world.tie_fire_phase(&mut CycleRecorder::disabled(0));
        assert_eq!(world.ties.len(), 2);
    }

    #[test]
    fn imported_tie_order_survives_index_rebuild_and_compacting_a_deletion() {
        let mut world = Engine::from_native_sim(include_bytes!(
            "../../tests/fixtures/db24832-original-user.sim"
        ))
        .unwrap();
        let id = world
            .slots
            .ids_in_slot_order()
            .into_iter()
            .find(|&id| world.slots.get(id).unwrap().ties.iter().flatten().count() >= 2)
            .expect("original fixture has a multi-tie robot");
        let origin = world.native_origin.as_ref().unwrap();
        let source = &origin.robots[world.slots.get(id).unwrap().native_record_index.unwrap()];
        let ids = world.slots.ids_in_slot_order();
        let expected = source
            .bytes("ties_core")
            .unwrap()
            .chunks_exact(38)
            .skip(1)
            .filter_map(|core| {
                let target = i16::from_le_bytes(core[2..4].try_into().unwrap());
                let index = origin
                    .robots
                    .iter()
                    .position(|r| r.word("old_slot").unwrap() == target)?;
                Some((
                    ids[index],
                    i32::from(i16::from_le_bytes(core[0..2].try_into().unwrap())),
                ))
            })
            .collect::<Vec<_>>();
        let visit_order = |world: &Engine| {
            world
                .tie_indices_for(id)
                .into_iter()
                .map(|i| {
                    let tie = &world.ties[i];
                    if tie.first == id {
                        (tie.second, tie.first_port)
                    } else {
                        (tie.first, tie.second_port)
                    }
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(visit_order(&world), expected);
        world.native_tie_index = Some(HashMap::new());
        world.rebuild_native_tie_index();
        assert_eq!(visit_order(&world), expected);
        let removed = world.tie_indices_for(id)[0];
        world.ties.remove(removed);
        world.rebuild_native_tie_index();
        world.refresh_tie_projections();
        assert_eq!(visit_order(&world), expected[1..]);
        let ids = world.slots.ids_in_slot_order();
        let new_target = ids
            .into_iter()
            .find(|candidate| {
                *candidate != id && !expected.iter().any(|(other, _)| other == candidate)
            })
            .unwrap();
        world
            .ties
            .push(TieState::legacy(id, new_target, 100.0, 9, 9));
        world.rebuild_native_tie_index();
        world.refresh_tie_projections();
        let mut appended = expected[1..].to_vec();
        appended.push((new_target, 9));
        assert_eq!(visit_order(&world), appended);
    }

    #[test]
    fn removal_reindexes_remaining_ties_before_the_next_endpoint_visit() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let ids: Vec<_> = [[200.0; 2], [300.0; 2], [600.0; 2], [700.0; 2]]
            .into_iter()
            .map(|position| world.spawn_at(dna.clone(), position).unwrap())
            .collect();
        world
            .ties
            .push(TieState::legacy(ids[0], ids[1], 100.0, 1, 1));
        world
            .ties
            .push(TieState::legacy(ids[2], ids[3], 100.0, 1, 1));
        world.native_tie_index = Some(HashMap::new());
        world.rebuild_native_tie_index();
        world.remove(ids[0]).unwrap();
        assert_eq!(world.tie_indices_for(ids[2]), vec![0]);
        world.native_tie_forces(ids[2]);
        world.refresh_tie_projections();
        assert_eq!(
            world.slots.get(ids[2]).unwrap().ties[0].unwrap().other,
            Some(ids[3])
        );
        // Exercise the independent command-deletion path too, which must
        // rebuild its index before refreshing endpoint projections.
        world
            .slots
            .get_mut(ids[2])
            .unwrap()
            .program
            .memory
            .write(467, 1);
        world.tie_exchange_phase(&mut CycleRecorder::disabled(0));
        assert!(world.ties.is_empty());
        assert!(world.tie_indices_for(ids[3]).is_empty());
    }

    #[test]
    fn queued_kill_does_not_release_capacity_until_after_reproduction_attempts() {
        let mut world = Engine::new(EngineConfig {
            organism_capacity: 2,
            metabolism_cost: 1,
            ..EngineConfig::testing()
        })
        .unwrap();
        let parent = world
            .spawn_at(
                LegacyDna::parse("cond *.robage 0 = start 50 .repro store stop").unwrap(),
                [500.0; 2],
            )
            .unwrap();
        let doomed = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [800.0; 2])
            .unwrap();
        world.set_organism_energy(doomed, 1).unwrap();
        let births = world.stats.births;
        world.tick().unwrap();
        assert!(world.slots.get(doomed).is_err());
        assert_eq!(world.population(), 1);
        assert_eq!(world.stats.births, births);
        assert_eq!(world.memory_at(parent, MEM_REPRO).unwrap(), 50);
        world.tick().unwrap();
        assert_eq!(world.population(), 2);
        assert_eq!(world.stats.births, births + 1);
    }

    fn tie_fixed_order(source_first: bool) -> (Engine, OrganismId, OrganismId) {
        let mut world = Engine::new(EngineConfig {
            metabolism_cost: 0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let first = world.spawn_at(dna.clone(), [500.0, 500.0]).unwrap();
        let second = world.spawn_at(dna, [700.0, 500.0]).unwrap();
        world
            .ties
            .push(TieState::legacy(first, second, 200.0, 1, 1));
        let (source, target) = if source_first {
            (first, second)
        } else {
            (second, first)
        };
        let robot = world.slots.get_mut(source).unwrap();
        robot.program.memory.write(455, 1);
        robot.program.memory.write(452, MEM_FIXED);
        robot.program.memory.write(453, 1);
        (world, source, target)
    }

    #[test]
    fn early_tie_write_fixes_later_robot_in_same_pre_pass() {
        let (mut world, _, target) = tie_fixed_order(true);
        world
            .native_pre_robot_pipeline(&mut CycleRecorder::disabled(0))
            .unwrap();
        let robot = world.slots.get(target).unwrap();
        assert_eq!(robot.program.memory.read(MEM_FIXED), 1);
        assert!(robot_is_fixed(robot, true));
    }

    #[test]
    fn late_tie_write_to_earlier_robot_waits_for_next_pre_pass() {
        let (mut world, _, target) = tie_fixed_order(false);
        world
            .native_pre_robot_pipeline(&mut CycleRecorder::disabled(0))
            .unwrap();
        let robot = world.slots.get(target).unwrap();
        assert_eq!(robot.program.memory.read(MEM_FIXED), 1);
        assert!(!robot_is_fixed(robot, true));
        world
            .native_pre_robot_pipeline(&mut CycleRecorder::disabled(0))
            .unwrap();
        assert!(robot_is_fixed(world.slots.get(target).unwrap(), true));
    }

    #[test]
    fn native_pre_trace_finishes_tie_data_before_next_robot_upkeep() {
        let (mut world, first, second) = tie_fixed_order(true);
        let trace = world.tick_with_trace().unwrap();
        let visits: Vec<_> = trace
            .events
            .iter()
            .filter_map(|event| match event {
                crate::CycleEvent::RobotVisited { phase, id, .. } => Some((*phase, *id)),
                _ => None,
            })
            .collect();
        let first_data = visits
            .iter()
            .position(|visit| *visit == (CyclePhase::TieData, first))
            .unwrap();
        let second_upkeep = visits
            .iter()
            .position(|visit| *visit == (CyclePhase::Upkeep, second))
            .unwrap();
        assert!(first_data < second_upkeep);
    }

    #[test]
    fn later_robot_observes_earlier_body_command_but_not_the_reverse() {
        let mut world = Engine::new(EngineConfig {
            metabolism_cost: 0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let dna = LegacyDna::parse("start 100 .strbody store stop").unwrap();
        let first = world.spawn_at(dna.clone(), [500.0, 500.0]).unwrap();
        let second = world.spawn_at(dna, [700.0, 500.0]).unwrap();
        world.set_organism_aim(first, 0).unwrap();
        world.set_organism_aim(second, 628).unwrap();
        let first_body = world.slots.get(first).unwrap().materials.body;
        let second_body = world.slots.get(second).unwrap().materials.body;
        world.tick().unwrap();
        assert_eq!(
            world.memory_at(first, MEM_REF_BODY).unwrap(),
            resource_readback(second_body)
        );
        assert_eq!(
            world.memory_at(second, MEM_REF_BODY).unwrap(),
            resource_readback(first_body + 10.0)
        );
        assert!(world.native_phase.is_none());
    }

    #[test]
    fn native_late_trace_finishes_first_robot_before_second_material_phase() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let first = world.spawn_at(dna.clone(), [200.0; 2]).unwrap();
        let second = world.spawn_at(dna, [700.0; 2]).unwrap();
        let trace = world.tick_with_trace().unwrap();
        let visits: Vec<_> = trace
            .events
            .iter()
            .filter_map(|event| match event {
                crate::CycleEvent::RobotVisited { phase, id, .. } => Some((*phase, *id)),
                _ => None,
            })
            .collect();
        let first_age = visits
            .iter()
            .position(|visit| *visit == (CyclePhase::AgeTimer, first))
            .unwrap();
        let second_material = visits
            .iter()
            .position(|visit| *visit == (CyclePhase::Material, second))
            .unwrap();
        assert!(first_age < second_material);
    }
}
