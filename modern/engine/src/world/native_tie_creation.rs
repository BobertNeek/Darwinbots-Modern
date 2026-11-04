//! Robots.FireTies and Ties.maketie. Native creation has separate VM timing
//! from the structural projections used to draw ties.
use super::*;

const NEW_NATIVE_TIE_LIMIT: usize = MAX_TIES_PER_ROBOT - 1;

impl Engine {
    fn native_live_slot(&self, slot: usize) -> Option<OrganismId> {
        if slot >= self.slots.len() {
            return None;
        }
        let state = &self.slots[slot];
        state.organism.as_ref().filter(|robot| robot.flags.exists)?;
        Some(OrganismId::new(slot as u32, state.generation))
    }

    pub(super) fn native_tie_target(&self, actor: OrganismId) -> Option<OrganismId> {
        let robot = self.slots.get(actor).ok()?;
        if robot.senses.focused_shape {
            return None;
        }
        if let Some(target) = robot.senses.focused_opponent {
            return self.native_live_slot(target.slot() as usize);
        }
        if robot.counters.age < 2 {
            // VB6 stores an AbsNum in parent but FireTies indexes rob(parent).
            // Preserve that raw one-based slot rule, rather than silently
            // substituting the resolved genealogical parent.
            if let Some(slot) = usize::try_from(robot.identity.parent_absolute_id)
                .ok()
                .and_then(|slot| slot.checked_sub(1))
                && let Some(parent) = self.native_live_slot(slot)
            {
                return Some(parent);
            }
        }
        robot
            .senses
            .touched_opponent
            .and_then(|other| self.native_live_slot(other.slot() as usize))
    }

    pub(super) fn native_fire_ties(&mut self, trace: &mut CycleRecorder) {
        for actor in self.phase_robot_ids() {
            let Ok(robot) = self.slots.get(actor) else {
                continue;
            };
            if robot.flags.corpse || !robot.flags.can_execute_dna {
                continue;
            }
            let port = robot.program.memory.read(330);
            if port == 0 {
                continue;
            }
            if self.ties_enabled
                && let Some(target) = self.native_tie_target(actor)
                && target != actor
            {
                let source = self.slots.get(actor).unwrap();
                let other = self.slots.get(target).unwrap();
                let distance =
                    distance_squared(source.kinematics.position, other.kinematics.position).sqrt();
                let radii = f64::from(source.materials.radius) + f64::from(other.materials.radius);
                if distance <= (radii + 480.0) as f32 {
                    let range = (radii + 240.0).round_ties_even() as i32;
                    let created = self.native_make_tie(actor, target, range, -20, port);
                    // FireTies applies disqualification after a maketie attempt,
                    // including slime, distance and capacity failures.
                    self.native_disqualify_action(actor, DisqualificationAction::Tie);
                    if created && trace.is_enabled() {
                        trace.effect(CyclePhase::TieFire, Some(actor), target, "tie_created");
                    }
                }
            }
            if let Ok(robot) = self.slots.get_mut(actor) {
                SysvarRuntime::new().finish_command(
                    330,
                    CommandOutcome::Consumed,
                    &mut robot.program.memory,
                );
            }
        }
    }

    fn native_refresh_tie_ends(&mut self, first: OrganismId, second: OrganismId) {
        self.rebuild_native_tie_index();
        let empty = HashMap::new();
        self.refresh_one_tie_projection(first.slot() as usize, false, &empty, false);
        self.refresh_one_tie_projection(second.slot() as usize, false, &empty, false);
    }

    pub(super) fn native_delete_pair(&mut self, first: OrganismId, second: OrganismId) {
        let Some(index) = self.ties.iter().position(|tie| {
            (tie.first == first && tie.second == second)
                || (tie.first == second && tie.second == first)
        }) else {
            return;
        };
        for (actor, other) in [(first, second), (second, first)] {
            let robot = self.slots.get_mut(actor).unwrap();
            let endpoints = robot.ties.iter().flatten().copied().collect::<Vec<_>>();
            if let Some(ordinal) = endpoints.iter().position(|end| end.other == Some(other))
                && robot.program.memory.read(MEM_TIE_PRESENT) == endpoints[ordinal].port
            {
                let previous = ordinal.checked_sub(1).map_or(0, |i| endpoints[i].port);
                robot.program.memory.write(MEM_TIE_PRESENT, previous);
            }
        }
        self.ties.remove(index);
        for actor in [first, second] {
            let count = self.ties.iter().filter(|tie| tie.contains(actor)).count();
            self.slots
                .get_mut(actor)
                .unwrap()
                .program
                .memory
                .write(MEM_NUMBER_OF_TIES, count as i32);
        }
        // Compact local endpoint caches before appending a replacement. Reusing
        // the old port must not put the new tie back in its deleted array slot.
        self.native_refresh_tie_ends(first, second);
    }

    pub(super) fn native_make_tie(
        &mut self,
        first: OrganismId,
        second: OrganismId,
        range: i32,
        last: i32,
        port: i32,
    ) -> bool {
        let (Ok(source), Ok(target)) = (self.slots.get(first), self.slots.get(second)) else {
            return false;
        };
        if !source.flags.exists || !target.flags.exists || first == second {
            return false;
        }
        let length = distance_squared(source.kinematics.position, target.kinematics.position)
            .sqrt()
            .round_ties_even() as i32;
        let deflect = self.native_random.as_mut().unwrap().integer(2, 92);
        let allowed = f64::from(length) <= f64::from(range) * 1.5
            && deflect as f32 >= self.slots.get(second).unwrap().materials.slime;
        if allowed {
            self.native_delete_pair(first, second);
        }
        let first_count = self.tie_indices_for(first).len();
        let second_count = self.tie_indices_for(second).len();
        let created =
            allowed && first_count < NEW_NATIVE_TIE_LIMIT && second_count < NEW_NATIVE_TIE_LIMIT;
        if created {
            let mut tie =
                TieState::new(first, second, length as f32, port, second_count as i32 + 1);
            tie.last = last;
            let mut ends = tie.native_end_states();
            for (side, actor) in [first, second].into_iter().enumerate() {
                ends[side].length = self
                    .slots
                    .get(actor)
                    .unwrap()
                    .native_record_index
                    .is_none()
                    .then_some(0);
                ends[side].saved_reciprocal = Some(if side == 0 {
                    second_count as i32 + 1
                } else {
                    first_count as i32 + 1
                });
            }
            tie.native_ends = Some(ends);
            tie.sync_native_primary();
            self.ties.push(tie);
            self.native_refresh_tie_ends(first, second);
            let source = self.slots.get_mut(first).unwrap();
            source
                .program
                .memory
                .write(MEM_NUMBER_OF_TIES, first_count as i32 + 1);
            source.program.memory.write(MEM_TIE_PRESENT, port);
            // ReadTRefVars runs before the reverse endpoint publishes its
            // new count. It addresses the new array entry, even for duplicate ports.
            if let Some(values) = self.tie_reference_values(first, second) {
                let source = self.slots.get_mut(first).unwrap();
                for (address, value) in values {
                    native_sensing::publish_integer(&mut source.program.memory, address, value);
                }
            }
            let target = self.slots.get_mut(second).unwrap();
            target
                .program
                .memory
                .write(MEM_NUMBER_OF_TIES, second_count as i32 + 1);
            target
                .program
                .memory
                .write(MEM_TIE_PRESENT, second_count as i32 + 1);
            self.stats.ties_created = self.stats.ties_created.saturating_add(1);
        }
        let target = self.slots.get_mut(second).unwrap();
        target.materials.slime = (target.materials.slime - 20.0).max(0.0);
        let count = self.tie_indices_for(first).len();
        let fee = self.active_dna_costs().action_cost_for(
            SimulationCostAction::TieFormation,
            1.0,
            self.cost_population(),
        ) / (count as f64 + 1.0);
        let source = self.slots.get_mut(first).unwrap();
        // Ties.maketie assigns Single nrg after the Double subtraction. It does
        // not refresh .nrg or other VM resource readbacks here.
        source.materials.energy = (f64::from(source.materials.energy) - fee) as f32;
        created
    }

