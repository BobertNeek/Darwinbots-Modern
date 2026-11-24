//! Physics.TieHooke/TieTorque are endpoint visits, not a simultaneous pair force.
use super::*;
use crate::physics::{native_full_magnitude, native_scalar};
use crate::tie::{HARD_TIE_B, HARD_TIE_K};

impl Engine {
    pub(super) fn native_tie_forces(&mut self, actor: OrganismId) {
        // Both TieHooke and TieTorque require the actor's runtime tie count.
        if self
            .slots
            .get(actor)
            .is_ok_and(|robot| robot.native_tie_count_pending)
        {
            return;
        }
        let mut ordinal = 0;
        loop {
            let indices = self.tie_indices_for(actor);
            let Some(&index) = indices.get(ordinal) else {
                break;
            };
            ordinal += 1;
            let tie = &self.ties[index];
            let forward = tie.first == actor;
            let side = usize::from(!forward);
            let other = if forward { tie.second } else { tie.first };
            let (Ok(robot), Ok(target)) = (self.slots.get(actor), self.slots.get(other)) else {
                // CheckRobot's exceptional cleanup only has the local array.
                // Normal lifecycle deletion keeps both records alive below.
                if let Ok(robot) = self.slots.get_mut(actor) {
                    let previous = ordinal
                        .checked_sub(2)
                        .and_then(|position| {
                            robot.ties.get(position).and_then(|endpoint| *endpoint)
                        })
                        .map_or(0, |endpoint| endpoint.port);
                    robot.program.memory.write(MEM_TIE_PRESENT, previous);
                }
                self.ties.remove(index);
                self.rebuild_native_tie_index();
                self.refresh_one_tie_projection(
                    actor.slot() as usize,
                    false,
                    &HashMap::new(),
                    false,
                );
                ordinal -= 1;
                continue;
            };
            let delta = std::array::from_fn::<_, 2, _>(|axis| {
                target.kinematics.position[axis] - robot.kinematics.position[axis]
            });
            let distance = native_full_magnitude(delta);
            if distance - robot.materials.radius - target.materials.radius > MAX_TIE_SURFACE_GAP {
                self.native_delete_pair(actor, other);
                continue;
            }
            let mut ends = tie.native_end_states();
            let end = &mut ends[side];
            if end.last > 1 {
                end.last -= 1;
            }
            if end.last < 0 {
                end.last += 1;
            }
            if end.last == 1 {
                // TieHooke advances k even after DeleteTie compacts the array,
                // so the next endpoint skips this robot's spring visit.
                self.ties[index].native_ends = Some(ends);
                self.ties[index].sync_native_primary();
                self.native_delete_pair(actor, other);
                continue;
            }
            let angle = (-delta[1]).atan2(delta[0]);
            if end.last == -1 {
                end.rest_length = distance;
                end.b = HARD_TIE_B;
                end.k = HARD_TIE_K;
                end.hardened = true;
                end.kind = 3;
                if !tie.is_native_back(actor) {
                    end.angle = normalize_tie_angle(angle - robot.heading());
                    end.angle_fixed = true;
                    end.angle_initialized = true;
                }
            }
            let hardened_now = end.last == -1;
            let mut spring = [0.0; 2];
            let displacement = end.rest_length - distance;
            if distance != 0.0 && displacement.abs() > TIE_DEADBAND {
                // TieHooke stores 1 / Length as a Single before VectorScalar.
                let unit = native_scalar(delta.map(|value| -value), 1.0 / distance);
                let relative = std::array::from_fn::<_, 2, _>(|axis| {
                    robot.kinematics.velocity[axis] - target.kinematics.velocity[axis]
                });
                let elastic = end.k * displacement.signum() * (displacement.abs() - TIE_DEADBAND);
                let damping = (relative[0] * unit[0] + relative[1] * unit[1]) * -end.b;
                spring = unit.map(|value| value * elastic + value * damping);
            }
            let robot = self.slots.get_mut(actor).unwrap();
            if hardened_now {
                robot.counters.multibot = true;
                robot.program.memory.write(MEM_MULTI, 1);
            }
            for (axis, force) in spring.into_iter().enumerate() {
                robot.kinematics.independent_impulse[axis] += force;
            }
            self.ties[index].native_ends = Some(ends);
            self.ties[index].sync_native_primary();
        }
        // TieTorque follows the complete TieHooke pass, and visits even a bond
        // that skipped its spring update when an earlier bond was deleted.
        let mut torque_sum = 0.0_f32;
        for index in self.tie_indices_for(actor) {
            let tie = &self.ties[index];
            let forward = tie.first == actor;
            let side = usize::from(!forward);
            let other = if forward { tie.second } else { tie.first };
            let (Ok(robot), Ok(target)) = (self.slots.get(actor), self.slots.get(other)) else {
                continue;
            };
            let delta = std::array::from_fn::<_, 2, _>(|axis| {
                target.kinematics.position[axis] - robot.kinematics.position[axis]
            });
            let distance = f64::from(delta[0] * delta[0] + delta[1] * delta[1]).sqrt() as f32;
            let angle = (-delta[1]).atan2(delta[0]);
            let mut ends = tie.native_end_states();
            let end = &mut ends[side];
            let mut torque = [0.0; 2];
            if !robot.flags.corpse && robot.flags.can_execute_dna && end.angle_fixed {
                let mismatch = tie_angle_difference(angle - robot.heading(), end.angle + end.bend);
                end.bend = 0.0;
                if mismatch.abs() > TIE_ANGLE_SLACK {
                    let correction = mismatch.signum() * (mismatch.abs() - TIE_ANGLE_SLACK);
                    torque_sum += correction;
                    torque = [
                        (-f64::from(angle).sin()
                            * f64::from(correction * 0.1)
                            * f64::from(distance)
                            / 10.0) as f32,
                        (-f64::from(angle).cos()
                            * f64::from(correction * 0.1)
                            * f64::from(distance)
                            / 10.0) as f32,
                    ]
                    .map(|value| value.clamp(-100.0, 100.0));
                }
            }
            let robot = self.slots.get_mut(actor).unwrap();
            for (axis, force) in torque.into_iter().enumerate() {
                robot.kinematics.independent_impulse[axis] += force;
            }
            let target = self.slots.get_mut(other).unwrap();
            for (axis, torque) in torque.into_iter().enumerate() {
                target.kinematics.independent_impulse[axis] -= torque;
            }
            self.ties[index].native_ends = Some(ends);
            self.ties[index].sync_native_primary();
        }
        if torque_sum != 0.0 && torque_sum.abs() <= std::f32::consts::TAU {
            self.slots
                .get_mut(actor)
                .unwrap()
                .kinematics
                .angular_momentum =
                torque_sum.clamp(-std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_4);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SaveFile, native_binary::NativeRecordFile};

    fn native_tie_countdowns(world: &Engine) -> HashMap<(i32, i32), i16> {
        let file = NativeRecordFile::decode(&world.export_native_sim().unwrap()).unwrap();
        let absolute_by_slot: HashMap<_, _> = file
            .robots
            .iter()
            .map(|robot| {
                (
                    robot.word("old_slot").unwrap(),
                    robot.long("absolute_id").unwrap(),
                )
            })
            .collect();
        let mut countdowns = HashMap::new();
        for robot in &file.robots {
            let absolute_id = robot.long("absolute_id").unwrap();
            for tie in robot.bytes("ties_core").unwrap().chunks_exact(38).skip(1) {
                let other_slot = i16::from_le_bytes(tie[2..4].try_into().unwrap());
                if let Some(&other_id) = absolute_by_slot.get(&other_slot) {
                    let last = i16::from_le_bytes(tie[26..28].try_into().unwrap());
                    countdowns.insert((absolute_id, other_id), last);
                }
            }
        }
        countdowns
    }

    #[test]
    fn loaded_ties_wait_for_a_recount_that_also_visits_dna_disabled_robots() {
        let mut fresh = Engine::new_native(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let first = fresh.spawn_at(dna.clone(), [300.0, 500.0]).unwrap();
        let second = fresh.spawn_at(dna, [650.0, 500.0]).unwrap();
        fresh.slots.get_mut(second).unwrap().flags.can_execute_dna = false;
        assert!(fresh.native_make_tie(first, second, 500, 65, 11));
        let mut loaded = Engine::from_native_sim(&fresh.export_native_sim().unwrap()).unwrap();
        let mut resumed = SaveFile::decode(&SaveFile::encode(&loaded).unwrap()).unwrap();

        assert_eq!(loaded.memory_at(first, MEM_NUMBER_OF_TIES).unwrap(), 1);
        assert!(!loaded.slots.get(second).unwrap().flags.can_execute_dna);
        fresh.tick().unwrap();
        assert_eq!(
            native_tie_countdowns(&fresh),
            HashMap::from([((1, 2), 64), ((2, 1), 64)])
        );

        for world in [&mut loaded, &mut resumed] {
            world.tick().unwrap();
            assert_eq!(
                native_tie_countdowns(world),
                HashMap::from([((1, 2), 65), ((2, 1), 65)])
            );
            assert_eq!(world.memory_at(second, MEM_NUMBER_OF_TIES).unwrap(), 1);
            world.tick().unwrap();
            assert_eq!(native_tie_countdowns(world), native_tie_countdowns(&fresh));
        }
    }

    #[test]
    fn creation_initializes_its_two_counts_and_expiry_waits_for_its_cold_peer() {
        let mut fresh = Engine::new_native(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let first = fresh.spawn_at(dna.clone(), [300.0, 500.0]).unwrap();
        let second = fresh.spawn_at(dna.clone(), [650.0, 500.0]).unwrap();
        let third = fresh.spawn_at(dna, [300.0, 800.0]).unwrap();
        assert!(fresh.native_make_tie(first, second, 500, 2, 11));
        let mut loaded = Engine::from_native_sim(&fresh.export_native_sim().unwrap()).unwrap();

        loaded.native_delete_pair(first, second);
        assert_eq!(loaded.ties.len(), 1);
        assert!(loaded.native_make_tie(first, third, 500, 65, 22));
        loaded.native_tie_forces(first);
        loaded.native_tie_forces(second);
        loaded.native_tie_forces(third);
        assert_eq!(
            native_tie_countdowns(&loaded),
            HashMap::from([((1, 2), 1), ((2, 1), 2), ((1, 3), 64), ((3, 1), 64)])
        );

        loaded.tick().unwrap();
        // The original endpoint still sees a zero-count peer in the first
        // pre-pass. The mid-pass recount enables both visits on the next cycle.
        assert!(native_tie_countdowns(&loaded).contains_key(&(1, 2)));
        loaded.tick().unwrap();
        assert!(!native_tie_countdowns(&loaded).contains_key(&(1, 2)));
    }

    #[test]
    fn populated_sim_matches_original_24832_first_visit_tie_checkpoints() {
        let input = include_bytes!("../../tests/fixtures/db24832-original-user.sim");
        let mut loaded = Engine::from_native_sim(input).unwrap();
        loaded.tick().unwrap();
        let countdowns = native_tie_countdowns(&loaded);
        // Captured from original Darwinbots 2.48.32 at cycle 282, the first
        // cycle after loading this cycle-281 save. IDs are absolute, not slots.
        for (first, second, expected) in [
            (114, 262, 65),
            (16, 210, 26),
            (190, 53, 10),
            (205, 64, 20),
            (217, 75, 31),
            (234, 85, 42),
            (243, 73, 47),
            (272, 92, 74),
        ] {
            assert_eq!(countdowns.get(&(first, second)), Some(&expected));
            assert_eq!(countdowns.get(&(second, first)), Some(&expected));
        }
    }

    #[test]
    fn second_endpoint_spring_observes_geometry_after_first_endpoint_visit() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let first = world.spawn_at(dna.clone(), [500.0, 500.0]).unwrap();
        let second = world.spawn_at(dna, [700.0, 500.0]).unwrap();
        world
            .ties
            .push(TieState::legacy(first, second, 100.0, 1, 1));
        world.native_tie_forces(first);
        assert_eq!(
            world
                .slots
                .get(first)
                .unwrap()
                .kinematics
                .independent_impulse,
            [4.0, 0.0]
        );
        assert_eq!(
            world
                .slots
                .get(second)
                .unwrap()
                .kinematics
                .independent_impulse,
            [0.0; 2]
        );
        world.slots.get_mut(first).unwrap().kinematics.position[0] = 400.0;
        world.native_tie_forces(second);
        assert_eq!(
            world
                .slots
                .get(second)
                .unwrap()
                .kinematics
                .independent_impulse,
            [-9.0, 0.0]
        );
    }

    #[test]
    fn native_tie_spring_rounds_the_reciprocal_before_scaling() {
        let mut world = Engine::new_native(EngineConfig {
            world_width: 16_000.0,
            world_height: 8_000.0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        // Reconstructed post-contact input to robot 205's spring visit in
        // original Darwinbots 2.48.32 cycle 292.
        let actor = world
            .spawn_at(dna.clone(), [0x464e_64f3, 0x453b_dcad].map(f32::from_bits))
            .unwrap();
        let peer = world
            .spawn_at(dna, [0x4649_6c41, 0x4564_87c9].map(f32::from_bits))
            .unwrap();
        world.slots.get_mut(actor).unwrap().kinematics.velocity =
            [0x3e2a_4c1a, 0xba54_6e78].map(f32::from_bits);
        world.slots.get_mut(peer).unwrap().kinematics.velocity =
            [0x3e04_8414, 0x3d89_4219].map(f32::from_bits);
        world.ties.push(TieState::new(peer, actor, 454.0, 1, 1));

        world.native_tie_forces(actor);

        let force = world
            .slots
            .get(actor)
            .unwrap()
            .kinematics
            .independent_impulse;
        assert_eq!(force.map(f32::to_bits), [0xbf8c_d55b, 0x4010_0231]);
    }

    #[test]
    fn native_tie_spring_uses_original_scaled_vector_length() {
        let mut world = Engine::new_native(EngineConfig {
            world_width: 16_000.0,
            world_height: 12_000.0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        // Reconstructed post-contact input to robot 346's spring visit in
        // original Darwinbots 2.48.32 cycle 329.
        let actor = world
            .spawn_at(dna.clone(), [0x465a_8c77, 0x4550_1222].map(f32::from_bits))
            .unwrap();
        let peer = world
            .spawn_at(dna, [0x4658_5945, 0x4548_e47a].map(f32::from_bits))
            .unwrap();
        world.slots.get_mut(actor).unwrap().kinematics.velocity =
            [0x3fa2_ff3e, 0xc017_c61e].map(f32::from_bits);
        world.slots.get_mut(peer).unwrap().kinematics.velocity =
            [0x3d79_e636, 0x3d7c_d5a0].map(f32::from_bits);
        world.ties.push(TieState::new(actor, peer, 236.0, 1, 1));

        world.native_tie_forces(actor);

        let force = world
            .slots
            .get(actor)
            .unwrap()
            .kinematics
            .independent_impulse;
        assert_eq!(force.map(f32::to_bits), [0x3e8c_d2f1, 0x3e65_bf82]);
    }

    #[test]
    fn hardening_captures_each_end_length_at_its_own_visit() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let first = world.spawn_at(dna.clone(), [500.0, 500.0]).unwrap();
        let second = world.spawn_at(dna, [700.0, 500.0]).unwrap();
        let mut tie = TieState::new(first, second, 100.0, 1, 1);
        tie.last = -2;
        world.ties.push(tie);
        world.native_tie_forces(first);
        world.slots.get_mut(first).unwrap().kinematics.position[0] = 400.0;
        world.native_tie_forces(second);
        let ends = world.ties[0].native_end_states();
        assert_eq!(ends[0].rest_length, 200.0);
        assert_eq!(ends[1].rest_length, 300.0);
        assert_eq!([ends[0].last, ends[1].last], [-1, -1]);
        assert!(ends[0].angle_fixed);
        assert!(!ends[1].angle_fixed);
        let saved = serde_json::to_vec(&world.ties).unwrap();
        let restored: Vec<TieState> = serde_json::from_slice(&saved).unwrap();
        assert_eq!(restored[0].native_end_states(), ends);
    }

    #[test]
    fn expiry_updates_both_counts_and_tiepres_then_skips_the_compacted_spring_entry() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let actor = world.spawn_at(dna.clone(), [500.0; 2]).unwrap();
        let first = world.spawn_at(dna.clone(), [700.0, 500.0]).unwrap();
        let second = world.spawn_at(dna, [500.0, 800.0]).unwrap();
        for id in [actor, first, second] {
            world.slots.get_mut(id).unwrap().materials.radius = 60.0;
        }
        world.slots.get_mut(actor).unwrap().set_heading(0.0);
        assert!(world.native_make_tie(actor, first, 500, -20, 11));
        assert!(world.native_make_tie(actor, second, 500, -20, 22));
        for tie in &mut world.ties {
            let mut ends = tie.native_end_states();
            ends[0].last = 2;
            ends[1].last = 2;
            tie.native_ends = Some(ends);
            tie.sync_native_primary();
        }
        let mut ends = world.ties[1].native_end_states();
        ends[0].angle_fixed = true;
        ends[0].angle = 0.0;
        world.ties[1].native_ends = Some(ends);
        world.ties[1].sync_native_primary();
        world
            .slots
            .get_mut(actor)
            .unwrap()
            .program
            .memory
            .write(MEM_TIE_PRESENT, 11);
        world.native_tie_forces(actor);
        assert_eq!(world.ties.len(), 1);
        assert!(world.ties[0].contains(second));
        // Compaction skips the second spring countdown, but TieTorque still
        // visits it after the spring pass and applies its fixed-angle request.
        assert_eq!(world.ties[0].native_end_states()[0].last, 2);
        assert_ne!(
            world.slots.get(actor).unwrap().kinematics.angular_momentum,
            0.0
        );
        assert_eq!(world.memory_at(actor, MEM_TIE_PRESENT).unwrap(), 0);
        assert_eq!(world.memory_at(actor, MEM_NUMBER_OF_TIES).unwrap(), 1);
        assert_eq!(world.memory_at(first, MEM_TIE_PRESENT).unwrap(), 0);
        assert_eq!(world.memory_at(first, MEM_NUMBER_OF_TIES).unwrap(), 0);
    }

    #[test]
    fn regang_publishes_multibot_on_the_hardening_endpoint_before_tie_readback() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let first = world.spawn_at(dna.clone(), [500.0; 2]).unwrap();
        let second = world.spawn_at(dna, [700.0, 500.0]).unwrap();
        let mut tie = TieState::new(first, second, 200.0, 11, 1);
        tie.last = -2;
        world.ties.push(tie);
        world.native_tie_forces(first);
        assert!(world.slots.get(first).unwrap().counters.multibot);
        assert_eq!(world.memory_at(first, MEM_MULTI).unwrap(), 1);
        assert!(!world.slots.get(second).unwrap().counters.multibot);
        assert_eq!(world.memory_at(second, MEM_MULTI).unwrap(), 0);
    }

    #[test]
    fn native_tie_keeps_the_pre_transfer_mass_through_sensing() {
        let mut world = Engine::new_native(EngineConfig {
            world_width: 3_000.0,
            world_height: 2_000.0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let recipient = world
            .spawn_at(
                LegacyDna::parse(
                    "start 11 .tienum store -1 .tieloc store -1000 .tieval store stop",
                )
                .unwrap(),
                [500.0, 1_000.0],
            )
            .unwrap();
        let payer = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [800.0, 1_000.0])
            .unwrap();
        world
            .ties
            .push(TieState::legacy(recipient, payer, 300.0, 11, 1));
        world.refresh_tie_projections();
        let receiver = world.slots.get_mut(recipient).unwrap();
        receiver.counters.age = 1;
        receiver.materials.body = 490.0;
        receiver.materials.shell = 200.0;
        let starting_mass = derived_mass(490.0, 200.0, 0.0);
        world.slots.get_mut(payer).unwrap().materials.energy = 3_000.0;

        world.tick().unwrap();
        let receiver = world.slots.get(recipient).unwrap();
        assert_eq!(receiver.materials.body, 519.0);
        assert_eq!(receiver.materials.mass, starting_mass);
        assert_eq!(world.memory_at(recipient, 10).unwrap(), 1);
        let next_mass = derived_mass(
            receiver.materials.body,
            receiver.materials.shell,
            receiver.materials.chloroplasts,
        );

        world.tick().unwrap();
        let receiver = world.slots.get(recipient).unwrap();
        assert_eq!(receiver.materials.mass, next_mass);
        assert_eq!(world.memory_at(recipient, 10).unwrap(), 2);
    }

    #[test]
    fn genuine_cycle_283_tie_recipient_retains_the_original_mass_readback() {
        // Original 2.48.32 cycle-283 capture SHA-256
        // fd2cb2bd5e36044d459d238f561dcf60cec70bf1efcb6d1b90dde3078ad9927e.
        let input = include_bytes!("../../tests/fixtures/db24832-original-user.sim");
        let mut world = Engine::from_native_sim(input).unwrap();
        world.tick_many(2).unwrap();
        assert_eq!(world.native_cycle, Some(283));
        let robot = world
            .slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .find(|robot| robot.identity.absolute_id == 143)
            .unwrap();
        assert!(robot.materials.body > 500.0);
        assert_eq!(robot.program.memory.read(10), 1);
    }
}
