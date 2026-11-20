//! startloaded's totvegs=-1, then UpdateBots' first displayed census.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum NativeLoadedCensusStage {
    Pending,
    FirstCycle,
}

impl Engine {
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
