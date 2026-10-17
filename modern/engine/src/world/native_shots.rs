//! Original updateshots ordering and createshot boundaries; the portable two-pass route remains separate.
use super::*;

pub(super) fn literal_owner_slot(owner: OrganismId) -> i32 {
    // Wrapping maps the zero sentinel to0 and retains signed native Word parents.
    owner.slot().wrapping_add(1) as i32
}

pub(super) fn owner_from_literal_slot(parent: i16) -> OrganismId {
    OrganismId::new((i32::from(parent) as u32).wrapping_sub(1), 0)
}

pub(super) fn returned_velocity(actual: [f32; 2], incoming: [f32; 2]) -> [f32; 2] {
    std::array::from_fn(|axis| (actual[axis] - incoming[axis]) + actual[axis] * 0.5)
}

pub(super) fn receive_energy(materials: &mut BiologyState, partial: f32) -> f32 {
    let total = f64::from(materials.energy) + f64::from(partial) * 0.95;
    let overflow = if total > 32_000.0 {
        (total - 32_000.0) as f32
    } else {
        0.0
    };
    materials.energy = total.min(32_000.0) as f32;
    materials.body =
        (f64::from(materials.body) + f64::from(partial) * 0.004 + f64::from(overflow) * 0.1)
            .min(32_000.0) as f32;
    materials.waste = (f64::from(materials.waste) + f64::from(partial) * 0.01) as f32;
    (f64::from(partial) * 0.95) as f32
}

impl Engine {
    pub(super) fn current_native_shot_owner(&self, owner: OrganismId) -> Option<OrganismId> {
        // Source rob(parent) addresses the current occupant, irrespective of old generation.
        let entry = self.slots[..].get(owner.slot() as usize)?;
        entry.organism.as_ref().filter(|robot| robot.flags.exists)?;
        Some(OrganismId::new(owner.slot(), entry.generation))
    }

    fn native_shot_collision(&mut self, shot: usize, reach: f32) -> Option<usize> {
        // NewShotCollision adjusts current position and velocity before searching.
        let wrapped = self.config.axis_topology();
        for (axis, extent) in [self.config.world_width, self.config.world_height]
            .into_iter()
            .enumerate()
        {
            let position = &mut self.projectiles.positions[shot][axis];
            if wrapped[axis] {
                if *position > extent {
                    *position -= extent;
                } else if *position < 0.0 {
                    *position += extent;
                }
            } else if *position > extent {
                *position = extent;
                self.projectiles.velocities[shot][axis] =
                    -self.projectiles.velocities[shot][axis].abs();
            } else if *position < 0.0 {
                *position = 0.0;
                self.projectiles.velocities[shot][axis] =
                    self.projectiles.velocities[shot][axis].abs();
            }
        }
        let start = self.projectiles.positions[shot];
        let velocity = self.projectiles.velocities[shot];
        let end = [start[0] + velocity[0], start[1] + velocity[1]];
        let previous = self.projectiles.previous_positions[shot];
        let owner = self.projectiles.owners[shot];
        let radius = crate::physics::organism_radius(32_000.0, 0.0, self.config.fixed_bot_radii);
        let separation = (f64::from(radius).powi(2)
            + (f64::from(self.config.physics.max_velocity) * 2.0 + 40.0).powi(2))
        .sqrt() as f32;
        let mut candidates = self.spatial.segment_candidates(start, end, reach);
        candidates.sort_unstable();
        let mut selected = None;
        let mut earliest = 100.0_f32;
        for target in candidates {
            let Some(robot) = self.slots[target].organism.as_ref() else {
                continue;
            };
            if !robot.flags.exists
                || robot.flags.hidden
                || target as i32 + 1 == literal_owner_slot(owner)
                || (previous[0] - robot.kinematics.position[0]).abs() >= separation
                || (previous[1] - robot.kinematics.position[1]).abs() >= separation
            {
                continue;
            }
            let center: [f32; 2] = std::array::from_fn(|axis| {
                (robot.kinematics.position[axis] - robot.kinematics.velocity[axis])
                    + robot.kinematics.actual_velocity[axis]
            });
            let p = [start[0] - center[0], start[1] - center[1]];
            let radius = robot.materials.radius;
            let p2 = p[0] * p[0] + p[1] * p[1];
            if (f64::from(p2).sqrt() as f32) < radius {
                earliest = 0.0;
                selected = Some(target);
                break;
            }
            let d = [
                velocity[0] - robot.kinematics.actual_velocity[0],
                velocity[1] - robot.kinematics.actual_velocity[1],
            ];
            let d2 = d[0] * d[0] + d[1] * d[1];
            if d2 == 0.0 {
                continue;
            }
            let dot = d[0] * p[0] + d[1] * p[1];
            let discriminant = (f64::from(dot).powi(2)
                - f64::from(d2) * (f64::from(p2) - f64::from(radius).powi(2)))
                as f32;
            if discriminant < 0.0 {
                continue;
            }
            let root = f64::from(discriminant).sqrt() as f32;
            let time0 = (-dot - root) / d2;
            let time1 = (-dot + root) / d2;
            let usable0 = time0 > 0.0 && time0 < 1.0;
            let usable1 = time1 > 0.0 && time1 < 1.0;
            let hit = match (usable0, usable1) {
                (true, true) => time0.min(time1),
                (true, false) => time0,
                (false, true) => time1,
                (false, false) => continue,
            };
            // Original assigns the returned target even when this root is later.
            selected = Some(target);
            earliest = earliest.min(hit);
            if earliest <= 0.2 {
                break;
            }
        }
        if earliest <= 1.0 {
            self.projectiles.positions[shot] = [
                start[0] + velocity[0] * earliest,
                start[1] + velocity[1] * earliest,
            ];
        }
        selected
    }
}

