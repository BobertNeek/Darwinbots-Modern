//! Physics.TieHooke/TieTorque are endpoint visits, not a simultaneous pair force.
use super::*;
use crate::tie::{HARD_TIE_B, HARD_TIE_K};

impl Engine {
    pub(super) fn native_tie_forces(&mut self, actor: OrganismId) {
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
            let distance = f64::from(delta[0] * delta[0] + delta[1] * delta[1]).sqrt() as f32;
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
                if forward {
                    end.angle = normalize_tie_angle(angle - robot.heading());
                    end.angle_fixed = true;
                    end.angle_initialized = true;
                }
            }
            let hardened_now = end.last == -1;
            let mut spring = [0.0; 2];
            let displacement = end.rest_length - distance;
            if distance != 0.0 && displacement.abs() > TIE_DEADBAND {
                let unit = delta.map(|value| -value / distance);
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
}