    pub(super) fn tie_reference_values(
        &mut self,
        source_id: OrganismId,
        other: OrganismId,
    ) -> Option<Vec<(u16, i32)>> {
        let source = self.slots.get(source_id).ok()?;
        let native = self.native_random.is_some();
        let source_aim = source.heading();
        let own_up = source.program.memory.read(MEM_VEL_UP);
        let own_dx = source.program.memory.read(MEM_VEL_DX);
        let memory_location = source.program.memory.read(MEM_TIE_MEMORY_LOCATION);
        let target = self.slots.get_mut(other).ok()?;
        // ReadTRefVars changes the target's physical velocity, but
        // copies the target's already-published own-frame cells.
        target.kinematics.velocity = target
            .kinematics
            .velocity
            .map(|value| value.clamp(-16000.0, 16000.0));
        let relative = native_reference_velocity::tie_relative_velocity(
            target.kinematics.velocity,
            source_aim,
            own_up,
            own_dx,
        );
        let target_velocity = [
            MEM_VEL_SCALAR,
            MEM_VEL_SX,
            MEM_VEL_DX,
            MEM_VEL_DN,
            MEM_VEL_UP,
        ]
        .map(|address| target.program.memory.read(address));
        let memory_value = if (1..=1_000).contains(&memory_location) {
            target.program.memory.read(memory_location)
        } else {
            0
        };
        let mut values = Vec::with_capacity(36);
        values.extend(
            (0..10).map(|index| (420 + index as u16, target.program.memory.read(410 + index))),
        );
        values.extend([
            (
                MEM_TREF_BODY as u16,
                if native {
                    if target.materials.body > -32000.0 && target.materials.body < 32000.0 {
                        target.materials.body.round_ties_even() as i32
                    } else {
                        32000
                    }
                } else {
                    resource_readback(target.materials.body)
                },
            ),
            (MEM_TREF_XPOS as u16, target.program.memory.read(MEM_XPOS)),
            (MEM_TREF_YPOS as u16, target.program.memory.read(MEM_YPOS)),
            (MEM_TREF_VEL_MY_SX as u16, relative[0]),
            (MEM_TREF_VEL_MY_DX as u16, relative[1]),
            (MEM_TREF_VEL_MY_DN as u16, relative[2]),
            (MEM_TREF_VEL_MY_UP as u16, relative[3]),
            (MEM_TREF_VEL_SCALAR as u16, target_velocity[0]),
            (MEM_TREF_VEL_YOUR_SX as u16, target_velocity[1]),
            (MEM_TREF_VEL_YOUR_DX as u16, target_velocity[2]),
            (MEM_TREF_VEL_YOUR_DN as u16, target_velocity[3]),
            (MEM_TREF_VEL_YOUR_UP as u16, target_velocity[4]),
            (
                MEM_TREF_SHELL as u16,
                if native {
                    target.materials.shell.round_ties_even() as i32
                } else {
                    resource_readback(target.materials.shell)
                },
            ),
        ]);
        for index in 0..8 {
            values.push((
                MEM_TREF_UP as u16 + index as u16,
                target
                    .senses
                    .occurrence_counts
                    .get(index)
                    .copied()
                    .unwrap_or(0),
            ));
        }
        // ReadTRefVars leaves trefnrg untouched at either strict energy bound.
        if !native || (target.materials.energy > -32000.0 && target.materials.energy < 32000.0) {
            values.push((
                MEM_TREF_NRG as u16,
                if native {
                    target.materials.energy.round_ties_even() as i32
                } else {
                    resource_readback(target.materials.energy)
                },
            ));
        }
        values.push((
            MEM_TREF_AGE as u16,
            target.counters.age.saturating_add(1).min(32000) as i32,
        ));
        if !native || (1..=1000).contains(&memory_location) {
            values.push((MEM_TIE_MEMORY_VALUE as u16, memory_value));
        }
        values.extend([
            (
                MEM_TREF_FIXED as u16,
                i32::from(robot_is_fixed(target, self.robot_fixing_allowed)),
            ),
            (
                MEM_TREF_AIM as u16,
                if native {
                    target.program.memory.read(18)
                } else {
                    target.materials.aim
                },
            ),
        ]);
        self.prepare_native_reference_values(source_id, other, &mut values, true);
        Some(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> Engine {
        let mut world = Engine::new(EngineConfig {
            world_width: 8000.0,
            world_height: 6000.0,
            organism_capacity: 64,
            metabolism_cost: 0,
            ..EngineConfig::testing()
        })
        .unwrap();
        world.mutations_enabled = false;
        world
    }

    fn bot(world: &mut Engine, position: [f32; 2]) -> OrganismId {
        world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), position)
            .unwrap()
    }

    fn requested(world: &mut Engine, actor: OrganismId, target: Option<OrganismId>) {
        let source = world.slots.get_mut(actor).unwrap();
        source.senses.focused_opponent = target;
        source.program.memory.write(330, 41);
    }