impl Engine {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn spawn_native_effect_packet(
        &mut self,
        owner: OrganismId,
        position: [f32; 2],
        velocity: [f32; 2],
        distance: f32,
        value: f32,
        kind: i32,
        color: u32,
    ) -> Result<u32, EngineError> {
        let (species, vegetable, memory_location, poison_value) = self
            .slots
            .get(owner)
            .map(|robot| {
                let name = if robot.identity.species_file.is_empty() {
                    self.species
                        .get(robot.identity.species.0 as usize)
                        .map(|s| s.name.clone())
                } else {
                    Some(robot.identity.species_file.clone())
                };
                (
                    name,
                    robot.flags.vegetable,
                    robot.program.memory.read(834),
                    robot.program.memory.read(839),
                )
            })
            .unwrap_or((None, false, 0, 0));
        self.projectiles.spawn_native_packet(
            owner,
            position,
            velocity,
            distance,
            value,
            kind,
            species,
            vegetable,
            memory_location,
            poison_value,
            color,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn spawn_return_energy_packet(
        &mut self,
        native_boundary: bool,
        owner: OrganismId,
        position: [f32; 2],
        legacy_velocity: [f32; 2],
        actual_velocity: [f32; 2],
        incoming_velocity: [f32; 2],
        range: f32,
        value: f32,
    ) -> Result<u32, EngineError> {
        if native_boundary {
            self.spawn_native_effect_packet(
                owner,
                position,
                returned_velocity(actual_velocity, incoming_velocity),
                range * 40.0,
                value,
                -2,
                0xffff_ffff,
            )
        } else {
            Ok(self
                .projectiles
                .spawn_energy_packet(owner, position, legacy_velocity, range, value))
        }
    }
}

impl Engine {
    pub(super) fn native_projectile_update_phase(
        &mut self,
        trace: &mut CycleRecorder,
    ) -> Result<(), EngineError> {
        self.projectiles.enable_native_slots();
        // VB's For upper bound is captured once; current slot contents stay mutable.
        let entry_limit = self.projectiles.capacity();
        let positions: Vec<_> = self
            .slots
            .iter()
            .map(|slot| {
                slot.organism
                    .as_ref()
                    .map_or([0.0; 2], |robot| robot.kinematics.position)
            })
            .collect();
        let alive: Vec<_> = self
            .slots
            .iter()
            .map(|slot| {
                slot.organism
                    .as_ref()
                    .is_some_and(|robot| robot.flags.exists && !robot.flags.hidden)
            })
            .collect();
        self.spatial.rebuild_from_soa(&positions, &alive, 64.0);
        let reach_for = |robot: &RobotState| {
            robot.materials.radius
                + (robot.kinematics.actual_velocity[0] - robot.kinematics.velocity[0])
                    .hypot(robot.kinematics.actual_velocity[1] - robot.kinematics.velocity[1])
                + robot.kinematics.actual_velocity[0].hypot(robot.kinematics.actual_velocity[1])
                + 0.001
        };
        let mut reach = self
            .slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .map(reach_for)
            .fold(1.0_f32, f32::max);
        let mut visited = 0_u64;
        for shot in 0..entry_limit {
            if shot >= self.projectiles.alive.len() {
                continue;
            }
            if self.projectiles.impact_flash[shot] {
                self.projectiles.deactivate(shot as u32);
                // Shots.UpdateShots310-314 clears flash even when an obstacle
                // or another source action already set exist=False.
                self.projectiles.impact_flash[shot] = false;
                self.projectiles.dna_lengths[shot] = 0;
                self.projectiles.payloads[shot] = None;
            }
            if !self.projectiles.alive[shot] {
                continue;
            }
            visited += 1;
            if self.projectiles.kinds[shot] == -2 {
                self.add_native_shot_energy(self.projectiles.energies[shot])?;
            }
            if self.projectiles.kinds[shot] != -100
                && !self.projectiles.stored[shot]
                && let Some(target) = self.native_shot_collision(shot, reach)
            {
                let owner = self.projectiles.owners[shot];
                let robot = self.slots[target].organism.as_ref().unwrap();
                // Shots.parent is a slot; rob.parent is an absolute ID, even when they differ.
                let immune = robot.counters.age <= 1
                    && robot.identity.parent_absolute_id as u32 as i32 == literal_owner_slot(owner);
                if !immune {
                    self.projectiles
                        .native_collision_decay(shot, &self.config.shots);
                    if self.projectiles.kinds[shot] > 0 {
                        self.projectiles.kinds[shot] =
                            (self.projectiles.kinds[shot] - 1) % 1_000 + 1;
                    }
                    let effect = projectile_effect(
                        self.projectiles.kinds[shot],
                        self.projectiles.values[shot],
                    );
                    let traced_kind = trace.is_enabled().then(|| format!("{effect:?}"));
                    let taste_kind = self.projectiles.kinds[shot];
                    let previous = self.projectiles.previous_positions[shot];
                    let payload = self.projectiles.payloads[shot].clone();
                    let source_species = self.projectiles.source_species_names[shot].clone();
                    let location = self.projectiles.toxin_memory_locations[shot];
                    let value = self.projectiles.toxin_memory_values[shot];
                    let position = self.projectiles.positions[shot];
                    let velocity = self.projectiles.velocities[shot];
                    let range = self.projectiles.ranges[shot];
                    let energy = self.projectiles.energies[shot];
                    let power = self
                        .projectiles
                        .impact_power(shot as u32, &self.config.shots);
                    let toxin_power = self.projectiles.toxin_impact_power(shot as u32);
                    self.stats.projectile_impacts = self.stats.projectile_impacts.saturating_add(1);
                    let effect_owner = self.current_native_shot_owner(owner).unwrap_or(owner);
                    let applied = self.apply_projectile_effect_at_boundary(
                        effect_owner,
                        ProjectileTarget::Organism(target),
                        effect,
                        power,
                        toxin_power,
                        payload,
                        source_species,
                        location,
                        value,
                        position,
                        velocity,
                        range,
                        energy,
                        true,
                    )?;
                    if applied {
                        self.stats.projectile_effects =
                            self.stats.projectile_effects.saturating_add(1);
                        if let Some(kind) = traced_kind {
                            let id = OrganismId::new(target as u32, self.slots[target].generation);
                            trace.effect(CyclePhase::ShotUpdate, Some(effect_owner), id, kind);
                        }
                    }
                    if let Some(robot) = self.slots[target].organism.as_mut() {
                        write_shot_taste(robot, previous, taste_kind);
                        reach = reach.max(reach_for(robot));
                    }
                    self.projectiles.impact_flash[shot] = true;
                }
            }
            // Obstacles use current impact/start position, even on flashed or stored shots.
            for obstacle in &self.obstacles {
                if obstacle.shot_policy == ShapeShotPolicy::PassThrough {
                    continue;
                }
                let position = self.projectiles.positions[shot];
                if position[0] >= obstacle.minimum[0]
                    && position[0] <= obstacle.maximum[0]
                    && position[1] >= obstacle.minimum[1]
                    && position[1] <= obstacle.maximum[1]
                {
                    if obstacle.shot_policy == ShapeShotPolicy::Absorb {
                        self.projectiles.alive[shot] = false;
                    }
                    for axis in 0..2 {
                        let previous = self.projectiles.previous_positions[shot][axis];
                        if previous < obstacle.minimum[axis] || previous > obstacle.maximum[axis] {
                            self.projectiles.velocities[shot][axis] =
                                -self.projectiles.velocities[shot][axis];
                        }
                    }
                }
            }
            self.projectiles.previous_positions[shot] = self.projectiles.positions[shot];
            self.projectiles.positions[shot][0] += self.projectiles.velocities[shot][0];
            self.projectiles.positions[shot][1] += self.projectiles.velocities[shot][1];
            let no_age = self.projectiles.stored[shot]
                || (self.projectiles.kinds[shot] == -2
                    && self.config.shots.energy_shots_do_not_decay)
                || (self.projectiles.kinds[shot] == -4
                    && self.config.shots.waste_shots_do_not_decay);
            if !no_age {
                self.projectiles.ages[shot] = self.projectiles.ages[shot].saturating_add(1);
            }
            if self.projectiles.ages[shot] as f32 > self.projectiles.ranges[shot]
                && !self.projectiles.impact_flash[shot]
            {
                self.projectiles.alive[shot] = false;
                self.projectiles.dna_lengths[shot] = 0;
                self.projectiles.payloads[shot] = None;
            }
        }
        self.stats.native_shots_visited = Some(visited);
        if self.projectiles.native_should_compact(visited as usize) {
            // Original removes orphan stored viruses inside the copy loop, retaining its j increment.
            let mut orphans = Vec::new();
            for shot in 0..self.projectiles.alive.len() {
                if !self.projectiles.alive[shot] || !self.projectiles.stored[shot] {
                    continue;
                }
                let current = self
                    .current_native_shot_owner(self.projectiles.owners[shot])
                    .filter(|&owner| self.slots.get(owner).is_ok_and(|robot| !robot.flags.hidden));
                if let Some(current) = current {
                    // CompactShots rewrites current rob(parent).virusshot to each copied j.
                    // Refresh only here, so stored_slot's reverse scan selects the last j.
                    self.projectiles.owners[shot] = current;
                } else {
                    orphans.push(shot);
                }
            }
            self.projectiles
                .compact_native_slots_with_orphans(visited as usize, &orphans);
            // Virus references are resolved by stored_slot(owner), not an old numeric index.
        }
        Ok(())
    }
}

#[cfg(test)]
mod native_loop_tests {
    use super::*;

    fn persisted_free_slots(world: &Engine) -> Vec<u32> {
        // Observe the saved pool field without exposing allocator internals.
        let pool = serde_json::to_value(&world.projectiles).unwrap();
        let free_slots = pool
            .get("free_slots")
            .expect("saved projectile pool must contain free_slots");
        serde_json::from_value(free_slots.clone())
            .expect("saved projectile free_slots must be an array of u32 slots")
    }

    fn feeding_world(capacity: usize) -> (Engine, OrganismId, OrganismId) {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let owner = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [50.0, 200.0])
            .unwrap();
        let target = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [200.0, 200.0])
            .unwrap();
        for id in [owner, target] {
            let robot = world.slots.get_mut(id).unwrap();
            robot.materials.radius = 10.0;
            robot.materials.energy = 20_000.0;
            robot.materials.body = 1_000.0;
            robot.counters.age = 10;
        }
        world.config.shots.feeding_proportional = false;
        world.config.shots.feeding_fixed_power = 100;
        world.projectiles.reset_native_slots(capacity);
        (world, owner, target)
    }

