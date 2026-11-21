use super::*;
use crate::physics::organism_radius_for_profile;

impl Engine {
    // Robots.bas: Shooting, ManageChlr, ManageBody, then ManageBouyancy.
    pub(super) fn body_commands_phase(&mut self) {
        let runtime = SysvarRuntime::new();
        let native = self.native_random.is_some();
        for slot_index in self.phase_slot_range() {
            let slot = &mut self.slots[slot_index];
            let Some(robot) = slot.organism.as_mut() else {
                continue;
            };
            if robot.flags.corpse || !robot.flags.can_execute_dna {
                continue;
            }
            let stored = robot.program.memory.read(313);
            if stored > 0 {
                let stored = stored.min(100);
                robot.materials.energy -= stored as f32;
                robot.materials.body =
                    (f64::from(robot.materials.body) + f64::from(stored) / 10.0) as f32;
                robot.materials.body = robot.materials.body.min(32_000.0);
                runtime.finish_command(313, CommandOutcome::Consumed, &mut robot.program.memory);
            }
            let fed = robot.program.memory.read(312);
            if fed > 0 {
                let fed = fed.min(100);
                robot.materials.energy = (robot.materials.energy + fed as f32).min(32_000.0);
                robot.materials.body =
                    (f64::from(robot.materials.body) - f64::from(fed) / 10.0) as f32;
                runtime.finish_command(312, CommandOutcome::Consumed, &mut robot.program.memory);
            }
            robot.materials.body = robot.materials.body.clamp(0.0, 32_000.0);
            if stored > 0 || fed > 0 {
                robot.materials.radius = organism_radius_for_profile(
                    robot.materials.body,
                    robot.materials.chloroplasts,
                    self.config.fixed_bot_radii,
                    native,
                );
            }
            robot
                .materials
                .publish_for_profile(&mut robot.program.memory, native);
        }
    }

    pub(super) fn chloroplast_phase(&mut self) {
        let runtime = SysvarRuntime::new();
        let native = self.native_random.is_some();
        let native_light = native.then(|| {
            (32_000.0 - self.native_light_availability() * 32_000.0).round_ties_even() as i32
        });
        let costs = self.active_dna_costs();
        let multiplier = costs.dynamic_multiplier(self.cost_population());
        let total_chloroplasts = self.phase_chloroplast_total();
        let pressure_refusal = (total_chloroplasts / 16_000.0).round_ties_even()
            > self.config.vegetable_population_cap as f64;
        for slot_index in self.phase_slot_range() {
            let slot = &mut self.slots[slot_index];
            let Some(robot) = slot.organism.as_mut() else {
                continue;
            };
            if robot.flags.corpse || !robot.flags.can_execute_dna || robot.flags.no_chloroplasts {
                continue;
            }
            let added = robot.program.memory.read(921);
            let removed = robot.program.memory.read(922);
            if added > 0 || removed > 0 {
                let prior = robot.materials.chloroplasts;
                // Each native assignment stores a Single; charge only a net increase.
                robot.materials.chloroplasts += added as f32;
                robot.materials.chloroplasts -= removed as f32;
                if robot.materials.chloroplasts > prior {
                    let energy = (f64::from(robot.materials.energy)
                        - f64::from(robot.materials.chloroplasts - prior)
                            * costs.chloroplast_construction
                            * multiplier) as f32;
                    if (pressure_refusal && robot.flags.vegetable) || energy < 100.0 {
                        robot.materials.chloroplasts = prior;
                    } else {
                        robot.materials.energy = energy;
                    }
                }
                runtime.finish_command(921, CommandOutcome::Consumed, &mut robot.program.memory);
                runtime.finish_command(922, CommandOutcome::Consumed, &mut robot.program.memory);
            }
            if self.native_random.is_none()
                && robot.materials.chloroplasts > 0.0
                && robot.counters.chloroplast_share_delay > 0
            {
                robot.counters.chloroplast_share_delay -= 1;
            }
            if let Some(light) = native_light {
                native_sensing::publish_integer(&mut robot.program.memory, MEM_LIGHT as u16, light);
            }
            robot.materials.decay_chloroplasts();
            robot.materials.radius = organism_radius_for_profile(
                robot.materials.body,
                robot.materials.chloroplasts,
                self.config.fixed_bot_radii,
                native,
            );
            robot
                .materials
                .publish_for_profile(&mut robot.program.memory, native);
        }
    }