    #[test]
    fn whole_cycle_uses_current_focus_instead_of_the_nearest_robot_behind() {
        let mut world = world();
        let actor = world
            .spawn_at(
                LegacyDna::parse("start 41 .tie store stop").unwrap(),
                [1000.0; 2],
            )
            .unwrap();
        let behind = bot(&mut world, [850.0, 1000.0]);
        let ahead = bot(&mut world, [1300.0, 1000.0]);
        world.slots.get_mut(actor).unwrap().set_heading(0.0);
        world.tick().unwrap();
        assert_eq!(
            world.slots.get(actor).unwrap().senses.focused_opponent,
            Some(ahead)
        );
        assert_eq!(world.ties.len(), 1);
        assert!(world.ties[0].contains(ahead));
        assert!(!world.ties[0].contains(behind));
        // UpdateTieAngles preceded this new bond; its geometry VM cells
        // are not published until the next source angle pass.
        assert_eq!(world.memory_at(actor, MEM_TIE_LENGTH).unwrap(), 0);
        assert_eq!(world.memory_at(actor, 484).unwrap(), 0);
    }

    #[test]
    fn no_focus_parent_or_contact_consumes_request_without_draw_or_fee() {
        let mut world = world();
        let actor = bot(&mut world, [1000.0; 2]);
        bot(&mut world, [1100.0, 1000.0]);
        world.config.dna_costs.tie_formation = 6.0;
        requested(&mut world, actor, None);
        let rng = world.native_random.clone();
        let energy = world.slots.get(actor).unwrap().materials.energy;
        world.native_fire_ties(&mut CycleRecorder::disabled(0));
        assert!(world.ties.is_empty());
        assert_eq!(world.memory_at(actor, 330).unwrap(), 0);
        assert_eq!(world.native_random, rng);
        assert_eq!(world.slots.get(actor).unwrap().materials.energy, energy);
    }

    #[test]
    fn shape_focus_blocks_parent_and_contact_fallback() {
        let mut world = world();
        let actor = bot(&mut world, [1000.0; 2]);
        let target = bot(&mut world, [1200.0, 1000.0]);
        requested(&mut world, actor, None);
        let source = world.slots.get_mut(actor).unwrap();
        source.senses.focused_shape = true;
        source.senses.touched_opponent = Some(target);
        source.identity.parent_absolute_id = u64::from(target.slot()) + 1;
        world.native_fire_ties(&mut CycleRecorder::disabled(0));
        assert!(world.ties.is_empty());
        assert_eq!(world.memory_at(actor, 330).unwrap(), 0);
    }

    #[test]
    fn newborn_fallback_uses_the_raw_parent_slot_before_contact() {
        let mut world = world();
        let genealogy = bot(&mut world, [1000.0; 2]);
        let raw_slot = bot(&mut world, [1200.0, 1000.0]);
        let actor = bot(&mut world, [1400.0, 1000.0]);
        world.slots.get_mut(genealogy).unwrap().identity.absolute_id = 2;
        let source = world.slots.get_mut(actor).unwrap();
        source.identity.parents[0] = Some(genealogy);
        source.identity.parent_absolute_id = 2;
        source.senses.touched_opponent = Some(genealogy);
        assert_eq!(world.native_tie_target(actor), Some(raw_slot));
        world.slots.get_mut(actor).unwrap().counters.age = 2;
        assert_eq!(world.native_tie_target(actor), Some(genealogy));
        world.slots.get_mut(actor).unwrap().senses.focused_opponent = Some(raw_slot);
        assert_eq!(world.native_tie_target(actor), Some(raw_slot));
    }

    #[test]
    fn replacement_appends_restarts_lifetime_and_retains_other_stale_reciprocal() {
        let mut world = world();
        let actor = bot(&mut world, [1000.0; 2]);
        let first = bot(&mut world, [1200.0, 1000.0]);
        let second = bot(&mut world, [1300.0, 1000.0]);
        assert!(world.native_make_tie(actor, first, 500, -20, 7));
        assert!(world.native_make_tie(actor, second, 500, -20, 8));
        assert!(world.native_make_tie(actor, first, 500, -20, 7));
        let order = world
            .tie_indices_for(actor)
            .into_iter()
            .map(|i| world.ties[i].second)
            .collect::<Vec<_>>();
        assert_eq!(order, vec![second, first]);
        assert_eq!(world.ties.len(), 2);
        let second_tie = world.ties.iter().find(|tie| tie.contains(second)).unwrap();
        assert_eq!(second_tie.native_end_states()[1].saved_reciprocal, Some(2));
        let replaced = world.ties.iter().find(|tie| tie.contains(first)).unwrap();
        assert_eq!(replaced.last, -20);
        assert_eq!(replaced.native_end_states()[1].saved_reciprocal, Some(2));
        assert_eq!(world.memory_at(actor, MEM_NUMBER_OF_TIES).unwrap(), 2);
        assert_eq!(world.memory_at(actor, MEM_TIE_PRESENT).unwrap(), 7);
    }

    #[test]
    fn capacity_is_nine_new_bonds_and_a_full_pair_can_be_replaced() {
        let mut world = world();
        let actor = bot(&mut world, [1000.0; 2]);
        let others = (0..10)
            .map(|i| bot(&mut world, [1200.0, 1000.0 + i as f32]))
            .collect::<Vec<_>>();
        for (i, target) in others.iter().take(9).enumerate() {
            assert!(world.native_make_tie(actor, *target, 500, -20, i as i32 + 1));
        }
        world.config.dna_costs.tie_formation = 6.0;
        let energy = world.slots.get(actor).unwrap().materials.energy;
        let mut expected_rng = world.native_random.clone().unwrap();
        expected_rng.integer(2, 92);
        assert!(!world.native_make_tie(actor, others[9], 500, -20, 10));
        assert_eq!(world.native_random.as_ref(), Some(&expected_rng));
        assert_eq!(
            world.slots.get(actor).unwrap().materials.energy,
            (f64::from(energy) - 0.6) as f32
        );
        assert_eq!(world.ties.len(), 9);
        assert_eq!(world.slots.get(actor).unwrap().ties.len(), 10);
        assert!(world.native_make_tie(actor, others[0], 500, -20, 77));
        assert_eq!(world.ties.len(), 9);
        assert_eq!(world.memory_at(actor, MEM_TIE_PRESENT).unwrap(), 77);
    }