    fn feeding_shot(world: &mut Engine, owner: OrganismId) -> usize {
        world
            .projectiles
            .spawn_native_packet(
                owner,
                [175.0, 200.0],
                [40.0, 0.0],
                400.0,
                100.0,
                -1,
                Some("owner.txt".into()),
                false,
                0,
                0,
                0xff00_0000,
            )
            .unwrap() as usize
    }

    fn phase(world: &mut Engine) {
        world
            .native_projectile_update_phase(&mut CycleRecorder::disabled(1))
            .unwrap();
        world.publish_snapshot();
    }

    #[test]
    fn native_flash_cleanup_also_clears_dead_slots_and_continues_exactly_after_db3s() {
        for was_alive in [false, true] {
            let (mut world, owner, _) = feeding_world(100);
            let shot = feeding_shot(&mut world, owner);
            let dna = LegacyDna::parse("start 123 900 store stop").unwrap();
            world.projectiles.kinds[shot] = -7;
            world.projectiles.stored[shot] = true;
            world.projectiles.dna_lengths[shot] = dna.blocks().len() as u32;
            world.projectiles.payloads[shot] = Some(dna);
            world.projectiles.gene_numbers[shot] = 3;
            world.projectiles.ages[shot] = 6;
            world.projectiles.alive[shot] = was_alive;
            world.projectiles.impact_flash[shot] = true;
            world.publish_snapshot();
            assert_eq!(world.snapshot().shots.len(), 1);
            assert_eq!(world.snapshot().shots[0].alive, was_alive);
            assert!(world.snapshot().shots[0].impact_flash);
            assert!(world.snapshot().shots[0].payload.is_some());
            let random = world.native_random.clone();
            let positions = world.projectiles.positions.clone();
            let previous_positions = world.projectiles.previous_positions.clone();
            let ages = world.projectiles.ages.clone();
            let free_slots = persisted_free_slots(&world);
            let saved = crate::SaveFile::encode(&world).unwrap();
            let mut restored = crate::SaveFile::decode(&saved).unwrap();
            assert_eq!(crate::SaveFile::encode(&restored).unwrap(), saved);
            assert_eq!(world.snapshot().shots, restored.snapshot().shots);
            for state in [&mut world, &mut restored] {
                phase(state);
                // Shots310-314 runs before the exist check, so neither old
                // flash state is visited, moved, aged, or charged to the RNG.
                assert!(!state.projectiles.alive[shot]);
                assert!(!state.projectiles.impact_flash[shot]);
                assert_eq!(state.projectiles.dna_lengths[shot], 0);
                assert!(state.projectiles.payloads[shot].is_none());
                assert_eq!(state.stats.native_shots_visited, Some(0));
                assert_eq!(state.stats.projectile_impacts, 0);
                assert_eq!(state.native_random, random);
                assert_eq!(state.projectiles.positions, positions);
                assert_eq!(state.projectiles.previous_positions, previous_positions);
                assert_eq!(state.projectiles.ages, ages);
                assert!(state.projectiles.stored[shot]);
                assert_eq!(state.projectiles.gene_numbers[shot], 3);
                let mut expected_free = free_slots.clone();
                if was_alive {
                    expected_free.push(shot as u32);
                }
                assert_eq!(persisted_free_slots(state), expected_free);
                assert!(state.snapshot().shots.is_empty());
            }
            assert_eq!(
                crate::SaveFile::encode(&world).unwrap(),
                crate::SaveFile::encode(&restored).unwrap()
            );
            for state in [&mut world, &mut restored] {
                let free_slots = persisted_free_slots(state);
                phase(state);
                assert_eq!(persisted_free_slots(state), free_slots);
                assert_eq!(state.stats.native_shots_visited, Some(0));
                assert_eq!(state.native_random, random);
                assert!(state.snapshot().shots.is_empty());
            }
            assert_eq!(
                crate::SaveFile::encode(&world).unwrap(),
                crate::SaveFile::encode(&restored).unwrap()
            );
        }
    }

