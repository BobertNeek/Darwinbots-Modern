//! Physics.TieHooke/TieTorque are endpoint visits, not a simultaneous pair force.
use super::*;
use crate::tie::{HARD_TIE_B, HARD_TIE_K};

impl Engine {
    pub(super) fn native_tie_forces(&mut self, actor: OrganismId) {
        let mut remove = Vec::new();
        let mut torque_sum = 0.0_f32;
        for index in self.tie_indices_for(actor) {
            let tie = &self.ties[index];
            let forward = tie.first == actor;
            let side = usize::from(!forward);
            let other = if forward { tie.second } else { tie.first };
            let (Ok(robot), Ok(target)) = (self.slots.get(actor), self.slots.get(other)) else {
                remove.push(index); continue;
            };
            let delta = std::array::from_fn::<_, 2, _>(|axis| target.kinematics.position[axis] - robot.kinematics.position[axis]);
            let distance = f64::from(delta[0] * delta[0] + delta[1] * delta[1]).sqrt() as f32;
            if distance - robot.materials.radius - target.materials.radius > MAX_TIE_SURFACE_GAP {
                remove.push(index); continue;
            }
            let mut ends = tie.native_end_states();
            let end = &mut ends[side];
            if end.last > 1 { end.last -= 1; }
            if end.last < 0 { end.last += 1; }
            if end.last == 1 { remove.push(index); continue; }
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
            let mut spring = [0.0; 2];
            let displacement = end.rest_length - distance;
            if distance != 0.0 && displacement.abs() > TIE_DEADBAND {
                let unit = delta.map(|value| -value / distance);
                let relative = std::array::from_fn::<_, 2, _>(|axis| robot.kinematics.velocity[axis] - target.kinematics.velocity[axis]);
                let elastic = end.k * displacement.signum() * (displacement.abs() - TIE_DEADBAND);
                let damping = (relative[0] * unit[0] + relative[1] * unit[1]) * -end.b;
                spring = unit.map(|value| value * elastic + value * damping);
            }
            let mut torque = [0.0; 2];
            if !robot.flags.corpse && robot.flags.can_execute_dna && end.angle_fixed {
                let mismatch = tie_angle_difference(angle - robot.heading(), end.angle + end.bend);
                end.bend = 0.0;
                if mismatch.abs() > TIE_ANGLE_SLACK {
                    let correction = mismatch.signum() * (mismatch.abs() - TIE_ANGLE_SLACK);
                    torque_sum += correction;
                    torque = [
                        (-f64::from(angle).sin() * f64::from(correction * 0.1) * f64::from(distance) / 10.0) as f32,
                        (-f64::from(angle).cos() * f64::from(correction * 0.1) * f64::from(distance) / 10.0) as f32,
                    ].map(|value| value.clamp(-100.0, 100.0));
                }
            }
            let robot = self.slots.get_mut(actor).unwrap();
            for axis in 0..2 {
                robot.kinematics.independent_impulse[axis] += spring[axis];
                robot.kinematics.independent_impulse[axis] += torque[axis];
            }
            let target = self.slots.get_mut(other).unwrap();
            for axis in 0..2 { target.kinematics.independent_impulse[axis] -= torque[axis]; }
            self.ties[index].native_ends = Some(ends);
            self.ties[index].sync_native_primary();
        }
        if torque_sum != 0.0 && torque_sum.abs() <= std::f32::consts::TAU {
            self.slots.get_mut(actor).unwrap().kinematics.angular_momentum = torque_sum
                .clamp(-std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_4);
        }
        if !remove.is_empty() {
            remove.sort_unstable();
            remove.dedup();
            for index in remove.into_iter().rev() { self.ties.remove(index); }
            self.rebuild_native_tie_index();
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
        world.ties.push(TieState::legacy(first, second, 100.0, 1, 1));
        world.native_tie_forces(first);
        assert_eq!(world.slots.get(first).unwrap().kinematics.independent_impulse, [4.0, 0.0]);
        assert_eq!(world.slots.get(second).unwrap().kinematics.independent_impulse, [0.0; 2]);
        world.slots.get_mut(first).unwrap().kinematics.position[0] = 400.0;
        world.native_tie_forces(second);
        assert_eq!(world.slots.get(second).unwrap().kinematics.independent_impulse, [-9.0, 0.0]);
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
}
