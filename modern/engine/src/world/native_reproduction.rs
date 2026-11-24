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
                self.commit_pending_births(&[birth], BirthCommitRoute::NativeRobotInfo, &mut trace)?
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
    fn newborn_actual_velocity_uses_native_inheritance_then_normal_motion() {
        for native in [true, false] {
            let config = EngineConfig {
                world_width: 3_000.0,
                world_height: 2_000.0,
                ..EngineConfig::testing()
            };
            let mut world = if native {
                Engine::new_native(config).unwrap()
            } else {
                let mut world = Engine::new(config).unwrap();
                world.native_random = None;
                world
            };
            let parent = world
                .spawn_at(
                    LegacyDna::parse("cond *.robage 0 = start 50 .repro store stop").unwrap(),
                    [1_000.0, 1_000.0],
                )
                .unwrap();
            let inherited = [11.25, -7.5];
            let kinematics = &mut world.slots.get_mut(parent).unwrap().kinematics;
            kinematics.velocity = [3.0, 2.0];
            kinematics.actual_velocity = inherited;

            world.tick().unwrap();
            assert_eq!(world.population(), 2);
            let child = world
                .slots
                .ids_in_slot_order()
                .into_iter()
                .find(|&id| id != parent)
                .unwrap();
            let expected = if native { inherited } else { [0.0; 2] };
            let saved = crate::SaveFile::encode(&world).unwrap();
            let mut world = crate::SaveFile::decode(&saved).unwrap();
            let kinematics = &world.slots.get(child).unwrap().kinematics;
            assert_eq!(kinematics.actual_velocity, expected);
            let birth_position = kinematics.position;
            world
                .replace_dna(child, LegacyDna::parse("stop").unwrap())
                .unwrap();

            world.tick().unwrap();
            let kinematics = &world.slots.get(child).unwrap().kinematics;
            let displacement =
                std::array::from_fn(|axis| kinematics.position[axis] - birth_position[axis]);
            assert_ne!(displacement, inherited);
            assert_ne!(displacement, [0.0; 2]);
            assert_eq!(kinematics.actual_velocity, displacement);
        }
    }

    #[test]
    fn genuine_cycle_283_newborn_velocities_match_the_original_capture() {
        // Original 2.48.32 cycle-283 capture SHA-256
        // fd2cb2bd5e36044d459d238f561dcf60cec70bf1efcb6d1b90dde3078ad9927e.
        let input = include_bytes!("../../tests/fixtures/db24832-original-user.sim");
        let mut world = Engine::from_native_sim(input).unwrap();
        world.tick_many(2).unwrap();
        assert_eq!(world.native_cycle, Some(283));
        let file = NativeRecordFile::decode(&world.export_native_sim().unwrap()).unwrap();
        for (id, parent, actual_velocity) in [
            (295, 34, [-0.16113281_f32, -0.020996094_f32]),
            (296, 293, [45.689453, 38.664307]),
            (297, 268, [11.238281, 21.178406]),
            (298, 276, [41.725586, 25.679688]),
        ] {
            let newborn = file
                .robots
                .iter()
                .find(|record| record.long("absolute_id").unwrap() == id)
                .unwrap();
            assert_eq!(newborn.long("parent").unwrap(), parent);
            assert_eq!(
                [
                    newborn.single("actual_velocity_x").unwrap(),
                    newborn.single("actual_velocity_y").unwrap(),
                ],
                actual_velocity,
                "absolute ID {id}",
            );
        }
    }

    #[test]
    fn robot_info_can_reproduce_a_retained_corpse_after_positive_single_energy() {
        let mut world = robot_info_test_world();
        world.config.corpse_decay.enabled = true;
        world.config.dna_costs.dna_copy = 0.0;
        world.config.dna_costs.tie_formation = 0.0;
        let species = world.register_species(SpeciesDefinition {
            name: "Vegetable witness.txt".into(),
            vegetable: true,
            ..SpeciesDefinition::default()
        });
        let dna = LegacyDna::parse("start stop").unwrap();
        let parent = world
            .spawn_species_at(dna.clone(), species, [1_000.0; 2])
            .unwrap();
        world
            .spawn_species_at(dna, species, [5_000.0, 3_000.0])
            .unwrap();
        world.set_organism_aim(parent, 0).unwrap();
        world.slots.get_mut(parent).unwrap().materials.energy = 1.5;
        world.tick().unwrap();
        let source = world.slots.get(parent).unwrap();
        assert!(source.flags.exists && source.flags.corpse && source.flags.can_reproduce);
        assert!(!source.flags.vegetable);
        assert!(world.species[source.identity.species.0 as usize].vegetable);
        assert_eq!(source.materials.energy, 0.0);
        let (tick, cycle, age, absolute_id) = (
            world.tick,
            world.native_cycle,
            source.counters.age,
            source.identity.absolute_id,
        );
        let deaths = world.stats.deaths;
        world
            .vegetation_runtime
            .set_native_chloroplast_population(world.config.vegetable_population_cap + 1);

        let mut expected_random = world.native_random.clone().unwrap();
        expected_random.integer(0, 10);
        assert_eq!(world.reproduce_native_robot_info(parent).unwrap(), None);
        assert_eq!(world.native_random.as_ref(), Some(&expected_random));
        assert_eq!(world.stats.births, 0);
        assert!(world.ties.is_empty());

        // Console energy writes the raw Single reserve.
        world.slots.get_mut(parent).unwrap().materials.energy = 1_000.5;
        world.asexual_reproduction_allowed = false;
        assert_eq!(world.reproduce_native_robot_info(parent).unwrap(), None);
        assert_eq!(world.native_random.as_ref(), Some(&expected_random));
        world.asexual_reproduction_allowed = true;
        expected_random.integer(0, 10);
        expected_random.integer(2, 92);

        let child = world.reproduce_native_robot_info(parent).unwrap().unwrap();
        assert_eq!((world.tick, world.native_cycle), (tick, cycle));
        assert_eq!(world.native_random.as_ref(), Some(&expected_random));
        assert_eq!(world.stats.births, 1);
        assert_eq!(world.stats.deaths, deaths);
        assert_eq!(world.stats.self_reproductions, 0);
        let source = world.slots.get(parent).unwrap();
        assert!(source.flags.corpse && source.flags.exists);
        assert_eq!(source.counters.age, age);
        assert_eq!(source.counters.offspring, 1);
        assert_eq!(source.materials.body, 500.0);

        let newborn = world.slots.get(child).unwrap();
        assert!(newborn.flags.exists && !newborn.flags.corpse && !newborn.flags.dead);
        assert!(!newborn.flags.vegetable && !newborn.flags.can_execute_dna);
        assert!(!newborn.flags.can_move && !newborn.flags.can_see && newborn.flags.virus_immune);
        assert!(newborn.flags.can_reproduce);
        assert_eq!(newborn.identity.parents, [Some(parent), None]);
        assert_eq!(newborn.identity.parent_absolute_id, absolute_id);
        assert_eq!(newborn.identity.species_file, "Corpse");
        assert_eq!(newborn.counters.age, 0);
        assert_eq!(newborn.counters.birth_cycle, cycle.unwrap() as u64);
        assert_eq!(newborn.materials.body, 500.0);
        assert_eq!(newborn.materials.energy, 499.749_76);
        assert_eq!(world.ties.len(), 1);
        assert_eq!(
            (
                world.ties[0].first,
                world.ties[0].second,
                world.ties[0].last
            ),
            (parent, child, 100)
        );
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
    fn native_reproduction_draw_precedes_energy_check_and_follows_body_check() {
        for sexual in [false, true] {
            for (body, energy) in [(4.5, 100.0), (5.0, 0.0), (5.0, -1.0)] {
                let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
                let dna = LegacyDna::parse("start stop").unwrap();
                let parent = world.spawn_at(dna.clone(), [500.0; 2]).unwrap();
                let address = if sexual { 302 } else { 300 };
                let robot = world.slots.get_mut(parent).unwrap();
                robot.materials.body = body;
                robot.materials.energy = energy;
                robot.program.memory.write(address, 50);
                if sexual {
                    robot.effects.fertilization_timer = 1;
                    robot.effects.stored_sperm = Some(StoredGeneticMaterial { dna, source: None });
                }
                let materials = robot.materials.clone();
                let births = world.stats.births;
                let mut expected_random = world.native_random.clone().unwrap();
                if body >= 5.0 {
                    expected_random.integer(0, if sexual { 9 } else { 10 });
                }
                let mut trace = CycleRecorder::disabled(1);
                let queues = world.prepare_lifecycle_queues(&mut trace);
                assert_eq!(queues.births.len(), 1);
                world.commit_lifecycle_queues(queues, &mut trace).unwrap();

                assert_eq!(world.native_random.as_ref(), Some(&expected_random));
                assert_eq!(world.stats.births, births);
                assert_eq!(world.population(), 1);
                assert_eq!(world.slots.get(parent).unwrap().materials, materials);
                assert_eq!(world.memory_at(parent, address).unwrap(), 50);
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
        world.vegetation_repopulation_phase(None).unwrap();
        let blocked = serde_json::to_value(&world.vegetation_runtime).unwrap();
        assert_eq!(
            blocked["repopulation_counter"],
            before["repopulation_counter"]
        );
        assert_eq!(blocked["native_repopulation_counter"], loaded_counter);
        assert_eq!(world.vegetation_runtime.native_chloroplast_population(), 0);
        world.native_loaded_census_stage = None;
        world.vegetation_repopulation_phase(None).unwrap();
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
    fn native_births_reuse_lowest_free_robot_slots_across_db3s_resume() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        world.mutations_enabled = false;
        let dna = LegacyDna::parse("stop").unwrap();
        let holes_and_parents = world
            .spawn_batch(
                [[70.0, 70.0], [250.0, 250.0], [900.0, 650.0], [650.0, 650.0]]
                    .into_iter()
                    .map(|position| (dna.clone(), position)),
            )
            .unwrap();
        let [lower, first_parent, higher, second_parent] = holes_and_parents.as_slice() else {
            unreachable!();
        };
        let holes = [*lower, *higher];
        let parents = [*first_parent, *second_parent];
        world.remove_many_with_death_poffs(&holes, true).unwrap();
        assert_eq!(world.slots.free_slots, [lower.slot(), higher.slot()]);
        let next_absolute_id = world.next_absolute_id;
        let mut restored =
            crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(restored.slots.free_slots, world.slots.free_slots);

        for world in [&mut world, &mut restored] {
            let births = parents
                .into_iter()
                .map(|parent| PendingBirth {
                    parent,
                    percentage: 50,
                    asexual_command: 50,
                    mutated_command: 0,
                    mutation_reproduction: false,
                    assisted: false,
                    sexual: false,
                    sperm_source: None,
                })
                .collect();
            world
                .commit_lifecycle_queues(
                    LifecycleQueues {
                        births,
                        deaths: Vec::new(),
                    },
                    &mut CycleRecorder::disabled(0),
                )
                .unwrap();

            for (index, (previous, parent)) in holes.into_iter().zip(parents).enumerate() {
                let child = OrganismId::new(previous.slot(), previous.generation() + 1);
                let identity = &world.slots.get(child).unwrap().identity;
                assert_eq!(identity.absolute_id, next_absolute_id + index as u64);
                assert_eq!(identity.parents[0], Some(parent));
                assert!(matches!(
                    world.slots.get(previous),
                    Err(EngineError::StaleOrganismId)
                ));
            }
            assert!(world.slots.free_slots.is_empty());
            assert_eq!(world.next_absolute_id, next_absolute_id + 2);
            world.publish_snapshot();
        }
        assert_eq!(
            crate::SaveFile::encode(&world).unwrap(),
            crate::SaveFile::encode(&restored).unwrap()
        );

        let native =
            crate::native_binary::NativeRecordFile::decode(&world.export_native_sim().unwrap())
                .unwrap();
        for (index, previous) in holes.into_iter().enumerate() {
            let absolute = (next_absolute_id + index as u64) as i32;
            let record = native
                .robots
                .iter()
                .find(|record| record.long("absolute_id").unwrap() == absolute)
                .unwrap();
            assert_eq!(record.word("old_slot").unwrap(), previous.slot() as i16 + 1);
        }
    }

    #[test]
    fn portable_robot_spawn_retains_lifo_order_across_db3s_resume() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        world.native_random = None;
        let dna = LegacyDna::parse("stop").unwrap();
        let first = world.spawn(dna.clone()).unwrap();
        let last = world.spawn(dna.clone()).unwrap();
        world.remove_many(&[first, last]).unwrap();
        let mut restored =
            crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        assert!(restored.native_random.is_none() && restored.native_cycle.is_none());

        assert_eq!(restored.spawn(dna.clone()).unwrap().slot(), last.slot());
        assert_eq!(restored.spawn(dna).unwrap().slot(), first.slot());
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