    #[test]
    fn native_return_into_later_free_slot_moves_this_cycle_and_flash_clears_next_visit() {
        let (mut world, owner, target) = feeding_world(100);
        assert_eq!(feeding_shot(&mut world, owner), 0);
        phase(&mut world);
        assert_eq!(world.projectiles.positions[0], [230.0, 200.0]);
        assert_eq!(world.projectiles.previous_positions[0], [190.0, 200.0]);
        assert!(world.projectiles.alive[0] && world.projectiles.impact_flash[0]);
        assert_eq!(world.projectiles.positions[1], [150.0, 200.0]);
        assert_eq!(world.projectiles.previous_positions[1], [190.0, 200.0]);
        assert_eq!(world.projectiles.ages[1], 1);
        assert_eq!(world.projectiles.ranges[1], 23.0);
        assert_eq!(world.projectiles.energies[1], 100.0);
        assert_eq!(world.slots.get(target).unwrap().materials.energy, 19_910.0);
        assert_eq!(world.slots.get(target).unwrap().materials.body, 999.0);
        assert_eq!(world.stats.native_shots_visited, Some(2));
        let mut restored =
            crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(restored.stats.native_shots_visited, Some(2));
        phase(&mut world);
        phase(&mut restored);
        assert!(!world.projectiles.alive[0]);
        assert_eq!(world.stats.native_shots_visited, Some(1));
        assert_eq!(
            serde_json::to_value(&world.projectiles).unwrap(),
            serde_json::to_value(&restored.projectiles).unwrap()
        );
        assert_eq!(
            world.stats.native_shots_visited,
            restored.stats.native_shots_visited
        );
    }

    #[test]
    fn native_new_slot_beyond_the_entry_capacity_waits_until_the_next_cycle() {
        let (mut world, owner, _) = feeding_world(50);
        world.projectiles.alive[..49].fill(true);
        world.projectiles.kinds[..49].fill(-100);
        world.projectiles.ranges[..49].fill(100.0);
        assert_eq!(feeding_shot(&mut world, owner), 49);
        phase(&mut world);
        assert_eq!(world.projectiles.capacity(), 55);
        assert_eq!(world.projectiles.positions[50], [190.0, 200.0]);
        assert_eq!(world.projectiles.previous_positions[50], [230.0, 200.0]);
        assert_eq!(world.projectiles.ages[50], 0);
        assert_eq!(world.stats.native_shots_visited, Some(50));
    }

    #[test]
    fn native_reused_earlier_slot_waits_and_is_not_counted_a_second_time() {
        let (mut world, owner, _) = feeding_world(100);
        world.projectiles.alive[..99].fill(true);
        world.projectiles.kinds[..99].fill(-100);
        world.projectiles.ranges[..99].fill(100.0);
        world.projectiles.impact_flash[0] = true;
        assert_eq!(feeding_shot(&mut world, owner), 99);
        phase(&mut world);
        assert_eq!(world.projectiles.positions[0], [190.0, 200.0]);
        assert_eq!(world.projectiles.ages[0], 0);
        assert!(!world.projectiles.impact_flash[0]);
        assert_eq!(world.stats.native_shots_visited, Some(99));
    }

