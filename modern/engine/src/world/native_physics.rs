//! Keep native force accumulation separate from collision velocities and integration.
use super::*;
use crate::physics::apply_uncapped_resistance;

fn limit_native_speed(velocity: [f32; 2], limit: f32) -> [f32; 2] {
    // UpdatePosition calls VectorMagnitudeSquare, which clamps its input in place.
    let velocity = velocity.map(|component| component.clamp(-32_000.0, 32_000.0));
    let [x, y] = velocity.map(f64::from);
    let squared = (x * x + y * y) as f32;
    if f64::from(squared) > f64::from(limit).powi(2) {
        // Common.bas uses separate Single writes for magnitude, inverse, and unit vector.
        let (maximum, minimum) = (x.abs().max(y.abs()), x.abs().min(y.abs()));
        let inverse = if maximum < 0.00001 {
            -1.0
        } else {
            let magnitude = (maximum * (1.0 + (minimum / maximum).powi(2)).sqrt()) as f32;
            (1.0 / f64::from(magnitude)) as f32
        };
        let scale = limit.clamp(-32_000.0, 32_000.0);
        return velocity.map(|component| (component * inverse) * scale);
    }
    velocity
}

impl Engine {
    pub(super) fn capture_native_added_mass(&mut self) {
        for slot in self.slots.iter_mut() {
            if let Some(robot) = slot.organism.as_mut() {
                if robot.flags.hidden {
                    continue;
                }
                robot.kinematics.native_added_mass = Some(fluid_added_mass(
                    robot.materials.radius,
                    self.config.physics.density,
                ));
            }
        }
    }

    pub(super) fn manage_native_fixed_state(&mut self) {
        for slot in self.phase_slot_range() {
            if let Some(robot) = self.slots[slot].organism.as_mut() {
                if self.robot_fixing_allowed {
                    robot.flags.fixed = robot.program.memory.read(MEM_FIXED) > 0;
                }
                robot.kinematics.native_fixed_state = Some(robot.flags.fixed);
            }
        }
    }

    pub(super) fn native_intent_phase(&mut self) {
        let settings = self.config.physics.clone();
        let scaling = self.tide_scaling();
        let mut gravity = self.config.gravity;
        if self.config.vegetation.tides > 0 {
            gravity[1] = (1.0 - scaling) * 4.0;
        }
        let brownian = if self.config.vegetation.tides > 0 {
            if scaling > 0.8 { 10.0 } else { 0.0 }
        } else {
            self.config.brownian_motion
        };
        for id in self.phase_robot_ids() {
            self.native_planet_forces(id);
            let robot = self.slots.get_mut(id).unwrap();
            let mass = native_mass(&robot.materials);
            robot.materials.mass = mass;
            if robot_is_fixed(robot, self.robot_fixing_allowed) {
                robot.kinematics.pending_velocity = [0.0; 2];
                continue;
            }
            // NetForces keeps the collision velocity until UpdatePosition
            // adds the full impulse and then applies the speed limit.
            apply_uncapped_resistance(
                &mut robot.kinematics.velocity,
                mass,
                robot.materials.radius,
                self.config.drag,
                &settings,
            );
            // FrictionForces and SphereDragForces both exit before checking
            // angular underflow when surface gravity and density are zero.
            if settings.surface_gravity != 0.0 || settings.density != 0.0 {
                apply_angular_damping(
                    &mut robot.kinematics.angular_momentum,
                    mass * settings.surface_gravity * settings.kinetic_friction,
                    &settings,
                );
            }
            if brownian != 0.0 {
                let random = self.native_random.as_mut().unwrap();
                let impulse = (f64::from(brownian) * 0.5 * f64::from(random.unit())) as f32;
                let angle = random.unit() * 2.0 * std::f32::consts::PI;
                let rotation = (f64::from(impulse) / 100.0) * (f64::from(random.unit()) - 0.5);
                robot.kinematics.independent_impulse[0] +=
                    (f64::from(angle).cos() * f64::from(impulse)) as f32;
                robot.kinematics.independent_impulse[1] +=
                    (f64::from(angle).sin() * f64::from(impulse)) as f32;
                // Physics.bas stores the entire Brownian turn expression in Single ma.
                robot.kinematics.angular_momentum =
                    (f64::from(robot.kinematics.angular_momentum) + rotation) as f32;
                robot.program.rng_state = u64::from(random.state());
            }
            let mut effective_gravity = gravity;
            if self.config.vegetation.pond_mode
                && !self.config.axis_topology()[1]
                && gravity[1] != 0.0
            {
                let threshold = 1.0 / scaling.max(f32::EPSILON)
                    - robot.kinematics.position[1] / self.config.world_height.max(1.0);
                effective_gravity[1] = if threshold > robot.buoyancy {
                    gravity[1].abs()
                } else {
                    -gravity[1].abs()
                };
            }
            if robot.kinematics.pending_velocity != [0.0; 2] {
                self.stats.intentional_movement_events =
                    self.stats.intentional_movement_events.saturating_add(1);
            }
            for (axis, gravity) in effective_gravity.into_iter().enumerate() {
                // pending_velocity holds VoluntaryForces' impulse, despite its
                // historical field name. Divide by effective mass only later.
                robot.kinematics.independent_impulse[axis] += gravity * mass;
                robot.kinematics.independent_impulse[axis] +=
                    robot.kinematics.pending_velocity[axis];
            }
            robot.kinematics.pending_velocity = [0.0; 2];
        }
    }