    #[test]
    fn slime_failure_consumes_one_draw_fee_and_slime_without_replacing_a_pair() {
        let mut world = world();
        let actor = bot(&mut world, [1000.0; 2]);
        let target = bot(&mut world, [1200.0, 1000.0]);
        assert!(world.native_make_tie(actor, target, 500, -20, 7));
        world.config.dna_costs.tie_formation = 6.0;
        world.slots.get_mut(target).unwrap().materials.slime = 1000.0;
        world
            .slots
            .get_mut(actor)
            .unwrap()
            .program
            .memory
            .write(MEM_NRG, 123);
        let energy = world.slots.get(actor).unwrap().materials.energy;
        let old = world.ties.clone();
        let mut expected_rng = world.native_random.clone().unwrap();
        expected_rng.integer(2, 92);
        assert!(!world.native_make_tie(actor, target, 500, -20, 9));
        assert_eq!(world.native_random.as_ref(), Some(&expected_rng));
        assert_eq!(world.ties, old);
        assert_eq!(world.slots.get(target).unwrap().materials.slime, 980.0);
        assert_eq!(
            world.slots.get(actor).unwrap().materials.energy,
            (f64::from(energy) - 3.0) as f32
        );
        assert_eq!(world.memory_at(actor, MEM_NRG).unwrap(), 123);
    }

    #[test]
    fn selected_focus_can_pass_outer_range_but_fail_the_inner_maketie_gate() {
        let mut world = world();
        let actor = bot(&mut world, [1000.0; 2]);
        let focused = bot(&mut world, [1400.0, 1000.0]);
        bot(&mut world, [1100.0, 1000.0]);
        for id in [actor, focused] {
            world.slots.get_mut(id).unwrap().materials.radius = 10.0;
        }
        requested(&mut world, actor, Some(focused));
        world.config.dna_costs.tie_formation = 6.0;
        let energy = world.slots.get(actor).unwrap().materials.energy;
        let mut expected_rng = world.native_random.clone().unwrap();
        expected_rng.integer(2, 92);
        world.native_fire_ties(&mut CycleRecorder::disabled(0));
        assert!(world.ties.is_empty());
        assert_eq!(world.native_random.as_ref(), Some(&expected_rng));
        assert_eq!(
            world.slots.get(actor).unwrap().materials.energy,
            (f64::from(energy) - 6.0) as f32
        );
    }

    #[test]
    fn new_array_entry_controls_tref_even_when_ports_are_duplicated() {
        let mut world = world();
        let actor = bot(&mut world, [1000.0; 2]);
        let first = bot(&mut world, [1200.0, 1000.0]);
        let second = bot(&mut world, [1300.0, 1000.0]);
        world.slots.get_mut(first).unwrap().materials.energy = 111.0;
        world.slots.get_mut(second).unwrap().materials.energy = 777.0;
        world.slots.get_mut(second).unwrap().counters.age = 17;
        assert!(world.native_make_tie(actor, first, 500, -20, 7));
        let source = world.slots.get_mut(actor).unwrap();
        source
            .program
            .memory
            .write(MEM_TIE_MEMORY_LOCATION, MEM_NUMBER_OF_TIES);
        source.program.memory.write(MEM_TIE_ANGLE, 123);
        source.program.memory.write(MEM_TIE_LENGTH, 234);
        source.program.memory.write(MEM_MULTI, 1);
        source.counters.multibot = true;
        assert!(world.native_make_tie(actor, second, 500, -20, 7));
        assert_eq!(world.memory_at(actor, MEM_TREF_NRG).unwrap(), 777);
        assert_eq!(world.memory_at(actor, MEM_TREF_AGE).unwrap(), 18);
        assert_eq!(world.memory_at(actor, MEM_TIE_MEMORY_VALUE).unwrap(), 0);
        assert_eq!(world.memory_at(second, MEM_NUMBER_OF_TIES).unwrap(), 1);
        assert_eq!(world.memory_at(actor, MEM_TIE_ANGLE).unwrap(), 123);
        assert_eq!(world.memory_at(actor, MEM_TIE_LENGTH).unwrap(), 234);
        assert_eq!(world.memory_at(actor, MEM_MULTI).unwrap(), 1);
        assert!(world.slots.get(actor).unwrap().counters.multibot);
    }

    fn read_references(world: &mut Engine, actor: OrganismId, target: OrganismId) {
        let values = world.tie_reference_values(actor, target).unwrap();
        let memory = &mut world.slots.get_mut(actor).unwrap().program.memory;
        for (address, value) in values {
            native_sensing::publish_integer(memory, address, value);
        }
    }

    #[test]
    fn native_tref_uses_strict_energy_bounds_retains_invalid_memory_and_adds_age() {
        let mut world = world();
        let actor = bot(&mut world, [1000.0; 2]);
        let target = bot(&mut world, [1200.0, 1000.0]);
        for energy in [-32001.0, -32000.0, 32000.0, 32001.0] {
            world
                .slots
                .get_mut(actor)
                .unwrap()
                .program
                .memory
                .write(MEM_TREF_NRG, 456);
            world.slots.get_mut(target).unwrap().materials.energy = energy;
            read_references(&mut world, actor, target);
            assert_eq!(
                world.memory_at(actor, MEM_TREF_NRG).unwrap(),
                456,
                "{energy}"
            );
        }
        for (energy, expected) in [(-31999.75, -32000), (-1.5, -2), (2.5, 2), (31999.75, 32000)] {
            world.slots.get_mut(target).unwrap().materials.energy = energy;
            read_references(&mut world, actor, target);
            assert_eq!(
                world.memory_at(actor, MEM_TREF_NRG).unwrap(),
                expected,
                "{energy}"
            );
        }
        for location in [-1, 0, 1001] {
            let source = world.slots.get_mut(actor).unwrap();
            source
                .program
                .memory
                .write(MEM_TIE_MEMORY_LOCATION, location);
            source.program.memory.write(MEM_TIE_MEMORY_VALUE, 1234);
            read_references(&mut world, actor, target);
            assert_eq!(world.memory_at(actor, MEM_TIE_MEMORY_VALUE).unwrap(), 1234);
        }
        for (age, expected) in [
            (0, 1),
            (17, 18),
            (31999, 32000),
            (32000, 32000),
            (80000, 32000),
        ] {
            world.slots.get_mut(target).unwrap().counters.age = age;
            read_references(&mut world, actor, target);
            assert_eq!(
                world.memory_at(actor, MEM_TREF_AGE).unwrap(),
                expected,
                "{age}"
            );
        }
    }

