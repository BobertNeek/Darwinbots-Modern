//! startloaded's totvegs=-1, then UpdateBots' first displayed census.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum NativeLoadedCensusStage {
    Pending,
    FirstCycle,
}

impl Engine {
    pub(crate) fn reproduce_native_robot_info(
        &mut self,
        parent: OrganismId,
    ) -> Result<Option<OrganismId>, EngineError> {
        if self.native_cycle.is_none() || !self.has_native_random() {
            return Err(EngineError::Invariant(
                "Robot Info reproduction requires original simulation mode".into(),
            ));
        }
        let robot = self.slots.get(parent)?;
        let child =
            if robot.flags.exists && (self.asexual_reproduction_allowed || robot.flags.vegetable) {
                let birth = PendingBirth {
                    parent,
                    percentage: 50,
                    asexual_command: 0,
                    mutated_command: 0,
                    mutation_reproduction: robot.program.memory.read(301) > 0,
                    assisted: true,
                    sexual: false,
                    sperm_source: None,
                };
                let mut trace = CycleRecorder::disabled(self.tick);
                self.commit_pending_births(&[birth], &mut trace)?
            } else {
                None
            };
        // robdata.repro_Click calls Form1.Redraw even when the attempt produced no child.
        self.native_redraw();
        Ok(child)
    }

    pub(super) fn mark_native_loaded_census(&mut self) {
        self.native_loaded_census_stage = self
            .native_random
            .as_ref()
            .map(|_| NativeLoadedCensusStage::Pending);
    }

    pub(super) fn advance_native_loaded_census(&mut self) {
        // UpdateBots copies totvegs into totvegsDisplayed before recounting.
        // Keep FirstCycle through the saved C1 boundary; C2 replaces it.
        self.native_loaded_census_stage = match self.native_loaded_census_stage {
            Some(NativeLoadedCensusStage::Pending) => Some(NativeLoadedCensusStage::FirstCycle),
            Some(NativeLoadedCensusStage::FirstCycle) | None => None,
        };
    }

