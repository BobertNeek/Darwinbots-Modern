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

// Vegs.feedveg2 stores its meal in Single variables, then consumes waste in
// the randomly chosen order. Each successful channel has its own strict cap
// and can exhaust the waste before the other channel runs.
fn native_waste_meal(
    materials: &mut crate::biology::RobotMaterials,
    feeding_to_body: f32,
    energy_first: bool,
) -> (f32, f32) {
    let chloroplasts = f64::from(materials.chloroplasts);
    // Classic VB promotes Single/Long64000 to Double. In contrast, Integer1
    // minus Single, and Single/Integer32000, retain Single intermediates.
    let energy_fraction = 1.0_f32 - feeding_to_body;
    let energy = (chloroplasts / 64_000.0 * f64::from(energy_fraction)) as f32;
    let body = (chloroplasts / 64_000.0 * f64::from(feeding_to_body) / 10.0) as f32;
    let waste_rate = materials.chloroplasts / 32_000.0_f32;
    let prior = (materials.energy, materials.body);
    for to_energy in [energy_first, !energy_first] {
        if materials.waste > 0.0 {
            if to_energy {
                if materials.energy + energy < 32_000.0 {
                    materials.energy += energy;
                    materials.waste -= waste_rate * energy_fraction;
                }
            } else if materials.body + body < 32_000.0 {
                materials.body += body;
                materials.waste -= waste_rate * feeding_to_body;
            }
            materials.waste = materials.waste.max(0.0);
        }
    }
    (materials.energy - prior.0, materials.body - prior.1)
}