    #[test]
    fn native_tref_body_sentinel_and_cached_integer_words_follow_readtrefvars() {
        let mut world = world();
        let actor = bot(&mut world, [1000.0; 2]);
        let target = bot(&mut world, [1200.0, 1000.0]);
        world
            .slots
            .get_mut(actor)
            .unwrap()
            .program
            .memory
            .write(MEM_TIE_MEMORY_LOCATION, 999);
        let other = world.slots.get_mut(target).unwrap();
        other.set_heading(1.0);
        other.materials.shell = 32000.75;
        for (address, value) in [
            (18, 32767),
            (MEM_XPOS, -32768),
            (MEM_YPOS, 32001),
            (410, 32767),
            (999, -32767),
        ] {
            other.program.memory.restore_native_cell(address, value);
        }
        read_references(&mut world, actor, target);
        for (address, expected) in [
            (MEM_TREF_AIM, 32767),
            (MEM_TREF_XPOS, -32768),
            (MEM_TREF_YPOS, 32001),
            (420, 32767),
            (MEM_TREF_SHELL, 32001),
            (MEM_TIE_MEMORY_VALUE, -32767),
        ] {
            assert_eq!(
                world.memory_at(actor, address).unwrap(),
                expected,
                "cell {address}"
            );
        }
        for (body, expected) in [
            (-32000.25, 32000),
            (-32000.0, 32000),
            (-31999.75, -32000),
            (1.5, 2),
            (31999.75, 32000),
            (32000.0, 32000),
        ] {
            world.slots.get_mut(target).unwrap().materials.body = body;
            read_references(&mut world, actor, target);
            assert_eq!(
                world.memory_at(actor, MEM_TREF_BODY).unwrap(),
                expected,
                "{body}"
            );
        }
    }

    #[test]
    fn tie_projection_retains_unassigned_reference_cells_for_a_valid_selected_tie() {
        let mut world = world();
        let actor = bot(&mut world, [1000.0; 2]);
        let target = bot(&mut world, [1200.0, 1000.0]);
        assert!(world.native_make_tie(actor, target, 500, -20, 7));
        let source = world.slots.get_mut(actor).unwrap();
        source.counters.simulation_age = 2;
        source.program.memory.write(MEM_READ_TIE, 7);
        source.program.memory.write(MEM_TREF_NRG, 345);
        source.program.memory.write(MEM_TIE_MEMORY_LOCATION, 1001);
        source.program.memory.write(MEM_TIE_MEMORY_VALUE, 678);
        let other = world.slots.get_mut(target).unwrap();
        other.materials.energy = 32000.0;
        other.program.memory.restore_native_cell(410, 32767);
        world.refresh_one_tie_projection(actor.slot() as usize, true, &HashMap::new(), true);
        assert_eq!(world.memory_at(actor, MEM_TREF_NRG).unwrap(), 345);
        assert_eq!(world.memory_at(actor, MEM_TIE_MEMORY_VALUE).unwrap(), 678);
        assert_eq!(world.memory_at(actor, 420).unwrap(), 32767);
    }

    fn tie_readback(world: &mut Engine, actor: OrganismId) {
        world.refresh_one_tie_projection(actor.slot() as usize, false, &HashMap::new(), true);
        world.refresh_one_tie_projection(actor.slot() as usize, true, &HashMap::new(), true);
    }

    fn projection_pair(world: &mut Engine, port: i32) -> (OrganismId, OrganismId) {
        let actor = bot(world, [1000.0; 2]);
        let target = bot(world, [1200.0, 1000.0]);
        for id in [actor, target] {
            world.slots.get_mut(id).unwrap().materials.radius = 60.0;
        }
        assert!(world.native_make_tie(actor, target, 500, -20, port));
        world.slots.get_mut(actor).unwrap().counters.simulation_age = 2;
        (actor, target)
    }

    #[test]
    fn readtie_fallback_uses_saved_tiepres_and_keeps_unmatched_and_zero_selectors() {
        let mut world = world();
        let (actor, first) = projection_pair(&mut world, 11);
        let second = bot(&mut world, [1300.0, 1000.0]);
        world.slots.get_mut(first).unwrap().materials.energy = 111.0;
        world.slots.get_mut(second).unwrap().materials.energy = 777.0;
        assert!(world.native_make_tie(actor, second, 500, -20, 22));
        for (present, expected) in [(11, 111), (22, 777), (999, 0), (0, 0)] {
            let source = world.slots.get_mut(actor).unwrap();
            source.program.memory.write(MEM_TIE_PRESENT, present);
            source.program.memory.write(MEM_READ_TIE, 0);
            tie_readback(&mut world, actor);
            assert_eq!(world.memory_at(actor, MEM_TREF_NRG).unwrap(), expected);
            assert_eq!(world.memory_at(actor, MEM_TIE_PRESENT).unwrap(), present);
            let mut resumed =
                crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
            tie_readback(&mut resumed, actor);
            assert_eq!(resumed.memory_at(actor, MEM_TREF_NRG).unwrap(), expected);
            assert_eq!(resumed.memory_at(actor, MEM_TIE_PRESENT).unwrap(), present);
        }
    }

    #[test]
    fn geometry_uses_tienum_while_reference_reads_use_readtie() {
        let mut world = world();
        let (actor, first) = projection_pair(&mut world, 11);
        let second = bot(&mut world, [1000.0, 1300.0]);
        world.slots.get_mut(second).unwrap().materials.radius = 60.0;
        world.slots.get_mut(actor).unwrap().set_heading(0.0);
        world.slots.get_mut(first).unwrap().materials.energy = 111.0;
        world.slots.get_mut(second).unwrap().materials.energy = 777.0;
        assert!(world.native_make_tie(actor, second, 500, -20, 22));
        for (geometry_port, reference_port, length, angle, energy) in [
            (11, 22, 80, 0, 777),
            (22, 11, 180, 314, 111),
            (999, 11, 0, 0, 111),
        ] {
            let source = world.slots.get_mut(actor).unwrap();
            source.program.memory.write(455, geometry_port);
            source.program.memory.write(MEM_READ_TIE, reference_port);
            tie_readback(&mut world, actor);
            assert_eq!(world.memory_at(actor, MEM_TIE_LENGTH).unwrap(), length);
            assert_eq!(world.memory_at(actor, MEM_TIE_ANGLE).unwrap(), angle);
            assert_eq!(world.memory_at(actor, MEM_TREF_NRG).unwrap(), energy);
            assert_eq!(world.memory_at(actor, MEM_TIE_PRESENT).unwrap(), 22);
        }
    }

