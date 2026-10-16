//! Recognition noise from Senses.lookoccurr and Ties.ReadTRefVars.
use super::*;

impl Engine {
    pub fn update_recognition_fudging(&mut self, mode: u8) -> Result<(), EngineError> {
        if mode > 2 {
            return Err(EngineError::Invariant(
                "recognition fudging must be 0, 1 or 2".into(),
            ));
        }
        self.config.recognition_fudging = mode;
        self.publish_snapshot();
        Ok(())
    }

    pub(super) fn prepare_native_reference_values(
        &mut self,
        observer: OrganismId,
        target: OrganismId,
        values: &mut [(u16, i32)],
        tied: bool,
    ) {
        if self.native_random.is_none() {
            return;
        }
        let source = self.slots.get(observer).unwrap();
        let other = self.slots.get(target).unwrap();
        let location = source.program.memory.read(if tied {
            MEM_TIE_MEMORY_LOCATION
        } else {
            MEM_REF_MEMORY_LOCATION
        });
        // ReadTRefVars checks the old trefaim rather than tmemloc for the eye
        // marker. Preserve that source quirk, including its valid-location guard.
        let eye_location = if tied {
            source.program.memory.read(MEM_TREF_AIM)
        } else {
            location
        };
        let viewed = (1..=1000).contains(&location) && (501..510).contains(&eye_location);
        // fixcam enables noise for F1 and restart modes other than 0, 1 and 9.
        // Simple survival is restart 4/5 and DOES enable it. League seeding
        // (restart 1, F1=False) and zerobot testing (restart 9) do not.
        let eligible =
            self.native_f1_enabled()
                || self.local_mode.config.as_ref().is_some_and(|config| {
                    (config.workflow != crate::LeagueWorkflow::PopulationSeeding
                        || self.classic_competitive_world())
                        && (config.workflow != crate::LeagueWorkflow::Seeding
                            || self.native_cycle.is_some())
                        && !self.local_mode.evolution.as_ref().is_some_and(|state| {
                            state.phase == crate::EvolutionPhase::ZerobotTesting
                        })
                });
        let mode = self.config.recognition_fudging;
        let fudge = eligible
            && mode != 0
            && !other.flags.vegetable
            && self.graph_species_name(source) != self.graph_species_name(other);
        if viewed {
            self.slots.get_mut(target).unwrap().flags.view = true;
        }
        if !fudge {
            return;
        }
        let random = self.native_random.as_mut().unwrap();
        let mut adjust = |address: u16, census: bool| {
            if let Some((_, value)) = values.iter_mut().find(|(cell, _)| *cell == address)
                && (census || *value != 0)
            {
                let draw = random.integer(0, 1);
                *value = if census && *value < 2 {
                    draw + 1
                } else {
                    *value + draw * 2 - 1
                };
            }
        };
        // The eye count precedes all other census draws in both native paths.
        adjust(if tied { 463 } else { 708 }, true);
        if mode == 2 {
            for cell in if tied { 456..=462 } else { 701..=707 } {
                adjust(cell, true);
            }
            for cell in if tied {
                420..430
            } else {
                MEM_IN_BASE as u16..(MEM_IN_BASE + 10) as u16
            } {
                adjust(cell, false);
            }
            if !tied {
                adjust(712, true);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EvolutionConfig, EvolutionGenome, EvolutionMode, EvolutionPhase, EvolutionState, SaveFile,
    };

    const CENSUS: [i32; 9] = [0, 1, 2, 3, 4, 32001, 9, 0, 1];
    const OUTPUTS: [i16; 10] = [32002, 0, -32002, 1, 0, 0, 0, 32765, 0, -32765];

    fn world(mode: u8) -> (Engine, OrganismId, OrganismId) {
        let mut world = Engine::new(EngineConfig {
            recognition_fudging: mode,
            metabolism_cost: 0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let observer = world.spawn_at(dna.clone(), [200.0, 500.0]).unwrap();
        let target = world.spawn_at(dna, [400.0, 500.0]).unwrap();
        world.slots.get_mut(observer).unwrap().identity.species_file = "observer.txt".into();
        let other = world.slots.get_mut(target).unwrap();
        other.identity.species_file = "opponent.txt".into();
        other.senses.occurrence_counts[..CENSUS.len()].copy_from_slice(&CENSUS);
        for (index, value) in CENSUS.into_iter().enumerate() {
            other.senses.vision_reference_occurrences[index] = value as u16;
        }
        for (index, value) in OUTPUTS.into_iter().enumerate() {
            other
                .program
                .memory
                .restore_native_cell(MEM_OUT_BASE + index as i32, value);
            other
                .program
                .memory
                .restore_native_cell(410 + index as i32, value);
        }
        // Spawn initialization also uses native randomness; isolate reference reads.
        world.native_random = Some(crate::native_random::NativeRandom::default());
        world.local_mode.config = Some(LocalModeConfig::default());
        world.local_mode.status = LocalModeStatus::Running;
        (world, observer, target)
    }

    fn values(
        world: &mut Engine,
        observer: OrganismId,
        target: OrganismId,
        tied: bool,
    ) -> Vec<(u16, i32)> {
        if tied {
            world.tie_reference_values(observer, target).unwrap()
        } else {
            let mut values = native_sensing::robot_reference_values(
                world.slots.get(observer).unwrap(),
                world.slots.get(target).unwrap(),
                world.robot_fixing_allowed,
                true,
            );
            world.prepare_native_reference_values(observer, target, &mut values, false);
            values
        }
    }

    fn at(values: &[(u16, i32)], address: u16) -> i32 {
        values.iter().find(|(cell, _)| *cell == address).unwrap().1
    }

    fn evolution(phase: EvolutionPhase) -> EvolutionState {
        let mut state = EvolutionState::new(
            EvolutionConfig {
                mode: EvolutionMode::Zerobot,
                ..EvolutionConfig::default()
            },
            vec![EvolutionGenome {
                dna: LegacyDna::parse("start stop").unwrap(),
                mutation_profile: StandardMutationProfile::default(),
            }],
        )
        .unwrap();
        state.phase = phase;
        state
    }

    #[test]
    fn no_fudging_preserves_every_reference_word_and_consumes_no_draws() {
        for tied in [false, true] {
            let (mut world, observer, target) = world(0);
            let before = world.native_random.clone();
            let read = values(&mut world, observer, target, tied);
            assert_eq!(world.native_random, before);
            let census_base = if tied { 456 } else { 701 };
            for (index, expected) in CENSUS.into_iter().take(8).enumerate() {
                assert_eq!(at(&read, census_base + index as u16), expected);
            }
            let input_base = if tied { 420 } else { MEM_IN_BASE as u16 };
            for (index, expected) in OUTPUTS.into_iter().enumerate() {
                assert_eq!(at(&read, input_base + index as u16), i32::from(expected));
            }
        }
    }

    #[test]
    fn filename_equality_and_vegetable_status_exclude_noise_without_random_draws() {
        for tied in [false, true] {
            for vegetable in [false, true] {
                let (mut world, observer, target) = world(2);
                let other = world.slots.get_mut(target).unwrap();
                if vegetable {
                    other.flags.vegetable = true;
                } else {
                    other.identity.species_file = "observer.txt".into();
                    // FName, rather than the numeric species ID, defines kinship.
                    other.identity.species = SpeciesId(99);
                }
                let before = world.native_random.clone();
                let read = values(&mut world, observer, target, tied);
                assert_eq!(world.native_random, before);
                assert_eq!(at(&read, if tied { 463 } else { 708 }), CENSUS[7]);
                assert_eq!(
                    at(&read, if tied { 420 } else { MEM_IN_BASE as u16 }),
                    32002
                );
            }
        }
    }

    #[test]
    fn eyes_only_changes_one_census_cell_with_one_shared_draw() {
        for tied in [false, true] {
            let (mut world, observer, target) = world(1);
            // Both robots have the same species ID but distinct source FNames.
            assert_eq!(
                world.slots.get(observer).unwrap().identity.species,
                world.slots.get(target).unwrap().identity.species
            );
            let mut expected_random = world.native_random.clone().unwrap();
            let expected_eye = expected_random.integer(0, 1) + 1;
            let read = values(&mut world, observer, target, tied);
            assert_eq!(at(&read, if tied { 463 } else { 708 }), expected_eye);
            assert_eq!(at(&read, if tied { 456 } else { 701 }), CENSUS[0]);
            assert_eq!(
                at(&read, if tied { 420 } else { MEM_IN_BASE as u16 }),
                32002
            );
            if !tied {
                assert_eq!(at(&read, 712), CENSUS[8]);
            }
            assert_eq!(world.native_random.as_ref(), Some(&expected_random));
        }
    }

    #[test]
    fn all_optical_noise_draws_eyes_then_census_then_nonzero_inputs_then_ties() {
        let (mut world, observer, target) = world(2);
        let mut expected_random = world.native_random.clone().unwrap();
        let draw_order = [
            708, 701, 702, 703, 704, 705, 706, 707, 810, 812, 813, 817, 819, 712,
        ];
        let baseline = native_sensing::robot_reference_values(
            world.slots.get(observer).unwrap(),
            world.slots.get(target).unwrap(),
            true,
            true,
        );
        let mut expected = BTreeMap::new();
        for address in draw_order {
            let base = at(&baseline, address);
            let bit = expected_random.integer(0, 1);
            let census = (701..=708).contains(&address) || address == 712;
            expected.insert(
                address,
                if census && base < 2 {
                    bit + 1
                } else {
                    base + bit * 2 - 1
                },
            );
        }
        world.publish_native_contact_reference(observer, target);
        for (address, base) in baseline {
            let expected = expected.get(&address).copied().unwrap_or(base);
            assert_eq!(
                world.memory_at(observer, i32::from(address)).unwrap(),
                expected,
                "cell {address}"
            );
        }
        assert!(world.memory_at(observer, MEM_IN_BASE).unwrap() > 32000);
        assert!(world.memory_at(observer, MEM_IN_BASE + 2).unwrap() < -32000);
        assert_eq!(world.native_random.as_ref(), Some(&expected_random));
    }

    #[test]
    fn all_tie_noise_uses_the_same_stream_and_does_not_add_an_optical_tie_draw() {
        let (mut world, observer, target) = world(0);
        let baseline = world.tie_reference_values(observer, target).unwrap();
        world.config.recognition_fudging = 2;
        let mut expected_random = world.native_random.clone().unwrap();
        let draw_order = [
            463, 456, 457, 458, 459, 460, 461, 462, 420, 422, 423, 427, 429,
        ];
        let mut expected = BTreeMap::new();
        for address in draw_order {
            let base = at(&baseline, address);
            let bit = expected_random.integer(0, 1);
            expected.insert(
                address,
                if address >= 456 && base < 2 {
                    bit + 1
                } else {
                    base + bit * 2 - 1
                },
            );
        }
        let read = world.tie_reference_values(observer, target).unwrap();
        for (address, base) in baseline {
            assert_eq!(
                at(&read, address),
                expected.get(&address).copied().unwrap_or(base),
                "cell {address}"
            );
        }
        assert_eq!(world.native_random.as_ref(), Some(&expected_random));
    }

    #[test]
    fn normal_seeding_and_zerobot_testing_exclude_fudging() {
        for excluded in 0..3 {
            let (mut world, observer, target) = world(2);
            match excluded {
                0 => world.local_mode.config = None,
                1 => {
                    world.local_mode.config.as_mut().unwrap().workflow =
                        crate::LeagueWorkflow::Seeding;
                }
                _ => {
                    world.local_mode.config.as_mut().unwrap().kind = LocalModeKind::Zerobot;
                    world.local_mode.evolution = Some(evolution(EvolutionPhase::ZerobotTesting));
                }
            }
            let before = world.native_random.clone();
            for tied in [false, true] {
                let read = values(&mut world, observer, target, tied);
                assert_eq!(at(&read, if tied { 463 } else { 708 }), 0);
            }
            assert_eq!(world.native_random, before);
        }
    }

    #[test]
    fn native_filter_enables_noise_but_population_seeding_excludes_it() {
        for workflow in [
            crate::LeagueWorkflow::Seeding,
            crate::LeagueWorkflow::PopulationSeeding,
        ] {
            let (mut world, observer, target) = world(1);
            world.native_cycle = Some(0);
            world.local_mode.config.as_mut().unwrap().workflow = workflow;
            let mut expected = world.native_random.clone().unwrap();
            let eye = if workflow == crate::LeagueWorkflow::Seeding {
                expected.integer(0, 1) + 1
            } else {
                CENSUS[7]
            };
            let read = values(&mut world, observer, target, false);
            assert_eq!(at(&read, 708), eye);
            assert_eq!(world.native_random.as_ref(), Some(&expected));
        }
    }

    #[test]
    fn saved_native_f1_enables_noise_without_inventing_external_restart_settings() {
        // .sim preserves SimOpts.F1. x_fudge and x_restartmode live outside the
        // file, so the caller supplies the noise mode and absent F1 stays off.
        for native_f1 in [None, Some(false), Some(true)] {
            for tied in [false, true] {
                let (mut world, observer, target) = world(1);
                world.local_mode.config = None;
                world.native_f1 = native_f1;
                let mut expected_random = world.native_random.clone().unwrap();
                let eye = if native_f1 == Some(true) {
                    expected_random.integer(0, 1) + 1
                } else {
                    CENSUS[7]
                };
                let read = values(&mut world, observer, target, tied);
                assert_eq!(at(&read, if tied { 463 } else { 708 }), eye);
                assert_eq!(world.native_random.as_ref(), Some(&expected_random));
            }
        }
    }

    #[test]
    fn explicit_local_mode_overrides_the_imported_f1_flag() {
        for workflow in [
            crate::LeagueWorkflow::Seeding,
            crate::LeagueWorkflow::Direct,
        ] {
            let (mut world, observer, target) = world(1);
            // Native F1=True must not enable seeding; an explicit ordinary
            // league must not be disabled by imported F1=False.
            world.native_f1 = Some(workflow == crate::LeagueWorkflow::Seeding);
            world.local_mode.config.as_mut().unwrap().workflow = workflow;
            let mut expected_random = world.native_random.clone().unwrap();
            let eye = if workflow == crate::LeagueWorkflow::Seeding {
                CENSUS[7]
            } else {
                expected_random.integer(0, 1) + 1
            };
            let read = values(&mut world, observer, target, false);
            assert_eq!(at(&read, 708), eye);
            assert_eq!(world.native_random.as_ref(), Some(&expected_random));
        }
    }

    #[test]
    fn league_contest_and_survival_restarts_enable_fudging_including_simple_survival() {
        for kind in [
            LocalModeKind::League,
            LocalModeKind::Contest,
            LocalModeKind::SimpleSurvival,
            LocalModeKind::EcoSurvival,
            LocalModeKind::Zerobot,
        ] {
            let (mut world, observer, target) = world(1);
            world.local_mode.config.as_mut().unwrap().kind = kind;
            if kind == LocalModeKind::Zerobot {
                world.local_mode.evolution = Some(evolution(EvolutionPhase::Evolving));
            }
            let mut expected_random = world.native_random.clone().unwrap();
            let eye = expected_random.integer(0, 1) + 1;
            assert_eq!(at(&values(&mut world, observer, target, false), 708), eye);
            assert_eq!(world.native_random.as_ref(), Some(&expected_random));
        }
    }

    #[test]
    fn optical_readmem_marks_only_the_nine_eye_addresses_even_without_fudging() {
        let (mut world, observer, target) = world(0);
        world.local_mode.config = None;
        for location in [0, 1, 500, 501, 505, 509, 510, 1000, 1001, -1] {
            world.slots.get_mut(target).unwrap().flags.view = false;
            world
                .slots
                .get_mut(observer)
                .unwrap()
                .program
                .memory
                .write(MEM_REF_MEMORY_LOCATION, location);
            let before = world.native_random.clone();
            world.publish_native_contact_reference(observer, target);
            assert_eq!(
                world.slots.get(target).unwrap().flags.view,
                (501..=509).contains(&location),
                "location {location}"
            );
            assert_eq!(world.native_random, before);
        }
        world
            .slots
            .get_mut(observer)
            .unwrap()
            .program
            .memory
            .write(MEM_REF_MEMORY_LOCATION, 501);
        world.publish_native_contact_reference(observer, target);
        world
            .slots
            .get_mut(observer)
            .unwrap()
            .program
            .memory
            .write(MEM_REF_MEMORY_LOCATION, 500);
        world.publish_native_contact_reference(observer, target);
        assert!(world.slots.get(target).unwrap().flags.view);
    }

    #[test]
    fn tie_readmem_checks_old_trefaim_but_requires_a_valid_tmemloc() {
        let (mut world, observer, target) = world(0);
        for (location, old_aim, viewed) in [
            (0, 505, false),
            (1001, 505, false),
            (1, 500, false),
            (1, 501, true),
            (1000, 509, true),
            (505, 510, false),
            (505, 123, false),
        ] {
            world.slots.get_mut(target).unwrap().flags.view = false;
            let source = world.slots.get_mut(observer).unwrap();
            source
                .program
                .memory
                .write(MEM_TIE_MEMORY_LOCATION, location);
            source.program.memory.write(MEM_TREF_AIM, old_aim);
            // A new trefaim of 505 must not replace the old value for the test.
            world
                .slots
                .get_mut(target)
                .unwrap()
                .program
                .memory
                .write(18, 505);
            let read = world.tie_reference_values(observer, target).unwrap();
            assert_eq!(
                world.slots.get(target).unwrap().flags.view,
                viewed,
                "location {location}, old aim {old_aim}"
            );
            assert_eq!(at(&read, MEM_TREF_AIM as u16), 505);
        }
    }

    #[test]
    fn save_resume_keeps_noise_setting_and_shared_optical_tie_draw_position() {
        let (mut world, observer, target) = world(2);
        world.publish_native_contact_reference(observer, target);
        let bytes = SaveFile::encode(&world).unwrap();
        let mut resumed = SaveFile::decode(&bytes).unwrap();
        assert_eq!(resumed.config.recognition_fudging, 2);
        assert_eq!(resumed.native_random, world.native_random);
        assert_eq!(
            values(&mut resumed, observer, target, true),
            values(&mut world, observer, target, true)
        );
        resumed.publish_native_contact_reference(observer, target);
        world.publish_native_contact_reference(observer, target);
        assert_eq!(resumed.native_random, world.native_random);
        assert_eq!(
            SaveFile::encode(&resumed).unwrap(),
            SaveFile::encode(&world).unwrap()
        );
    }

    #[test]
    fn invalid_fudging_setting_does_not_change_the_active_mode_or_random_stream() {
        let (mut world, _, _) = world(1);
        let before = world.native_random.clone();
        assert!(world.update_recognition_fudging(3).is_err());
        assert_eq!(world.config.recognition_fudging, 1);
        assert_eq!(world.native_random, before);
        world.update_recognition_fudging(2).unwrap();
        assert_eq!(world.config.recognition_fudging, 2);
    }

    #[test]
    fn explicit_f1_takes_precedence_over_the_restart_nine_noise_exclusion() {
        let (mut world, observer, target) = world(1);
        world.local_mode.config.as_mut().unwrap().kind = LocalModeKind::League;
        world.local_mode.evolution = Some(evolution(EvolutionPhase::ZerobotTesting));
        let mut expected = world.native_random.clone().unwrap();
        let eye = expected.integer(0, 1) + 1;
        assert_eq!(at(&values(&mut world, observer, target, false), 708), eye);
        assert_eq!(world.native_random.as_ref(), Some(&expected));
    }
}
