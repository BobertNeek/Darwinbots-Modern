//! Keep native force accumulation separate from collision velocities and integration.
use super::*;

impl Engine {
    pub(super) fn capture_native_added_mass(&mut self) {
        for slot in self.slots.iter_mut() {
            if let Some(robot) = slot.organism.as_mut() {
                if robot.flags.hidden { continue; }
                robot.kinematics.native_added_mass = Some(fluid_added_mass(
                    robot.materials.radius, self.config.physics.density));
            }
        }
    }

    pub(super) fn manage_native_fixed_state(&mut self) {
        for slot in self.phase_slot_range() {
            if let Some(robot) = self.slots[slot].organism.as_mut() {
                if self.robot_fixing_allowed { robot.flags.fixed = robot.program.memory.read(MEM_FIXED) > 0; }
                robot.kinematics.native_fixed_state = Some(robot.flags.fixed);
            }
        }
    }

    pub(super) fn native_intent_phase(&mut self) {
        let settings = self.config.physics.clone();
        let scaling = self.tide_scaling();
        let mut gravity = self.config.gravity;
        if self.config.vegetation.tides > 0 { gravity[1] = (1.0 - scaling) * 4.0; }
        let brownian = if self.config.vegetation.tides > 0 {
            if scaling > 0.8 { 10.0 } else { 0.0 }
        } else { self.config.brownian_motion };
        for id in self.phase_robot_ids() {
            self.native_planet_forces(id);
            let robot = self.slots.get_mut(id).unwrap();
            let mass = derived_mass(robot.materials.body, robot.materials.shell,
                robot.materials.chloroplasts).clamp(1.0, 32_000.0);
            robot.materials.mass = mass;
            if robot_is_fixed(robot, self.robot_fixing_allowed) {
                robot.kinematics.pending_velocity = [0.0; 2];
                continue;
            }
            apply_resistance(&mut robot.kinematics.velocity, mass,
                robot.materials.radius, self.config.drag, &settings);
            apply_angular_damping(&mut robot.kinematics.angular_momentum,
                mass * settings.surface_gravity * settings.kinetic_friction, &settings);
            if brownian != 0.0 {
                let random = self.native_random.as_mut().unwrap();
                let impulse = (f64::from(brownian) * 0.5 * f64::from(random.unit())) as f32;
                let angle = random.unit() * 2.0 * std::f32::consts::PI;
                let rotation = (f64::from(impulse / 100.0) * (f64::from(random.unit()) - 0.5)) as f32;
                robot.kinematics.independent_impulse[0] += (f64::from(angle).cos() * f64::from(impulse)) as f32;
                robot.kinematics.independent_impulse[1] += (f64::from(angle).sin() * f64::from(impulse)) as f32;
                robot.kinematics.angular_momentum += rotation;
                robot.program.rng_state = u64::from(random.state());
            }
            let mut effective_gravity = gravity;
            if self.config.vegetation.pond_mode && !self.config.axis_topology()[1] && gravity[1] != 0.0 {
                let threshold = 1.0 / scaling.max(f32::EPSILON)
                    - robot.kinematics.position[1] / self.config.world_height.max(1.0);
                effective_gravity[1] = if threshold > robot.buoyancy { gravity[1].abs() } else { -gravity[1].abs() };
            }
            if robot.kinematics.pending_velocity != [0.0; 2] {
                self.stats.intentional_movement_events = self.stats.intentional_movement_events.saturating_add(1);
            }
            for axis in 0..2 {
                // pending_velocity holds VoluntaryForces' impulse, despite its
                // historical field name. Divide by effective mass only later.
                robot.kinematics.independent_impulse[axis] +=
                    effective_gravity[axis] * mass;
                robot.kinematics.independent_impulse[axis] += robot.kinematics.pending_velocity[axis];
            }
            robot.kinematics.pending_velocity = [0.0; 2];
        }
    }

    fn native_planet_forces(&mut self, id: OrganismId) {
        let robot = self.slots.get(id).unwrap();
        if !self.config.physics.planet_eaters || robot_is_fixed(robot, self.robot_fixing_allowed) { return; }
        let position = robot.kinematics.position;
        let mass = robot.materials.mass.min(192.0);
        let mut own = [0.0; 2];
        for slot in id.slot() as usize + 1..self.slots.len() {
            let Some(other) = self.slots[slot].organism.as_mut() else { continue };
            let delta = std::array::from_fn::<_, 2, _>(|axis| other.kinematics.position[axis] - position[axis]);
            let squared = delta[0] * delta[0] + delta[1] * delta[1];
            if squared == 0.0 || other.materials.mass == 0.0 { continue; }
            let distance = f64::from(squared).sqrt() as f32;
            let force = self.config.physics.planet_eaters_g * mass * other.materials.mass.min(192.0) / squared;
            for axis in 0..2 {
                let impulse = (delta[axis] / distance) * force;
                own[axis] += impulse;
                other.kinematics.independent_impulse[axis] -= impulse;
            }
        }
        let robot = self.slots.get_mut(id).unwrap();
        for axis in 0..2 { robot.kinematics.independent_impulse[axis] += own[axis]; }
    }

