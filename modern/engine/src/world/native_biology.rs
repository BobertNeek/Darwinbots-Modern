use super::*;

impl Engine {
    // Robots.bas: Shooting, ManageChlr, ManageBody, then ManageBouyancy.
    pub(super) fn body_commands_phase(&mut self) {
        let runtime = SysvarRuntime::new();
        for slot in self.slots.iter_mut() {
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
                robot.materials.radius = crate::physics::organism_radius(
                    robot.materials.body,
                    robot.materials.chloroplasts,
                    self.config.fixed_bot_radii,
                );
            }
            robot.materials.publish(&mut robot.program.memory);
        }
    }

    pub(super) fn chloroplast_phase(&mut self) {
        let runtime = SysvarRuntime::new();
        let costs = self.active_dna_costs();
        let multiplier = costs.dynamic_multiplier(self.cost_population());
        let total_chloroplasts: f64 = self
            .slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .map(|robot| f64::from(robot.materials.chloroplasts))
            .sum();
        let pressure_refusal = (total_chloroplasts / 16_000.0).round_ties_even()
            > self.config.vegetable_population_cap as f64;
        for slot in self.slots.iter_mut() {
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
            if robot.materials.chloroplasts > 0.0 && robot.counters.chloroplast_share_delay > 0 {
                robot.counters.chloroplast_share_delay -= 1;
            }
            robot.materials.decay_chloroplasts();
            robot.materials.radius = crate::physics::organism_radius(
                robot.materials.body,
                robot.materials.chloroplasts,
                self.config.fixed_bot_radii,
            );
            robot.materials.publish(&mut robot.program.memory);
        }
    }

    pub(super) fn buoyancy_commands_phase(&mut self) {
        let runtime = SysvarRuntime::new();
        for slot in self.slots.iter_mut() {
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