    pub(super) fn native_loaded_census_blocks_births(&self) -> bool {
        self.native_random.is_some()
            && self.native_loaded_census_stage == Some(NativeLoadedCensusStage::FirstCycle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_binary::{NativeRecordFile, NativeSimulationTail, NativeValue};
    const REPRODUCTION_START: &[u8] =
        include_bytes!("../../tests/fixtures/db24832-reproduction-start-48032.sim");

    fn robot_info_test_world() -> Engine {
        let mut world = Engine::new_native(EngineConfig {
            world_width: 8_000.0,
            world_height: 6_000.0,
            fixed_bot_radii: true,
            ..EngineConfig::testing()
        })
        .unwrap();
        world.mutations_enabled = false;
        world
    }

    #[test]
    fn robot_info_reproduces_at_heading_and_redraws_without_advancing_other_lifecycle_work() {
        let mut world = robot_info_test_world();
        world.tick = 12;
        world.native_cycle = Some(11);
        world.ties_enabled = false;
        world.config.dna_costs.tie_formation = 10.0;
        world.config.dna_costs.dna_copy = 2.0;
        let dna = LegacyDna::parse("start stop").unwrap();
        let copy_fee = dna.blocks().len() as f32 * 2.0;
        let parent = world.spawn_at(dna.clone(), [1_000.0; 2]).unwrap();
        let unrelated = world
            .spawn_at(dna.clone(), [5_000.0, f32::from_bits(0x4412_3a39)])
            .unwrap();
        let missing = world.register_species(SpeciesDefinition {
            name: "Reseed witness".into(),
            reseed: true,
            minimum_population: 1,
            ..SpeciesDefinition::default()
        });
        world.species_templates[missing.0 as usize] = Some(SpeciesTemplate {
            retired: false,
            comment: String::new(),
            dna,
            source_text: None,
            initial_energy: 1_000,
            initial_population: 1,
        });
        world.set_organism_aim(parent, 0).unwrap();
        let state = world.slots.get_mut(parent).unwrap();
        state.counters.age = 4;
        state.kinematics.velocity = [12.25, -7.5];
        state.kinematics.actual_velocity = state.kinematics.velocity;
        state.program.memory.write(MEM_REPRO, 12);
        state.program.memory.write(12, 123);
        let witness = world.slots.get_mut(unrelated).unwrap();
        witness.flags.dead = true;
        witness.counters.age = 8;
        witness.kinematics.velocity[1] = f32::from_bits(0x4225_ab20);
        witness.kinematics.actual_velocity[1] = f32::from_bits(0x41b7_4190);

        let child = world.reproduce_native_robot_info(parent).unwrap().unwrap();

        assert_eq!(world.tick, 12);
        assert_eq!(world.snapshot.native_cycle, Some(11));
        assert_eq!(world.stats.births, 1);
        assert_eq!(world.stats.deaths, 0);
        assert_eq!(world.stats.reseeds, 0);
        assert_eq!(world.stats.self_reproductions, 0);
        assert_eq!(world.slots.ids_in_slot_order().len(), 3);
        let witness = world.slots.get(unrelated).unwrap();
        assert!(witness.flags.dead);
        assert_eq!(witness.counters.age, 8);
        assert_eq!(witness.kinematics.position[1].to_bits(), 0x4412_3a3a);
        let newborn = world.slots.get(child).unwrap();
        assert_eq!(newborn.identity.parents, [Some(parent), None]);
        assert_eq!(newborn.counters.birth_cycle, 11);
        assert_eq!(newborn.counters.age, 0);
        assert_eq!(newborn.kinematics.position, [1_120.0, 1_000.0]);
        assert_eq!(newborn.kinematics.velocity, [12.25, -7.5]);
        assert_eq!(newborn.kinematics.actual_velocity, [12.25, -7.5]);
        assert_eq!(newborn.heading(), std::f32::consts::PI);
        assert_eq!(newborn.materials.energy, 499.5);
        assert_eq!(newborn.materials.body, 500.0);
        assert_eq!(newborn.program.memory.read(12), 123);
        let source = world.slots.get(parent).unwrap();
        assert_eq!(source.counters.age, 4);
        assert_eq!(source.materials.body, 500.0);
        assert_eq!(source.materials.previous_energy, 494.5);
        assert_eq!(source.materials.energy, 494.5 - copy_fee);
        assert_eq!(source.program.memory.read(MEM_REPRO), 0);
        let tie = &world.ties[0];
        assert_eq!(world.ties.len(), 1);
        assert_eq!((tie.first, tie.second), (parent, child));
        assert_eq!(tie.last, 100);
        assert_eq!((tie.first_port, tie.second_port), (0, 1));
    }

    #[test]
    fn robot_info_blocked_birth_draws_once_then_keeps_parent_and_positioned_command() {
        let mut world = robot_info_test_world();
        let dna = LegacyDna::parse("start stop").unwrap();
        let parent = world.spawn_at(dna.clone(), [1_000.0; 2]).unwrap();
        let witness = world
            .spawn_at(dna, [5_000.0, f32::from_bits(0x4412_3a39)])
            .unwrap();
        world.set_organism_aim(parent, 0).unwrap();
        let kinematics = &mut world.slots.get_mut(witness).unwrap().kinematics;
        kinematics.velocity[1] = f32::from_bits(0x4225_ab20);
        kinematics.actual_velocity[1] = f32::from_bits(0x41b7_4190);
        // An additional actor occupies the actual forward birth position.
        world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [1_120.0, 1_000.0])
            .unwrap();
        let source = world.slots.get(parent).unwrap().materials.clone();
        let mut expected_random = world.native_random.clone().unwrap();
        expected_random.integer(0, 10);

        assert_eq!(world.reproduce_native_robot_info(parent).unwrap(), None);
        assert_eq!(world.native_random.as_ref(), Some(&expected_random));
        assert_eq!(world.tick, 0);
        assert_eq!(world.stats.births, 0);
        assert_eq!(world.slots.get(parent).unwrap().materials, source);
        assert_eq!(
            world.slots.get(witness).unwrap().kinematics.position[1].to_bits(),
            0x4412_3a3a
        );
        assert!(world.ties.is_empty());

        let positioned = world.manual_reproduce(parent, None, [1_020.0; 2]).unwrap();
        assert_eq!(
            world.slots.get(positioned).unwrap().kinematics.position,
            [1_020.0; 2]
        );
        assert!(world.ties.is_empty());
    }

    #[test]
    fn robot_info_honors_native_restriction_but_accepts_positive_single_energy() {
        let mut world = robot_info_test_world();
        let parent = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [1_000.0; 2])
            .unwrap();
        world.asexual_reproduction_allowed = false;
        let initial_random = world.native_random.clone();
        assert_eq!(world.reproduce_native_robot_info(parent).unwrap(), None);
        assert_eq!(world.native_random, initial_random);

        world.asexual_reproduction_allowed = true;
        let state = world.slots.get_mut(parent).unwrap();
        state.flags.can_execute_dna = false;
        state.materials.energy = 1.5;
        assert!(world.reproduce_native_robot_info(parent).unwrap().is_some());
        assert_eq!(world.tick, 0);
    }