    pub(super) fn native_finalize_forces(&mut self) {
        for slot in self.phase_slot_range() {
            let Some(robot) = self.slots[slot].organism.as_mut() else { continue };
            let impulse = robot.kinematics.independent_impulse;
            let magnitude = impulse[0].hypot(impulse[1]);
            let velocity = robot.kinematics.velocity;
            let speed = velocity[0].hypot(velocity[1]);
            let limit = robot.materials.mass * self.config.physics.surface_gravity
                * self.config.physics.static_friction;
            let perpendicular = if speed == 0.0 || magnitude == 0.0 { 1.0 }
                else { ((velocity[0] / speed) * (impulse[1] / magnitude)
                    - (velocity[1] / speed) * (impulse[0] / magnitude)).abs() };
            if limit > 0.0 && limit * perpendicular > magnitude {
                robot.kinematics.independent_impulse = [0.0; 2];
            }
            for axis in 0..2 {
                robot.kinematics.independent_impulse[axis] -= robot.kinematics.resistive_impulse[axis];
            }
        }
    }

    pub(super) fn native_integration_velocities(&self, batch: &mut RobotPhysicsBatch) {
        for (index, id) in batch.ids.iter().enumerate() {
            let robot = self.slots.get(*id).unwrap();
            if robot_is_fixed(robot, self.robot_fixing_allowed) { batch.batch.velocities[index] = [0.0; 2]; continue; }
            let added = robot.kinematics.native_added_mass.unwrap_or_else(|| {
                fluid_added_mass(robot.materials.radius, self.config.physics.density)
            });
            let mass = (robot.materials.mass + added).max(0.25);
            let inverse = 1.0 / mass;
            let mut velocity = robot.kinematics.velocity;
            for (axis, component) in velocity.iter_mut().enumerate() {
                *component += robot.kinematics.independent_impulse[axis] * inverse;
            }
            let speed = velocity[0].hypot(velocity[1]);
            if speed > self.config.physics.max_velocity && speed > 0.0 {
                for component in &mut velocity { *component = *component / speed * self.config.physics.max_velocity; }
            }
            batch.batch.velocities[index] = velocity;
        }
    }

    pub(super) fn finish_native_integration(&mut self, id: OrganismId) -> Result<(), EngineError> {
        let robot = self.slots.get_mut(id)?;
        if !robot_is_fixed(robot, self.robot_fixing_allowed) {
            self.native_buckets.relocate(id, robot.kinematics.position);
        }
        for address in [1, 2, 3, 4] { robot.program.memory.write(address, 0); }
        robot.kinematics.independent_impulse = [0.0; 2];
        robot.kinematics.resistive_impulse = [0.0; 2];
        if self.config.physics.zero_momentum { robot.kinematics.velocity = [0.0; 2]; }
        let [scalar, sx, dx, dn, up] = velocity_readbacks(robot.kinematics.velocity, robot.heading());
        for (address, value) in [(MEM_VEL_SCALAR, scalar), (MEM_VEL_SX, sx), (MEM_VEL_DX, dx),
            (MEM_VEL_DN, dn), (MEM_VEL_UP, up), (10, resource_readback(robot.materials.mass)),
            (MEM_MAX_VELOCITY, sensor_value(self.config.physics.max_velocity))] {
            robot.program.memory.write(address, value);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_fixpos_is_unfixed_and_later_writes_wait_for_manage_fixed() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world.spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2]).unwrap();
        world.slots.get_mut(id).unwrap().program.memory.write(MEM_FIXED, -1);
        world.manage_native_fixed_state();
        assert!(!robot_is_fixed(world.slots.get(id).unwrap(), true));
        world.slots.get_mut(id).unwrap().program.memory.write(MEM_FIXED, 1);
        assert!(!robot_is_fixed(world.slots.get(id).unwrap(), true));
        world.manage_native_fixed_state();
        assert!(robot_is_fixed(world.slots.get(id).unwrap(), true));
    }

    #[test]
    fn sampled_added_mass_is_not_recomputed_after_radius_changes() {
        let mut config = EngineConfig::testing(); config.physics.density = 0.001;
        let mut world = Engine::new(config).unwrap();
        let id = world.spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2]).unwrap();
        world.capture_native_added_mass();
        let cached = world.slots.get(id).unwrap().kinematics.native_added_mass.unwrap();
        let robot = world.slots.get_mut(id).unwrap();
        robot.materials.radius = 1.0;
        robot.kinematics.independent_impulse = [10.0, 0.0];
        let mut batch = world.physics_batch();
        world.native_integration_velocities(&mut batch);
        assert_eq!(batch.batch.velocities[0][0], 10.0 * (1.0 / (1.0 + cached)));
        assert!(batch.batch.velocities[0][0] < 1.0);
    }

    #[test]
    fn force_waits_until_integration_and_divides_by_mass() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world.spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2]).unwrap();
        let robot = world.slots.get_mut(id).unwrap();
        robot.materials.body = 2000.0;
        robot.kinematics.pending_velocity = [20.0, 0.0];
        world.native_intent_phase();
        assert_eq!(world.slots.get(id).unwrap().kinematics.velocity, [0.0; 2]);
        world.native_finalize_forces();
        world.physics_phase().unwrap();
        assert_eq!(world.slots.get(id).unwrap().kinematics.position, [510.0, 500.0]);
    }

    #[test]
    fn zero_momentum_moves_then_clears_velocity_and_sensors() {
        let mut config = EngineConfig::testing(); config.physics.zero_momentum = true;
        let mut world = Engine::new(config).unwrap();
        let id = world.spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2]).unwrap();
        world.slots.get_mut(id).unwrap().kinematics.velocity = [10.0, 0.0];
        world.tick().unwrap();
        assert_eq!(world.slots.get(id).unwrap().kinematics.position, [510.0, 500.0]);
        assert_eq!(world.slots.get(id).unwrap().kinematics.velocity, [0.0; 2]);
        assert_eq!(world.memory_at(id, MEM_VEL_SCALAR).unwrap(), 0);
    }
}