    fn native_planet_forces(&mut self, id: OrganismId) {
        let robot = self.slots.get(id).unwrap();
        if !self.config.physics.planet_eaters || robot_is_fixed(robot, self.robot_fixing_allowed) {
            return;
        }
        let position = robot.kinematics.position;
        let mass = robot.materials.mass.min(192.0);
        let mut own = [0.0; 2];
        for slot in id.slot() as usize + 1..self.slots.len() {
            let Some(other) = self.slots[slot].organism.as_mut() else {
                continue;
            };
            let delta = std::array::from_fn::<_, 2, _>(|axis| {
                other.kinematics.position[axis] - position[axis]
            });
            let squared = delta[0] * delta[0] + delta[1] * delta[1];
            if squared == 0.0 || other.materials.mass == 0.0 {
                continue;
            }
            let distance = f64::from(squared).sqrt() as f32;
            let force =
                self.config.physics.planet_eaters_g * mass * other.materials.mass.min(192.0)
                    / squared;
            for axis in 0..2 {
                let impulse = (delta[axis] / distance) * force;
                own[axis] += impulse;
                other.kinematics.independent_impulse[axis] -= impulse;
            }
        }
        let robot = self.slots.get_mut(id).unwrap();
        for (axis, force) in own.into_iter().enumerate() {
            robot.kinematics.independent_impulse[axis] += force;
        }
    }

    pub(super) fn native_finalize_forces(&mut self) {
        for slot in self.phase_slot_range() {
            let Some(robot) = self.slots[slot].organism.as_mut() else {
                continue;
            };
            let impulse = robot.kinematics.independent_impulse;
            let magnitude = impulse[0].hypot(impulse[1]);
            let velocity = robot.kinematics.velocity;
            let speed = velocity[0].hypot(velocity[1]);
            let limit = robot.materials.mass
                * self.config.physics.surface_gravity
                * self.config.physics.static_friction;
            let perpendicular = if speed == 0.0 || magnitude == 0.0 {
                1.0
            } else {
                ((velocity[0] / speed) * (impulse[1] / magnitude)
                    - (velocity[1] / speed) * (impulse[0] / magnitude))
                    .abs()
            };
            if limit > 0.0 && limit * perpendicular > magnitude {
                robot.kinematics.independent_impulse = [0.0; 2];
            }
            for axis in 0..2 {
                robot.kinematics.independent_impulse[axis] -=
                    robot.kinematics.resistive_impulse[axis];
            }
        }
    }

    pub(super) fn native_integration_velocities(&self, batch: &mut RobotPhysicsBatch) {
        for (index, id) in batch.ids.iter().enumerate() {
            let robot = self.slots.get(*id).unwrap();
            if robot_is_fixed(robot, self.robot_fixing_allowed) {
                batch.batch.velocities[index] = [0.0; 2];
                continue;
            }
            let added = robot.kinematics.native_added_mass.unwrap_or_else(|| {
                fluid_added_mass(robot.materials.radius, self.config.physics.density)
            });
            let mass = (robot.materials.mass + added).max(0.25);
            let inverse = 1.0 / mass;
            let mut velocity = robot.kinematics.velocity;
            for (axis, component) in velocity.iter_mut().enumerate() {
                *component += robot.kinematics.independent_impulse[axis] * inverse;
            }
            batch.batch.velocities[index] =
                limit_native_speed(velocity, self.config.physics.max_velocity);
        }
    }