    #[test]
    fn native_load_repopulation_waits_until_c21_and_preserves_negative_db3s_delay_and_caller_draws()
    {
        let mut input = NativeRecordFile::decode(REPRODUCTION_START).unwrap();
        input.robots.clear();
        let mut tail = NativeSimulationTail::decode(&input.tail).unwrap();
        for (field, value) in [
            ("minimum_plants", 50),
            ("repop_amount", 10),
            ("repop_cooldown", 10),
        ] {
            tail.set_value(field, NativeValue::Word(value)).unwrap();
        }
        input.tail = tail.encode().unwrap();
        let mut continuous = Engine::from_native_sim(&input.encode().unwrap()).unwrap();
        assert!(continuous.species.iter().all(|species| !species.vegetable));
        assert!(!continuous.config.vegetation.weather_enabled);
        assert!(!continuous.config.vegetation.sun_on_random);
        assert_eq!(
            continuous
                .read_native_process_caches()
                .unwrap()
                .repopulation_counter,
            Some(-10)
        );
        let mut resumed =
            crate::SaveFile::decode(&crate::SaveFile::encode(&continuous).unwrap()).unwrap();
        let initial_random = continuous.native_random.clone().unwrap();
        let mut expected_random = initial_random.clone();
        let mut expected_counter = -10;
        for cycle in 1..=31 {
            if cycle > 1 {
                expected_counter += 1;
                if expected_counter >= 10 {
                    expected_counter -= 10;
                }
            }
            // C1 is the loaded-census guard. The 20th eligible call is C21,
            // not C11; no eligible species still evaluates two caller Randoms
            // for each of the ten requested plants (Vegs28-36/Globals338ff).
            if cycle == 21 || cycle == 31 {
                for _ in 0..20 {
                    expected_random.unit();
                }
            }
            for world in [&mut continuous, &mut resumed] {
                world.tick().unwrap();
                assert_eq!(world.native_cycle, Some(cycle));
                assert_eq!(world.population(), 0);
                assert_eq!(world.stats.reseeds, 0);
                assert_eq!(
                    world
                        .read_native_process_caches()
                        .unwrap()
                        .repopulation_counter,
                    Some(expected_counter)
                );
                assert_eq!(world.native_random.as_ref(), Some(&expected_random));
            }
            if cycle == 7 {
                let saved = crate::SaveFile::encode(&resumed).unwrap();
                assert_eq!(expected_counter, -4);
                let payload = resumed.db3s_v3_payload().unwrap();
                assert_eq!(
                    payload["vegetation_runtime"]["native_repopulation_counter"],
                    -4
                );
                resumed = crate::SaveFile::decode(&saved).unwrap();
                assert_eq!(crate::SaveFile::encode(&resumed).unwrap(), saved);
            }
            if cycle == 11 {
                assert_eq!(continuous.native_random.as_ref(), Some(&initial_random));
                assert_eq!(expected_counter, 0);
            }
            assert_eq!(
                crate::SaveFile::encode(&continuous).unwrap(),
                crate::SaveFile::encode(&resumed).unwrap()
            );
        }
    }

    #[test]
    fn loaded_first_frame_retains_the_request_and_checkpoint_until_next_birth_attempt() {
        let mut world = Engine::from_native_sim(REPRODUCTION_START).unwrap();
        assert_eq!(
            world.native_loaded_census_stage,
            Some(NativeLoadedCensusStage::Pending)
        );
        world = crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        world.tick().unwrap();
        assert_eq!(
            world.native_loaded_census_stage,
            Some(NativeLoadedCensusStage::FirstCycle)
        );
        assert_eq!(world.population(), 1);
        let parent = world.snapshot().organisms[0].id;
        assert_eq!(world.memory_at(parent, 300).unwrap(), 50);
        assert_eq!(world.organism(parent).unwrap().age, 1);
        let mut restored =
            crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(
            restored.native_loaded_census_stage,
            world.native_loaded_census_stage
        );
        world.tick().unwrap();
        restored.tick().unwrap();
        assert!(world.population() > 1);
        assert_eq!(world.native_loaded_census_stage, None);
        assert_eq!(
            crate::SaveFile::encode(&restored).unwrap(),
            crate::SaveFile::encode(&world).unwrap()
        );
    }

    #[test]
    fn bare_typed_modern_and_new_native_starts_keep_immediate_reproduction() {
        for kind in 0..3 {
            let mut world = if kind == 1 {
                Engine::new_native(EngineConfig::testing()).unwrap()
            } else {
                Engine::new(EngineConfig::testing()).unwrap()
            };
            if kind == 2 {
                world.native_random = None;
            }
            assert_eq!(world.native_loaded_census_stage, None);
            world
                .spawn_at(
                    LegacyDna::parse("cond *.robage 0 = start 50 .repro store stop").unwrap(),
                    [500.0; 2],
                )
                .unwrap();
            world.tick().unwrap();
            assert!(world.population() > 1);
        }
    }