impl Engine {
    pub(super) fn waste_phase(&mut self) -> Result<(), EngineError> {
        let settings = self.config.waste;
        let mut packets = Vec::new();
        for id in self.phase_robot_ids() {
            let robot = self.slots.get_mut(id).unwrap();
            if robot.flags.corpse || !robot.flags.can_execute_dna {
                continue;
            }
            robot.materials.waste = robot.materials.waste.max(0.0);
            robot.materials.permanent_waste = robot.materials.permanent_waste.max(0.0);
            if self.native_random.is_some()
                && robot.materials.waste > 0.0
                && robot.materials.chloroplasts > 0.0
            {
                // Robots.HandleWaste calls feedveg2 before Alzheimer, shots,
                // chloroplast decay, body commands and queued reproduction.
                // Darkness and NoChlr do not suppress this one Rndy draw.
                let energy_first =
                    world_random_unit(&mut self.native_random, &mut robot.program.rng_state) < 0.5;
                let (energy, body) = native_waste_meal(
                    &mut robot.materials,
                    self.config.vegetation.feeding_to_body,
                    energy_first,
                );
                self.stats.plant_energy_generated = self
                    .stats
                    .plant_energy_generated
                    .saturating_add(energy.max(0.0) as u64);
                self.stats.plant_body_produced = self
                    .stats
                    .plant_body_produced
                    .saturating_add(body.max(0.0) as u64);
            }
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
            robot
                .materials
                .publish_for_profile(&mut robot.program.memory, self.native_random.is_some());
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
            self.projectiles.try_spawn(
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
            )?;
            robot
                .materials
                .publish_for_profile(&mut robot.program.memory, self.native_random.is_some());
            self.stats.shots_fired = self.stats.shots_fired.saturating_add(1);
        }
        Ok(())
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
        world.waste_phase().unwrap();
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
        world.waste_phase().unwrap();
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
    #[test]
    fn native_waste_meal_uses_source_order_exhaustion_and_strict_store_caps() {
        for (energy_first, energy, body) in [(true, 100.25, 10.0), (false, 100.0, 10.025)] {
            let mut materials = crate::biology::RobotMaterials {
                energy: 100.0,
                body: 10.0,
                waste: 0.125,
                chloroplasts: 32_000.0,
                ..Default::default()
            };
            native_waste_meal(&mut materials, 0.5, energy_first);
            // Either channel's half-unit waste cost exhausts the initial waste;
            // the other channel must not receive its otherwise valid meal.
            assert_eq!(
                (materials.energy, materials.body, materials.waste),
                (energy, body, 0.0)
            );
        }
        for energy_first in [true, false] {
            for energy in [31_999.5, 31_999.75] {
                let mut materials = crate::biology::RobotMaterials {
                    energy,
                    body: 32_000.0,
                    waste: 4.0,
                    chloroplasts: 32_000.0,
                    ..Default::default()
                };
                let before = materials.clone();
                native_waste_meal(&mut materials, 0.0, energy_first);
                // A meal reaching or exceeding32000 is rejected, as is body
                // already at the cap. Rejected stores consume no waste.
                assert_eq!(materials, before);
                materials.energy = 31_999.0;
                native_waste_meal(&mut materials, 0.0, energy_first);
                assert_eq!(
                    (materials.energy, materials.body, materials.waste),
                    (31_999.5, 32_000.0, 3.0)
                );
            }
        }

        // Literal bit patterns derived from the classic VB operand widths,
        // not an original-runtime observation. Fraction .4 distinguishes the
        // Single complement; .1 body-first distinguishes Single waste costs.
        for (fraction, energy_first, expected) in [
            (0.1, true, [0x3e31_c71b, 0x3afc_d6e8, 0x3f1d_3c0d]),
            (0.1, false, [0x3e31_c71b, 0x3afc_d6e8, 0x3f1d_3c0e]),
            (0.4, true, [0x3ded_097a, 0x3bfc_d6e8, 0x3f1d_3c0e]),
            (0.4, false, [0x3ded_097a, 0x3bfc_d6e8, 0x3f1d_3c0e]),
        ] {
            let mut materials = crate::biology::RobotMaterials {
                energy: 0.0,
                body: 0.0,
                waste: 1.0,
                chloroplasts: 12_345.678,
                ..Default::default()
            };
            native_waste_meal(&mut materials, fraction, energy_first);
            assert_eq!(
                [
                    materials.energy.to_bits(),
                    materials.body.to_bits(),
                    materials.waste.to_bits()
                ],
                expected
            );
        }

        for seed in [1, 48_032] {
            let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
            world.config.vegetation.feeding_to_body = 0.5;
            world.config.waste.alzheimer_threshold = 0;
            world.config.waste.defecation_threshold = i32::MAX;
            let id = world
                .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
                .unwrap();
            world.native_random = Some(crate::native_random::NativeRandom::with_user_seed(seed));
            let mut expected_random = world.native_random.clone().unwrap();
            let energy_first = expected_random.unit() < 0.5;
            let robot = world.slots.get_mut(id).unwrap();
            robot.materials.energy = 100.0;
            robot.materials.body = 10.0;
            robot.materials.waste = 0.125;
            robot.materials.chloroplasts = 32_000.0;
            world.waste_phase().unwrap();
            let after = &world.slots.get(id).unwrap().materials;
            let expected = if energy_first {
                (100.25, 10.0, 0.0)
            } else {
                (100.0, 10.025, 0.0)
            };
            assert_eq!((after.energy, after.body, after.waste), expected);
            assert_eq!(world.native_random.as_ref(), Some(&expected_random));
        }
    }

    #[test]
    fn native_handle_waste_feeds_in_darkness_before_birth_and_not_again_in_feedveg() {
        let mut config = EngineConfig::testing();
        config.metabolism_cost = 0;
        config.vegetable_energy_per_tick = 0;
        config.vegetation.day_night_enabled = false;
        config.vegetation.daytime = false;
        config.waste.alzheimer_threshold = 0;
        config.waste.defecation_threshold = i32::MAX;
        let mut world = Engine::new_native(config).unwrap();
        world.mutations_enabled = false;
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        let robot = world.slots.get_mut(id).unwrap();
        robot.materials.energy = 4_000.0;
        robot.materials.body = 400.0;
        robot.materials.waste = 40.0;
        robot.materials.chloroplasts = 200.0;
        robot.materials.radius = crate::physics::organism_radius(400.0, 200.0, false);
        robot.flags.no_chloroplasts = true;
        let original = crate::SaveFile::encode(&world).unwrap();
        let mut expected_random = world.native_random.clone().unwrap();
        expected_random.unit();
        world.waste_phase().unwrap();
        // Original feedveg2: E=CSng(200/64000*.25), B=CSng(200/64000*.75/10).
        // Each successful channel subtracts its own Single waste expression
        // into the Single store. Both possible orders agree for this fixture.
        let converted = world.slots.get(id).unwrap().materials.clone();
        assert_eq!(converted.energy, 4_000.000_7);
        assert_eq!(converted.body, 400.000_24);
        assert_eq!(converted.waste, 39.993_748);
        assert_eq!(world.native_random.as_ref(), Some(&expected_random));
        assert_eq!(
            world.slots.get(id).unwrap().program.rng_state,
            u64::from(expected_random.state())
        );
        assert_eq!(world.slots.get(id).unwrap().program.memory.read(828), 40);
        world.vegetation_feed_phase();
        assert_eq!(world.slots.get(id).unwrap().materials, converted);
        assert_eq!(world.native_random.as_ref(), Some(&expected_random));

        let mut birth = crate::SaveFile::decode(&original).unwrap();
        birth
            .slots
            .get_mut(id)
            .unwrap()
            .program
            .memory
            .write(300, 25);
        birth.tick().unwrap();
        let child = birth
            .snapshot()
            .organisms
            .iter()
            .find(|robot| robot.parents == [Some(id), None])
            .unwrap()
            .id;
        let parent = birth.slots.get(id).unwrap();
        let child = birth.slots.get(child).unwrap();
        // Reproduce splits the already converted stores, then removes0.1%
        // of the transferred energy. Late FeedVeg cannot feed either actor.
        assert_eq!(parent.materials.energy, 2_999.000_5);
        assert_eq!(parent.materials.body, 300.000_24);
        assert_eq!(parent.materials.waste, 29.995_312);
        assert_eq!(child.materials.energy, 999.000_2);
        assert_eq!(child.materials.body, 100.0);
        assert_eq!(child.materials.waste, 9.998_437);
        let saved = crate::SaveFile::encode(&birth).unwrap();
        let restored = crate::SaveFile::decode(&saved).unwrap();
        assert_eq!(crate::SaveFile::encode(&restored).unwrap(), saved);

        let mut portable = crate::SaveFile::decode(&original).unwrap();
        portable.native_random = None;
        let before = portable.slots.get(id).unwrap().materials.clone();
        portable.waste_phase().unwrap();
        assert_eq!(portable.slots.get(id).unwrap().materials, before);
        portable.vegetation_feed_phase();
        let after = &portable.slots.get(id).unwrap().materials;
        assert_eq!(after.energy, before.energy + 200.0_f32 / 64_000.0 * 0.25);
        assert_eq!(after.body, before.body + 200.0_f32 / 64_000.0 * 0.75 / 10.0);
        assert_eq!(after.waste, before.waste - 200.0_f32 / 32_000.0);
    }
}
