//! Robots.HandleWaste/altzheimer and Shots.defacate.

use super::*;

fn alzheimer_write_count(total: f32, threshold: i32) -> usize {
    if threshold <= 0 || total <= threshold as f32 {
        return 0;
    }
    // loops is Integer; its Single expression is converted with CInt semantics.
    ((total - threshold as f32) / 4.0)
        .round_ties_even()
        .clamp(0.0, i16::MAX as f32) as usize
}

impl Engine {
    pub(super) fn waste_phase(&mut self) {
        let settings = self.config.waste;
        let mut packets = Vec::new();
        for id in self.phase_robot_ids() {
            let robot = self.slots.get_mut(id).unwrap();
            if robot.flags.corpse || !robot.flags.can_execute_dna {
                continue;
            }
            robot.materials.waste = robot.materials.waste.max(0.0);
            robot.materials.permanent_waste = robot.materials.permanent_waste.max(0.0);
            let total = robot.materials.waste + robot.materials.permanent_waste;
            for _ in 0..alzheimer_write_count(total, settings.alzheimer_threshold) {
                let address = loop {
                    let address = world_random_integer(
                        &mut self.native_random,
                        &mut robot.program.rng_state,
                        1,
                        1000,
                    );
                    if !matches!(address, 921 | 922) {
                        break address;
                    }
                };
                let value = world_random_integer(
                    &mut self.native_random,
                    &mut robot.program.rng_state,
                    -32000,
                    32000,
                );
                robot.program.memory.write(address, value);
                robot.effects.mark_reproduction_input(address, value);
            }
            if robot.materials.waste > settings.defecation_threshold.max(0) as f32 {
                let amount = if robot.materials.waste > 32_000.0 {
                    robot.materials.waste = 31_500.0;
                    500.0
                } else {
                    robot.materials.waste.min(200.0)
                };
                if amount > 0.0 {
                    robot.materials.waste -= amount;
                    robot.materials.permanent_waste += amount / 1_000.0;
                    packets.push((id, amount));
                }
            }
            robot.materials.permanent_waste = robot.materials.permanent_waste.min(32_000.0);
            robot.materials.publish(&mut robot.program.memory);
        }
        let costs = self.active_dna_costs();
        let flat_cost = costs.action_cost_for(
            SimulationCostAction::ShotFormation,
            1.0,
            self.cost_population(),
        );
        for (owner, amount) in packets {
            let virtual_body = self.virtual_body_for(owner);
            let robot = self.slots.get_mut(owner).unwrap();
            let divisor = (robot.ties.iter().flatten().count() + 1) as f64;
            robot.materials.energy =
                (f64::from(robot.materials.energy) - flat_cost / divisor) as f32;
            let mut angle = robot.heading();
            if robot.program.memory.read(900) != 0 {
                angle = (angle - std::f32::consts::PI).rem_euclid(std::f32::consts::TAU);
                robot.program.memory.write(900, 0);
            }
            let aimed = robot.program.memory.read(901);
            if aimed != 0 {
                angle = (f64::from(robot.heading()) - f64::from(aimed % 1256) / 200.0) as f32;
                robot.program.memory.write(901, 0);
            }
            // newshot's unused range draw precedes its angle scatter.
            world_random_unit(&mut self.native_random, &mut robot.program.rng_state);
            let scatter = world_random_integer(
                &mut self.native_random,
                &mut robot.program.rng_state,
                -20,
                20,
            );
            angle = (f64::from(angle) + f64::from(scatter) / 200.0) as f32;
            let name = self.species[robot.identity.species.0 as usize].name.clone();
            self.projectiles.spawn(
                ProjectileSpawn {
                    owner,
                    owner_position: robot.kinematics.position,
                    owner_velocity: robot.kinematics.velocity,
                    owner_actual_velocity: robot.kinematics.actual_velocity,
                    correct_position: true,
                    owner_radius: robot.materials.radius,
                    owner_virtual_body: virtual_body,
                    range_multiplier: 1.0,
                    angle,
                    kind: -4,
                    value: amount,
                    payload: None,
                    source_species_name: Some(name),
                    toxin_memory_location: robot.program.memory.read(835),
                    toxin_memory_value: robot.program.memory.read(836),
                    color: robot.phenotype.color,
                    source_vegetable: robot.flags.vegetable,
                    stored: false,
                    gene_number: 0,
                    dna_length: 0,
                },
                &self.config.shots,
            );
            robot.materials.publish(&mut robot.program.memory);
            self.stats.shots_fired = self.stats.shots_fired.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_count_honors_disabled_zero_and_integer_half_rounding() {
        assert_eq!(alzheimer_write_count(32_000.0, 0), 0);
        assert_eq!(alzheimer_write_count(11.0, 1), 2);
        assert_eq!(alzheimer_write_count(15.0, 1), 4);
    }

    #[test]
    fn zero_threshold_does_not_touch_memory_or_rng() {
        let mut world = Engine::new(EngineConfig {
            waste: WasteSettings {
                alzheimer_threshold: 0,
                defecation_threshold: i32::MAX,
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        world.slots.get_mut(id).unwrap().materials.waste = 4_000.0;
        let before = world.slots.get(id).unwrap().program.clone();
        world.waste_phase();
        let after = &world.slots.get(id).unwrap().program;
        assert_eq!(after.rng_state, before.rng_state);
        assert_eq!(after.memory.read(500), before.memory.read(500));
    }

    #[test]
    fn defecation_charges_fee_and_spawns_a_radial_native_shot() {
        let mut world = Engine::new(EngineConfig {
            waste: WasteSettings {
                alzheimer_threshold: 0,
                defecation_threshold: 200,
            },
            dna_costs: DnaCosts {
                shot_formation: 2.0,
                ..Default::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        world.tick().unwrap();
        let robot = world.slots.get_mut(id).unwrap();
        robot.materials.waste = 250.0;
        robot.materials.permanent_waste = 7.0;
        robot.kinematics.velocity = [2.0, 0.0];
        robot.kinematics.actual_velocity = [3.0, 0.0];
        let radius = robot.materials.radius;
        world.waste_phase();
        let robot = world.slots.get(id).unwrap();
        assert_eq!(robot.materials.energy, 998.0);
        assert_eq!(robot.materials.waste, 50.0);
        assert!((robot.materials.permanent_waste - 7.2).abs() < 0.000001);
        let shot = &world.projectiles.snapshots()[0];
        assert_eq!(
            (shot.kind, shot.value, shot.range, shot.energy),
            (-4, 200, 11, 440.0)
        );
        assert!(((shot.end[0] - 501.0).hypot(shot.end[1] - 500.0) - radius).abs() < 0.001);
        assert!(((shot.velocity[0] - 3.0).hypot(shot.velocity[1]) - 40.0).abs() < 0.001);
    }

    #[test]
    fn player_bot_reproduction_is_assisted() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        world.player_bot.enabled = true;
        world.player_bot.focus = Some(id);
        world.player_bot.bindings.push(PlayerBotBinding {
            key: 82,
            memory_address: 300,
            value: 50,
            invert: false,
        });
        world.player_bot.pressed_keys.push(82);
        world.tick().unwrap();
        assert_eq!(world.stats.births, 1);
        assert_eq!(world.stats.self_reproductions, 0);
    }
}