    pub(super) fn buoyancy_commands_phase(&mut self) {
        let runtime = SysvarRuntime::new();
        for slot_index in self.phase_slot_range() {
            let slot = &mut self.slots[slot_index];
            let Some(robot) = slot.organism.as_mut() else {
                continue;
            };
            if robot.flags.corpse || !robot.flags.can_execute_dna {
                continue;
            }
            let command = robot.program.memory.read(MEM_SET_BUOYANCY);
            if command != 0 {
                robot.buoyancy = (f64::from(robot.buoyancy) + f64::from(command) / 32_000.0)
                    .clamp(0.0, 1.0) as f32;
                robot.program.memory.write(
                    MEM_READ_BUOYANCY,
                    (f64::from(robot.buoyancy) * 32_000.0).round_ties_even() as i32,
                );
                runtime.finish_command(
                    MEM_SET_BUOYANCY as u16,
                    CommandOutcome::Consumed,
                    &mut robot.program.memory,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(dna: &str) -> (Engine, OrganismId) {
        let mut engine = Engine::new(EngineConfig {
            metabolism_cost: 0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = engine
            .spawn_at(LegacyDna::parse(dna).unwrap(), [500.0, 500.0])
            .unwrap();
        (engine, id)
    }

    #[test]
    fn native_venom_memory_tracks_the_original_writers() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("stop").unwrap();
        let id = world.spawn_at(dna.clone(), [500.0; 2]).unwrap();
        let robot = world.slots.get_mut(id).unwrap();
        robot.materials.venom = 0.75;
        robot.program.memory.write(824, 1);

        world.tick().unwrap();
        let robot = world.slots.get(id).unwrap();
        assert_eq!(robot.materials.venom, 1.75);
        assert_eq!(robot.program.memory.read(825), 1);
        assert_eq!(robot.program.memory.read(824), 0);

        world.tick().unwrap();
        assert_eq!(world.memory_at(id, 825).unwrap(), 1);

        let robot = world.slots.get_mut(id).unwrap();
        robot.program.memory.write(824, 1);
        robot.program.memory.write(7, -3);
        // The default venom shot runs after storevenom and rounds its readback.
        world.tick().unwrap();
        let after_shot = 2.75_f32 - 2.75_f32 / 20.0;
        assert_eq!(world.slots.get(id).unwrap().materials.venom, after_shot);
        assert_eq!(world.memory_at(id, 825).unwrap(), 3);
        assert_eq!(world.projectiles.kinds[0], -3);

        world.projectiles.reset_native_slots(100);
        let thief = world.spawn_at(dna, [600.0, 500.0]).unwrap();
        assert!(world.native_make_tie(thief, id, 100, 0, 7));
        let robot = world.slots.get_mut(thief).unwrap();
        robot.program.memory.write(455, 7);
        robot.program.memory.write(452, -3);
        robot.program.memory.write(453, -2);

        world.tick().unwrap();
        let target = world.slots.get(id).unwrap();
        assert_eq!(target.materials.venom, after_shot - 2.0);
        assert_eq!(target.program.memory.read(825), 3);
        let thief = world.slots.get(thief).unwrap();
        assert_eq!(thief.materials.venom, 2.0);
        assert_eq!(thief.program.memory.read(825), 2);

        let mut portable_memory = VmMemory::default();
        target.materials.publish(&mut portable_memory);
        assert_eq!(portable_memory.read(825), 1);
    }

    #[test]
    fn shot_uses_body_and_energy_before_body_commands() {
        let (mut control, id) = world("start\n-1 .shoot store\nstop");
        let (mut store, store_id) = world("start\n100 .strbody store\n-1 .shoot store\nstop");
        control.tick().unwrap();
        store.tick().unwrap();
        assert_eq!(
            control.snapshot().shots[0].value,
            store.snapshot().shots[0].value
        );
        assert_eq!(
            control.snapshot().shots[0].range,
            store.snapshot().shots[0].range
        );
        assert_eq!(store.slots.get(store_id).unwrap().materials.body, 1010.0);
        assert_eq!(
            store.slots.get(store_id).unwrap().materials.energy,
            control.slots.get(id).unwrap().materials.energy - 100.0
        );
    }

    #[test]
    fn chloroplast_and_body_phases_round_radius_for_the_active_profile() {
        let body = f32::from_bits(0x4517_efef);
        let chloroplasts = f32::from_bits(0x46e4_b4a6);
        for (native, expected_radius) in [(false, 0x43c4_a3a0), (true, 0x43c4_a39f)] {
            let config = EngineConfig::testing();
            let mut engine = if native {
                Engine::new_native(config)
            } else {
                Engine::new(config)
            }
            .unwrap();
            if !native {
                engine.native_random = None;
            }
            let id = engine
                .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
                .unwrap();
            let robot = engine.slots.get_mut(id).unwrap();
            robot.materials.body = body;
            robot.materials.chloroplasts = chloroplasts;

            engine.chloroplast_phase();
            let robot = engine.slots.get_mut(id).unwrap();
            assert_eq!(
                robot.materials.chloroplasts.to_bits(),
                chloroplasts.to_bits()
            );
            assert_eq!(robot.materials.radius.to_bits(), expected_radius);

            robot.materials.body = body - 1.0;
            robot.materials.radius = 1.0;
            robot.program.memory.write(313, 10);
            engine.body_commands_phase();
            let robot = engine.slots.get(id).unwrap();
            assert_eq!(robot.materials.body.to_bits(), body.to_bits());
            assert_eq!(robot.materials.radius.to_bits(), expected_radius);
        }
    }

    #[test]
    fn native_shock_caps_body_and_refreshes_radius_without_resampling_mass() {
        let body = f32::from_bits(0x4517_efef);
        let chloroplasts = f32::from_bits(0x46e4_b4a6);
        let old_radius = 50.0_f32;
        for (native, starting_body, expected_body, expected_radius) in [
            (true, body, body, 0x43c4_a39f),
            (true, 33_000.0, 32_000.0, 0x43cf_852e),
            (false, body, body, old_radius.to_bits()),
        ] {
            let mut engine = Engine::new_native(EngineConfig::testing()).unwrap();
            if !native {
                engine.native_random = None;
            }
            let id = engine
                .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
                .unwrap();
            let robot = engine.slots.get_mut(id).unwrap();
            robot.materials.energy = 4000.0;
            robot.materials.previous_energy = 10000.0;
            robot.materials.body = starting_body;
            robot.materials.chloroplasts = chloroplasts;
            robot.materials.radius = old_radius;
            robot.materials.mass = 90.0;
            robot.kinematics.native_added_mass = Some(200.0);

            engine.shock_phase();

            let robot = engine.slots.get(id).unwrap();
            assert_eq!(robot.materials.energy, 0.0);
            assert_eq!(robot.materials.previous_energy, 10000.0);
            assert_eq!(robot.materials.body, expected_body);
            assert_eq!(robot.materials.radius.to_bits(), expected_radius);
            assert_eq!(robot.materials.mass, 90.0);
            assert_eq!(robot.kinematics.native_added_mass, Some(200.0));
        }
    }

    #[test]
    fn chloroplast_request_is_atomic_and_net_growth_is_charged_after_shots() {
        let (mut engine, id) = world("start\nstop");
        engine.config.dna_costs.chloroplast_construction = 2.0;
        let robot = engine.slots.get_mut(id).unwrap();
        robot.materials.energy = 150.0;
        robot.program.memory.write(921, 100);
        engine.chloroplast_phase();
        assert_eq!(engine.slots.get(id).unwrap().materials.chloroplasts, 0.0);
        assert_eq!(engine.slots.get(id).unwrap().materials.energy, 150.0);
        let robot = engine.slots.get_mut(id).unwrap();
        robot.program.memory.write(921, 100);
        robot.program.memory.write(922, 90);
        engine.chloroplast_phase();
        let robot = engine.slots.get(id).unwrap();
        assert_eq!(robot.materials.energy, 130.0);
        assert!(robot.materials.chloroplasts > 9.0 && robot.materials.chloroplasts < 10.0);
        assert_eq!(robot.program.memory.read(921), 0);
        assert_eq!(robot.program.memory.read(922), 0);
    }

    #[test]
    fn chloroplast_construction_and_body_feeding_cannot_fund_an_earlier_shot() {
        let (mut control, _) = world("start\n-2 .shoot store\n20 .shootval store\nstop");
        let (mut changed, id) = world(
            "start\n100 .fdbody store\n100 .mkchlr store\n-2 .shoot store\n20 .shootval store\nstop",
        );
        control.config.dna_costs.chloroplast_construction = 10.0;
        changed.config.dna_costs.chloroplast_construction = 10.0;
        control.tick().unwrap();
        changed.tick().unwrap();
        assert_eq!(
            control.snapshot().shots[0].value,
            changed.snapshot().shots[0].value
        );
        let robot = changed.slots.get(id).unwrap();
        assert_eq!(
            robot.materials.chloroplasts, 0.0,
            "unaffordable growth is refused before body feeding"
        );
        assert_eq!(robot.materials.body, 990.0);
        assert_eq!(robot.materials.energy, 1080.0);
        assert_eq!(robot.materials.pain, -80);
        assert_eq!(robot.materials.pleasure, 80);
    }

    #[test]
    fn upkeep_rounds_each_fee_and_excludes_corpse_stores() {
        let (mut engine, id) = world("start\nstop");
        let dna_len = engine.slots.get(id).unwrap().program.dna.blocks().len();
        engine.config.dna_costs.age_base = 0.00002;
        engine.config.dna_costs.body_upkeep = 0.00000002;
        engine.config.dna_costs.dna_upkeep = 0.00002 / (dna_len - 1) as f64;
        engine.slots.get_mut(id).unwrap().counters.age = 1;
        engine.upkeep_phase();
        assert_eq!(
            engine.slots.get(id).unwrap().materials.energy.to_bits(),
            1000.0_f32.to_bits()
        );
        let robot = engine.slots.get_mut(id).unwrap();
        robot.flags.corpse = true;
        robot.materials.slime = 100.0;
        robot.materials.poison = 100.0;
        robot.effects.paralysis_active = true;
        robot.effects.paralysis_countdown = 7.0;
        engine.config.metabolism_cost = 10;
        engine.upkeep_phase();
        engine.poison_venom_phase();
        let robot = engine.slots.get(id).unwrap();
        assert_eq!(robot.materials.energy, 1000.0);
        assert_eq!(robot.materials.slime, 100.0);
        assert_eq!(robot.materials.poison, 100.0);
        assert_eq!(robot.effects.paralysis_countdown, 7.0);
    }

    #[test]
    fn native_age_cost_rounds_start_and_log_mode_overrides_linear() {
        let costs = DnaCosts {
            age_base: 4.0,
            age_start: 2.5,
            age_linear: 100.0,
            age_logarithmic: 1.0,
            ..DnaCosts::default()
        };
        assert_eq!(costs.age_cost(0, 0), 0.0);
        assert_eq!(costs.age_cost(2, 0), 0.0);
        assert_eq!(costs.age_cost(3, 0), 0.0, "log(1) is zero");
        assert_eq!(costs.age_cost(4, 0), 2.772588722239781);
        let linear = DnaCosts {
            age_logarithmic: 0.0,
            ..costs
        };
        assert_eq!(linear.age_cost(3, 0), 104.0);
    }

    #[test]
    fn native_linear_age_cost_flag_roundtrips_independently_of_its_rate() {
        use crate::native_binary::{NativeRecordFile, NativeSimulationTail, NativeValue};

        let mut seed = Engine::new_native(EngineConfig::testing()).unwrap();
        let id = seed
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2])
            .unwrap();
        seed.slots.get_mut(id).unwrap().counters.age = 4;
        let mut original = NativeRecordFile::decode(&seed.export_native_sim().unwrap()).unwrap();
        let mut tail = NativeSimulationTail::decode(&original.tail).unwrap();
        let mut costs = tail.bytes("costs").unwrap().to_vec();

        for (slot, value) in [(31, 4.0_f32), (32, 2.0), (33, 0.25), (54, 1.0)] {
            costs[slot * 4..slot * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        // Robots.Upkeep uses the separate slot 60 switch, even when slot 33
        // contains a nonzero coefficient. Compare actual Single energy.
        for (enabled, expected_energy) in [(false, 996.0), (true, 995.5)] {
            let flag = if enabled { 1.0_f32 } else { 0.0 };
            costs[60 * 4..61 * 4].copy_from_slice(&flag.to_le_bytes());
            tail.set_value("costs", NativeValue::Bytes(costs.clone()))
                .unwrap();
            original.tail = tail.encode().unwrap();
            let imported = Engine::from_native_sim(&original.encode().unwrap()).unwrap();
            let modern = crate::SaveFile::encode(&imported).unwrap();
            let mut resumed = crate::SaveFile::decode(&modern).unwrap();
            assert_eq!(resumed.config.dna_costs.age_linear, 0.25);
            assert_eq!(resumed.config.dna_costs.age_linear_enabled, Some(enabled));

            let native = NativeRecordFile::decode(&resumed.export_native_sim().unwrap()).unwrap();
            let saved_tail = NativeSimulationTail::decode(&native.tail).unwrap();
            let saved_costs = saved_tail.bytes("costs").unwrap();
            assert_eq!(&saved_costs[33 * 4..34 * 4], &0.25_f32.to_le_bytes());
            assert_eq!(&saved_costs[60 * 4..61 * 4], &flag.to_le_bytes());

            let robot = resumed.slots.ids_in_slot_order()[0];
            assert_eq!(resumed.slots.get(robot).unwrap().materials.energy, 1_000.0);
            resumed.upkeep_phase();
            assert_eq!(
                resumed.slots.get(robot).unwrap().materials.energy,
                expected_energy
            );
        }
    }

    #[test]
    fn native_corpse_death_uses_fifteen_after_senses_including_older_zero_defaults() {
        for threshold in [0, 15] {
            for (energy, expected_corpses) in [(0.0, 1), (14.5, 1), (15.0, 0)] {
                let (mut engine, id) = world("start\nstop");
                engine.config.corpse_decay.enabled = true;
                engine.config.corpse_decay.death_energy_threshold = threshold;
                engine.slots.get_mut(id).unwrap().materials.energy = energy;
                if threshold == 0 && energy == 14.5 {
                    engine = crate::SaveFile::decode(&crate::SaveFile::encode(&engine).unwrap())
                        .unwrap();
                    assert_eq!(engine.config.corpse_decay.death_energy_threshold, 0);
                }
                engine.tick().unwrap();
                assert_eq!(engine.snapshot().corpses.len(), expected_corpses);
                assert_eq!(engine.population(), 1 - expected_corpses);
                engine.validate_invariants().unwrap();
            }
        }
    }

    #[test]
    fn excluded_commands_remain_pending_and_negative_body_commands_are_ignored() {
        let (mut engine, id) = world("start\nstop");
        let robot = engine.slots.get_mut(id).unwrap();
        robot.flags.no_chloroplasts = true;
        robot.program.memory.write(921, 100);
        robot.program.memory.write(313, -10);
        robot.program.memory.write(314, 16000);
        engine.chloroplast_phase();
        engine.body_commands_phase();
        engine.buoyancy_commands_phase();
        let robot = engine.slots.get(id).unwrap();
        assert_eq!(robot.program.memory.read(921), 100);
        assert_eq!(robot.program.memory.read(313), -10);
        assert_eq!(robot.buoyancy, 0.5);
        assert_eq!(robot.program.memory.read(315), 16000);
        let robot = engine.slots.get_mut(id).unwrap();
        robot.flags.can_execute_dna = false;
        robot.program.memory.write(300, 50);
        let mut trace = CycleRecorder::disabled(0);
        assert!(
            engine
                .prepare_lifecycle_queues(&mut trace)
                .births
                .is_empty()
        );
        assert_eq!(engine.slots.get(id).unwrap().program.memory.read(300), 50);
    }
}

#[cfg(test)]
mod native_light_publication55_tests {
    use super::*;

    #[test]
    fn manage_chloroplasts_publishes_cached_double_light_only_for_eligible_actors() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        for (availability, expected) in [(0.0, 32_000), (0.125, 28_000), (1.0, 0)] {
            world.set_native_light_availability(availability);
            world.chloroplast_phase();
            assert_eq!(
                world.slots.get(id).unwrap().program.memory.read(923),
                expected
            );
        }
        for skip in 0..3 {
            let robot = world.slots.get_mut(id).unwrap();
            robot.flags.corpse = skip == 0;
            robot.flags.can_execute_dna = skip != 1;
            robot.flags.no_chloroplasts = skip == 2;
            robot.program.memory.write(923, 12_345);
            world.chloroplast_phase();
            assert_eq!(
                world.slots.get(id).unwrap().program.memory.read(923),
                12_345
            );
            world.vegetation_feed_phase();
            assert_eq!(
                world.slots.get(id).unwrap().program.memory.read(923),
                12_345
            );
        }
    }
}