    #[test]
    fn native_visited_count_includes_expiry_and_persists_some_zero_and_legacy_none() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        world.projectiles.reset_native_slots(100);
        world
            .projectiles
            .spawn_native_packet(
                OrganismId::new(u32::MAX, 0),
                [100.0; 2],
                [0.0; 2],
                0.0,
                10.0,
                -100,
                None,
                false,
                0,
                0,
                0,
            )
            .unwrap();
        phase(&mut world); // Range1, age1.
        phase(&mut world); // Visits it, age2 expires it.
        assert_eq!(world.stats.native_shots_visited, Some(1));
        assert_eq!(world.projectiles.len(), 0);
        phase(&mut world);
        assert_eq!(world.stats.native_shots_visited, Some(0));
        let restored = crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(restored.stats.native_shots_visited, Some(0));
        let mut legacy = Engine::new(EngineConfig::testing()).unwrap();
        legacy.native_random = None;
        legacy.stats.native_shots_visited = Some(7);
        legacy
            .projectile_update_phase(&mut CycleRecorder::disabled(1))
            .unwrap();
        assert_eq!(legacy.stats.native_shots_visited, None);
        let restored = crate::SaveFile::decode(&crate::SaveFile::encode(&legacy).unwrap()).unwrap();
        assert_eq!(restored.stats.native_shots_visited, None);
    }

    #[test]
    fn native_collision_retains_original_later_target_earlier_position_quirk() {
        let (mut world, owner, first) = feeding_world(100);
        let second = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [216.0, 200.0])
            .unwrap();
        world.slots.get_mut(second).unwrap().materials.radius = 10.0;
        let shot = feeding_shot(&mut world, owner);
        world.projectiles.positions[shot] = [174.0, 200.0];
        let positions = world
            .slots
            .iter()
            .map(|slot| {
                slot.organism
                    .as_ref()
                    .map_or([0.0; 2], |robot| robot.kinematics.position)
            })
            .collect::<Vec<_>>();
        world
            .spatial
            .rebuild_from_soa(&positions, &vec![true; positions.len()], 64.0);
        assert_eq!(
            world.native_shot_collision(shot, 100.0),
            Some(second.slot() as usize)
        );
        assert_eq!(world.projectiles.positions[shot], [190.0, 200.0]);
        world.projectiles.positions[shot] = [182.0, 200.0];
        assert_eq!(
            world.native_shot_collision(shot, 100.0),
            Some(first.slot() as usize)
        );
    }

    #[test]
    fn native_primary_newborn_immunity_still_moves_from_the_collision_point() {
        let (mut world, owner, child) = feeding_world(100);
        let adult = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [216.0, 200.0])
            .unwrap();
        world.slots.get_mut(adult).unwrap().materials.radius = 10.0;
        let child_state = world.slots.get_mut(child).unwrap();
        child_state.counters.age = 1;
        child_state.identity.parents[0] = Some(owner);
        child_state.identity.parent_absolute_id = literal_owner_slot(owner) as u32 as u64;
        let child_energy = child_state.materials.energy;
        let adult_energy = world.slots.get(adult).unwrap().materials.energy;
        let shot = feeding_shot(&mut world, owner);
        world.projectiles.positions[shot] = [182.0, 200.0];
        phase(&mut world);
        assert_eq!(world.projectiles.positions[shot], [230.0, 200.0]);
        assert!(!world.projectiles.impact_flash[shot]);
        assert_eq!(
            world.slots.get(child).unwrap().materials.energy,
            child_energy
        );
        assert_eq!(
            world.slots.get(adult).unwrap().materials.energy,
            adult_energy
        );
        assert_eq!(world.stats.projectile_impacts, 0);
    }

    #[test]
    fn native_takenrg_uses_one_single_assignment_after_double_body_overflow_terms() {
        let mut materials = BiologyState {
            energy: 31_999.0,
            body: 1_000.0,
            ..Default::default()
        };
        assert_eq!(receive_energy(&mut materials, 32_000.0), 30_400.0);
        assert_eq!(materials.energy, 32_000.0);
        assert_eq!(materials.body, 4_167.9);
        assert_eq!(materials.waste, 320.0);
        assert_eq!(returned_velocity([3.0, -5.0], [11.0, 7.0]), [-6.5, -14.5]);
    }

    #[test]
    fn native_stored_orphan_survives_without_compaction_and_no_decay_freezes_age() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        world.projectiles.reset_native_slots(100);
        let shot = world
            .projectiles
            .spawn_native_packet(
                OrganismId::new(u32::MAX, 0),
                [1_100.0, 500.0],
                [5.0, 0.0],
                40.0,
                10.0,
                -7,
                None,
                false,
                0,
                0,
                0,
            )
            .unwrap() as usize;
        // createshot stores Range+40+1 as energy; only kind-2 uses val as nrg.
        assert_eq!(world.projectiles.energies[shot], 81.0);
        assert_eq!(world.projectiles.values[shot], 10);
        assert_eq!(world.projectiles.ranges[shot], 2.0);
        world.projectiles.stored[shot] = true;
        phase(&mut world);
        assert!(world.projectiles.alive[shot]);
        assert_eq!(world.projectiles.positions[shot], [1_105.0, 500.0]);
        assert_eq!(world.projectiles.ages[shot], 0);
        let mut restored =
            crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        assert!(restored.projectiles.alive[shot] && restored.projectiles.stored[shot]);
        for state in [&mut world, &mut restored] {
            state.projectiles.stored[shot] = false;
            state.projectiles.kinds[shot] = -2;
            state.projectiles.positions[shot] = [500.0; 2];
            state.projectiles.velocities[shot] = [0.0; 2];
            state.config.shots.energy_shots_do_not_decay = true;
        }
        for _ in 0..3 {
            for state in [&mut world, &mut restored] {
                phase(state);
                assert!(state.projectiles.alive[shot]);
                assert_eq!(state.projectiles.ages[shot], 0);
                assert_eq!(state.projectiles.energies[shot], 81.0);
                assert_eq!(state.stats.native_shots_visited, Some(1));
            }
        }
        for state in [&mut world, &mut restored] {
            state.config.shots.energy_shots_do_not_decay = false;
        }
        for expected_age in 1..=3 {
            for state in [&mut world, &mut restored] {
                phase(state);
                assert_eq!(state.projectiles.ages[shot], expected_age);
                assert_eq!(state.projectiles.alive[shot], expected_age <= 2);
                assert_eq!(state.projectiles.energies[shot], 81.0);
                assert_eq!(state.stats.native_shots_visited, Some(1));
            }
        }
        assert_eq!(
            serde_json::to_value(&world.projectiles).unwrap(),
            serde_json::to_value(&restored.projectiles).unwrap()
        );
        assert_eq!(world.native_energy, restored.native_energy);
    }

    #[test]
    fn native_zero_shadow_full_save_survives_and_original_sim_load_resets_it() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        world.projectiles.reset_native_slots(200);
        world.projectiles.compact_native_slots(0);
        let sentinel = world
            .projectiles
            .spawn_native_packet(
                OrganismId::new(u32::MAX, 0),
                [100.0; 2],
                [1.0; 2],
                40.0,
                10.0,
                -2,
                None,
                false,
                0,
                0,
                0,
            )
            .unwrap();
        assert_eq!(sentinel, u32::MAX);
        phase(&mut world);
        assert!(world.snapshot().shots.is_empty());
        let mut restored =
            crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(
            restored
                .projectiles
                .spawn_native_packet(
                    OrganismId::new(u32::MAX, 0),
                    [100.0; 2],
                    [1.0; 2],
                    40.0,
                    10.0,
                    -2,
                    None,
                    false,
                    0,
                    0,
                    0
                )
                .unwrap(),
            0
        );
        let raw = world.export_native_sim().unwrap();
        let mut loaded = Engine::from_native_sim(&raw).unwrap();
        assert_eq!(
            loaded
                .projectiles
                .spawn_native_packet(
                    OrganismId::new(u32::MAX, 0),
                    [100.0; 2],
                    [1.0; 2],
                    40.0,
                    10.0,
                    -2,
                    None,
                    false,
                    0,
                    0,
                    0
                )
                .unwrap(),
            0
        );
        assert_eq!(loaded.projectiles.len(), 1);
    }
    #[test]
    fn native_packet_conversion_error_restores_cursor_payload_and_stats_with_cycle_checkpoint() {
        let (mut world, owner, target) = feeding_world(100);
        let robot = world.slots.get_mut(target).unwrap();
        robot.kinematics.velocity = [40_000.0, 0.0];
        robot.kinematics.actual_velocity = [40_000.0, 0.0];
        let shot = feeding_shot(&mut world, owner);
        world.projectiles.positions[shot] = [200.0, 200.0];
        world.publish_snapshot();
        let before = crate::SaveFile::encode(&world).unwrap();
        let checkpoint = world.cycle_checkpoint();
        // Return velocity is (40000-40)+40000*0.5=59960: a VB Integer overflow.
        assert!(
            world
                .native_projectile_update_phase(&mut CycleRecorder::disabled(1))
                .is_err()
        );
        world.restore_cycle_checkpoint(checkpoint);
        assert_eq!(crate::SaveFile::encode(&world).unwrap(), before);
    }

    #[test]
    fn native_visited_energy_carrier_contributes_before_its_expiry_and_survives_checkpoint() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        world.projectiles.reset_native_slots(100);
        let shot = world
            .projectiles
            .spawn_native_packet(
                OrganismId::new(u32::MAX, 0),
                [100.0; 2],
                [0.0; 2],
                0.0,
                10.0,
                -2,
                None,
                false,
                0,
                0,
                0,
            )
            .unwrap() as usize;
        world.projectiles.ages[shot] = 1;
        phase(&mut world);
        assert!(!world.projectiles.alive[shot]);
        assert_eq!(world.stats.native_shots_visited, Some(1));
        assert_eq!(world.native_energy.ring[0], 10);
        let mut restored =
            crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        restored.tick = 1;
        restored.begin_native_energy_cycle().unwrap();
        assert_eq!(restored.native_energy_displayed(), 10);
        assert_eq!(restored.native_energy.ring[1], 0);
    }

    #[test]
    fn native_parent_immunity_compares_literal_slot_to_absolute_parent_id_at_ages_zero_one_two() {
        for age in [0, 1, 2] {
            for parent_absolute_id in [1, 9001] {
                let (mut world, owner, target) = feeding_world(100);
                world.slots.get_mut(owner).unwrap().identity.absolute_id = 9001;
                let robot = world.slots.get_mut(target).unwrap();
                robot.counters.age = age;
                robot.identity.parents[0] = Some(owner);
                robot.identity.parent_absolute_id = parent_absolute_id;
                let before = robot.materials.energy;
                let shot = feeding_shot(&mut world, owner);
                phase(&mut world);
                let immune = age <= 1 && parent_absolute_id == 1;
                assert_eq!(
                    world.projectiles.impact_flash[shot], !immune,
                    "age{age}/absolute parent{parent_absolute_id}"
                );
                assert_eq!(
                    world.slots.get(target).unwrap().materials.energy,
                    if immune { before } else { before - 90.0 }
                );
            }
        }
    }

    #[test]
    fn native_unowned_parent_zero_immunity_is_literal_at_ages_zero_one_two() {
        for age in [0, 1, 2] {
            for parent_absolute_id in [0, 1] {
                let (mut world, _, target) = feeding_world(100);
                let robot = world.slots.get_mut(target).unwrap();
                robot.counters.age = age;
                robot.identity.parents = [None, None];
                robot.identity.parent_absolute_id = parent_absolute_id;
                let before = robot.materials.energy;
                let shot = feeding_shot(&mut world, OrganismId::new(u32::MAX, 0));
                phase(&mut world);
                let immune = age <= 1 && parent_absolute_id == 0;
                assert_eq!(
                    world.projectiles.impact_flash[shot], !immune,
                    "unowned age{age}/absolute parent{parent_absolute_id}"
                );
                assert_eq!(
                    world.slots.get(target).unwrap().materials.energy,
                    if immune { before } else { before - 90.0 }
                );
            }
        }
    }

    #[test]
    fn native_owner_exclusion_uses_recycled_numeric_slot_without_generation_identity() {
        let (mut world, old_owner, target) = feeding_world(100);
        let shot = feeding_shot(&mut world, old_owner);
        world.slots.remove(old_owner).unwrap();
        let replacement = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [185.0, 200.0])
            .unwrap();
        assert_eq!(replacement.slot(), old_owner.slot());
        assert_ne!(replacement, old_owner);
        let robot = world.slots.get_mut(replacement).unwrap();
        robot.materials.radius = 10.0;
        robot.counters.age = 10; // Keep newborn parent-zero immunity out of the control.
        assert_eq!(robot.materials.energy, 1_000.0);
        assert_eq!(robot.materials.body, 1_000.0);
        assert_eq!(robot.materials.poison, 0.0);
        assert!(!robot.flags.corpse);
        // A nearby second victim would emit a legitimate return packet into the
        // replacement this same cycle. Isolate numeric-owner exclusion instead.
        world.slots.get_mut(target).unwrap().kinematics.position = [800.0, 800.0];
        world.publish_snapshot();
        let checkpoint = crate::SaveFile::encode(&world).unwrap();
        let mut restored = crate::SaveFile::decode(&checkpoint).unwrap();
        for state in [&mut world, &mut restored] {
            phase(state);
            assert_eq!(
                state.slots.get(replacement).unwrap().materials.energy,
                1_000.0
            );
            assert_eq!(
                state.slots.get(replacement).unwrap().materials.body,
                1_000.0
            );
            assert_eq!(state.slots.get(target).unwrap().materials.energy, 20_000.0);
            assert!(!state.projectiles.impact_flash[shot]);
            assert_eq!(state.projectiles.positions[shot], [215.0, 200.0]);
            assert_eq!(state.projectiles.ages[shot], 1);
            assert_eq!(state.stats.projectile_impacts, 0);
            assert_eq!(state.stats.native_shots_visited, Some(1));
        }
        assert_eq!(
            serde_json::to_value(&world.projectiles).unwrap(),
            serde_json::to_value(&restored.projectiles).unwrap()
        );

        // The same geometry must collide when its parent is not the reused slot.
        let mut control = crate::SaveFile::decode(&checkpoint).unwrap();
        control.projectiles.owners[shot] = owner_from_literal_slot(0);
        assert!(!control.config.shots.feeding_proportional);
        assert_eq!(control.config.shots.feeding_fixed_power, 100);
        // releasenrg: fixed power 100 debits 90 energy and 1 body, then emits 100.
        phase(&mut control);
        assert_eq!(
            control.slots.get(replacement).unwrap().materials.energy,
            910.0
        );
        assert_eq!(
            control.slots.get(replacement).unwrap().materials.body,
            999.0
        );
        assert_eq!(
            control.slots.get(target).unwrap().materials.energy,
            20_000.0
        );
        assert!(control.projectiles.impact_flash[shot]);
        assert_eq!(control.stats.projectile_impacts, 1);
        let returned = shot + 1;
        assert_eq!(control.projectiles.owners[returned], replacement);
        assert_eq!(control.projectiles.kinds[returned], -2);
        assert_eq!(control.projectiles.energies[returned], 100.0);
        assert_eq!(control.projectiles.values[returned], 100);
        assert!(!control.projectiles.impact_flash[returned]);
        assert_eq!(control.stats.native_shots_visited, Some(2));
    }

    #[test]
    fn native_effect_credit_resolves_recycled_owner_slot_and_dead_owner_shots_survive_checkpoint() {
        for reuse in [false, true] {
            let (mut world, old_owner, target) = feeding_world(100);
            let shot = feeding_shot(&mut world, old_owner);
            world.remove(old_owner).unwrap();
            assert!(world.projectiles.alive[shot]);
            let replacement = if reuse {
                let replacement = world
                    .spawn_at(LegacyDna::parse("stop").unwrap(), [50.0, 200.0])
                    .unwrap();
                assert_eq!(replacement.slot(), old_owner.slot());
                assert_ne!(replacement, old_owner);
                Some(replacement)
            } else {
                None
            };
            world.slots.get_mut(target).unwrap().materials.energy = 80.0;
            world.publish_snapshot();
            let mut restored =
                crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
            phase(&mut world);
            phase(&mut restored);
            assert!(world.slots.get(target).unwrap().flags.dead);
            assert_eq!(world.slots.get(target).unwrap().materials.energy, 0.0);
            if let Some(replacement) = replacement {
                assert_eq!(world.slots.get(replacement).unwrap().counters.kills, 1);
                assert_eq!(
                    world
                        .slots
                        .get(replacement)
                        .unwrap()
                        .program
                        .memory
                        .read(220),
                    1
                );
                assert_eq!(restored.slots.get(replacement).unwrap().counters.kills, 1);
            } else {
                assert!(world.slots.get(old_owner).is_err());
                assert!(world.current_native_shot_owner(old_owner).is_none());
            }
            assert_eq!(
                serde_json::to_value(&world.projectiles).unwrap(),
                serde_json::to_value(&restored.projectiles).unwrap()
            );
        }
    }

    #[test]
    fn native_compaction_rebinds_last_stored_virus_to_reused_owner_generation_only_when_compacting()
    {
        for capacity in [100, 200] {
            let (mut world, old_owner, _) = feeding_world(capacity);
            // Assemble the source-visible stale stored slot left by an older owner.
            world.slots.remove(old_owner).unwrap();
            let owner = world
                .spawn_at(LegacyDna::parse("stop").unwrap(), [50.0, 200.0])
                .unwrap();
            assert_eq!(owner.slot(), old_owner.slot());
            assert_ne!(owner, old_owner);
            for (index, stored_owner) in [owner, old_owner].into_iter().enumerate() {
                let shot = world
                    .projectiles
                    .spawn_native_packet(
                        stored_owner,
                        [700.0, 700.0],
                        [0.0; 2],
                        400.0,
                        (index + 1) as f32,
                        -7,
                        None,
                        false,
                        0,
                        0,
                        0,
                    )
                    .unwrap() as usize;
                assert_eq!(shot, index);
                world.projectiles.stored[shot] = true;
                world.projectiles.payloads[shot] =
                    Some(LegacyDna::parse("start 123 900 store stop").unwrap());
                world.projectiles.dna_lengths[shot] = 5;
                world.projectiles.gene_numbers[shot] = (index + 1) as i32;
            }
            phase(&mut world);
            let compacted = capacity == 200;
            assert_eq!(
                world.projectiles.owners[1],
                if compacted { owner } else { old_owner }
            );
            assert_eq!(world.projectiles.capacity(), 100);
            world.config.dna_costs.shot_formation = 0.0;
            world.config.dna_costs.global_multiplier = 1.0;
            let robot = world.slots.get_mut(owner).unwrap();
            robot.program.memory.write(335, 0);
            robot.program.memory.write(338, 1);
            robot.effects.virus_build_timer = 1;
            robot.effects.stored_virus = None; // No compatibility fallback can choose for us.
            let energy = robot.materials.energy;
            let mut restored =
                crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
            world.virus_commands_phase().unwrap();
            restored.virus_commands_phase().unwrap();
            let fired = usize::from(compacted);
            assert!(!world.projectiles.stored[fired]);
            assert!(world.projectiles.stored[1 - fired]);
            assert_eq!(world.projectiles.gene_numbers[fired], fired as i32 + 1);
            assert_eq!(
                world.slots.get(owner).unwrap().materials.energy,
                energy - 2.0
            );
            assert_eq!(
                serde_json::to_value(&world.projectiles).unwrap(),
                serde_json::to_value(&restored.projectiles).unwrap()
            );
        }
    }
    #[test]
    fn native_shape_policies_use_current_points_without_flash_and_continue_across_db3s() {
        for (policy, wire) in [
            (ShapeShotPolicy::Absorb, "absorb"),
            (ShapeShotPolicy::Reflect, "reflect"),
            (ShapeShotPolicy::PassThrough, "pass_through"),
        ] {
            let mut world = Engine::new(EngineConfig::testing()).unwrap();
            world.projectiles.reset_native_slots(100);
            let mut shape = Obstacle::new(1, [200.0, 190.0], [230.0, 210.0]);
            shape.shot_policy = policy;
            world.add_obstacle(shape).unwrap();
            let shot = world
                .projectiles
                .spawn_native_packet(
                    owner_from_literal_slot(0),
                    [190.0, 200.0],
                    [40.0, 0.0],
                    400.0,
                    37.0,
                    500,
                    None,
                    false,
                    0,
                    0,
                    0,
                )
                .unwrap() as usize;
            // Crossing the shape's boundary is not enough: current 190 is outside.
            phase(&mut world);
            assert!(world.projectiles.alive[shot]);
            assert_eq!(world.projectiles.positions[shot], [230.0, 200.0]);
            assert_eq!(world.projectiles.velocities[shot], [40.0, 0.0]);
            assert_eq!(world.stats.projectile_impacts, 0);
            let encoded = crate::SaveFile::encode(&world).unwrap();
            let payload: serde_json::Value = serde_json::from_slice(&encoded[10..]).unwrap();
            assert_eq!(payload["obstacles"][0]["shot_policy"], wire);
            let mut restored = crate::SaveFile::decode(&encoded).unwrap();
            for state in [&mut world, &mut restored] {
                // Current 230 is inclusively inside; previous 190 is outside x only.
                phase(state);
                let bypass = policy == ShapeShotPolicy::PassThrough;
                assert_eq!(
                    state.projectiles.alive[shot],
                    policy != ShapeShotPolicy::Absorb
                );
                assert_eq!(
                    state.projectiles.positions[shot],
                    if bypass {
                        [270.0, 200.0]
                    } else {
                        [190.0, 200.0]
                    }
                );
                assert_eq!(
                    state.projectiles.velocities[shot],
                    if bypass { [40.0, 0.0] } else { [-40.0, 0.0] }
                );
                assert_eq!(state.projectiles.ages[shot], 2);
                assert!(!state.projectiles.impact_flash[shot]);
                assert_eq!(state.stats.projectile_impacts, 0);
                assert_eq!(state.stats.projectile_effects, 0);
                assert_eq!(state.stats.native_shots_visited, Some(1));
            }
            assert_eq!(
                serde_json::to_value(&world.projectiles).unwrap(),
                serde_json::to_value(&restored.projectiles).unwrap()
            );
        }
    }

    #[test]
    fn portable_shape_policies_keep_swept_absorb_and_point_reflect_timing() {
        for policy in [
            ShapeShotPolicy::Absorb,
            ShapeShotPolicy::Reflect,
            ShapeShotPolicy::PassThrough,
        ] {
            let mut world = Engine::new(EngineConfig::testing()).unwrap();
            world.native_random = None;
            world.projectiles.reset_native_slots(100);
            let mut shape = Obstacle::new(1, [200.0, 190.0], [230.0, 210.0]);
            shape.shot_policy = policy;
            world.add_obstacle(shape).unwrap();
            let shot = world
                .projectiles
                .spawn_native_packet(
                    owner_from_literal_slot(0),
                    [190.0, 200.0],
                    [40.0, 0.0],
                    400.0,
                    37.0,
                    500,
                    None,
                    false,
                    0,
                    0,
                    0,
                )
                .unwrap() as usize;
            world
                .projectile_update_phase(&mut CycleRecorder::disabled(1))
                .unwrap();
            world.publish_snapshot();
            let absorb = policy == ShapeShotPolicy::Absorb;
            assert_eq!(world.projectiles.impact_flash[shot], absorb);
            assert_eq!(
                world.projectiles.positions[shot],
                if absorb {
                    [200.0, 200.0]
                } else {
                    [230.0, 200.0]
                }
            );
            assert_eq!(
                world.projectiles.velocities[shot],
                if policy == ShapeShotPolicy::Reflect {
                    [-40.0, 0.0]
                } else {
                    [40.0, 0.0]
                }
            );
            assert_eq!(world.stats.projectile_impacts, if absorb { 1 } else { 0 });
            assert_eq!(world.stats.projectile_effects, 0);
            assert_eq!(world.stats.native_shots_visited, None);
            let mut restored =
                crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
            for state in [&mut world, &mut restored] {
                state
                    .projectile_update_phase(&mut CycleRecorder::disabled(2))
                    .unwrap();
                state.publish_snapshot();
                if absorb {
                    assert!(state.snapshot().shots.is_empty());
                } else {
                    assert_eq!(
                        state.projectiles.positions[shot],
                        if policy == ShapeShotPolicy::Reflect {
                            [190.0, 200.0]
                        } else {
                            [270.0, 200.0]
                        }
                    );
                    assert!(!state.projectiles.impact_flash[shot]);
                    assert_eq!(state.stats.projectile_impacts, 0);
                }
            }
            assert_eq!(
                serde_json::to_value(&world.projectiles).unwrap(),
                serde_json::to_value(&restored.projectiles).unwrap()
            );
        }
    }
}