    #[test]
    fn loaded_guard_keeps_eager_animal_and_mixed_command_random_draws() {
        for vegetable in [false, true] {
            for (mixed, sexual) in [(false, false), (true, false), (false, true)] {
                let mut world = Engine::from_native_sim(REPRODUCTION_START).unwrap();
                world.native_loaded_census_stage = Some(NativeLoadedCensusStage::FirstCycle);
                let parent = world.snapshot().organisms[0].id;
                let species = world.slots.get(parent).unwrap().identity.species.0 as usize;
                world.species[species].vegetable = vegetable;
                let robot = world.slots.get_mut(parent).unwrap();
                robot.flags.vegetable = vegetable;
                robot.program.memory.write(300, 50);
                let mut expected_random = world.native_random.clone().unwrap();
                if mixed {
                    expected_random.unit();
                }
                if !vegetable {
                    expected_random.integer(0, if sexual { 9 } else { 10 });
                }
                let mut queues = LifecycleQueues::default();
                queues.births.push(PendingBirth {
                    parent,
                    percentage: 50,
                    asexual_command: 50,
                    mutated_command: if mixed { 25 } else { 0 },
                    mutation_reproduction: mixed,
                    assisted: false,
                    sexual,
                    sperm_source: None,
                });
                world
                    .commit_lifecycle_queues(queues, &mut CycleRecorder::disabled(1))
                    .unwrap();
                assert_eq!(world.native_random.as_ref().unwrap(), &expected_random);
                assert_eq!(world.population(), 1);
                assert_eq!(world.memory_at(parent, 300).unwrap(), 50);
            }
        }
    }

    #[test]
    fn loaded_guard_skips_first_repopulation_cooldown_but_refreshes_chloroplast_census() {
        let mut world = Engine::from_native_sim(REPRODUCTION_START).unwrap();
        world.native_loaded_census_stage = Some(NativeLoadedCensusStage::FirstCycle);
        world.config.vegetation.minimum_chloroplast_equivalents = 1;
        world.config.vegetation.repopulation_cooldown = 1_000;
        let before = serde_json::to_value(&world.vegetation_runtime).unwrap();
        let loaded_counter = world
            .vegetation_runtime
            .native_repopulation_counter()
            .unwrap();
        assert_eq!(loaded_counter, -10);
        world.vegetation_repopulation_phase().unwrap();
        let blocked = serde_json::to_value(&world.vegetation_runtime).unwrap();
        assert_eq!(
            blocked["repopulation_counter"],
            before["repopulation_counter"]
        );
        assert_eq!(blocked["native_repopulation_counter"], loaded_counter);
        assert_eq!(world.vegetation_runtime.native_chloroplast_population(), 0);
        world.native_loaded_census_stage = None;
        world.vegetation_repopulation_phase().unwrap();
        let next = serde_json::to_value(&world.vegetation_runtime).unwrap();
        assert_eq!(
            next["native_repopulation_counter"],
            serde_json::json!(loaded_counter + 1)
        );
        assert_eq!(next["repopulation_counter"], before["repopulation_counter"]);
    }

    #[test]
    fn loaded_stage_rolls_back_and_ordinary_starts_clear_it() {
        let mut world = Engine::from_native_sim(REPRODUCTION_START).unwrap();
        let checkpoint = world.cycle_checkpoint();
        world.advance_native_loaded_census();
        assert!(world.native_loaded_census_blocks_births());
        world.restore_cycle_checkpoint(checkpoint);
        assert_eq!(
            world.native_loaded_census_stage,
            Some(NativeLoadedCensusStage::Pending)
        );
        for boundary in [
            native_random_world::NativeStartBoundary::ConfiguredStart,
            native_random_world::NativeStartBoundary::SameProcessRound,
            native_random_world::NativeStartBoundary::ExternalProcess,
        ] {
            world.native_loaded_census_stage = Some(NativeLoadedCensusStage::FirstCycle);
            world.prepare_native_restart(boundary);
            assert_eq!(world.native_loaded_census_stage, None);
        }
    }

    #[test]
    fn absent_stage_keeps_older_checkpoints_and_invalid_stages_reject() {
        let world = Engine::new_native(EngineConfig::testing()).unwrap();
        let mut old = serde_json::to_value(&world).unwrap();
        assert!(old.get("native_loaded_census_stage").is_none());
        let restored = Engine::restore(serde_json::from_value(old.clone()).unwrap()).unwrap();
        assert_eq!(restored.native_loaded_census_stage, None);
        old["native_loaded_census_stage"] = serde_json::json!("invalid");
        assert!(serde_json::from_value::<Engine>(old).is_err());
    }
}