    #[test]
    fn duplicate_ports_read_first_reference_but_last_geometry() {
        let mut world = world();
        let (actor, first) = projection_pair(&mut world, 7);
        let second = bot(&mut world, [1000.0, 1300.0]);
        world.slots.get_mut(second).unwrap().materials.radius = 60.0;
        world.slots.get_mut(actor).unwrap().set_heading(0.0);
        world.slots.get_mut(first).unwrap().materials.energy = 111.0;
        world.slots.get_mut(second).unwrap().materials.energy = 777.0;
        assert!(world.native_make_tie(actor, second, 500, -20, 7));
        tie_readback(&mut world, actor);
        assert_eq!(world.memory_at(actor, MEM_TREF_NRG).unwrap(), 111);
        assert_eq!(world.memory_at(actor, MEM_TIE_LENGTH).unwrap(), 180);
        assert_eq!(world.memory_at(actor, MEM_TIE_ANGLE).unwrap(), 314);
    }

    #[test]
    fn port_zero_birth_bond_is_readable_but_has_no_default_selected_geometry() {
        let mut world = world();
        let (actor, target) = projection_pair(&mut world, 0);
        world.slots.get_mut(target).unwrap().materials.energy = 123.0;
        tie_readback(&mut world, actor);
        assert_eq!(world.memory_at(actor, MEM_TREF_NRG).unwrap(), 123);
        assert_eq!(world.memory_at(actor, MEM_TIE_LENGTH).unwrap(), 0);
        assert_eq!(world.memory_at(actor, MEM_TIE_ANGLE).unwrap(), 0);
    }

    #[test]
    fn native_surface_length_can_be_negative_and_caps_center_before_subtracting_radii() {
        let mut world = world();
        let (actor, target) = projection_pair(&mut world, 7);
        for (position, expected) in [
            ([1100.0, 1000.0], -20),
            ([1100.5, 1000.0], -20),
            ([1101.5, 1000.0], -18),
            ([41000.0, 1000.0], 31880),
        ] {
            world.slots.get_mut(target).unwrap().kinematics.position = position;
            tie_readback(&mut world, actor);
            assert_eq!(world.memory_at(actor, MEM_TIE_LENGTH).unwrap(), expected);
        }
    }

    #[test]
    fn missing_tie_clears_source_tref_range_but_retains_trefshell() {
        let mut world = world();
        let actor = bot(&mut world, [1000.0; 2]);
        let source = world.slots.get_mut(actor).unwrap();
        source.counters.simulation_age = 2;
        for address in TIED_REFERENCE_ADDRESSES {
            source.program.memory.restore_native_cell(address, 32767);
        }
        source.program.memory.write(MEM_TIE_MEMORY_LOCATION, 509);
        tie_readback(&mut world, actor);
        for address in TIED_REFERENCE_ADDRESSES {
            assert_eq!(
                world.memory_at(actor, address).unwrap(),
                if address == MEM_TREF_SHELL { 32767 } else { 0 },
                "cell {address}"
            );
        }
        assert_eq!(
            world.memory_at(actor, MEM_TIE_MEMORY_LOCATION).unwrap(),
            509
        );
    }

    #[test]
    fn native_indexed_geometry_publishes_only_hardened_existing_endpoints_during_update_ties() {
        let mut world = world();
        let actor = bot(&mut world, [1000.0; 2]);
        let first = bot(&mut world, [1000.0, 1200.0]);
        let second = bot(&mut world, [1200.0, 1000.0]);
        let soft = bot(&mut world, [1300.0, 1000.0]);
        for id in [actor, first, second, soft] {
            world.slots.get_mut(id).unwrap().materials.radius = 60.0;
        }
        world.slots.get_mut(actor).unwrap().set_heading(0.0);
        world
            .ties
            .push(TieState::legacy(actor, first, 200.0, 11, 1));
        world
            .ties
            .push(TieState::legacy(actor, second, 200.0, 22, 1));
        world.ties.push(TieState::new(actor, soft, 300.0, 33, 1));
        world.refresh_tie_projections_impl(false, &HashMap::new(), false);
        let source = world.slots.get_mut(actor).unwrap();
        source.counters.multibot = true;
        source.program.memory.write(MEM_TIE_PRESENT, 11);
        source.program.memory.write(MEM_TIE_ANGLE, 432);
        source.program.memory.write(MEM_TIE_LENGTH, 543);
        for (address, value) in [
            (480, 123),
            (481, 234),
            (482, 345),
            (483, 456),
            (484, 567),
            (485, 678),
            (486, 789),
            (487, 890),
        ] {
            source.program.memory.write(address, value);
        }
        source.program.tie_angle_overwrites[2] = true;
        source.program.tie_length_overwrites[3] = true;
        world.tie_exchange_phase(&mut CycleRecorder::disabled(0));
        for (address, expected) in [
            (480, 942),
            (481, 0),
            (482, 345),
            (483, 456),
            (484, 80),
            (485, 80),
            (486, 789),
            (487, 890),
        ] {
            assert_eq!(
                world.memory_at(actor, address).unwrap(),
                expected,
                "cell {address}"
            );
        }
        // Selected geometry belongs to the later UpdateTieAngles call.
        assert_eq!(world.memory_at(actor, MEM_TIE_ANGLE).unwrap(), 432);
        assert_eq!(world.memory_at(actor, MEM_TIE_LENGTH).unwrap(), 543);
        let source = world.slots.get(actor).unwrap();
        assert!(source.program.tie_angle_overwrites[2]);
        assert!(source.program.tie_length_overwrites[3]);
    }

    #[test]
    fn indexed_native_controls_accept_negative_length_and_keep_positive_wrapped_angle() {
        let mut world = world();
        let (actor, target) = projection_pair(&mut world, 7);
        world.ties[0] = TieState::legacy(actor, target, 200.0, 7, 1);
        let source = world.slots.get_mut(actor).unwrap();
        source.counters.multibot = true;
        source.program.memory.write(480, -100);
        source.program.memory.write(484, -10);
        source.program.tie_angle_overwrites[0] = true;
        source.program.tie_length_overwrites[0] = true;
        world.tie_exchange_phase(&mut CycleRecorder::disabled(0));
        let ends = world.ties[0].native_end_states();
        assert_eq!([ends[0].rest_length, ends[1].rest_length], [110.0; 2]);
        assert_eq!(ends[0].angle, std::f32::consts::TAU - 0.5);
        let source = world.slots.get(actor).unwrap();
        assert!(!source.program.tie_angle_overwrites[0]);
        assert!(!source.program.tie_length_overwrites[0]);
    }