    pub(super) fn finish_native_integration(&mut self, id: OrganismId) -> Result<(), EngineError> {
        let robot = self.slots.get_mut(id)?;
        if !robot_is_fixed(robot, self.robot_fixing_allowed) {
            self.native_buckets.relocate(id, robot.kinematics.position);
        }
        for address in [1, 2, 3, 4] {
            robot.program.memory.write(address, 0);
        }
        robot.kinematics.independent_impulse = [0.0; 2];
        robot.kinematics.resistive_impulse = [0.0; 2];
        if self.config.physics.zero_momentum {
            robot.kinematics.velocity = [0.0; 2];
        }
        let [scalar, sx, dx, dn, up] =
            velocity_readbacks(robot.kinematics.velocity, robot.heading());
        for (address, value) in [
            (MEM_VEL_SCALAR, scalar),
            (MEM_VEL_SX, sx),
            (MEM_VEL_DX, dx),
            (MEM_VEL_DN, dn),
            (MEM_VEL_UP, up),
            (10, resource_readback(robot.materials.mass)),
            (
                MEM_MAX_VELOCITY,
                sensor_value(self.config.physics.max_velocity),
            ),
        ] {
            robot.program.memory.write(address, value);
        }
        Ok(())
    }
}

pub(super) fn native_mass(materials: &BiologyState) -> f32 {
    let body = f64::from(materials.body.max(0.0));
    let shell = f64::from(materials.shell.max(0.0));
    let chloroplasts = f64::from(materials.chloroplasts.clamp(0.0, 32_000.0));
    // Physics.bas CalcMass stores the full expression in Single mass before clamping.
    ((body / 1_000.0 + shell / 200.0 + chloroplasts / 32_000.0 * 31_680.0) as f32)
        .clamp(1.0, 32_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_brownian_turn_retains_small_momentum_without_environmental_resistance() {
        // Original 2.48.32 robot 145's transition from cycle 394 to 395.
        for (surface_gravity, expected) in [(0.0, 0xb96c_78e5), (1.0, 0xb96c_9142)] {
            let mut config = EngineConfig::testing();
            config.brownian_motion = 0.5;
            config.physics.density = 0.0;
            config.physics.surface_gravity = surface_gravity;
            let mut world = Engine::new_native(config).unwrap();
            let id = world
                .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2])
                .unwrap();
            world.slots.get_mut(id).unwrap().kinematics.angular_momentum =
                f32::from_bits(0x33c2_e7ef);
            world.native_random =
                Some(serde_json::from_value(serde_json::json!({ "state": 0x0e9953_u32 })).unwrap());

            world.native_intent_phase();

            let robot = world.slots.get(id).unwrap();
            assert_eq!(robot.kinematics.angular_momentum.to_bits(), expected);
        }
    }

    #[test]
    fn native_brownian_turn_rounds_at_the_angular_momentum_assignment() {
        let mut world = Engine::new_native(EngineConfig {
            brownian_motion: 0.5,
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2])
            .unwrap();
        // Original 2.48.32 multi-robot checkpoint, absolute robot 15, cycle 281 to 282.
        world.slots.get_mut(id).unwrap().kinematics.angular_momentum = f32::from_bits(0xb9a0_c26c);
        world.native_random =
            Some(serde_json::from_value(serde_json::json!({ "state": 0x7c3bf2_u32 })).unwrap());

        world.native_intent_phase();

        let robot = world.slots.get(id).unwrap();
        assert_eq!(robot.kinematics.angular_momentum.to_bits(), 0xba16_9553);
        assert_eq!(world.native_random.as_ref().unwrap().state(), 0x3aa0cf);
    }

    #[test]
    fn negative_fixpos_is_unfixed_and_later_writes_wait_for_manage_fixed() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2])
            .unwrap();
        world
            .slots
            .get_mut(id)
            .unwrap()
            .program
            .memory
            .write(MEM_FIXED, -1);
        world.manage_native_fixed_state();
        assert!(!robot_is_fixed(world.slots.get(id).unwrap(), true));
        world
            .slots
            .get_mut(id)
            .unwrap()
            .program
            .memory
            .write(MEM_FIXED, 1);
        assert!(!robot_is_fixed(world.slots.get(id).unwrap(), true));
        world.manage_native_fixed_state();
        assert!(robot_is_fixed(world.slots.get(id).unwrap(), true));
    }

    #[test]
    fn sampled_added_mass_is_not_recomputed_after_radius_changes() {
        let mut config = EngineConfig::testing();
        config.physics.density = 0.001;
        let mut world = Engine::new(config).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2])
            .unwrap();
        world.capture_native_added_mass();
        let cached = world
            .slots
            .get(id)
            .unwrap()
            .kinematics
            .native_added_mass
            .unwrap();
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
        let id = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2])
            .unwrap();
        let robot = world.slots.get_mut(id).unwrap();
        robot.materials.body = 2000.0;
        robot.kinematics.pending_velocity = [20.0, 0.0];
        world.native_intent_phase();
        assert_eq!(world.slots.get(id).unwrap().kinematics.velocity, [0.0; 2]);
        world.native_finalize_forces();
        world.physics_phase().unwrap();
        assert_eq!(
            world.slots.get(id).unwrap().kinematics.position,
            [510.0, 500.0]
        );
    }

    #[test]
    fn native_integration_caps_the_combined_velocity_and_impulse() {
        let mut config = EngineConfig::testing();
        config.physics.max_velocity = 60.0;
        let mut world = Engine::new_native(config).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        let collision_velocity = [0.0, -80.0];
        let robot = world.slots.get_mut(id).unwrap();
        robot.kinematics.velocity = collision_velocity;
        robot.kinematics.pending_velocity = [20.0, 0.0];
        world.native_intent_phase();
        assert_eq!(
            world.slots.get(id).unwrap().kinematics.velocity,
            collision_velocity
        );
        world.native_finalize_forces();
        world.physics_phase().unwrap();
        let expected = [20.0, -80.0].map(|component| component / 20.0_f32.hypot(80.0) * 60.0);
        assert_eq!(world.slots.get(id).unwrap().kinematics.velocity, expected);

        let mut portable_velocity = collision_velocity;
        apply_resistance(
            &mut portable_velocity,
            1.0,
            60.0,
            0.0,
            &world.config.physics,
        );
        assert_eq!(portable_velocity, [0.0, -60.0]);
    }

    #[test]
    fn native_velocity_limit_matches_genuine_cycle_282_vectors() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        for (before, expected) in [
            // Reconstructed pre-cap vectors and the original checkpoint for robots 117 and 218.
            ([0xc1a7_5380, 0xc261_1c27], [0xc1a7_3801, 0xc260_f729]),
            ([0x4243_4835, 0xc20c_81e3], [0x4242_d043, 0xc20c_2b96]),
            // The cap does not change a velocity below the limit.
            ([0xb8d0_56ca, 0x3933_a6f1], [0xb8d0_56ca, 0x3933_a6f1]),
        ] {
            world.slots.get_mut(id).unwrap().kinematics.velocity = before.map(f32::from_bits);
            let mut batch = world.physics_batch();
            world.native_integration_velocities(&mut batch);
            assert_eq!(batch.batch.velocities[0].map(f32::to_bits), expected);
        }
    }

    #[test]
    fn genuine_cycle_282_collision_velocity_matches_the_original_capture() {
        // Original 2.48.32 cycle-282 capture SHA-256
        // c49c002f66898a52db81645b78b11ecbb2ed1941dbeea03fe7d53dd3b24b9e91.
        let input = include_bytes!("../../tests/fixtures/db24832-original-user.sim");
        let mut world = Engine::from_native_sim(input).unwrap();
        world.tick().unwrap();
        assert_eq!(world.native_cycle, Some(282));
        let robot = world
            .slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .find(|robot| robot.identity.absolute_id == 139)
            .unwrap();
        assert_eq!(robot.kinematics.actual_velocity, [-22.375, -56.375]);
        for (actual, expected) in robot
            .kinematics
            .velocity
            .into_iter()
            .zip([-20.592_808, -56.355_446])
        {
            assert!(
                (actual - expected).abs() <= 0.000_01,
                "{actual} != {expected}"
            );
        }
    }

    #[test]
    fn zero_momentum_moves_then_clears_velocity_and_sensors() {
        let mut config = EngineConfig::testing();
        config.physics.zero_momentum = true;
        let mut world = Engine::new(config).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2])
            .unwrap();
        world.slots.get_mut(id).unwrap().kinematics.velocity = [10.0, 0.0];
        world.tick().unwrap();
        assert_eq!(
            world.slots.get(id).unwrap().kinematics.position,
            [510.0, 500.0]
        );
        assert_eq!(world.slots.get(id).unwrap().kinematics.velocity, [0.0; 2]);
        assert_eq!(world.memory_at(id, MEM_VEL_SCALAR).unwrap(), 0);
    }

    #[test]
    fn native_mass_and_uncapped_velocity_match_genuine_robot_85() {
        let mut world = Engine::new_native(EngineConfig {
            brownian_motion: 0.5,
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        // Original 2.48.32 absolute robot 85, cycle 281 to 282.
        let robot = world.slots.get_mut(id).unwrap();
        robot.materials.body = f32::from_bits(0x44b9_dff1);
        robot.materials.shell = 0.0;
        robot.materials.chloroplasts = f32::from_bits(0x46a8_14cb);
        robot.kinematics.velocity = [0xb8c6_c734, 0x3933_c656].map(f32::from_bits);
        world.native_random =
            Some(serde_json::from_value(serde_json::json!({ "state": 0xc8608f_u32 })).unwrap());

        world.native_intent_phase();
        assert_eq!(
            world.slots.get(id).unwrap().materials.mass.to_bits(),
            0x46a6_697b
        );
        let mut batch = world.physics_batch();
        world.native_integration_velocities(&mut batch);
        assert_eq!(
            batch.batch.velocities[0].map(f32::to_bits),
            [0xb8d0_56c9, 0x3933_a6f1]
        );
    }
}