    #[test]
    fn zero_tiepres_skips_native_structural_controls_and_keeps_indexed_commands() {
        let mut world = world();
        let (actor, target) = projection_pair(&mut world, 7);
        world.ties[0] = TieState::legacy(actor, target, 200.0, 7, 1);
        let source = world.slots.get_mut(actor).unwrap();
        source.counters.multibot = true;
        for (address, value) in [
            (MEM_TIE_PRESENT, 0),
            (468, 20),
            (469, 40),
            (331, 33),
            (480, 100),
            (484, 200),
        ] {
            source.program.memory.write(address, value);
        }
        source.program.tie_angle_overwrites[0] = true;
        source.program.tie_length_overwrites[0] = true;
        world.tie_exchange_phase(&mut CycleRecorder::disabled(0));
        for (address, expected) in [
            (MEM_TIE_PRESENT, 0),
            (468, 20),
            (469, 40),
            (331, 33),
            (480, 100),
            (484, 200),
        ] {
            assert_eq!(
                world.memory_at(actor, address).unwrap(),
                expected,
                "cell {address}"
            );
        }
        let source = world.slots.get(actor).unwrap();
        assert!(source.program.tie_angle_overwrites[0]);
        assert!(source.program.tie_length_overwrites[0]);
        assert_eq!(world.ties[0].rest_length, 200.0);
    }

    #[test]
    fn delete_and_public_remove_preserve_source_previous_port_instead_of_latest_remaining() {
        let mut world = world();
        let (actor, first) = projection_pair(&mut world, 11);
        let second = bot(&mut world, [1300.0, 1000.0]);
        assert!(world.native_make_tie(actor, second, 500, -20, 22));
        world
            .slots
            .get_mut(actor)
            .unwrap()
            .program
            .memory
            .write(MEM_TIE_PRESENT, 11);
        world.native_delete_pair(actor, first);
        assert_eq!(world.memory_at(actor, MEM_TIE_PRESENT).unwrap(), 0);
        assert_eq!(world.memory_at(actor, MEM_NUMBER_OF_TIES).unwrap(), 1);
        assert_eq!(world.memory_at(first, MEM_TIE_PRESENT).unwrap(), 0);
        assert_eq!(world.memory_at(first, MEM_NUMBER_OF_TIES).unwrap(), 0);
        let source = world.slots.get_mut(actor).unwrap();
        source.program.memory.write(MEM_TIE_PRESENT, 123);
        source.program.memory.write(MEM_TIE_ANGLE, 234);
        source.program.memory.write(MEM_TIE_LENGTH, 345);
        world.remove(second).unwrap();
        assert_eq!(world.memory_at(actor, MEM_TIE_PRESENT).unwrap(), 123);
        assert_eq!(world.memory_at(actor, MEM_NUMBER_OF_TIES).unwrap(), 0);
        assert_eq!(world.memory_at(actor, MEM_TIE_ANGLE).unwrap(), 234);
        assert_eq!(world.memory_at(actor, MEM_TIE_LENGTH).unwrap(), 345);
    }

    #[test]
    fn native_deltie_preserves_the_compacting_for_loop_skip_for_duplicate_ports() {
        let mut world = world();
        let (actor, first) = projection_pair(&mut world, 7);
        let middle = bot(&mut world, [1250.0, 1000.0]);
        let last = bot(&mut world, [1300.0, 1000.0]);
        assert!(world.native_make_tie(actor, middle, 500, -20, 7));
        assert!(world.native_make_tie(actor, last, 500, -20, 7));
        world
            .slots
            .get_mut(actor)
            .unwrap()
            .program
            .memory
            .write(467, 7);
        world.tie_exchange_phase(&mut CycleRecorder::disabled(0));
        assert_eq!(world.ties.len(), 1);
        assert!(world.ties[0].contains(middle));
        assert!(!world.ties[0].contains(first));
        assert!(!world.ties[0].contains(last));
        assert_eq!(world.memory_at(actor, MEM_TIE_PRESENT).unwrap(), 0);
        assert_eq!(world.memory_at(actor, MEM_NUMBER_OF_TIES).unwrap(), 1);
        assert_eq!(world.memory_at(actor, 467).unwrap(), 0);
    }

    #[test]
    fn multibot_flag_is_not_recomputed_from_soft_bonds_or_view_refresh() {
        let mut world = world();
        let (actor, target) = projection_pair(&mut world, 7);
        let source = world.slots.get_mut(actor).unwrap();
        source.counters.multibot = true;
        source.program.memory.write(MEM_MULTI, 7);
        world.refresh_tie_projections();
        world.tie_exchange_phase(&mut CycleRecorder::disabled(0));
        assert!(world.slots.get(actor).unwrap().counters.multibot);
        assert_eq!(world.memory_at(actor, MEM_MULTI).unwrap(), 7);
        world.remove(target).unwrap();
        assert!(world.slots.get(actor).unwrap().counters.multibot);
        assert_eq!(world.memory_at(actor, MEM_MULTI).unwrap(), 7);
        world.tie_exchange_phase(&mut CycleRecorder::disabled(0));
        assert!(!world.slots.get(actor).unwrap().counters.multibot);
        assert_eq!(world.memory_at(actor, MEM_MULTI).unwrap(), 0);
    }

    #[test]
    fn whole_cycle_reads_dna_selected_tiepres_without_a_projection_overwrite() {
        let mut world = world();
        let (actor, first) = projection_pair(&mut world, 11);
        let second = bot(&mut world, [1300.0, 1000.0]);
        assert!(world.native_make_tie(actor, second, 500, -20, 22));
        world
            .replace_dna(actor, LegacyDna::parse("start 11 454 store stop").unwrap())
            .unwrap();
        world
            .slots
            .get_mut(actor)
            .unwrap()
            .program
            .memory
            .write(MEM_TIE_MEMORY_LOCATION, 999);
        world
            .slots
            .get_mut(first)
            .unwrap()
            .program
            .memory
            .write(999, 1111);
        world
            .slots
            .get_mut(second)
            .unwrap()
            .program
            .memory
            .write(999, 7777);
        world.tick().unwrap();
        assert_eq!(world.memory_at(actor, MEM_TIE_MEMORY_VALUE).unwrap(), 1111);
        assert_eq!(world.memory_at(actor, MEM_TIE_PRESENT).unwrap(), 11);
    }

    #[test]
    fn later_robot_geometry_observes_earlier_body_radius_change_in_the_same_cycle() {
        let mut world = world();
        let first = world
            .spawn_at(
                LegacyDna::parse("start 100 .strbody store stop").unwrap(),
                [1000.0; 2],
            )
            .unwrap();
        let second = bot(&mut world, [1800.0, 1000.0]);
        let first_state = world.slots.get_mut(first).unwrap();
        first_state.materials.energy = 10000.0;
        // storebody caps at 100 and adds command/10. Start at 100 so the ten-body
        // increase changes the Integer length, instead of rounding both equal.
        first_state.materials.body = 100.0;
        first_state.materials.radius = crate::physics::organism_radius(
            first_state.materials.body,
            first_state.materials.chloroplasts,
            world.config.fixed_bot_radii,
        );
        let first_radius = world.slots.get(first).unwrap().materials.radius;
        let second_radius = world.slots.get(second).unwrap().materials.radius;
        assert!(world.native_make_tie(first, second, 1000, -20, 7));
        world.tick().unwrap();
        let first_state = world.slots.get(first).unwrap();
        let second_state = world.slots.get(second).unwrap();
        assert_eq!(first_state.materials.body, 110.0);
        assert!(first_state.materials.radius - first_radius > 1.0);
        let distance =
            (second_state.kinematics.position[0] - first_state.kinematics.position[0]).abs();
        assert_eq!(
            world.memory_at(first, MEM_TIE_LENGTH).unwrap(),
            (distance - first_radius - second_radius).round_ties_even() as i32
        );
        assert_eq!(
            world.memory_at(second, MEM_TIE_LENGTH).unwrap(),
            (distance - first_state.materials.radius - second_state.materials.radius)
                .round_ties_even() as i32
        );
        assert_ne!(
            world.memory_at(first, MEM_TIE_LENGTH).unwrap(),
            world.memory_at(second, MEM_TIE_LENGTH).unwrap()
        );
    }

    #[test]
    fn no_native_ties_retains_unreached_controls_and_the_requested_port() {
        let mut world = world();
        let actor = bot(&mut world, [1000.0; 2]);
        let source = world.slots.get_mut(actor).unwrap();
        let pending = [
            (455, 7),
            (452, -4),
            (453, 10),
            (331, 99),
            (467, 7),
            (468, 20),
            (469, 40),
            (480, 123),
            (484, 234),
        ];
        for (address, value) in pending {
            source.program.memory.write(address, value);
        }
        source.program.tie_angle_overwrites[0] = true;
        source.program.tie_length_overwrites[0] = true;
        world.tie_exchange_phase(&mut CycleRecorder::disabled(0));
        for (address, expected) in pending {
            assert_eq!(
                world.memory_at(actor, address).unwrap(),
                expected,
                "cell {address}"
            );
        }
        let source = world.slots.get(actor).unwrap();
        assert!(source.program.tie_angle_overwrites[0]);
        assert!(source.program.tie_length_overwrites[0]);
    }

    #[test]
    fn native_negative_resource_commands_use_tiepres_fallback_and_waste_has_no_age_gate() {
        for age in [0, 2] {
            let mut world = world();
            let (actor, target) = projection_pair(&mut world, 7);
            let source = world.slots.get_mut(actor).unwrap();
            source.counters.age = age;
            source.materials.waste = 50.0;
            source.materials.permanent_waste = 0.0;
            source.program.memory.write(455, 0);
            source.program.memory.write(452, -4);
            source.program.memory.write(453, 20);
            world.tie_exchange_phase(&mut CycleRecorder::disabled(0));
            assert_eq!(world.slots.get(actor).unwrap().materials.waste, 30.0);
            assert_eq!(world.slots.get(target).unwrap().materials.waste, 19.8);
            assert_eq!(
                world.slots.get(actor).unwrap().materials.permanent_waste,
                0.2
            );
            assert_eq!(world.memory_at(actor, 452).unwrap(), 0);
            assert_eq!(world.memory_at(actor, 453).unwrap(), 0);
        }
    }

    #[test]
    fn reached_native_negative_resource_command_clears_even_an_unhandled_location() {
        let mut world = world();
        let (actor, _) = projection_pair(&mut world, 7);
        let source = world.slots.get_mut(actor).unwrap();
        source.program.memory.write(452, -2);
        source.program.memory.write(453, 10);
        world.tie_exchange_phase(&mut CycleRecorder::disabled(0));
        assert_eq!(world.memory_at(actor, 452).unwrap(), 0);
        assert_eq!(world.memory_at(actor, 453).unwrap(), 0);
    }

    #[test]
    fn native_reference_phase_does_not_publish_selected_geometry_before_the_angle_pass() {
        let mut world = world();
        let (actor, target) = projection_pair(&mut world, 7);
        for (id, angle, length) in [(actor, 111, 333), (target, 222, 444)] {
            let robot = world.slots.get_mut(id).unwrap();
            robot.counters.simulation_age = 2;
            robot.program.memory.write(MEM_TIE_ANGLE, angle);
            robot.program.memory.write(MEM_TIE_LENGTH, length);
            robot
                .program
                .memory
                .write(MEM_TIE_MEMORY_LOCATION, MEM_TIE_ANGLE);
            robot.program.memory.write(MEM_NUMBER_OF_TIES, 555);
        }
        for id in [actor, target] {
            world.refresh_one_tie_projection(id.slot() as usize, true, &HashMap::new(), true);
        }
        assert_eq!(world.memory_at(actor, MEM_TIE_MEMORY_VALUE).unwrap(), 222);
        assert_eq!(world.memory_at(target, MEM_TIE_MEMORY_VALUE).unwrap(), 111);
        for (id, angle, length) in [(actor, 111, 333), (target, 222, 444)] {
            assert_eq!(world.memory_at(id, MEM_TIE_ANGLE).unwrap(), angle);
            assert_eq!(world.memory_at(id, MEM_TIE_LENGTH).unwrap(), length);
        }
        world.refresh_one_tie_projection(actor.slot() as usize, false, &HashMap::new(), true);
        assert_eq!(world.memory_at(actor, MEM_TIE_LENGTH).unwrap(), 80);
        assert_eq!(world.memory_at(actor, MEM_NUMBER_OF_TIES).unwrap(), 555);
        // Update_Ties, rather than readtie/UpdateTieAngles, publishes the count.
        world.tie_exchange_phase(&mut CycleRecorder::disabled(0));
        assert_eq!(world.memory_at(actor, MEM_NUMBER_OF_TIES).unwrap(), 1);
    }
}
